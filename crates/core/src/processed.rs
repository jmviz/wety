use crate::{
    HashMap, HashSet, assertions,
    descendant_langs::DescendantLangs,
    ety_graph::{Certainty, EdgeBasis, EtyEdge, EtyKind, SourceLang},
    ety_graph::{EtyEdgeAccess, EtyGraph, Progenitors},
    etymology_templates::EtyMode,
    items::{Item, ItemId},
    languages::Lang,
    response::{
        AssertionJson, ClaimJson, CognatesJson, CreditJson, DescendantsJson, EtyJson,
        EtymologyJson, GlossStr, HypothesisJson, ItemJson, LinkJson, OriginJson, OtherEtyJson,
        OtherParentJson, ReadingJson, SourceLangJson, SourcePageJson, TrailStepJson,
    },
    string_pool::StringPool,
    synthesis::Reading,
};

use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    path::Path,
    time::Instant,
};

use anyhow::{Ok, Result};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use itertools::Itertools;
use memmap2::Mmap;
use ngrammatic::{Corpus, CorpusBuilder, Pad};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use trie_rs::map::{Trie as TrieMap, TrieBuilder as TrieMapBuilder};

#[derive(Serialize, Deserialize)]
pub struct Data {
    #[serde(default)]
    pub reference_fingerprint: Option<String>,
    /// The synthesis rules the graph's readings were made with.
    pub synthesis_rules: u32,
    pub string_pool: StringPool,
    pub graph: EtyGraph,
    pub progenitors: HashMap<ItemId, Progenitors>,
    pub descendant_langs: DescendantLangs,
}

impl Data {
    #[must_use]
    pub fn new(string_pool: StringPool, graph: EtyGraph) -> Self {
        let progenitors = graph.all_progenitors();
        let descendant_langs = graph.all_descendant_langs();
        Self {
            reference_fingerprint: Some(env!("WETY_REFERENCE_FINGERPRINT").to_owned()),
            synthesis_rules: crate::synthesis::SYNTHESIS_RULES,
            string_pool,
            graph,
            progenitors,
            descendant_langs,
        }
    }

    /// # Errors
    ///
    /// Returns an error if the output file cannot be created or written.
    pub fn serialize(&self, path: &Path) -> Result<()> {
        let t = Instant::now();
        println!("Serializing processed data to {}...", path.display());
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = File::create(path)?;
        let should_gz_compress = path.extension().is_some_and(|ext| ext == "gz");
        let writer: Box<dyn Write> = if should_gz_compress {
            Box::new(GzEncoder::new(file, Compression::fast()))
        } else {
            Box::new(BufWriter::new(file))
        };
        serde_json::to_writer(writer, &self)?;
        println!("Finished. Took {:.2?}.", t.elapsed());
        Ok(())
    }
}

/// A short identity for a graph file. Clients send it back with numeric item
/// ids, which only mean something within one graph.
///
/// # Errors
///
/// Returns an error if the file cannot be read.
pub fn graph_version(path: &Path) -> Result<String> {
    let mut hasher = xxhash_rust::xxh3::Xxh3::new();
    let mut file = File::open(path)?;
    let mut buf = vec![0; 1 << 20];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:016x}", hasher.digest()))
}

// private methods for use within pub methods below
impl Data {
    fn item(&self, id: ItemId) -> &Item {
        self.graph.item(id)
    }

    fn term(&self, item: ItemId) -> &str {
        self.item(item).term().resolve(&self.string_pool)
    }

    fn ety_num(&self, item: ItemId) -> u8 {
        self.item(item).ety_num()
    }
}

/// `descendant_total` stops counting here; clients show the cap as "more".
pub const TOTAL_CAP: usize = 100_000;

// pub methods for server
impl Data {
    #[must_use]
    pub fn lang(&self, item: ItemId) -> Lang {
        self.item(item).lang()
    }

    /// The item's ancestors in `langs`, or all of them when `langs` is empty
    /// (every language is drawn).
    #[must_use]
    pub fn ancestors_in_langs(&self, item: ItemId, langs: &[Lang]) -> Vec<ItemId> {
        if langs.is_empty() {
            return self
                .graph
                .ancestor_edges(item)
                .map(|e| e.parent())
                .unique()
                .collect();
        }
        self.graph.ancestors_in_langs(item, langs).collect()
    }

    /// # Errors
    ///
    /// Will return `Err` if any unexpected issue arises in the deserialization.
    pub fn deserialize(path: &Path) -> Result<Self> {
        Self::deserialize_with_legacy(path, false)
    }

    pub fn deserialize_with_legacy(path: &Path, allow_legacy: bool) -> Result<Self> {
        let t = Instant::now();
        println!("Deserializing processed data {}...", path.display());
        let file = File::open(path)?;
        let is_gz_compressed = path.extension().is_some_and(|ext| ext == "gz");
        let data: Self = if is_gz_compressed {
            serde_json::from_reader(GzDecoder::new(BufReader::new(file)))?
        } else {
            // SAFETY: `Mmap::map` is only unsound if the file is modified while
            // mapped; the processed data file is written once by `processor`
            // and only read here.
            let mmap = unsafe { Mmap::map(&file)? };
            mmap.advise(memmap2::Advice::Sequential)?;
            serde_json::from_slice(&mmap)?
        };
        validate_reference_fingerprint(data.reference_fingerprint.as_deref(), allow_legacy)?;
        println!("Finished. Took {:#?}.", t.elapsed());
        Ok(data)
    }

    #[must_use]
    pub fn has_item(&self, item: ItemId) -> bool {
        self.graph.graph.contains_node(item)
    }

