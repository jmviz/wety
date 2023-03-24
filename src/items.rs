use crate::{
    descendants::RawDescendants,
    embeddings::{EmbeddingComparand, Embeddings, EmbeddingsConfig, ItemEmbedding},
    ety_graph::EtyGraph,
    etymology::RawEtymology,
    gloss::Gloss,
    langterm::{Lang, LangTerm, Term},
    pos::Pos,
    progress_bar,
    redirects::Redirects,
    root::RawRoot,
    string_pool::StringPool,
    wiktextract_json::wiktextract_lines,
};

use std::{path::Path, rc::Rc};

use anyhow::{Ok, Result};
use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use serde_json_any_key::any_key_map;
use simd_json::to_borrowed_value;
use smallvec::{smallvec, SmallVec};

pub(crate) type ItemId = u32; // wiktionary has about ~10M items including imputations

#[derive(Hash, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub(crate) struct Item {
    pub(crate) is_imputed: bool,
    pub(crate) i: ItemId, // the i-th item seen, used as id for RDF
    pub(crate) lang: Lang,
    pub(crate) term: Term,
    pub(crate) page_term: Option<Term>, // i.e. the term stripped of diacritics etc. at the top of the page
    pub(crate) ety_num: Option<u8>,     // the nth numbered ety for this term-lang combo (1,2,...)
    pub(crate) pos: Option<Pos>,        // e.g. "noun"
    pub(crate) gloss: Option<Gloss>,
}

impl Item {
    pub(crate) fn new_imputed(langterm: LangTerm, pos: Option<Pos>) -> Self {
        Self {
            is_imputed: true,
            i: 0, // temp value, will be changed by imputed_items.store.add()
            lang: langterm.lang,
            term: langterm.term,
            pos,
            page_term: None,
            ety_num: None,
            gloss: None,
        }
    }

