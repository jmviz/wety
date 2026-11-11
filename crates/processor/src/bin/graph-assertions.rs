// Run from workspace root.
//
// Writes the assertions a processed graph holds, Wiktionary's and wety's rules',
// in the assertion schema (packages/etymology/schema), as JSON Lines: each
// etymology is a hypothesis about its word (one link), asserted by Wiktionary's
// editors on the page whose section states it, or compiled by a wety rule; a
// word's alternatives form an exclusive alternative set. Ids are digests as
// `etymology.assertions` computes them, so that module validates the output.
// Works are written as the bibliography the build read gives them.
//
// cargo run --release --bin graph-assertions -- GRAPH RECEIPT OUT.jsonl.gz \
//     --counts counts.json [--every 1000]

#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

use wety_core::{
    HashSet,
    assertions::{self, Asserted, Work, digest},
    ety_graph::{Certainty, EdgeBasis, EtyEdgeAccess, EtyKind, LanguageOnly, SourceLang},
    etymology_templates::EtyMode,
    items::ItemId,
    processed::Data,
};

use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::Parser;
use flate2::{Compression, write::GzEncoder};
use itertools::Itertools;
use serde_json::{Value, json};

const SOURCE_NAME: &str = "wiktionary";
const PROFILE: &str = "data/workspaces/etymology/sources/wiktionary/etc/source.json";
const EDITORS: &str = "agent:wiktionary-editors";

#[derive(Parser)]
struct Args {
    /// The processed graph (wety.json or wety.json.gz)
    graph: PathBuf,
    /// The run receipt of the build that wrote the graph
    receipt: PathBuf,
    /// Where to write the assertions (gzipped if it ends in .gz)
    out: PathBuf,
    /// Where to write counts and sizes
    #[arg(long)]
    counts: Option<PathBuf>,
    /// Write only every nth word's assertions, with what they refer to: a
    /// sample small enough to validate
    #[arg(long)]
    every: Option<usize>,
}

struct Writer {
    out: Box<dyn Write>,
    written: HashSet<String>,
}

impl Writer {
    fn add(&mut self, object: Value) -> Result<String> {
        let id = object["id"].as_str().context("an id")?.to_owned();
        if self.written.insert(id.clone()) {
            writeln!(self.out, "{object}")?;
        }
        Ok(id)
    }
}

// The source object for the Wiktionary profile, as etymology.convert.source.
fn source() -> Result<Value> {
    let p: Value = serde_json::from_reader(File::open(PROFILE)?)?;
    let mut s = json!({
        "type": "source",
        "id": format!("{SOURCE_NAME}@{}", p["release"].as_str().context("release")?),
        "title": p["title"],
        "licence": p["licence"],
        "may_display": p["may_display"],
        "may_export": p["may_export"],
        "profile": PROFILE,
    });
    for key in ["doi", "url", "published"] {
        if let Some(v) = p.get(key).filter(|v| !v.is_null()) {
            s[key] = v.clone();
        }
    }
    Ok(s)
}

// The build's extraction record, from its run receipt: the processor at the
// revision the graph was built at, over the inputs it read.
fn extraction(receipt: &Path) -> Result<Value> {
    let r: Value = serde_json::from_reader(File::open(receipt)?)?;
    let inputs = r["inputs"]
        .as_object()
        .context("receipt inputs")?
        .values()
        .map(|i| json!({"path": i["path"], "sha256": i["sha256"]}))
        .sorted_by_key(|i| i["path"].as_str().unwrap_or_default().to_owned())
        .collect_vec();
    let body = json!({
        "method": "parser",
        "tool": "processor",
        "revision": r["git_revision"],
        "dirty": r["git_dirty"],
        "inputs": inputs,
    });
    let mut e = body.clone();
    e["type"] = json!("extraction");
    e["id"] = json!(format!("extraction:{}", digest(&body)));
    Ok(e)
}

// The bibliography the build read (its run receipt names it): each work's
// full object, with the agents it names.
#[derive(Default)]
struct Bibliography {
    objects: HashMap<String, Value>,
}

impl Bibliography {
    fn read(receipt: &Path) -> Result<Self> {
        let r: Value = serde_json::from_reader(File::open(receipt)?)?;
        let mut bib = Self::default();
        let Some(path) = r["inputs"]["bibliography"]["path"].as_str() else {
            return Ok(bib);
        };
        for line in BufReader::new(File::open(path).context(path.to_owned())?).lines() {
            let object: Value = serde_json::from_str(&line?)?;
            if matches!(object["type"].as_str(), Some("work" | "agent")) {
                let id = object["id"].as_str().context("an id")?.to_owned();
                bib.objects.insert(id, object);
            }
        }
        Ok(bib)
    }