    /// Full item payload, as served at `/item/:id`.
    #[must_use]
    pub fn item_json(&self, item_id: ItemId) -> ItemJson<'_> {
        self.item_json_inner(item_id, false)
    }

    /// `lean` omits the glosses, by far the largest part of a node's payload.
    fn item_json_inner(&self, item_id: ItemId, lean: bool) -> ItemJson<'_> {
        let item = self.item(item_id);
        ItemJson {
            id: item_id,
            ety_num: item.ety_num(),
            ety: item.link_ety(),
            lang: item.lang().info(),
            term: item.term().resolve(&self.string_pool),
            imputed: item.is_imputed(),
            reconstructed: item.is_reconstructed(),
            url: item.url(&self.string_pool),
            pos: item.pos().map(|pos| pos.iter().map(|p| p.name()).collect()),
            gloss: (!lean).then(|| {
                item.gloss().map(|glosses| {
                    glosses
                        .iter()
                        .map(|gloss| GlossStr {
                            gloss,
                            string_pool: &self.string_pool,
                        })
                        .collect()
                })
            }),
            romanization: item.romanization().map(|r| r.resolve(&self.string_pool)),
        }
    }

    /// The descendants of `item_id` that reach `view.desc_langs` (empty: every
    /// language). A word formed from a word of its own language is drawn only
    /// when `view.within` is set or something below it is in another
    /// language, so limelight and limelighter stay off a tree of English lime
    /// unless asked for.
    #[must_use]
    pub fn item_descendants_json(
        &self,
        item_id: ItemId,
        view: &TreeView<'_>,
    ) -> DescendantsJson<'_> {
        self.item_descendants_json_inner(item_id, view, None)
    }

    /// `via`: the edge from the item to its tree parent; none for the root.
    /// An edge walks do not follow draws the item as possibly related to its
    /// parent, without its descendants (they are drawn under the etymology
    /// it is said to have).
    fn item_descendants_json_inner(
        &self,
        item_id: ItemId,
        view: &TreeView<'_>,
        via: Option<EtyEdge<'_>>,
    ) -> DescendantsJson<'_> {
        let &TreeView {
            dist_lang,
            desc_langs,
            within,
            keep: req_item_ancestors_within_desc_langs,
            lean,
        } = view;
        let item = self.item(item_id);
        let item_lang = item.lang();
        let expand = via.is_none_or(|e| self.graph.follows(e));

        let mut children: Vec<DescendantsJson<'_>> = self
            .graph
            .child_edges(item_id)
            .filter(|_| expand)
            .filter(|e| {
                let child = e.child();
                let child_lang = self.item(child).lang();
                let drawn = desc_langs.is_empty() || desc_langs.contains(&child_lang);
                let reaches_another = if desc_langs.is_empty() {
                    self.descendant_langs
                        .get(child)
                        .iter()
                        .any(|&l| l != item_lang)
                } else {
                    desc_langs
                        .iter()
                        .any(|&dl| dl != item_lang && self.descendant_langs.contains(child, dl))
                };
                req_item_ancestors_within_desc_langs.contains(&item_id)
                    || (drawn && (within || child_lang != item_lang))
                    || reaches_another
            })
            .map(|e| self.item_descendants_json_inner(e.child(), view, Some(e)))
            .collect();
        // Words possibly related: drawn where their language is, unless the
        // walk already draws them here.
        let followed: HashSet<ItemId> =
            self.graph.child_edges(item_id).map(|e| e.child()).collect();
        children.extend(
            self.graph
                .unfollowed_child_edges(item_id)
                .filter(|_| expand)
                .filter(|e| !followed.contains(&e.child()))
                .filter(|e| {
                    let child_lang = self.item(e.child()).lang();
                    (desc_langs.is_empty() || desc_langs.contains(&child_lang))
                        && (within || child_lang != item_lang)
                })
                .unique_by(|e| e.child())
                .map(|e| self.item_descendants_json_inner(e.child(), view, Some(e))),
        );
        let descendant_count = children.iter().map(|c| 1 + c.descendant_count).sum();

        // The etymology the row is drawn under: that of its tree edge, or for
        // a root, the one walks follow (the first, when none is preferred).
        // Its other parents are those of the same etymology; another
        // etymology's are not co-parents but alternatives.
        let (etys, primary) = self.etymologies_of(item_id);
        let drawn = via.map(|e| e.ety()).or(primary).or_else(|| {
            etys.iter()
                .position(|(_, edges)| !edges.is_empty())
                .and_then(|i| u8::try_from(i).ok())
        });
        let parent_json = |e: &EtyEdge<'_>| OtherParentJson {
            item: self.item_json_inner(e.parent(), lean),
            ety_order: e.order(),
            lang_distance: self.item(e.parent()).lang().distance_from(dist_lang),
            certainty: certainty_json(e.certainty()),
        };
        let mut ety_mode = None;
        let mut other_parents = vec![];
        let mut alternatives = vec![];
        for (n, (kind, edges)) in etys.iter().enumerate() {
            if u8::try_from(n).ok() == drawn {
                ety_mode = edges.first().map(|e| e.mode().as_str());
                other_parents = edges
                    .iter()
                    .filter(|e| via.is_none_or(|v| v.parent() != e.parent()))
                    .map(parent_json)
                    .collect();
            } else {
                alternatives.push(OtherEtyJson {
                    kind: (*kind).into(),
                    mode: mode_of(*kind, edges),
                    source_lang: source_lang_json(*kind),
                    preferred: primary.is_some_and(|p| usize::from(p) == n),
                    parents: edges.iter().map(parent_json).collect(),
                });
            }
        }
        let standing = (etys.len() > 1).then(|| match primary {
            None => "unsettled",
            Some(p) if Some(p) == drawn => "preferred",
            Some(_) => "other",
        });

        DescendantsJson {
            item: self.item_json_inner(item_id, lean),
            children,
            descendant_count,
            lang_distance: item_lang.distance_from(dist_lang),
            ety_mode,
            other_parents,
            parent_ety_order: via.map(|e| e.order()),
            certainty: via.and_then(|e| certainty_json(e.certainty())),
            standing,
            alternatives,
        }
    }

    /// Words in `root`'s unfiltered descendants tree, counting at most `cap`
    /// (a word reached by two paths is drawn, and counted, twice).
    #[must_use]
    pub fn descendant_total(&self, root: ItemId, cap: usize) -> usize {
        self.graph.descendant_edges(root).take(cap).count()
    }

    /// Where a cognates tree starts by default: as far back as the item's
    /// head line goes. Each step takes an etymology's sole source, else its
    /// head, else its one root entry (a term ending, not starting, with a
    /// hyphen): English tabula ← Latin tabula ← Proto-Italic *taðlā ← PIE
    /// *(s)th₂-dʰló-m ← *steh₂-. The line ends at a word with no source, a
    /// compound with neither (ἀστρονομία from ἄστρον + νομή), before a
    /// suffix (*méh₂tēr, whose only recorded source is *-tḗr), or before a
    /// possible or doubtful link (*wĺ̥kʷos, possibly from *wl̥kʷós). Etymologies
    /// none of which is preferred go on to the nearest word all their lines
    /// reach (English mare: Middle English mare, from Old English mīere or
    /// mearh, both from Proto-Germanic *marhaz). The tree starts at the oldest
    /// word on it that the trail shows, so the reader sees where. Deeper is
    /// better (maintainer, 2026-11-09): a very large family is part of the
    /// fun, and readers usually filter it to a few languages; the line stops
    /// only where going on would be wrong.
    #[must_use]
    pub fn default_root(&self, item: ItemId) -> ItemId {
        let trail = self.lineage(item);
        let top = *trail.last().expect("a trail starts with its item");
        let mut shown: HashSet<ItemId> = trail.into_iter().collect();
        shown.extend(
            self.graph
                .immediate_ety(top)
                .map_or_else(Vec::new, |e| e.items),
        );
        shown.extend(self.ultimate_sources(item));
        let mut root = item;
        let mut seen = HashSet::from_iter([item]);
        let mut current = item;
        while let Some(next) = self.head_step(current) {
            if !seen.insert(next) {
                break;
            }
            if shown.contains(&next) {
                root = next;
            }
            current = next;
        }
        root
    }

    /// The next word on an item's head line (see `default_root`), if any. Of
    /// etymologies none of which is preferred, the first word of the first
    /// one's line that every other one's line reaches too.
    fn head_step(&self, item: ItemId) -> Option<ItemId> {
        let mut edges = self.graph.parent_edges(item).collect_vec();
        edges.sort_unstable_by_key(|e| (e.ety(), e.order()));
        let etys = edges.chunk_by(|a, b| a.ety() == b.ety()).collect_vec();
        match etys[..] {
            [] => None,
            [ety] => self.ety_step(ety),
            _ => {
                let lines = etys
                    .iter()
                    .map(|ety| {
                        let mut line = vec![];
                        let mut next = self.ety_step(ety);
                        while let Some(n) = next {
                            if n == item || line.contains(&n) {
                                break;
                            }
                            line.push(n);
                            next = self.head_step(n);
                        }
                        line
                    })
                    .collect_vec();
                lines[0]
                    .iter()
                    .copied()
                    .find(|id| lines[1..].iter().all(|line| line.contains(id)))
            }
        }
    }

    /// The word one etymology's head line goes on to: its sole source, else
    /// its head, else its one root entry. None for a compound with neither, a
    /// suffix (its descendants are every word it formed, not a family) or a
    /// possible or doubtful link (no ground to start a family).
    fn ety_step(&self, edges: &[EtyEdge<'_>]) -> Option<ItemId> {
        let roots = edges
            .iter()
            .copied()
            .filter(|e| self.is_root_entry(e.parent()))
            .collect_vec();
        let edge = match (edges, edges.iter().find(|e| e.head()), &roots[..]) {
            ([source], _, _) | (_, Some(source), _) | (_, None, [source]) => *source,
            _ => return None,
        };
        let dubious = matches!(edge.certainty(), Certainty::Possible | Certainty::Doubtful);
        (!dubious && !self.term(edge.parent()).starts_with('-')).then(|| edge.parent())
    }

    /// The item's ultimate ancestors, in the order its etymology names them:
    /// the forms at the ends of its ancestry, which have no source of their
    /// own (English astronomy: PIE *h₂eHs- and *nem- among them).
    #[must_use]
    pub fn ultimate_sources(&self, item: ItemId) -> Vec<ItemId> {
        let mut out = vec![];
        let mut seen = HashSet::default();
        let mut stack = vec![item];
        while let Some(id) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }
            match self.graph.immediate_ety(id) {
                // Reversed onto the stack, so the first source is visited first.
                Some(ety) => stack.extend(ety.items.into_iter().rev()),
                None if id != item => out.push(id),
                None => {}
            }
        }
        out
    }

    /// Whether a term is a root entry: it ends, but does not start, with a
    /// hyphen (PIE *peh₂-, not the suffix *-tḗr).
    fn is_root_entry(&self, id: ItemId) -> bool {
        let t = self.term(id);
        t.ends_with('-') && !t.starts_with('-')
    }

    /// The item's trail: the item, then each older form that is the same
    /// word, stepping to an etymology's single source while its mode passes
    /// the word on. Ends at a new formation, a meaning-only loan, an etymology
    /// with several sources, or a word with none.
    #[must_use]
    pub fn lineage(&self, item: ItemId) -> Vec<ItemId> {
        let mut trail = vec![item];
        let mut current = item;
        while let Some(ety) = self.graph.immediate_ety(current) {
            let [source] = ety.items[..] else { break };
            if !ety.mode.passes_word_on() || trail.contains(&source) {
                break;
            }
            trail.push(source);
            current = source;
        }
        trail
    }

    /// The item or one of its ancestors with this lang code, term and (when
    /// given) link ety, for starting a cognates tree there.
    #[must_use]
    pub fn ancestor_named(
        &self,
        item: ItemId,
        lang: &str,
        term: &str,
        ety: Option<&str>,
    ) -> Option<ItemId> {
        let matches = |id: ItemId| {
            let it = self.item(id);
            it.lang().code() == lang
                && self.term(id) == term
                && ety.is_none_or(|e| it.link_ety().as_deref() == Some(e))
        };
        std::iter::once(item)
            .chain(self.graph.ancestor_edges(item).map(|e| e.parent()))
            .find(|&id| matches(id))
    }

    /// The descendants of `root` (default: see `default_root`), keeping the
    /// path to the item visible. Empty `view.desc_langs` means every language.
    #[must_use]
    pub fn item_cognates_json(
        &self,
        item_id: ItemId,
        root: Option<ItemId>,
        view: &TreeView<'_>,
    ) -> CognatesJson<'_> {
        let lean = view.lean;
        let trail = self.lineage(item_id);
        let top = *trail.last().expect("a trail starts with its item");
        let default_root = self.default_root(item_id);
        let root = root.unwrap_or(default_root);
        let ety_mode = |id: ItemId| self.graph.immediate_ety(id).map(|e| e.mode.as_str());
        let sources = self
            .graph
            .immediate_ety(top)
            .map_or_else(Vec::new, |e| e.items);
        // By etymology: alternatives none of which is preferred are told apart.
        let mut edges = self.graph.parent_edges(top).collect_vec();
        edges.sort_unstable_by_key(|e| (e.ety(), e.order()));
        let source_etys = edges
            .chunk_by(|a, b| a.ety() == b.ety())
            .map(|ety| {
                ety.iter()
                    .map(|e| self.item_json_inner(e.parent(), lean))
                    .collect()
            })
            .collect();
        // Roots first, then affixes and other ends, leaving out what the
        // trail and its sources already name.
        let (roots, others): (Vec<_>, Vec<_>) = self
            .ultimate_sources(item_id)
            .into_iter()
            .filter(|id| !trail.contains(id) && !sources.contains(id))
            .partition(|&id| self.is_root_entry(id));
        CognatesJson {
            trail: trail
                .iter()
                .map(|&id| TrailStepJson {
                    item: self.item_json_inner(id, lean),
                    ety_mode: ety_mode(id),
                })
                .collect(),
            sources: source_etys,
            ultimate_roots: roots
                .iter()
                .map(|&id| self.item_json_inner(id, lean))
                .collect(),
            ultimate_others: others
                .iter()
                .map(|&id| self.item_json_inner(id, lean))
                .collect(),
            root,
            default_root,
            total: self.descendant_total(root, TOTAL_CAP),
            all_langs: view.desc_langs.is_empty(),
            tree: self.item_descendants_json(root, view),
        }
    }

    /// An item's etymologies, numbered as the graph numbers them, each with
    /// its edges in order, and the one walks follow. An unknown origin is one
    /// etymology with no edges.
    fn etymologies_of(&self, item: ItemId) -> (Vec<(EtyKind, Vec<EtyEdge<'_>>)>, Option<u8>) {
        let mut etys: Vec<(EtyKind, Vec<EtyEdge<'_>>)> = match self.graph.etymologies(item) {
            Some(e) => e.kinds.iter().map(|&k| (k, vec![])).collect(),
            None => vec![],
        };
        for e in self.graph.all_parent_edges(item) {
            let n = usize::from(e.ety());
            if etys.len() <= n {
                etys.resize_with(n + 1, || (EtyKind::Derivation, vec![]));
            }
            etys[n].1.push(e);
        }
        for (_, edges) in &mut etys {
            edges.sort_unstable_by_key(EtyEdgeAccess::order);
        }
        let primary = match self.graph.etymologies(item) {
            Some(e) => e.primary,
            None => (!etys.is_empty()).then_some(0),
        };
        (etys, primary)
    }

    /// The etymology chart above an item: each of its etymologies, and theirs.
    #[must_use]
    pub fn item_etymology_json(&self, item_id: ItemId, req_lang: Lang) -> EtymologyJson<'_> {
        self.etymology_node(item_id, 0, None, req_lang)
    }

    fn etymology_node(
        &self,
        item_id: ItemId,
        ety_order: u8,
        certainty: Option<Certainty>,
        req_lang: Lang,
    ) -> EtymologyJson<'_> {
        let (etys, primary) = self.etymologies_of(item_id);
        EtymologyJson {
            item: self.item_json(item_id),
            ety_order,
            lang_distance: self.item(item_id).lang().distance_from(req_lang),
            certainty: certainty
                .filter(|&c| c != Certainty::Stated)
                .map(Into::into),
            etymologies: etys
                .iter()
                .map(|(kind, edges)| EtyJson {
                    kind: (*kind).into(),
                    mode: mode_of(*kind, edges),
                    source_lang: source_lang_json(*kind),
                    inferred: edges.first().is_some_and(|e| e.basis().is_rule()),
                    source: edges
                        .first()
                        .is_some_and(|e| e.basis() == EdgeBasis::Source),
                    parents: edges
                        .iter()
                        .map(|e| {
                            self.etymology_node(
                                e.parent(),
                                e.order(),
                                Some(e.certainty()),
                                req_lang,
                            )
                        })
                        .collect(),
                })
                .collect(),
            primary,
            reading: self
                .graph
                .etymologies(item_id)
                .map(|e| e.reading)
                .filter(|r| *r != Reading::default())
                .map(reading_json),
        }
    }

    /// An item's etymologies with who asserts each, where, and on what works.
    #[must_use]
    pub fn item_origin_json(&self, item_id: ItemId) -> OriginJson<'_> {
        let (etys, primary) = self.etymologies_of(item_id);
        let hypotheses = etys
            .iter()
            .enumerate()
            .map(|(n, (kind, edges))| {
                let asserted = u8::try_from(n)
                    .ok()
                    .and_then(|n| self.graph.asserted(item_id, n));
                // an unknown origin is stated in the word's own section
                let basis = edges.first().map_or(EdgeBasis::Template, |e| e.basis());
                let rule = assertions::rule(basis);
                let ety = u8::try_from(n).unwrap_or(u8::MAX);
                let agent = |i: u32| self.graph.agents.get(i as usize);
                let claims = self
                    .graph
                    .claims
                    .get(&item_id)
                    .into_iter()
                    .flatten()
                    .filter(|c| c.ety == ety)
                    .map(|c| ClaimJson {
                        by: agent(c.agent).map_or("", |a| &a.id),
                        by_name: agent(c.agent).map_or("", |a| &a.name),
                        stance: c.stance.into(),
                        relation: c.relation.into(),
                        certainty: (c.certainty != Certainty::Stated).then(|| c.certainty.into()),
                        mode: c.mode.map(|m| m.as_str()),
                        compiled: c.compiled,
                        locator: &c.locator,
                        page: claim_page(&c.source, &c.locator),
                    })
                    .collect();
                HypothesisJson {
                    kind: (*kind).into(),
                    mode: mode_of(*kind, edges),
                    source_lang: source_lang_json(*kind),
                    parents: edges
                        .iter()
                        .map(|e| LinkJson {
                            item: self.item_json_inner(e.parent(), true),
                            certainty: (e.certainty() != Certainty::Stated)
                                .then(|| e.certainty().into()),
                            head: e.head(),
                        })
                        .collect(),
                    claims,
                    asserted: (basis != EdgeBasis::Source).then(|| AssertionJson {
                        by: rule.unwrap_or("wiktionary-editors"),
                        by_name: rule.map_or("Wiktionary's editors", assertions::rule_name),
                        following: asserted
                            .and_then(|a| a.origin)
                            .and_then(agent)
                            .map(|a| &*a.name),
                        words: rule
                            .map(|_| assertions::compiled_words(asserted.map_or(0, |a| a.inputs))),
                        page: self.item_json_inner(
                            asserted.and_then(|a| a.page).unwrap_or(item_id),
                            true,
                        ),
                        section: assertions::section(basis),
                        credits: asserted
                            .map(|a| &a.credits[..])
                            .unwrap_or_default()
                            .iter()
                            .map(|c| {
                                let work = &self.graph.works[c.work as usize];
                                CreditJson {
                                    citation: &work.citation,
                                    year: work.year,
                                    locator: c.locator.as_deref(),
                                    role: c.role.into(),
                                    scope: c.scope.into(),
                                }
                            })
                            .collect(),
                    }),
                }
            })
            .collect();
        OriginJson {
            primary,
            reading: reading_json(
                self.graph
                    .etymologies(item_id)
                    .map(|e| e.reading)
                    .unwrap_or_default(),
            ),
            hypotheses,
        }
    }
}

