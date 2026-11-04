// Other sources' claims, joined to items through their matches, and the
// readings of the words they touch (synthesis tiers 1 to 4). Each source is a
// claims product (the acd_claims or lila_claims workflow): its converted
// assertions, and the matches of its records to Wiktionary records, with the
// relays of Wiktionary's assertions that credit it. A claim is one link of a
// hypothesis a source asserts; it is used when its child and every parent are
// matched. Against the child's etymologies it is the same (same parents), a
// refinement (one parent the other's ancestor) or a competitor, which becomes
// an etymology of its own. A wety rule's compiled link competes only where the
// word has no etymology. A Wiktionary assertion whose relay finds that it
// follows the source takes the source's assertion as its origin.

use crate::items::Items;

use wety_core::{
    HashMap, HashSet,
    assertions::{self, Agent, Claim, Relation, Stance},
    ety_graph::{Certainty, EdgeBasis, EtyEdgeAccess, EtyKind, Etymologies, NewEtymology},
    etymology_templates::EtyMode,
    items::ItemId,
    string_pool::StringPool,
    synthesis::{self, Evidence, Reading, Tier},
};

use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    str::FromStr,
};

use anyhow::{Context, Result};
use itertools::Itertools;
use serde_json::{Value, json};

const EDITORS: &str = "agent:wiktionary-editors";

/// One source's claims product: its assertions and its matches.
pub struct ClaimsSource<'a> {
    pub assertions: &'a Path,
    pub matches: &'a Path,
}

fn read_jsonl(path: &Path) -> Result<Vec<Value>> {
    let file = File::open(path).with_context(|| path.display().to_string())?;
    BufReader::new(file)
        .lines()
        .map(|line| Ok(serde_json::from_str(&line?)?))
        .collect()
}

fn certainty(level: &str) -> Option<Certainty> {
    Some(match level {
        "probable" => Certainty::Probable,
        "possible" => Certainty::Possible,
        "doubtful" => Certainty::Doubtful,
        _ => return None,
    })
}

// A link as a source states it, its records matched to items.
struct Link {
    child: ItemId,
    parents: Vec<ItemId>,
    mode: EtyMode,
    certainty: Certainty,
    agent: u32,
    stance: Stance,
    compiled: bool,
    assertion: Box<str>,
    locator: Box<str>,
}

/// A reviewer's choice for one word, which applies while the word's evidence
/// is what it was when chosen (`digest`).
#[derive(serde::Deserialize)]
struct Override {
    word: Box<str>,
    /// The chosen etymology's parents, as record ids; None: no primary.
    parents: Option<Vec<Box<str>>>,
    digest: Box<str>,
}

pub(crate) type Counts = BTreeMap<String, u64>;

impl Items {
    fn agent(&mut self, ids: &mut HashMap<Box<str>, u32>, obj: &Value) -> Result<u32> {
        let id: Box<str> = obj["id"].as_str().context("an agent id")?.into();
        if let Some(&n) = ids.get(&id) {
            return Ok(n);
        }
        let n = u32::try_from(self.graph.agents.len())?;
        self.graph.agents.push(Agent {
            id: id.clone(),
            name: obj["name"].as_str().unwrap_or_default().into(),
            kind: obj["kind"].as_str().unwrap_or_default().into(),
        });
        ids.insert(id, n);
        Ok(n)
    }