    // A work as the bibliography gives it, else as the graph cites it.
    fn write(&self, out: &mut Writer, work: &Work) -> Result<()> {
        let Some(object) = self.objects.get(&*work.id) else {
            let mut o = json!({"type": "work", "id": work.id, "citation": work.citation});
            if let Some(year) = work.year {
                o["year"] = json!(year);
            }
            out.add(o)?;
            return Ok(());
        };
        for key in ["authors", "editors"] {
            for agent in object[key].as_array().into_iter().flatten() {
                let agent = agent.as_str().context("an agent id")?;
                out.add(self.objects.get(agent).context("an agent")?.clone())?;
            }
        }
        out.add(object.clone())?;
        Ok(())
    }
}

struct Export<'a> {
    data: &'a Data,
    source: String,
    extraction: String,
    work: String,
    counts: BTreeMap<String, u64>,
}

impl Export<'_> {
    fn count(&mut self, key: impl Into<String>) {
        *self.counts.entry(key.into()).or_default() += 1;
    }

    fn record_id(&self, item: ItemId) -> String {
        let i = self.data.graph.item(item);
        format!(
            "{SOURCE_NAME}:{}:{}#{}{}",
            i.lang().code(),
            i.term().resolve(&self.data.string_pool),
            i.ety_num(),
            if i.is_imputed() { "?" } else { "" }
        )
    }

    // A word as Wiktionary writes it. An imputed item is a term cited but
    // missing from the data; it is a record all the same.
    fn record(&self, out: &mut Writer, item: ItemId) -> Result<String> {
        let i = self.data.graph.item(item);
        let lang = i.lang();
        let mut r = json!({
            "type": "record",
            "id": self.record_id(item),
            "source": self.source,
            "language": {"label": lang.name(), "code": lang.code(), "relation": "exact"},
            "form": i.term().resolve(&self.data.string_pool),
            "reconstructed": lang.is_reconstructed(),
        });
        if i.ety_num() > 1 || i.ety_label().is_some() {
            r["homograph"] = json!(
                i.ety_label()
                    .map_or_else(|| format!("Etymology {}", i.ety_num()), str::to_owned)
            );
        }
        out.add(r)
    }

    // One etymology as a hypothesis about its word: a single link.
    fn hypothesis(
        &self,
        out: &mut Writer,
        item: ItemId,
        parents: &[(ItemId, EtyMode, bool)],
    ) -> Result<String> {
        let word = self.record(out, item)?;
        let link = match parents {
            [] => json!({"child": word, "parents": [], "relation": "unknown"}),
            [(_, EtyMode::Root, _), ..] => json!({
                "child": word,
                "parents": parents.iter().map(|&(p, _, _)| self.record_id(p)).collect_vec(),
                "relation": "ultimate",
            }),
            [(_, mode, _), ..] => {
                let mut link = json!({
                    "child": word,
                    "parents": parents.iter().map(|&(p, _, _)| self.record_id(p)).collect_vec(),
                    "relation": "derivation",
                    "mode": mode.as_ref(),
                });
                if let Some(head) = parents.iter().position(|&(_, _, head)| head) {
                    link["head"] = json!(head);
                }
                link
            }
        };
        for &(p, _, _) in parents {
            self.record(out, p)?;
        }
        let links = json!([link]);
        let id = format!("hyp:{}", digest(&json!([word, links])));
        out.add(json!({"type": "hypothesis", "id": id, "word": word, "links": links}))
    }

    // An etymology naming a language or family but no word: a link to a
    // record of that language with no form.
    fn language_only_hypothesis(
        &self,
        out: &mut Writer,
        item: ItemId,
        origin: LanguageOnly,
    ) -> Result<String> {
        let word = self.record(out, item)?;
        let (code, label, reconstructed) = match origin.source {
            SourceLang::Lang(lang) => {
                (lang.code(), lang.name().to_owned(), lang.is_reconstructed())
            }
            SourceLang::Family(family) => (family.code(), family.name(), false),
        };
        let parent = out.add(json!({
            "type": "record",
            "id": format!("{SOURCE_NAME}:{code}:"),
            "source": self.source,
            "language": {"label": label, "code": code, "relation": "exact"},
            "form": null,
            "reconstructed": reconstructed,
        }))?;
        let links = json!([{
            "child": word,
            "parents": [parent],
            "relation": "derivation",
            "mode": origin.mode.as_ref(),
        }]);
        let id = format!("hyp:{}", digest(&json!([word, links])));
        out.add(json!({"type": "hypothesis", "id": id, "word": word, "links": links}))
    }

    #[allow(clippy::too_many_arguments)]
    fn assertion(
        &mut self,
        out: &mut Writer,
        item: ItemId,
        basis: EdgeBasis,
        target: &str,
        asserted: Option<&Asserted>,
        certainty: Certainty,
        in_set: Option<&str>,
    ) -> Result<()> {
        let page = asserted.and_then(|a| a.page).unwrap_or(item);
        let locator = format!("{}/{}", self.record_id(page), assertions::section(basis));
        let stance = "endorses";
        let id = format!(
            "as:{}",
            digest(&json!([self.source, locator, target, stance]))
        );
        let rule = assertions::rule(basis);
        let asserter = rule.map_or_else(|| EDITORS.to_owned(), |r| format!("agent:{r}"));
        let mut a = json!({
            "type": "assertion",
            "id": id,
            "source": self.source,
            "asserter": asserter,
            "holder": asserter,
            "stance": stance,
            "target": target,
            "locator": locator,
            "extraction": self.extraction,
        });
        if let Some(set) = in_set {
            a["in_sets"] = json!([set]);
        }
        if certainty != Certainty::Stated {
            a["hedges"] = json!([{"level": certainty.as_ref(), "words": "<unc>", "link": 0}]);
        }
        let mut credits = vec![];
        for c in asserted.map(|a| &a.credits[..]).unwrap_or_default() {
            let mut credit = json!({
                "work": self.data.graph.works[c.work as usize].id,
                "role": c.role.as_ref(),
                "scope": c.scope.as_ref(),
            });
            if let Some(locator) = &c.locator {
                credit["locator"] = json!(locator);
            }
            self.count(format!(
                "credits: {} ({})",
                c.scope.as_ref(),
                c.role.as_ref()
            ));
            credits.push(credit);
        }
        for scope in asserted
            .map(|a| a.credits.iter().map(|c| c.scope).unique().collect_vec())
            .unwrap_or_default()
        {
            self.count(format!("assertions credited: {}", scope.as_ref()));
        }
        if let Some(rule) = rule {
            // a rule credits what it read: the source's own structure
            let words = assertions::compiled_words(asserted.map_or(0, |a| a.inputs));
            a["compiled"] = json!({"from": locator, "words": words});
            credits.push(json!({"work": self.work, "role": "follows", "scope": "entry"}));
            self.count(format!("assertions by {rule}"));
        } else {
            self.count(format!(
                "assertions by Wiktionary's editors ({})",
                basis.as_ref()
            ));
        }
        if !credits.is_empty() {
            a["credits"] = json!(credits);
        }
        if certainty != Certainty::Stated {
            self.count(format!("assertions hedged {}", certainty.as_ref()));
        }
        if page != item {
            self.count(format!(
                "assertions stated on another page ({})",
                basis.as_ref()
            ));
        }
        out.add(a)?;
        Ok(())
    }

    // An item's etymologies: each a hypothesis, asserted; several, an
    // exclusive alternative set, asserted too.
    fn word(&mut self, out: &mut Writer, item: ItemId) -> Result<()> {
        let graph = &self.data.graph;
        let etymologies = graph.etymologies(item);
        if etymologies.is_some_and(|e| e.kinds[..] == [EtyKind::Unknown]) {
            let target = self.hypothesis(out, item, &[])?;
            self.count("words of unknown origin");
            let asserted = graph.asserted(item, 0).cloned();
            return self.assertion(
                out,
                item,
                EdgeBasis::Template,
                &target,
                asserted.as_ref(),
                Certainty::Stated,
                None,
            );
        }
        let mut by_ety: BTreeMap<u8, Vec<_>> = BTreeMap::new();
        // an etymology only another source gives is that source's assertion
        for e in graph
            .all_parent_edges(item)
            .filter(|e| e.basis() != EdgeBasis::Source)
            .sorted_by_key(|e| e.order())
        {
            by_ety.entry(e.ety()).or_default().push(e);
        }
        let language_only = etymologies
            .into_iter()
            .flat_map(|e| e.kinds.iter().enumerate())
            .filter_map(|(n, k)| match *k {
                EtyKind::LanguageOnly(l) => Some((u8::try_from(n).ok()?, l)),
                EtyKind::Derivation | EtyKind::Unknown => None,
            })
            .collect_vec();
        if by_ety.is_empty() && language_only.is_empty() {
            return Ok(());
        }
        let mut members = vec![];
        for &(ety, origin) in &language_only {
            members.push((
                self.language_only_hypothesis(out, item, origin)?,
                EdgeBasis::Template,
                origin.certainty,
                ety,
            ));
            self.count("etymologies naming only a language");
        }
        for (&ety, edges) in &by_ety {
            let parents = edges
                .iter()
                .map(|e| (e.parent(), e.mode(), e.head()))
                .collect_vec();
            let weakest = edges
                .iter()
                .map(|e| e.certainty())
                .max()
                .unwrap_or_default();
            members.push((
                self.hypothesis(out, item, &parents)?,
                edges[0].basis(),
                weakest,
                ety,
            ));
        }
        members.sort_by_key(|m| m.3);
        // A source's alternatives are a set it asserts. A rule's (a root named
        // under several senses) are the rule's own reading, compiled one by
        // one: the schema lets a rule compile only hypotheses.
        let set = if members.len() > 1 && !members[0].1.is_rule() {
            // the source's order of preference, where its hedges fix one
            let ranks = members.iter().map(|m| m.2).collect_vec();
            let preference = (ranks.iter().all_unique()).then(|| {
                members
                    .iter()
                    .sorted_by_key(|m| m.2)
                    .map(|m| m.0.clone())
                    .collect_vec()
            });
            let ids = members.iter().map(|m| m.0.clone()).collect_vec();
            let id = format!("alt:{}", digest(&json!([ids, true, preference])));
            let set = out.add(json!({
                "type": "alternative_set",
                "id": id,
                "members": ids,
                "exclusive": true,
                "preference": preference,
            }))?;
            self.count("alternative sets");
            let asserted = graph.asserted(item, 0).cloned();
            self.assertion(
                out,
                item,
                members[0].1,
                &set,
                asserted.as_ref(),
                Certainty::Stated,
                None,
            )?;
            Some(set)
        } else {
            None
        };
        for (target, basis, certainty, ety) in members {
            let asserted = graph.asserted(item, ety).cloned();
            self.assertion(
                out,
                item,
                basis,
                &target,
                asserted.as_ref(),
                certainty,
                set.as_deref(),
            )?;
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let data = Data::deserialize(&args.graph)?;
    let file = File::create(&args.out)?;
    let out: Box<dyn Write> = if args.out.extension().is_some_and(|e| e == "gz") {
        Box::new(BufWriter::new(GzEncoder::new(file, Compression::default())))
    } else {
        Box::new(BufWriter::new(file))
    };
    let mut out = Writer {
        out,
        written: HashSet::default(),
    };
    let source = out.add(source()?)?;
    let extraction = out.add(extraction(&args.receipt)?)?;
    out.add(
        json!({"type": "agent", "id": EDITORS, "kind": "editors", "name": "Wiktionary's editors"}),
    )?;
    let rules = [
        EdgeBasis::Chain,
        EdgeBasis::Mention,
        EdgeBasis::FormOf,
        EdgeBasis::Root,
        EdgeBasis::RootVote,
    ];
    for basis in rules {
        let rule = assertions::rule(basis).context("a rule")?;
        out.add(json!({"type": "agent", "id": format!("agent:{rule}"), "kind": "rule", "name": assertions::rule_name(rule)}))?;
    }
    // what a rule credits: Wiktionary itself
    let work = out.add(json!({
        "type": "work",
        "id": format!("work:{SOURCE_NAME}:dump"),
        "citation": "English Wiktionary, dump of 2026-09-01",
        "source": source,
    }))?;
    let bibliography = Bibliography::read(&args.receipt)?;
    for w in &data.graph.works {
        bibliography.write(&mut out, w)?;
    }
    let mut export = Export {
        data: &data,
        source,
        extraction,
        work,
        counts: BTreeMap::new(),
    };
    let mut words: HashSet<ItemId> = HashSet::default();
    for (item, _) in data.graph.iter() {
        if args.every.is_none_or(|n| item.index() % n == 0) && words.insert(item) {
            export.word(&mut out, item)?;
        }
    }
    out.out.flush()?;
    drop(out);
    let mut counts: BTreeMap<String, Value> = export
        .counts
        .into_iter()
        .map(|(k, v)| (k, json!(v)))
        .collect();
    // what the graph stores beside its edges
    counts.insert(
        "graph: items with a stored assertion".into(),
        json!(data.graph.assertions.len()),
    );
    counts.insert("graph: works".into(), json!(data.graph.works.len()));
    counts.insert(
        "graph: assertion tables, serialized bytes".into(),
        json!(
            serde_json::to_vec(&data.graph.assertions)?.len()
                + serde_json::to_vec(&data.graph.works)?.len()
        ),
    );
    counts.insert("synthesis rules".into(), json!(data.synthesis_rules));
    if let Some(path) = args.counts {
        serde_json::to_writer_pretty(File::create(path)?, &counts)?;
    }
    println!("{}", serde_json::to_string_pretty(&counts)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn every_mode_is_one_the_schema_names() {
        let schema: Value = serde_json::from_str(include_str!(
            "../../../../packages/etymology/schema/assertion.schema.json"
        ))
        .unwrap();
        let modes = schema["$defs"]["Mode"]["enum"].as_array().unwrap();
        for mode in EtyMode::iter() {
            assert!(modes.contains(&json!(mode.as_ref())), "{}", mode.as_ref());
        }
    }
}