/// How an etymology's word came from its source: its links' mode, or the
/// mode of one naming only a language.
fn mode_of(kind: EtyKind, edges: &[EtyEdge<'_>]) -> Option<&'static str> {
    match kind {
        EtyKind::LanguageOnly(l) => Some(l.mode.as_str()),
        EtyKind::Derivation | EtyKind::Unknown => edges.first().map(|e| e.mode().as_str()),
    }
}

/// The language or family an etymology names without a word, if it does.
fn source_lang_json(kind: EtyKind) -> Option<SourceLangJson> {
    let EtyKind::LanguageOnly(l) = kind else {
        return None;
    };
    let (code, name, family) = match l.source {
        SourceLang::Lang(lang) => (lang.code(), lang.name().to_owned(), false),
        SourceLang::Family(f) => (f.code(), f.name(), true),
    };
    Some(SourceLangJson {
        code,
        name,
        family,
        certainty: certainty_json(l.certainty),
    })
}

fn certainty_json(certainty: Certainty) -> Option<&'static str> {
    (certainty != Certainty::Stated).then(|| certainty.into())
}

fn reading_json(reading: Reading) -> ReadingJson {
    ReadingJson {
        tier: reading.tier.into(),
        disputed: reading.disputed,
    }
}

/// How a descendants or cognates tree is drawn.
#[derive(Clone, Copy)]
pub struct TreeView<'a> {
    /// The language distances are measured from.
    pub dist_lang: Lang,
    /// The languages drawn; empty: every language.
    pub desc_langs: &'a [Lang],
    /// Whether words formed within a language are drawn (see
    /// `Data::item_descendants_json`).
    pub within: bool,
    /// Items whose children are all kept: the requested item's ancestors
    /// within `desc_langs`, so the path to it stays visible.
    pub keep: &'a [ItemId],
    /// Whether items leave out their glosses.
    pub lean: bool,
}