    pub(crate) fn langterm(&self) -> LangTerm {
        LangTerm {
            lang: self.lang,
            term: self.term,
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
pub(crate) struct ItemStore {
    start_id: ItemId,
    vec: Vec<Item>,
}

impl ItemStore {
    pub(crate) fn new(start_id: ItemId) -> Self {
        Self {
            start_id,
            ..Default::default()
        }
    }
    pub(crate) fn len(&self) -> usize {
        self.vec.len()
    }

    pub(crate) fn get_index(&self, id: ItemId) -> usize {
        (id - self.start_id) as usize
    }

    pub(crate) fn get(&self, id: ItemId) -> &Item {
        &self.vec[self.get_index(id)]
    }

    pub(crate) fn next_id(&self) -> ItemId {
        ItemId::try_from(self.len()).expect("len less than ItemId::MAX items") + self.start_id
    }

    pub(crate) fn add(&mut self, item: Item) -> ItemId {
        let id = self.next_id();
        item.i = id;
        self.vec.push(item);
        id
    }
}

pub(crate) type Dupes = SmallVec<[ItemId; 1]>; // most items don't have langterm dupes

#[derive(Default, Serialize, Deserialize)]
pub(crate) struct Items {
    pub(crate) store: ItemStore,
    #[serde(with = "any_key_map")]
    pub(crate) dupes: HashMap<LangTerm, Dupes>,
}

impl Items {
    pub(crate) fn new(start_id: ItemId) -> Self {
        Self {
            store: ItemStore::new(start_id),
            ..Default::default()
        }
    }

    pub(crate) fn next_id(&self) -> ItemId {
        self.store.next_id()
    }

    pub(crate) fn len(&self) -> usize {
        self.store.len()
    }

    pub(crate) fn get(&self, id: ItemId) -> &Item {
        &self.store.get(id)
    }

    // returns all items that share the same lang and term
    pub(crate) fn get_dupes(&self, langterm: LangTerm) -> Option<&Dupes> {
        self.dupes.get(&langterm)
    }

    pub(crate) fn add(&mut self, item: Item) {
        let langterm = item.langterm();
        let id = self.store.add(item);
        if let Some(ids) = self.dupes.get_mut(&langterm) {
            ids.push(id);
            return;
        }
        self.dupes.insert(langterm, smallvec![id]);
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &Item> {
        self.store.vec.iter()
    }
}

#[derive(Default)]
pub(crate) struct RawTemplates {
    pub(crate) ety: HashMap<ItemId, RawEtymology>,
    pub(crate) desc: HashMap<ItemId, RawDescendants>,
    pub(crate) root: HashMap<ItemId, RawRoot>,
}

#[derive(Default)]
pub(crate) struct RawItems {
    pub(crate) items: Items,
    pub(crate) redirects: Redirects,
    pub(crate) raw_templates: RawTemplates,
    pub(crate) lines: HashMap<usize, ItemId>,
    pub(crate) total_ok_lines_in_file: usize,
}

pub(crate) struct Retrieval {
    pub(crate) item_id: ItemId,
    pub(crate) confidence: f32,
    // is_imputed: bool,
    pub(crate) is_newly_imputed: bool,
}

impl RawItems {
    pub(crate) fn len(&self) -> usize {
        self.items.store.len()
    }

    pub(crate) fn get(&self, id: ItemId) -> &Item {
        &self.items.store.get(id)
    }

    pub(crate) fn contains(&self, langterm: LangTerm) -> bool {
        let langterm = self.redirects.rectify_langterm(langterm);
        self.items.dupes.contains_key(&langterm)
    }

    pub(crate) fn get_disambiguated_item_id<'a>(
        &self,
        embeddings: &'a Embeddings,
        embedding_comp: impl EmbeddingComparand<ItemEmbedding<'a>> + Copy,
        langterm: LangTerm,
    ) -> Option<(ItemId, f32)> {
        let langterm = self.redirects.rectify_langterm(langterm);
        let candidates = self.items.get_dupes(langterm)?;
        let mut max_similarity = 0f32;
        let mut best_candidate = 0usize;
        for (i, candidate) in candidates.iter().enumerate() {
            let candidate_embedding = embeddings.get(candidate);
            let similarity = embedding_comp.cosine_similarity(candidate_embedding);
            let old_max_similarity = max_similarity;
            max_similarity = max_similarity.max(similarity);
            if max_similarity > old_max_similarity {
                best_candidate = i;
            }
        }
        Some((candidates[best_candidate], max_similarity))
    }

    pub(crate) fn get_or_impute_item<'a>(
        &self,
        ety_graph: &mut EtyGraph,
        embeddings: &'a Embeddings,
        embedding_comp: impl EmbeddingComparand<ItemEmbedding<'a>> + Copy,
        langterm: LangTerm,
    ) -> Retrieval {
        if let Some((item_id, confidence)) =
            self.get_disambiguated_item_id(embeddings, embedding_comp, langterm)
        {
            return Retrieval {
                item_id,
                confidence,
                // is_imputed: false,
                is_newly_imputed: false,
            };
        }
        if let Some(item_id) = ety_graph.get_imputed_item_id(langterm) {
            return Retrieval {
                item_id,
                confidence: 0.0,
                // is_imputed: true,
                is_newly_imputed: false,
            };
        }
        let item_id = ety_graph.add_imputed(langterm, None);
        Retrieval {
            item_id,
            confidence: 0.0,
            // is_imputed: true,
            is_newly_imputed: true,
        }
    }

    // We determine that an item needs an embedding if it has any
    // raw_(descendants|etymology|root) (raw_*), since any ambiguous lang-terms
    // within those will need to have their associated items' embeddings
    // compared to the parent's embedding. Further, all of the items that
    // correspond to any lang-term in any of the raw_* also need embeddings.
    // This is because these will be acting as their own parent items during the
    // processing of raw_* for any descendants they may have, and so the
    // reasoning for the case of the original item applies for each of these as
    // well. This will end up generating a lot of embeddings, but still much
    // less than if we simply generated embeddings for every item. For example,
    // there are many, many Latin items like "reminiscebatur" which are simply
    // inflections of a main item, which have no raw_* and are extremely
    // unlikely to appear in any other item's raw_*. Our method will thus
    // disclude all these.
    fn get_items_needing_embedding(&self, item_id: ItemId) -> HashSet<ItemId> {
        let mut items_needing_embedding = HashSet::new();
        if let Some(raw_etymology) = self.raw_templates.ety.get(&item_id) {
            items_needing_embedding
                .extend(self.get_ety_items_needing_embedding(item_id, raw_etymology));
        }
        if let Some(raw_descendants) = self.raw_templates.desc.get(&item_id) {
            items_needing_embedding
                .extend(self.get_desc_items_needing_embedding(item_id, raw_descendants));
        }
        if let Some(raw_root) = self.raw_templates.root.get(&item_id)
            && let Some(root_items) = self.items.get_dupes(raw_root.langterm)
            && root_items.len() > 1
        {
            items_needing_embedding.insert(item_id);
            for &root_item in root_items {
                items_needing_embedding.insert(root_item);
            }
        }
        items_needing_embedding
    }

    fn get_all_items_needing_embedding(&self) -> Result<HashSet<ItemId>> {
        let pb = progress_bar(self.len(), "Determining which items need embeddings")?;
        let mut items_needing_embedding = HashSet::new();
        for item_id in self.items.iter().map(|item| item.i) {
            let items_to_embed = self.get_items_needing_embedding(item_id);
            for &item_to_embed in items_to_embed.iter() {
                items_needing_embedding.insert(item_to_embed);
            }
            pb.inc(1);
        }
        pb.finish();
        Ok(items_needing_embedding)
    }

    // We go through the wiktextract file again, generating embeddings for all
    // ambiguous terms we found the first time.
    pub(crate) fn generate_embeddings(
        &self,
        string_pool: &StringPool,
        wiktextract_path: &Path,
        embeddings_config: &EmbeddingsConfig,
    ) -> Result<Embeddings> {
        let mut added = 0;
        let items_needing_embedding = self.get_all_items_needing_embedding()?;
        let pb = progress_bar(items_needing_embedding.len(), "Generating embeddings")?;
        pb.inc(0);
        let mut embeddings = Embeddings::new(embeddings_config)?;
        for (line_number, mut line) in wiktextract_lines(wiktextract_path)?.enumerate() {
            // Items were only inserted into the line map if they were added to
            // the term_map in process_json_item.
            if let Some(item_id) = self.lines.get(&line_number)
                && items_needing_embedding.contains(item_id)
            {
                let json_item = to_borrowed_value(&mut line)?;
                let item = self.get(item_id);
                let lang_name = item.langterm.lang.name();
                let term = json_item.get_term().expect("known good item");
                embeddings.add(&json_item, lang.name(), term, item_id.i)?;
                added += 1;
                if added % embeddings_config.progress_update_interval == 0 {
                    pb.inc(embeddings_config.progress_update_interval as u64);
                }
            }
        }
        embeddings.flush()?;
        pb.finish();
        Ok(embeddings)
    }

    fn add_all_to_ety_graph(&self, ety_graph: &mut EtyGraph) -> Result<()> {
        let pb = progress_bar(self.n, "Adding items to ety graph")?;
        for lang_map in self.langterm_map.values() {
            for ety_map in lang_map.values() {
                for pos_map in ety_map.values() {
                    for gloss_map in pos_map.values() {
                        for item in gloss_map.values() {
                            ety_graph.add(&Rc::clone(item));
                            pb.inc(1);
                        }
                    }
                }
            }
        }

        pb.finish();
        Ok(())
    }

    pub(crate) fn generate_ety_graph(
        &self,
        string_pool: &StringPool,
        embeddings: &Embeddings,
    ) -> Result<EtyGraph> {
        let mut ety_graph = EtyGraph::default();
        self.add_all_to_ety_graph(&mut ety_graph)?;
        self.impute_root_items(&mut ety_graph)?;
        self.process_raw_descendants(embeddings, &mut ety_graph)?;
        ety_graph.remove_cycles(string_pool, 1)?;
        self.process_raw_etymologies(embeddings, &mut ety_graph)?;
        ety_graph.remove_cycles(string_pool, 2)?;
        self.impute_root_etys(embeddings, &mut ety_graph)?;
        ety_graph.remove_cycles(string_pool, 3)?;
        Ok(ety_graph)
    }
}