    /// Join each source's claims to items and read the words they touch, and
    /// apply the overrides. Run once the graph's own etymologies are final.
    pub(crate) fn apply_claims(
        &mut self,
        string_pool: &StringPool,
        sources: &[ClaimsSource],
        overrides: Option<&Path>,
    ) -> Result<Counts> {
        let mut counts = Counts::new();
        let mut count = |k: &str| *counts.entry(k.to_owned()).or_default() += 1;
        let mut agent_ids: HashMap<Box<str>, u32> = HashMap::default();
        self.agent(
            &mut agent_ids,
            &json!({"id": EDITORS, "name": "Wiktionary's editors", "kind": "editors"}),
        )?;

        // the matches, the relays, and the records they name
        let mut matched: HashMap<Box<str>, Box<str>> = HashMap::default();
        let mut relays = vec![];
        let mut objects: HashMap<Box<str>, Value> = HashMap::default();
        for source in sources {
            for obj in read_jsonl(source.matches)? {
                match obj["type"].as_str() {
                    Some("match") => {
                        if let Some(target) = obj["target"].as_str() {
                            let record = obj["record"].as_str().context("a record")?;
                            matched.insert(record.into(), target.into());
                        }
                    }
                    Some("relay") => relays.push(obj),
                    Some("hypothesis" | "assertion" | "agent") => {
                        let id = obj["id"].as_str().context("an id")?.into();
                        objects.insert(id, obj);
                    }
                    _ => {}
                }
            }
        }
        let overrides: Vec<Override> = match overrides {
            Some(path) => {
                let v: Value = serde_json::from_reader(File::open(path)?)?;
                serde_json::from_value(v["overrides"].clone())?
            }
            None => vec![],
        };
        let mut wanted: HashSet<&str> = matched.values().map(|t| &**t).collect();
        for o in objects.values() {
            for link in o["links"].as_array().into_iter().flatten() {
                for r in std::iter::once(&link["child"])
                    .chain(link["parents"].as_array().into_iter().flatten())
                {
                    if let Some(r) = r.as_str().filter(|r| r.starts_with("wiktionary:")) {
                        wanted.insert(r);
                    }
                }
            }
        }
        for o in &overrides {
            wanted.insert(&o.word);
            wanted.extend(o.parents.iter().flatten().map(|p| &**p));
        }
        let mut item_of: HashMap<Box<str>, ItemId> = HashMap::default();
        for (id, item) in self.graph.iter() {
            let record = assertions::record_id(item, string_pool);
            if wanted.contains(record.as_str()) {
                item_of.insert(record.into(), id);
            }
        }
        for target in matched.values() {
            if !item_of.contains_key(target) {
                count("matches: target not in the graph");
            }
        }
        let item = |record: &str| -> Option<ItemId> {
            if record.starts_with("wiktionary:") {
                item_of.get(record).copied()
            } else {
                matched.get(record).and_then(|t| item_of.get(t)).copied()
            }
        };

        // the links each source asserts
        let mut links = vec![];
        for source in sources {
            let objs = read_jsonl(source.assertions)?;
            let mut hypotheses: HashMap<&str, &Value> = HashMap::default();
            let mut agents: HashMap<&str, &Value> = HashMap::default();
            for o in &objs {
                match o["type"].as_str() {
                    Some("hypothesis") => {
                        hypotheses.insert(o["id"].as_str().context("an id")?, o);
                    }
                    Some("agent") => {
                        agents.insert(o["id"].as_str().context("an id")?, o);
                    }
                    _ => {}
                }
            }
            for a in objs.iter().filter(|o| o["type"] == "assertion") {
                let Some(h) = a["target"].as_str().and_then(|t| hypotheses.get(t)) else {
                    continue;
                };
                let stance = match a["stance"].as_str() {
                    Some("endorses") => Stance::Endorses,
                    Some("rejects") => Stance::Rejects,
                    _ => Stance::Reports,
                };
                let asserter = a["asserter"].as_str().context("an asserter")?;
                let agent =
                    self.agent(&mut agent_ids, agents.get(asserter).context("its agent")?)?;
                for (n, link) in h["links"].as_array().context("links")?.iter().enumerate() {
                    count("links asserted");
                    let mode = match link["relation"].as_str() {
                        Some("ultimate") => EtyMode::Root,
                        Some("derivation") => {
                            EtyMode::from_str(link["mode"].as_str().context("a mode")?)?
                        }
                        _ => {
                            count("links: relation not a derivation");
                            continue;
                        }
                    };
                    let child = link["child"].as_str().and_then(item);
                    let parents = link["parents"]
                        .as_array()
                        .context("parents")?
                        .iter()
                        .map(|p| p.as_str().and_then(item))
                        .collect::<Option<Vec<_>>>();
                    let (Some(child), Some(parents)) = (child, parents) else {
                        count(if child.is_none() {
                            "links: child not matched"
                        } else {
                            "links: a parent not matched"
                        });
                        continue;
                    };
                    if parents.is_empty() || parents.contains(&child) {
                        count("links: no parent, or itself");
                        continue;
                    }
                    let hedge = a["hedges"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter(|h| h["link"].as_u64() == Some(n as u64))
                        .filter_map(|h| h["level"].as_str().and_then(certainty))
                        .max()
                        .unwrap_or(Certainty::Stated);
                    links.push(Link {
                        child,
                        parents,
                        mode,
                        certainty: hedge,
                        agent,
                        stance,
                        compiled: a.get("compiled").is_some(),
                        assertion: a["id"].as_str().context("an id")?.into(),
                        locator: a["locator"].as_str().unwrap_or_default().into(),
                    });
                }
            }
        }

        // relays: a Wiktionary assertion that follows the source takes its
        // origin; a locator alone is no difference of substance
        let mut touched: HashSet<ItemId> = HashSet::default();
        for relay in &relays {
            let agrees = relay["agrees"].as_bool() == Some(true)
                || relay["differences"]
                    .as_array()
                    .is_some_and(|d| d.iter().all(|d| d == "locator"));
            if !agrees {
                count("relays: differing");
                continue;
            }
            let theirs = relay["relays"]
                .as_str()
                .and_then(|r| objects.get(r))
                .and_then(|a| a["asserter"].as_str())
                .and_then(|a| objects.get(a));
            let ours = relay["assertion"]
                .as_str()
                .and_then(|r| objects.get(r))
                .and_then(|a| a["target"].as_str())
                .and_then(|h| objects.get(h))
                .and_then(|h| h["links"].get(0));
            let (Some(theirs), Some(link)) = (theirs, ours) else {
                count("relays: objects missing");
                continue;
            };
            let child = link["child"].as_str().and_then(item);
            let parents: Option<Vec<ItemId>> = link["parents"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|p| p.as_str().and_then(item))
                .collect();
            let (Some(child), Some(parents)) = (child, parents) else {
                count("relays: records not in the graph");
                continue;
            };
            let Some(ety) = self.ety_with_parents(child, &parents) else {
                count("relays: etymology not in the graph");
                continue;
            };
            let origin = self.agent(&mut agent_ids, theirs)?;
            let mut asserted = (0..self.graph.ety_count(child))
                .map(|n| {
                    self.graph
                        .asserted(child, u8::try_from(n).unwrap_or(u8::MAX))
                        .cloned()
                        .unwrap_or_default()
                })
                .collect_vec();
            asserted[usize::from(ety)].origin = Some(origin);
            self.graph.set_asserted(child, asserted);
            touched.insert(child);
            count("relays: origin set");
        }

        // each link against its child's etymologies
        for link in links {
            let relation;
            let ety = if let Some(ety) = self.ety_with_parents(link.child, &link.parents) {
                relation = Relation::Same;
                ety
            } else if let Some(ety) = self.ety_refined_by(link.child, &link.parents) {
                relation = Relation::Refines;
                ety
            } else if link.stance != Stance::Endorses
                || (link.compiled && self.graph.ety_count(link.child) > 0)
            {
                count("links: rejected or compiled, with nothing to attach to");
                continue;
            } else {
                relation = Relation::Competes;
                let n = link.parents.len();
                self.graph.push_etymology(
                    link.child,
                    &NewEtymology {
                        mode: link.mode,
                        head: (n == 1).then_some(0),
                        basis: EdgeBasis::Source,
                        items: &link.parents,
                        confidences: &vec![1.0; n],
                        certainties: &vec![link.certainty; n],
                    },
                )
            };
            count(&format!("links: {}", relation.as_ref()));
            let ety_mode = self
                .graph
                .all_parent_edges(link.child)
                .find(|e| e.ety() == ety)
                .map(|e| e.mode());
            let claim = Claim {
                ety,
                agent: link.agent,
                origin: link.agent,
                stance: link.stance,
                relation,
                certainty: link.certainty,
                mode: (ety_mode != Some(link.mode)).then_some(link.mode),
                compiled: link.compiled,
                assertion: link.assertion,
                locator: link.locator,
            };
            let mut claims = self
                .graph
                .claims
                .remove(&link.child)
                .map(Vec::from)
                .unwrap_or_default();
            if !claims.contains(&claim) {
                claims.push(claim);
            }
            self.graph
                .claims
                .insert(link.child, claims.into_boxed_slice());
            touched.insert(link.child);
        }

        // a claim may close a cycle; then the readings
        self.break_cycles();
        let overrides: HashMap<ItemId, &Override> = overrides
            .iter()
            .filter_map(|o| item_of.get(&o.word).map(|&i| (i, o)))
            .collect();
        touched.extend(overrides.keys());
        for &item in touched.iter().sorted() {
            let reading = self.read(item, string_pool, &item_of, overrides.get(&item).copied());
            count(&format!("readings: {}", reading.as_ref()));
        }
        Ok(counts)
    }

    // The etymology of `item` whose parents are `parents`, in any order.
    fn ety_with_parents(&self, item: ItemId, parents: &[ItemId]) -> Option<u8> {
        let wanted: HashSet<ItemId> = parents.iter().copied().collect();
        self.graph
            .all_parent_edges(item)
            .into_group_map_by(|e| e.ety())
            .into_iter()
            .find(|(_, edges)| {
                edges.len() == wanted.len() && edges.iter().all(|e| wanted.contains(&e.parent()))
            })
            .map(|(ety, _)| ety)
    }

    // An etymology of `item` with one parent that is the claimed single
    // parent's ancestor or descendant along followed links.
    fn ety_refined_by(&self, item: ItemId, parents: &[ItemId]) -> Option<u8> {
        let &[claimed] = parents else {
            return None;
        };
        let ancestors = |of: ItemId| -> HashSet<ItemId> {
            self.graph
                .ancestor_edges(of)
                .map(|e| e.parent())
                .take(10_000)
                .collect()
        };
        let of_claimed = ancestors(claimed);
        self.graph
            .all_parent_edges(item)
            .into_group_map_by(|e| e.ety())
            .into_iter()
            .filter_map(|(ety, edges)| match edges[..] {
                [e] => Some((ety, e.parent())),
                _ => None,
            })
            .find(|&(_, parent)| {
                of_claimed.contains(&parent) || ancestors(parent).contains(&claimed)
            })
            .map(|(ety, _)| ety)
    }

    // What the word's assertions say for each of its etymologies.
    fn evidence(&self, item: ItemId) -> Vec<Evidence> {
        let n = self.graph.ety_count(item);
        let mut etys = vec![Evidence::default(); n];
        let mut weakest = vec![Certainty::Stated; n];
        let mut basis = vec![None; n];
        for e in self.graph.all_parent_edges(item) {
            let i = usize::from(e.ety());
            weakest[i] = weakest[i].max(e.certainty());
            basis[i] = Some(e.basis());
        }
        for (i, ev) in etys.iter_mut().enumerate() {
            ev.firmest = Certainty::Doubtful;
            match basis[i] {
                // an etymology only another source gives: its claims say
                Some(EdgeBasis::Source) => ev.rule = true,
                Some(b) if b.is_rule() => {
                    ev.rule = true;
                    ev.origins.push(u32::MAX);
                    ev.firmest = weakest[i];
                }
                // Wiktionary's own, or its unknown origin
                _ => {
                    let asserted = self
                        .graph
                        .asserted(item, u8::try_from(i).unwrap_or(u8::MAX));
                    ev.origins
                        .push(asserted.and_then(|a| a.origin).unwrap_or(0));
                    ev.firmest = weakest[i];
                }
            }
        }
        for c in self.graph.claims.get(&item).into_iter().flatten() {
            let ev = &mut etys[usize::from(c.ety)];
            match c.stance {
                Stance::Endorses if !c.compiled => {
                    ev.rule = false;
                    ev.origins.push(c.origin);
                    ev.firmest = ev.firmest.min(c.certainty);
                }
                Stance::Endorses => {
                    ev.origins.push(c.origin);
                    ev.firmest = ev.firmest.min(c.certainty);
                }
                Stance::Rejects => ev.rejected_by.push(c.origin),
                Stance::Reports => {}
            }
        }
        for ev in &mut etys {
            ev.origins.sort_unstable();
            ev.origins.dedup();
        }
        etys
    }

    // The digest an override names: the word's etymologies (parents as
    // record ids, mode) and their evidence.
    fn evidence_digest(&self, item: ItemId, sp: &StringPool, evidence: &[Evidence]) -> String {
        let mut etys: Vec<Value> = vec![];
        for (i, ev) in evidence.iter().enumerate() {
            let edges = self
                .graph
                .all_parent_edges(item)
                .filter(|e| usize::from(e.ety()) == i)
                .collect_vec();
            let parents = edges
                .iter()
                .map(|e| assertions::record_id(self.graph.item(e.parent()), sp))
                .sorted()
                .collect_vec();
            let origins = ev
                .origins
                .iter()
                .map(|&o| self.graph.agents.get(o as usize).map_or("rule", |a| &*a.id))
                .collect_vec();
            etys.push(json!({
                "parents": parents,
                "mode": edges.first().map(|e| e.mode().as_str()),
                "origins": origins,
                "firmest": ev.firmest.as_ref(),
            }));
        }
        assertions::digest(&json!(etys))
    }

    // A word's reading: its override while current, else tiers 2 to 4.
    fn read(
        &mut self,
        item: ItemId,
        sp: &StringPool,
        item_of: &HashMap<Box<str>, ItemId>,
        override_: Option<&Override>,
    ) -> Tier {
        let evidence = self.evidence(item);
        let (mut primary, mut reading) = synthesis::read(&evidence);
        if let Some(o) = override_
            && *o.digest == self.evidence_digest(item, sp, &evidence)
        {
            let chosen = o.parents.as_ref().and_then(|ps| {
                let ps: Option<Vec<ItemId>> = ps.iter().map(|p| item_of.get(p).copied()).collect();
                self.ety_with_parents(item, &ps?)
            });
            if o.parents.is_none() || chosen.is_some() {
                primary = chosen;
                reading = Reading {
                    tier: Tier::Override,
                    disputed: reading.disputed,
                };
            }
        }
        let n = self.graph.ety_count(item);
        let old = self.graph.etymologies.remove(&item);
        if n > 1 || reading != Reading::default() || old.is_some() {
            let kinds = old.map_or_else(
                || vec![EtyKind::Derivation; n.max(1)].into_boxed_slice(),
                |e| e.kinds,
            );
            self.graph.etymologies.insert(
                item,
                Etymologies {
                    kinds,
                    primary,
                    reading,
                },
            );
        }
        reading.tier
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wety_core::{gloss::Gloss, items::RealItem, langterm::Term, languages::Lang, pos::Pos};

    fn add(items: &mut Items, sp: &mut StringPool, lang: &str, term: &str) -> ItemId {
        items
            .add_real(RealItem {
                ety_num: 1,
                ety_label: None,
                lang: Lang::from_str(lang).unwrap(),
                term: Term::new(sp, term),
                pos: vec![Pos::from_str("noun").unwrap()],
                gloss: vec![Gloss::new(sp, "x")],
                page_term: None,
                romanization: None,
                is_reconstructed: false,
            })
            .0
    }

    fn write(path: &Path, objs: &[Value]) {
        let text: String = objs.iter().map(|o| format!("{o}\n")).collect();
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn claims_join_their_words_and_read_them() {
        let (mut items, mut sp) = (Items::new().unwrap(), StringPool::new());
        let lupus = add(&mut items, &mut sp, "la", "lupus");
        let lukos = add(&mut items, &mut sp, "itc-pro", "lukʷos");
        let wlkos = add(&mut items, &mut sp, "ine-pro", "wĺ̥kʷos");
        let canis = add(&mut items, &mut sp, "la", "canis");
        let kwon = add(&mut items, &mut sp, "ine-pro", "ḱwṓ");
        let other = add(&mut items, &mut sp, "ine-pro", "kan-");
        for (child, parent) in [(lupus, lukos), (canis, kwon)] {
            items.graph.add_ety(
                child,
                EtyMode::Inherited,
                Some(0),
                EdgeBasis::Template,
                &[parent],
                &[1.0],
            );
        }
        let dir = std::env::temp_dir().join("wety-claims-test");
        std::fs::create_dir_all(&dir).unwrap();
        let link = |child: &str, parent: &str| json!({"child": child, "parents": [parent], "relation": "derivation", "mode": "inherited"});
        let devaan = json!({"type": "agent", "id": "agent:de-vaan", "kind": "person", "name": "Michiel de Vaan"});
        let assertion = |id: &str, target: &str, asserter: &str| json!({"type": "assertion", "id": id, "target": target, "stance": "endorses", "asserter": asserter, "locator": "x"});
        write(
            &dir.join("lila.jsonl"),
            &[
                devaan.clone(),
                json!({"type": "hypothesis", "id": "hyp:1", "links": [link("l:lupus", "l:lukos"), link("l:lukos", "l:wlkos")]}),
                json!({"type": "hypothesis", "id": "hyp:2", "links": [link("l:canis", "l:kan")]}),
                assertion("as:1", "hyp:1", "agent:de-vaan"),
                assertion("as:2", "hyp:2", "agent:de-vaan"),
            ],
        );
        let matched = |record: &str, target: &str| json!({"type": "match", "record": record, "target": target});
        write(
            &dir.join("matches.jsonl"),
            &[
                matched("l:lupus", "wiktionary:la:lupus#1"),
                matched("l:lukos", "wiktionary:itc-pro:lukʷos#1"),
                matched("l:wlkos", "wiktionary:ine-pro:wĺ̥kʷos#1"),
                matched("l:canis", "wiktionary:la:canis#1"),
                matched("l:kan", "wiktionary:ine-pro:kan-#1"),
                devaan,
                json!({"type": "hypothesis", "id": "hyp:w", "links": [link("wiktionary:la:lupus#1", "wiktionary:itc-pro:lukʷos#1")]}),
                assertion("as:w", "hyp:w", "agent:wiktionary-editors"),
                assertion("as:1", "hyp:1", "agent:de-vaan"),
                json!({"type": "relay", "assertion": "as:w", "relays": "as:1", "agrees": false, "differences": ["locator"]}),
            ],
        );
        let source = ClaimsSource {
            assertions: &dir.join("lila.jsonl"),
            matches: &dir.join("matches.jsonl"),
        };
        let counts = items.apply_claims(&sp, &[source], None).unwrap();
        let graph = &items.graph;
        // lupus: de Vaan's link is Wiktionary's, which follows him: one origin
        assert_eq!(graph.claims[&lupus][0].relation, Relation::Same);
        assert_eq!(graph.asserted(lupus, 0).and_then(|a| a.origin), Some(1));
        assert!(graph.etymologies(lupus).is_none());
        // *lukʷos had no etymology: de Vaan's is its own
        assert_eq!(graph.claims[&lukos][0].relation, Relation::Competes);
        assert_eq!(graph.immediate_ety(lukos).unwrap().items, [wlkos]);
        // canis: two origins, firmly apart
        let reading = graph.etymologies(canis).unwrap();
        assert_eq!(
            (reading.primary, reading.reading.tier),
            (None, Tier::Unsettled)
        );
        assert!(reading.reading.disputed);
        assert_eq!(counts["links: same"], 1);
        assert_eq!(counts["links: competes"], 2);
        let _ = other;
    }
}