#[derive(Default)]
struct LangData {
    lang: Lang,
    items: usize,
}

pub struct Search {
    normalized_langs: HashMap<String, LangData>,
    langs: Corpus,
    terms: HashMap<Lang, TrieMap<u8, Vec<ItemId>>>,
}

/// A term as a reader types it: lowercase, without diacritics (Latin pāter as
/// "pater"), modifier letters as plain ones (ʰ as h), subscript digits as
/// digits (h₂ as h2) and no reconstruction asterisk. Only the diacritic blocks
/// are stripped, so vowel signs of Indic scripts stay.
fn fold_term(term: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    term.trim_start_matches('*')
        .nfd()
        .filter(|c| !matches!(*c, '\u{0300}'..='\u{036F}' | '\u{1DC0}'..='\u{1DFF}'))
        .map(|c| match c {
            'ʰ' => 'h',
            'ʷ' => 'w',
            'ʲ' => 'j',
            '₀'..='₉' => char::from_digit(c as u32 - '₀' as u32, 10).expect("a digit"),
            _ => c,
        })
        .flat_map(char::to_lowercase)
        .collect()
}

fn normalize_lang_name(name: &str) -> String {
    name.chars()
        .filter(|c| !matches!(c, '(' | ')'))
        .map(|c| match c {
            '-' => ' ',
            _ => c.to_ascii_lowercase(),
        })
        .collect()
}

impl Data {
    #[must_use]
    pub fn build_search(&self) -> Search {
        let t = Instant::now();
        println!("Building search tries...");
        let mut normalized_langs = HashMap::<String, LangData>::default();
        let mut langs = CorpusBuilder::new()
            .arity(4)
            .pad_full(Pad::Auto)
            .key_trans(Box::new(normalize_lang_name))
            .finish();
        // Pre-aggregate items by (lang, lowered_term) since the map trie
        // only stores one value per key.
        let mut term_items = HashMap::<Lang, HashMap<String, Vec<ItemId>>>::default();
        for (item_id, item) in self.graph.iter().filter(|(_, item)| !item.is_imputed()) {
            let norm_lang = normalize_lang_name(item.lang().name());
            let lang_terms = term_items.entry(item.lang()).or_default();
            // Found by its spelling as a reader types it, and by its
            // romanization (Greek πατήρ by "pater").
            let term = item.term().resolve(&self.string_pool);
            let romanization = item.romanization().map(|r| r.resolve(&self.string_pool));
            // The trie takes no empty key (a term that is all diacritics).
            for key in std::iter::once(term)
                .chain(romanization)
                .map(fold_term)
                .filter(|k| !k.is_empty())
                .unique()
            {
                lang_terms.entry(key).or_default().push(item_id);
            }
            if let Some(lang_data) = normalized_langs.get_mut(&norm_lang) {
                lang_data.items += 1;
            } else {
                normalized_langs.insert(
                    norm_lang,
                    LangData {
                        lang: item.lang(),
                        items: 1,
                    },
                );
                langs.add_text(item.lang().name());
            }
        }
        let mut terms = HashMap::<Lang, TrieMap<u8, Vec<ItemId>>>::default();
        for (lang, lang_terms) in term_items {
            let mut builder = TrieMapBuilder::new();
            for (term, ids) in lang_terms {
                builder.push(term, ids);
            }
            terms.insert(lang, builder.build());
        }
        println!("Finished. Took {:#?}.", t.elapsed());
        Search {
            normalized_langs,
            langs,
            terms,
        }
    }
}

