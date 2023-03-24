use crate::{
    descendants::RawDescendants,
    embeddings::{EmbeddingComparand, Embeddings, EmbeddingsConfig, ItemEmbedding},
    ety_graph::EtyGraph,
    etymology::RawEtymology,
    gloss::Gloss,
    lang_phf::LANG_CODE2NAME,
    langterm::{Lang, LangTerm, Term},
    phf_ext::OrderedMapExt,
    pos::Pos,
    progress_bar,
    redirects::Redirects,
    root::RawRoot,
    string_pool::{StringPool, Symbol},
    wiktextract_json::wiktextract_lines,
};

use std::{path::Path, rc::Rc};

use anyhow::{Ok, Result};
use hashbrown::{HashMap, HashSet};
use simd_json::to_borrowed_value;

pub(crate) type ItemId = usize;

#[derive(Hash, Eq, PartialEq, Debug)]
pub(crate) struct RawItem {
    pub(crate) is_imputed: bool,
    pub(crate) i: ItemId, // the i-th item seen, used as id for RDF
    pub(crate) langterm: LangTerm,
    pub(crate) page_term: Option<Term>, // i.e. the term stripped of diacritics etc. at the top of the page
    pub(crate) ety_num: Option<u8>,     // the nth numbered ety for this term-lang combo (1,2,...)
    pub(crate) pos: Option<Pos>,        // e.g. "noun"
    pub(crate) gloss: Option<Gloss>,
}

impl RawItem {
    pub(crate) fn new_imputed(i: usize, lang: usize, term: Symbol, pos: Option<usize>) -> Self {
        Self {
            is_imputed: true,
            i,
            lang,
            term,
            pos,
            page_term: None,
            ety_num: None,
            gloss: None,
        }
    }
}

#[derive(Default)]
pub(crate) struct RawItems {
    pub(crate) items: Vec<RawItem>,
    pub(crate) langterm_map: HashMap<LangTerm, Vec<ItemId>>,
    pub(crate) ety_map: HashMap<ItemId, RawEtymology>,
    pub(crate) desc_map: HashMap<ItemId, RawDescendants>,
    pub(crate) root_map: HashMap<ItemId, RawRoot>,
    pub(crate) redirects: Redirects,
    pub(crate) line_map: HashMap<usize, ItemId>,
    pub(crate) total_ok_lines_in_file: usize,
}

pub(crate) struct Retrieval {
    pub(crate) item: Rc<RawItem>,
    pub(crate) confidence: f32,
    // is_imputed: bool,
    pub(crate) is_newly_imputed: bool,
}

impl RawItems {
    pub(crate) fn len(&self) -> usize {
        self.items.len()
    }

    pub(crate) fn get(&self, id: ItemId) -> &RawItem {
        &self.items[id]
    }

    pub(crate) fn add(&mut self, item: RawItem) {
        let id = self.items.len();
        item.i = id;
        let langterm = LangTerm::new(item.lang, item.term);
        self.items.push(item);
        if let Some(ids) = self.langterm_map.get_mut(&langterm) {
            ids.push(id);
            return;
        }
        self.langterm_map.insert(langterm, vec![id]);
    }

    pub(crate) fn contains(&self, langterm: LangTerm) -> bool {
        let langterm = self.redirects.rectify_langterm(langterm);
        self.langterm_map.contains_key(&langterm)
    }

    pub(crate) fn get_disambiguated_item<'a>(
        &self,
        embeddings: &'a Embeddings,
        embedding_comp: impl EmbeddingComparand<ItemEmbedding<'a>> + Copy,
        langterm: LangTerm,
    ) -> Option<(Rc<RawItem>, f32)> {
        let langterm = self.redirects.rectify_langterm(langterm);
        let candidates = self.get_all_langterm_ids(langterm)?;
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
        Some((candidates[best_candidate].clone(), max_similarity))
    }

    pub(crate) fn get_or_impute_item<'a>(
        &self,
        ety_graph: &mut EtyGraph,
        embeddings: &'a Embeddings,
        embedding_comp: impl EmbeddingComparand<ItemEmbedding<'a>> + Copy,
        langterm: LangTerm,
    ) -> Retrieval {
        if let Some((item, confidence)) =
            self.get_disambiguated_item(embeddings, embedding_comp, langterm)
        {
            return Retrieval {
                item,
                confidence,
                // is_imputed: false,
                is_newly_imputed: false,
            };
        }
        if let Some(item) = ety_graph.imputed_items.get(langterm) {
            return Retrieval {
                item,
                confidence: 0.0,
                // is_imputed: true,
                is_newly_imputed: false,
            };
        }
        let n = self.n + ety_graph.imputed_items.n;
        let imputed_item = Rc::from(RawItem::new_imputed(n, langterm, None));
        ety_graph.add_imputed(&imputed_item);
        Retrieval {
            item: imputed_item,
            confidence: 0.0,
            // is_imputed: true,
            is_newly_imputed: true,
        }
    }

    // returns all items that share the same lang and term
    pub(crate) fn get_all_langterm_ids(&self, langterm: LangTerm) -> Option<&Vec<ItemId>> {
        self.langterm_map.get(&langterm)
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
    fn get_items_needing_embedding(&self, item: &Rc<RawItem>) -> HashSet<Rc<RawItem>> {
        let mut items_needing_embedding = HashSet::new();
        if let Some(raw_etymology) = &item.raw_etymology {
            items_needing_embedding
                .extend(self.get_ety_items_needing_embedding(item, raw_etymology));
        }
        if let Some(raw_descendants) = &item.raw_descendants {
            items_needing_embedding
                .extend(self.get_desc_items_needing_embedding(item, raw_descendants));
        }
        if let Some(raw_root) = &item.raw_root
            && let Some(root_items) = self.get_all_langterm_ids(raw_root.lang, raw_root.term)
            && root_items.len() > 1
        {
            items_needing_embedding.insert(Rc::clone(item));
            for root_item in &root_items {
                items_needing_embedding.insert(Rc::clone(root_item));
            }
        }
        items_needing_embedding
    }

    fn get_all_items_needing_embedding(&self) -> Result<HashSet<ItemId>> {
        let pb = progress_bar(self.n, "Determining which items need embeddings")?;
        let mut items_needing_embedding = HashSet::new();
        for items in self.langterm_map.values() {
            for item in items {
                let more = self.get_items_needing_embedding(item);
                for m in more.iter() {
                    items_needing_embedding.insert(item.i);
                }
                pb.inc(1);
            }
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
            if let Some(item) = self.line_map.get(&line_number)
                && items_needing_embedding.contains(item)
            {
                let json_item = to_borrowed_value(&mut line)?;
                let lang = LANG_CODE2NAME.get_expected_index_value(item.lang)?;
                let term = string_pool.resolve(item.term);
                embeddings.add(&json_item, lang, term, item.i)?;
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