const MAX_LANG_RESULTS: usize = 20;

impl Search {
    /// Languages whose name is the query, starts with it, or has a word
    /// starting with it ("fren" finds Old French), the largest first; then
    /// near spellings.
    #[must_use]
    pub fn langs(&self, lang: &str) -> Value {
        let query = normalize_lang_name(lang.trim());
        let rank = |name: &str| {
            if name == query {
                Some(0)
            } else if name.starts_with(&query) {
                Some(1)
            } else if name.split(' ').any(|w| w.starts_with(&query)) {
                Some(2)
            } else {
                None
            }
        };
        let mut named = self
            .normalized_langs
            .iter()
            .filter_map(|(name, data)| rank(name).map(|r| (r, data)))
            .collect_vec();
        named.sort_unstable_by(|a, b| a.0.cmp(&b.0).then(b.1.items.cmp(&a.1.items)));
        let mut near = self
            .langs
            .search(lang, 0.4)
            .iter()
            .filter(|r| rank(&r.text).is_none())
            .filter_map(|r| {
                self.normalized_langs
                    .get(&r.text)
                    .map(|lang_data| (r.similarity, lang_data))
            })
            .collect_vec();
        near.sort_unstable_by(|a, b| {
            if (a.0 - b.0).abs() < 0.1 {
                b.1.items.cmp(&a.1.items)
            } else {
                b.0.total_cmp(&a.0)
            }
        });
        let matches = named
            .iter()
            .map(|(_, d)| d)
            .chain(near.iter().map(|(_, d)| d))
            .take(MAX_LANG_RESULTS)
            .map(|lang_data| lang_data.lang.json())
            .collect_vec();
        json!(matches)
    }
}

const MAX_SEARCH_RESULTS: usize = 20;
/// Prefix matches considered before ranking, so headwords further along in
/// the trie can outrank inflected forms that sort earlier.
const SEARCH_CANDIDATES: usize = 300;

impl Search {
    /// Words in `lang` starting with `term` as a reader types it (see
    /// `fold_term`). The exact spelling comes first (then in another case,
    /// then with other diacritics), then headwords before
    /// forms of other words, words with an etymology or descendants before
    /// bare entries, then shorter words.
    #[must_use]
    pub fn items(&self, data: &Data, lang: Lang, term: &str) -> Value {
        let folded = fold_term(term);
        let typed = term.trim_start_matches('*');
        let typed_lower = typed.to_lowercase();
        let mut items: Vec<ItemId> = Vec::new();
        if folded.is_empty() {
            return json!(items);
        }
        if let Some(lang_trie) = self.terms.get(&lang) {
            // predictive_search returns results in lexicographic byte order,
            // so shorter terms come first. Break early once we have enough.
            for (_, item_ids) in lang_trie.predictive_search::<String, _>(&folded) {
                items.extend(item_ids.iter().copied());
                if items.len() >= SEARCH_CANDIDATES {
                    break;
                }
            }
        }
        let items = items.into_iter().unique().collect_vec();
        let key = |id: ItemId| {
            let t = data.term(id);
            let exact = if t == typed {
                0
            } else if t.to_lowercase() == typed_lower {
                1
            } else if fold_term(t) == folded {
                2
            } else {
                3
            };
            let ety = data.graph.immediate_ety(id);
            let form = ety.as_ref().is_some_and(|e| e.mode == EtyMode::Form);
            let bare = ety.is_none() && data.graph.child_edges(id).next().is_none();
            (exact, form, bare, t.chars().count(), t, data.ety_num(id))
        };
        let mut items = items.into_iter().map(|id| (key(id), id)).collect_vec();
        items.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        items.truncate(MAX_SEARCH_RESULTS);
        json!(
            items
                .iter()
                .map(|&(_, id)| data.item_json(id))
                .collect_vec()
        )
    }
}

fn validate_reference_fingerprint(value: Option<&str>, allow_legacy: bool) -> Result<()> {
    match value {
        Some(env!("WETY_REFERENCE_FINGERPRINT")) => Ok(()),
        Some(_) => anyhow::bail!(
            "Graph language/POS tables differ from this binary; rebuild with the graph's pinned references"
        ),
        None if allow_legacy => {
            eprintln!(
                "Loading legacy graph with unverified language/POS identity by explicit opt-in"
            );
            Ok(())
        }
        None => anyhow::bail!(
            "Legacy graph has no reference fingerprint; use WETY_ALLOW_LEGACY_GRAPH=1 only after accepting unverified numeric IDs"
        ),
    }
}

#[cfg(test)]
mod reference_tests {
    use super::validate_reference_fingerprint;
    #[test]
    fn reject_mismatched_and_unidentified_tables() {
        assert!(
            validate_reference_fingerprint(Some(env!("WETY_REFERENCE_FINGERPRINT")), false).is_ok()
        );
        assert!(validate_reference_fingerprint(Some("different"), true).is_err());
        assert!(validate_reference_fingerprint(None, false).is_err());
        assert!(validate_reference_fingerprint(None, true).is_ok());
    }
}

/// The page a claim's source shows it on, with the site's name, by the source's
/// locator scheme: LiLa's EDL names an etymology (`etymology/1352`), WOLD a
/// word (`word/72141734767181903`), the other CLLD sources a row of their CLDF
/// tables, of which a cognate set (or the ACD's etymon, the CSD's entry) has a
/// page (`cognatesets.csv#24748_1`). The MCD has no web edition to link to.
fn claim_page(source: &str, locator: &str) -> Option<SourcePageJson> {
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let page = |site: &'static str, url: String| Some(SourcePageJson { site, url });
    // a cognate set's id, without a compiled link's member (`#5083:1-2-3`) or an
    // ACD set's subset (`#24748_1`)
    let set = locator
        .strip_prefix("cognatesets.csv#")
        .map(|id| id.split_once(':').map_or(id, |(set, _)| set));
    match source {
        "lila-edl" => {
            let n = locator.strip_prefix("etymology/").filter(|n| digits(n))?;
            page(
                "LiLa",
                format!("http://lila-erc.eu/data/lexicalResources/BrillEDL/id/etymology/{n}"),
            )
        }
        "acd" => {
            let n = set
                .map(|id| id.split_once('_').map_or(id, |(set, _)| set))
                .or_else(|| locator.strip_prefix("etyma.csv#"))
                .filter(|n| digits(n))?;
            page("ACD", format!("https://acd.clld.org/cognatesets/{n}"))
        }
        "wold" => {
            let n = locator.strip_prefix("word/").filter(|n| digits(n))?;
            page("WOLD", format!("https://wold.clld.org/word/{n}"))
        }
        "tlopo" => page(
            "TLOPO",
            format!("https://tlopo.clld.org/parameters/{}", set?),
        ),
        "csd" => page("CSD", format!("https://csd.clld.org/parameters/{}", set?)),
        "cariban" => page(
            "Cariban DB",
            format!("https://cariban.clld.org/cognateset/{}", set?),
        ),
        "iecor" => page(
            "IE-CoR",
            format!("https://iecor.clld.org/cognatesets/{}", set?),
        ),
        _ => None,
    }
}

#[cfg(test)]
mod claim_page_tests {
    use super::claim_page;

    #[test]
    fn locators_that_name_a_page_link_to_it() {
        let page = |s: &str, l: &str| claim_page(s, l).map(|p| (p.site, p.url));
        assert_eq!(
            page("lila-edl", "etymology/1352"),
            Some((
                "LiLa",
                "http://lila-erc.eu/data/lexicalResources/BrillEDL/id/etymology/1352".into()
            ))
        );
        assert_eq!(
            page("acd", "cognatesets.csv#24748_1"),
            Some(("ACD", "https://acd.clld.org/cognatesets/24748".into()))
        );
        assert_eq!(
            page("acd", "etyma.csv#24758"),
            Some(("ACD", "https://acd.clld.org/cognatesets/24758".into()))
        );
        assert_eq!(
            page("tlopo", "cognatesets.csv#1-3-3-3-48-POc-rumaq-a"),
            Some((
                "TLOPO",
                "https://tlopo.clld.org/parameters/1-3-3-3-48-POc-rumaq-a".into()
            ))
        );
        assert_eq!(
            page("iecor", "cognatesets.csv#5083:3-11-1"),
            Some(("IE-CoR", "https://iecor.clld.org/cognatesets/5083".into()))
        );
        assert_eq!(
            page("wold", "word/72141734767181903"),
            Some((
                "WOLD",
                "https://wold.clld.org/word/72141734767181903".into()
            ))
        );
        assert_eq!(page("mcd", "cognatesets.csv#3073"), None);
        assert_eq!(page("acd", "cognates.csv#560"), None);
        assert_eq!(page("wiktionary", "wiktionary:la:domus#Etymology"), None);
    }
}

#[cfg(test)]
mod search_tests {
    use super::fold_term;

    #[test]
    fn terms_fold_to_what_a_reader_types() {
        assert_eq!(fold_term("pāternitās"), "paternitas");
        assert_eq!(fold_term("*bʰer-"), "bher-");
        assert_eq!(fold_term("ph₂tḗr"), "ph2ter");
        assert_eq!(fold_term("Πατήρ"), "πατηρ");
        // Indic vowel signs are part of the letter, not diacritics.
        assert_eq!(fold_term("शर्करा"), "शर्करा");
    }
}

#[cfg(test)]
mod lineage_tests {
    use super::*;
    use crate::{ety_graph::EdgeBasis, items::ImputedItem, langterm::Term};
    use std::str::FromStr;

    fn add(graph: &mut EtyGraph, pool: &mut StringPool, code: &str, term: &str) -> ItemId {
        graph.add(Item::Imputed(ImputedItem {
            ety_num: 1,
            lang: Lang::from_str(code).unwrap(),
            term: Term::new(pool, term),
            romanization: None,
            from: ItemId::from(0),
        }))
    }

    fn ety(graph: &mut EtyGraph, item: ItemId, mode: EtyMode, sources: &[ItemId]) {
        let head = (sources.len() == 1).then_some(0);
        graph.add_ety(
            item,
            mode,
            head,
            EdgeBasis::Template,
            sources,
            &vec![1.0; sources.len()],
        );
    }

    fn view(dist_lang: Lang, desc_langs: &[Lang], within: bool) -> TreeView<'_> {
        TreeView {
            dist_lang,
            desc_langs,
            within,
            keep: &[],
            lean: false,
        }
    }

    #[test]
    fn the_trail_follows_the_same_word_back_to_its_formation() {
        let (mut graph, mut pool) = (EtyGraph::default(), StringPool::default());
        let mother = add(&mut graph, &mut pool, "en", "mother");
        let moder = add(&mut graph, &mut pool, "enm", "moder");
        let mater = add(&mut graph, &mut pool, "ine-pro", "méh₂tēr");
        let base = add(&mut graph, &mut pool, "ine-pro", "méh₂");
        let suffix = add(&mut graph, &mut pool, "ine-pro", "-tḗr");
        let matrix = add(&mut graph, &mut pool, "la", "mātrīx");
        ety(&mut graph, mother, EtyMode::Inherited, &[moder]);
        ety(&mut graph, moder, EtyMode::Inherited, &[mater]);
        ety(&mut graph, mater, EtyMode::Suffix, &[base, suffix]);
        ety(&mut graph, matrix, EtyMode::Suffix, &[mater, suffix]);
        let data = Data::new(pool, graph);
        assert_eq!(data.lineage(mother), [mother, moder, mater]);

        let en = Lang::from_str("en").unwrap();
        let la = Lang::from_str("la").unwrap();
        let cognates = data.item_cognates_json(mother, None, &view(en, &[en, la], false));
        // No source of *méh₂tēr is a root entry, so the tree starts there.
        assert_eq!((cognates.root, cognates.default_root), (mater, mater));
        assert_eq!(cognates.trail.len(), 3);
        assert_eq!(cognates.trail[0].ety_mode, Some("inherited"));
        assert_eq!(cognates.trail[2].ety_mode, Some("suffix"));
        assert_eq!(
            cognates
                .sources
                .iter()
                .map(|ety| ety.iter().map(|s| s.id).collect_vec())
                .collect_vec(),
            [[base, suffix]]
        );
        let children = cognates
            .tree
            .children
            .iter()
            .map(|c| c.item.id)
            .collect_vec();
        assert!(children.contains(&moder) && children.contains(&matrix));

        // A source of the oldest form is a valid starting point; an unrelated
        // word is not.
        assert_eq!(
            data.ancestor_named(mother, "ine-pro", "-tḗr", None),
            Some(suffix)
        );
        assert_eq!(data.ancestor_named(mother, "la", "mātrīx", None), None);
        let from_suffix =
            data.item_cognates_json(mother, Some(suffix), &view(en, &[en, la], false));
        assert_eq!(from_suffix.root, suffix);
        assert_eq!(from_suffix.tree.item.id, suffix);
    }

    #[test]
    fn a_form_built_on_a_root_starts_the_tree_at_the_root() {
        let (mut graph, mut pool) = (EtyGraph::default(), StringPool::default());
        let pater = add(&mut graph, &mut pool, "la", "pater");
        let ph2ter = add(&mut graph, &mut pool, "ine-pro", "ph₂tḗr");
        let peh2 = add(&mut graph, &mut pool, "ine-pro", "peh₂-");
        let ter = add(&mut graph, &mut pool, "ine-pro", "-tḗr");
        let pastor = add(&mut graph, &mut pool, "la", "pāstor");
        ety(&mut graph, pater, EtyMode::Inherited, &[ph2ter]);
        ety(&mut graph, ph2ter, EtyMode::Suffix, &[peh2, ter]);
        ety(&mut graph, pastor, EtyMode::Inherited, &[peh2]);
        let data = Data::new(pool, graph);
        assert_eq!(data.lineage(pater), [pater, ph2ter]);
        assert_eq!(data.default_root(pater), peh2);
        let la = Lang::from_str("la").unwrap();
        let cognates = data.item_cognates_json(pater, None, &view(la, &[la], false));
        assert_eq!((cognates.root, cognates.default_root), (peh2, peh2));
        assert!(cognates.tree.children.iter().any(|c| c.item.id == pastor));
        // The suffix is not a root entry: an affix alone never becomes the default.
        assert_eq!(
            data.ancestor_named(pater, "ine-pro", "-tḗr", None),
            Some(ter)
        );
    }

    #[test]
    fn the_tree_starts_as_far_back_as_the_head_line_goes() {
        let (mut graph, mut pool) = (EtyGraph::default(), StringPool::default());
        let tabula_en = add(&mut graph, &mut pool, "en", "tabula");
        let tabula = add(&mut graph, &mut pool, "la", "tabula");
        let tadla = add(&mut graph, &mut pool, "itc-pro", "taðlā");
        let stem = add(&mut graph, &mut pool, "ine-pro", "(s)th₂-dʰló-m");
        let steh2 = add(&mut graph, &mut pool, "ine-pro", "steh₂-");
        let suffix = add(&mut graph, &mut pool, "ine-pro", "-dʰlom");
        ety(&mut graph, tabula_en, EtyMode::LearnedBorrowing, &[tabula]);
        ety(&mut graph, tabula, EtyMode::Inherited, &[tadla]);
        ety(&mut graph, tadla, EtyMode::UndefinedDerivation, &[stem]);
        ety(&mut graph, stem, EtyMode::Suffix, &[steh2, suffix]);
        let data = Data::new(pool, graph);
        // The trail stops at the new formation; the tree goes on to the root.
        assert_eq!(data.lineage(tabula_en), [tabula_en, tabula, tadla]);
        assert_eq!(data.default_root(tabula_en), steh2);
        let en = Lang::from_str("en").unwrap();
        let c = data.item_cognates_json(tabula_en, None, &view(en, &[en], false));
        assert_eq!((c.root, c.default_root), (steh2, steh2));
        assert_eq!(c.ultimate_roots.iter().map(|i| i.id).collect_vec(), [steh2]);

        // A suffix that is a word's only source does not start it.
        let (mut graph, mut pool) = (EtyGraph::default(), StringPool::default());
        let mother = add(&mut graph, &mut pool, "en", "mother");
        let mater = add(&mut graph, &mut pool, "ine-pro", "méh₂tēr");
        let suffix = add(&mut graph, &mut pool, "ine-pro", "-tḗr");
        ety(&mut graph, mother, EtyMode::Inherited, &[mater]);
        ety(&mut graph, mater, EtyMode::Suffix, &[suffix]);
        assert_eq!(Data::new(pool, graph).default_root(mother), mater);

        // Nor does a possible source.
        let (mut graph, mut pool) = (EtyGraph::default(), StringPool::default());
        let wolf = add(&mut graph, &mut pool, "gem-pro", "wulfaz");
        let wlkwos = add(&mut graph, &mut pool, "ine-pro", "wĺ̥kʷos");
        let source = add(&mut graph, &mut pool, "ine-pro", "wl̥kʷós");
        ety(&mut graph, wolf, EtyMode::Inherited, &[wlkwos]);
        ety(
            &mut graph,
            wlkwos,
            EtyMode::MorphologicalDerivation,
            &[source],
        );
        for e in graph.graph.edge_weights_mut() {
            if e.mode == EtyMode::MorphologicalDerivation {
                e.certainty = Certainty::Possible;
            }
        }
        assert_eq!(Data::new(pool, graph).default_root(wolf), wlkwos);
    }

    #[test]
    fn ultimate_ancestors_come_in_etymology_order_roots_first() {
        let (mut graph, mut pool) = (EtyGraph::default(), StringPool::default());
        let astronomy = add(&mut graph, &mut pool, "en", "astronomy");
        let astronomia = add(&mut graph, &mut pool, "grc", "ἀστρονομία");
        let astron = add(&mut graph, &mut pool, "grc", "ἄστρον");
        let nome = add(&mut graph, &mut pool, "grc", "νομή");
        let star = add(&mut graph, &mut pool, "ine-pro", "h₂stḗr");
        let burn = add(&mut graph, &mut pool, "ine-pro", "h₂eHs-");
        let ter = add(&mut graph, &mut pool, "ine-pro", "-tḗr");
        let nem = add(&mut graph, &mut pool, "ine-pro", "nem-");
        let eh2 = add(&mut graph, &mut pool, "ine-pro", "-eh₂");
        ety(&mut graph, astronomy, EtyMode::Borrowed, &[astronomia]);
        ety(&mut graph, astronomia, EtyMode::Compound, &[astron, nome]);
        ety(&mut graph, astron, EtyMode::Inherited, &[star]);
        ety(&mut graph, star, EtyMode::Suffix, &[burn, ter]);
        ety(&mut graph, nome, EtyMode::Suffix, &[nem, eh2]);
        let data = Data::new(pool, graph);
        assert_eq!(data.ultimate_sources(astronomy), [burn, ter, nem, eh2]);
        let en = Lang::from_str("en").unwrap();
        let c = data.item_cognates_json(astronomy, None, &view(en, &[en], false));
        assert_eq!(c.root, astronomia);
        assert_eq!(
            c.ultimate_roots.iter().map(|i| i.id).collect_vec(),
            [burn, nem]
        );
        assert_eq!(
            c.ultimate_others.iter().map(|i| i.id).collect_vec(),
            [ter, eh2]
        );
        // An ultimate form is an ancestor, so a link can start the tree there.
        assert_eq!(
            data.ancestor_named(astronomy, "ine-pro", "nem-", None),
            Some(nem)
        );
    }

    #[test]
    fn meaning_loans_and_several_sources_end_the_trail() {
        let (mut graph, mut pool) = (EtyGraph::default(), StringPool::default());
        let halbinsel = add(&mut graph, &mut pool, "de", "Halbinsel");
        let paeninsula = add(&mut graph, &mut pool, "la", "paenīnsula");
        let word = add(&mut graph, &mut pool, "en", "word");
        let fr = add(&mut graph, &mut pool, "fr", "mot");
        let la = add(&mut graph, &mut pool, "la", "verbum");
        ety(&mut graph, halbinsel, EtyMode::Calque, &[paeninsula]);
        ety(&mut graph, word, EtyMode::Borrowed, &[fr, la]);
        let data = Data::new(pool, graph);
        assert_eq!(data.lineage(halbinsel), [halbinsel]);
        assert_eq!(data.lineage(word), [word]);
    }

    #[test]
    fn derived_and_inherited_steps_reach_roots_and_stems() {
        let (mut graph, mut pool) = (EtyGraph::default(), StringPool::default());
        let lime = add(&mut graph, &mut pool, "en", "lime");
        let limaz = add(&mut graph, &mut pool, "gem-pro", "līmaz");
        let root = add(&mut graph, &mut pool, "ine-pro", "(s)ley-");
        let two = add(&mut graph, &mut pool, "ang", "twā");
        let stem = add(&mut graph, &mut pool, "gmw-pro", "twai-");
        let dwoh = add(&mut graph, &mut pool, "ine-pro", "dwóh₁");
        let hound = add(&mut graph, &mut pool, "gem-pro", "hundaz");
        let kwon = add(&mut graph, &mut pool, "ine-pro", "ḱwṓ");
        ety(&mut graph, lime, EtyMode::Inherited, &[limaz]);
        ety(&mut graph, limaz, EtyMode::Derived, &[root]);
        ety(&mut graph, two, EtyMode::Inherited, &[stem]);
        ety(&mut graph, stem, EtyMode::Inherited, &[dwoh]);
        ety(&mut graph, hound, EtyMode::Derived, &[kwon]);
        let data = Data::new(pool, graph);
        assert_eq!(data.lineage(lime), [lime, limaz, root]);
        assert_eq!(data.lineage(two), [two, stem, dwoh]);
        assert_eq!(data.lineage(hound), [hound, kwon]);
    }

    #[test]
    fn no_languages_means_every_language() {
        let (mut graph, mut pool) = (EtyGraph::default(), StringPool::default());
        let pater = add(&mut graph, &mut pool, "la", "pater");
        let paternus = add(&mut graph, &mut pool, "la", "paternus");
        let pere = add(&mut graph, &mut pool, "fr", "père");
        let suffix = add(&mut graph, &mut pool, "la", "-nus");
        ety(&mut graph, paternus, EtyMode::Suffix, &[pater, suffix]);
        ety(&mut graph, pere, EtyMode::Inherited, &[pater]);
        let data = Data::new(pool, graph);
        let la = Lang::from_str("la").unwrap();
        let fr = Lang::from_str("fr").unwrap();
        let ids = |t: &DescendantsJson<'_>| t.children.iter().map(|c| c.item.id).collect_vec();
        // A same-language child with no descendants elsewhere drops out of a
        // filtered tree, but not out of the every-language one.
        let filtered = data.item_descendants_json(pater, &view(la, &[fr, la], false));
        assert_eq!(ids(&filtered), [pere]);
        let all = data.item_descendants_json(pater, &view(la, &[], true));
        assert_eq!(all.children.len(), 2);
        assert!(ids(&all).contains(&paternus));
        // Without `within`, a word formed within Latin drops out of the
        // every-language tree too, unless something below it is elsewhere...
        let all_without = data.item_descendants_json(pater, &view(la, &[], false));
        assert_eq!(ids(&all_without), [pere]);
        // ...and `within` brings it back into a filtered one.
        let filtered_within = data.item_descendants_json(pater, &view(la, &[fr, la], true));
        assert!(ids(&filtered_within).contains(&paternus));
        assert_eq!(data.descendant_total(pater, TOTAL_CAP), 2);
        assert_eq!(data.descendant_total(pater, 1), 1);
    }

    #[test]
    fn alternatives_are_drawn_apart_and_say_who_asserts_them() {
        use crate::{
            assertions::{Asserted, Credit, Role, Scope, Work},
            ety_graph::{Certainty, NewEtymology},
        };
        let (mut graph, mut pool) = (EtyGraph::default(), StringPool::default());
        let doubt = add(&mut graph, &mut pool, "en", "doubt");
        let douten = add(&mut graph, &mut pool, "enm", "douten");
        let doute = add(&mut graph, &mut pool, "enm", "doute");
        // "from douten, or possibly doute"
        graph.add_etymologies(
            doubt,
            &[
                NewEtymology {
                    mode: EtyMode::Inherited,
                    head: Some(0),
                    basis: EdgeBasis::Etymon,
                    items: &[douten],
                    confidences: &[1.0],
                    certainties: &[Certainty::Stated],
                },
                NewEtymology {
                    mode: EtyMode::Inherited,
                    head: Some(0),
                    basis: EdgeBasis::Etymon,
                    items: &[doute],
                    confidences: &[1.0],
                    certainties: &[Certainty::Possible],
                },
            ],
        );
        graph.works.push(Work {
            id: "work:wiktionary:R:OED".into(),
            citation: "OED".into(),
            year: None,
        });
        let credited = Asserted {
            credits: Box::new([Credit {
                work: 0,
                role: Role::Follows,
                scope: Scope::Entry,
                locator: None,
            }]),
            ..Asserted::default()
        };
        graph.set_asserted(doubt, vec![credited.clone(), credited]);
        let unk = add(&mut graph, &mut pool, "ro", "prăștilă");
        graph.set_unknown(unk);
        let data = Data::new(pool, graph);
        let en = Lang::from_str("en").unwrap();

        let chart = data.item_etymology_json(doubt, en);
        assert_eq!(chart.primary, Some(0));
        let alts: Vec<_> = chart
            .etymologies
            .iter()
            .map(|e| (e.mode, e.parents.len(), e.parents[0].certainty))
            .collect();
        assert_eq!(
            alts,
            [
                (Some("inherited"), 1, None),
                (Some("inherited"), 1, Some("possible"))
            ]
        );
        let unknown = data.item_etymology_json(unk, en);
        assert_eq!(unknown.etymologies[0].kind, "unknown");

        let origin = data.item_origin_json(doubt);
        assert_eq!(origin.hypotheses.len(), 2);
        let asserted = origin.hypotheses[1].asserted.as_ref().unwrap();
        assert_eq!(
            (asserted.by, asserted.section, asserted.page.id),
            ("wiktionary-editors", "Etymology", doubt)
        );
        assert_eq!(
            (asserted.credits[0].citation, asserted.credits[0].scope),
            ("OED", "entry")
        );

        // under douten, doute is another alternative, not a co-parent
        let tree = data.item_descendants_json(douten, &view(en, &[], true));
        let row = &tree.children[0];
        assert!(row.other_parents.is_empty());
        assert_eq!((row.standing, row.certainty), (Some("preferred"), None));
        let alt = &row.alternatives[..];
        assert_eq!(alt.len(), 1);
        assert_eq!(
            (
                alt[0].preferred,
                alt[0].parents[0].item.id,
                alt[0].parents[0].certainty
            ),
            (false, doute, Some("possible"))
        );
        // under doute, doubt is possibly related: drawn, marked, not expanded
        let tree = data.item_descendants_json(doute, &view(en, &[], true));
        let row = &tree.children[0];
        assert_eq!(row.item.id, doubt);
        assert_eq!(
            (row.standing, row.certainty),
            (Some("other"), Some("possible"))
        );
        assert_eq!(row.ety_mode, Some("inherited"));
        assert!(row.alternatives[0].preferred);
    }

    #[test]
    fn a_word_with_no_preferred_etymology_is_drawn_under_each() {
        use crate::ety_graph::{Certainty, NewEtymology};
        let (mut graph, mut pool) = (EtyGraph::default(), StringPool::default());
        let marhaz = add(&mut graph, &mut pool, "gem-pro", "marhaz");
        let miere = add(&mut graph, &mut pool, "ang", "mīere");
        let mearh = add(&mut graph, &mut pool, "ang", "mearh");
        let mare = add(&mut graph, &mut pool, "enm", "mare");
        let mare_en = add(&mut graph, &mut pool, "en", "mare");
        ety(&mut graph, miere, EtyMode::Inherited, &[marhaz]);
        ety(&mut graph, mearh, EtyMode::Inherited, &[marhaz]);
        ety(&mut graph, mare_en, EtyMode::Inherited, &[mare]);
        let one = |items| NewEtymology {
            mode: EtyMode::Inherited,
            head: Some(0),
            basis: EdgeBasis::Template,
            items,
            confidences: &[1.0],
            certainties: &[Certainty::Stated],
        };
        // "from mīere or mearh"
        graph.add_etymologies(mare, &[one(&[miere]), one(&[mearh])]);
        let data = Data::new(pool, graph);
        let en = Lang::from_str("en").unwrap();
        let tree = data.item_descendants_json(marhaz, &view(en, &[], true));
        for (old, other) in [(miere, mearh), (mearh, miere)] {
            let under = tree.children.iter().find(|c| c.item.id == old).unwrap();
            let row = &under.children[0];
            assert_eq!((row.item.id, row.standing), (mare, Some("unsettled")));
            assert_eq!(row.alternatives[0].parents[0].item.id, other);
            assert!(!row.alternatives[0].preferred);
            assert_eq!(row.children[0].item.id, mare_en);
        }
        // as a root, it is drawn under its first, the other an alternative
        let root = data.item_descendants_json(mare, &view(en, &[], true));
        assert_eq!(root.other_parents[0].item.id, miere);
        assert_eq!(root.alternatives[0].parents[0].item.id, mearh);
        // the trail ends there, with its sources told apart
        let c = data.item_cognates_json(mare_en, None, &view(en, &[], true));
        assert_eq!(
            c.sources
                .iter()
                .map(|ety| ety.iter().map(|s| s.id).collect_vec())
                .collect_vec(),
            [[miere], [mearh]]
        );
        // and the tree starts where both alternatives' lines meet
        assert_eq!((c.root, c.default_root), (marhaz, marhaz));
    }

    #[test]
    fn alternatives_that_never_meet_start_no_deeper_tree() {
        use crate::ety_graph::{Certainty, NewEtymology};
        let (mut graph, mut pool) = (EtyGraph::default(), StringPool::default());
        let marhaz = add(&mut graph, &mut pool, "gem-pro", "marhaz");
        let miere = add(&mut graph, &mut pool, "ang", "mīere");
        let other = add(&mut graph, &mut pool, "non", "merr");
        let mare = add(&mut graph, &mut pool, "enm", "mare");
        let mare_en = add(&mut graph, &mut pool, "en", "mare");
        ety(&mut graph, miere, EtyMode::Inherited, &[marhaz]);
        ety(&mut graph, mare_en, EtyMode::Inherited, &[mare]);
        let one = |items| NewEtymology {
            mode: EtyMode::Inherited,
            head: Some(0),
            basis: EdgeBasis::Template,
            items,
            confidences: &[1.0],
            certainties: &[Certainty::Stated],
        };
        graph.add_etymologies(mare, &[one(&[miere]), one(&[other])]);
        assert_eq!(Data::new(pool, graph).default_root(mare_en), mare);
    }
}
