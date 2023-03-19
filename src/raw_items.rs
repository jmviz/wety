use crate::{
    descendants::RawDescendants,
    embeddings::{EmbeddingComparand, Embeddings, ItemEmbedding, EMBEDDING_BATCH_SIZE},
    ety_graph::EtyGraph,
    etymology::RawEtymology,
    lang::is_reconstructed_lang,
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

#[derive(Hash, Eq, PartialEq, Debug)]
pub(crate) struct RawItem {
    pub(crate) line: Option<usize>, // the line-th ok line in the wiktextract file, if it was in the file
    pub(crate) is_imputed: bool,
    pub(crate) is_reconstructed: bool, // i.e. Reconstruction: namespace page, or an imputed item of form *term
    pub(crate) i: usize,               // the i-th item seen, used as id for RDF
    pub(crate) lang: usize,            // e.g "en", i.e. the wiktextract lang_code
    pub(crate) term: Symbol,           // e.g. "bank"
    pub(crate) page_title: Option<Symbol>, // i.e. the term stripped of diacritics etc. at the top of the page
    pub(crate) ety_num: Option<u8>, // the nth numbered ety for this term-lang combo (1,2,...)
    pub(crate) pos: Option<usize>,  // e.g. "noun"
    pub(crate) gloss: Option<Symbol>, // e.g. "An institution where one can place and borrow money...
    pub(crate) gloss_num: u8,         // the nth gloss encountered for this term-lang-ety-pos combo
    pub(crate) raw_etymology: Option<RawEtymology>,
    pub(crate) raw_root: Option<RawRoot>,
    pub(crate) raw_descendants: Option<RawDescendants>,
}

impl RawItem {
    pub(crate) fn new_imputed(i: usize, lang: usize, term: Symbol, pos: Option<usize>) -> Self {
        Self {
            line: None,
            is_imputed: true,
            // $$ This will not catch all reconstructed terms, since some terms
            // in attested languages are reconstructed. Some better inference
            // should be done based on "*" prefix for terms.
            is_reconstructed: is_reconstructed_lang(lang),
            i,
            lang,
            term,
            pos,
            page_title: None,
            ety_num: None,
            gloss_num: 0,
            gloss: None,
            raw_etymology: None,
            raw_root: None,
            raw_descendants: None,
        }
    }
}

type GlossMap = HashMap<Option<Symbol>, Rc<RawItem>>;
type PosMap = HashMap<Option<usize>, GlossMap>;
type EtyMap = HashMap<Option<u8>, PosMap>;
type LangMap = HashMap<usize, EtyMap>;
type TermMap = HashMap<Symbol, LangMap>;

#[derive(Default)]
pub(crate) struct RawItems {
    pub(crate) term_map: TermMap,
    pub(crate) n: usize,
    pub(crate) redirects: Redirects,
    pub(crate) line_map: HashMap<usize, Rc<RawItem>>,
    pub(crate) total_ok_lines_in_file: usize,
}

impl RawItems {
    pub(crate) fn add_to_term_map(&mut self, mut item: RawItem) -> Result<Option<Rc<RawItem>>> {
        // check if the item's term has been seen before
        if !self.term_map.contains_key(&item.term) {
            let mut gloss_map = GlossMap::new();
            let mut pos_map = PosMap::new();
            let mut ety_map = EtyMap::new();
            let mut lang_map = LangMap::new();
            let (pos, ety_num, lang, term) = (item.pos, item.ety_num, item.lang, item.term);
            let item = Rc::from(item);
            gloss_map.insert(item.gloss, Rc::clone(&item));
            pos_map.insert(pos, gloss_map);
            ety_map.insert(ety_num, pos_map);
            lang_map.insert(lang, ety_map);
            self.term_map.insert(term, lang_map);
            self.n += 1;
            return Ok(Some(item));
        }
        // since term has been seen before, there must be at least one lang for it
        // check if item's lang has been seen before
        let lang_map: &mut LangMap = self.term_map.get_mut(&item.term).unwrap();
        if !lang_map.contains_key(&item.lang) {
            let mut gloss_map = GlossMap::new();
            let mut pos_map = PosMap::new();
            let mut ety_map = EtyMap::new();
            let (pos, ety_num, lang) = (item.pos, item.ety_num, item.lang);
            let item = Rc::from(item);
            gloss_map.insert(item.gloss, Rc::clone(&item));
            pos_map.insert(pos, gloss_map);
            ety_map.insert(ety_num, pos_map);
            lang_map.insert(lang, ety_map);
            self.n += 1;
            return Ok(Some(item));
        }
        // since lang has been seen before, there must be at least one ety (possibly None)
        // check if this ety has been seen in this lang before
        let ety_map: &mut EtyMap = lang_map.get_mut(&item.lang).unwrap();
        if !ety_map.contains_key(&item.ety_num) {
            let mut gloss_map = GlossMap::new();
            let mut pos_map = PosMap::new();
            let (pos, ety_num) = (item.pos, item.ety_num);
            let item = Rc::from(item);
            gloss_map.insert(item.gloss, Rc::clone(&item));
            pos_map.insert(pos, gloss_map);
            ety_map.insert(ety_num, pos_map);
            self.n += 1;
            return Ok(Some(item));
        }
        // since ety has been seen before, there must be at least one pos
        // check if this pos has been seen for this ety before
        let pos_map: &mut PosMap = ety_map.get_mut(&item.ety_num).unwrap();
        if !pos_map.contains_key(&item.pos) {
            let mut gloss_map = GlossMap::new();
            let pos = item.pos;
            let item = Rc::from(item);
            gloss_map.insert(item.gloss, Rc::clone(&item));
            pos_map.insert(pos, gloss_map);
            self.n += 1;
            return Ok(Some(item));
        }
        // since pos has been seen before, there must be at least one gloss (possibly None)
        let gloss_map: &mut GlossMap = pos_map.get_mut(&item.pos).unwrap();
        if !gloss_map.contains_key(&item.gloss) {
            item.gloss_num = u8::try_from(gloss_map.len())?;
            let item = Rc::from(item);
            gloss_map.insert(item.gloss, Rc::clone(&item));
            self.n += 1;
            return Ok(Some(item));
        }
        Ok(None)
    }

    pub(crate) fn contains(&self, lang: usize, term: Symbol) -> bool {
        let (lang, term) = self.redirects.rectify_lang_term(lang, term);
        self.term_map
            .get(&term)
            .map_or(false, |lang_map| lang_map.contains_key(&lang))
    }

    pub(crate) fn get_disambiguated_item<'a>(
        &self,
        // string_pool: &StringPool,
        embeddings: &'a Embeddings,
        embedding: impl EmbeddingComparand<ItemEmbedding<'a>> + Copy,
        lang: usize,
        term: Symbol,
    ) -> Option<Rc<RawItem>> {
        // if LANG_CODE2NAME.get_index_value(lang) == "English" && string_pool.resolve(term) == "min" {
        //     println!("hey");
        // }
        let (lang, term) = self.redirects.rectify_lang_term(lang, term);
        let candidate_items = self.get_all_lang_term_items(lang, term)?;
        let mut max_similarity = 0f32;
        let mut best_candidate = 0usize;
        for (i, candidate) in candidate_items.iter().enumerate() {
            let candidate_embedding = embeddings.get(candidate);
            let similarity = embedding.cosine_similarity(candidate_embedding);
            let old_max_similarity = max_similarity;
            max_similarity = max_similarity.max(similarity);
            if max_similarity > old_max_similarity {
                best_candidate = i;
            }
        }
        Some(candidate_items[best_candidate].clone())
    }

    // returns all items that share the same lang and term
    pub(crate) fn get_all_lang_term_items(
        &self,
        lang: usize,
        term: Symbol,
    ) -> Option<Vec<Rc<RawItem>>> {
        let lang_map = self.term_map.get(&term)?;
        let ety_map = lang_map.get(&lang)?;
        let mut items = vec![];
        for pos_map in ety_map.values() {
            for gloss_map in pos_map.values() {
                for item in gloss_map.values() {
                    items.push(item.clone());
                }
            }
        }
        (!items.is_empty()).then_some(items)
    }

    // // since get_all_lang_term_items will return at least the item itself, we
    // // need the len of items to be > 1
    // fn item_has_duplicates(&self, item: &Rc<Item>) -> bool {
    //     self.get_all_lang_term_items(item.lang, item.term)
    //         .is_some_and(|items| items.len() > 1)
    // }

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
            && let Some(root_items) = self.get_all_lang_term_items(raw_root.lang, raw_root.term)
            && root_items.len() > 1
        {
            items_needing_embedding.insert(Rc::clone(item));
            for root_item in &root_items {
                items_needing_embedding.insert(Rc::clone(root_item));
            }
        }
        items_needing_embedding
    }

    fn get_all_items_needing_embedding(&self) -> Result<HashSet<Rc<RawItem>>> {
        let pb = progress_bar(self.n, "Determining which items need embeddings")?;
        let mut items_needing_embedding = HashSet::new();
        for lang_map in self.term_map.values() {
            for ety_map in lang_map.values() {
                for pos_map in ety_map.values() {
                    for gloss_map in pos_map.values() {
                        for item in gloss_map.values() {
                            let more = self.get_items_needing_embedding(item);
                            for m in more.iter() {
                                items_needing_embedding.insert(Rc::clone(m));
                            }
                            pb.inc(1);
                        }
                    }
                }
            }
        }
        pb.finish();
        Ok(items_needing_embedding)
    }

    // We go through the wiktextract file again, generating embeddings for all
    // ambiguous terms we found the first time.
    pub(crate) fn generate_embeddings(&self, path: &Path) -> Result<Embeddings> {
        const EMBEDDINGS_UPDATE_INTERVAL: usize = EMBEDDING_BATCH_SIZE * 10;
        let mut added = 0;
        let items_needing_embedding = self.get_all_items_needing_embedding()?;
        let pb = progress_bar(items_needing_embedding.len(), "Generating embeddings")?;
        let mut embeddings = Embeddings::new()?;
        for (line_number, mut line) in wiktextract_lines(path)?.enumerate() {
            // Items were only inserted into the line map if they were added to
            // the term_map in process_json_item.
            if let Some(item) = self.line_map.get(&line_number)
                && items_needing_embedding.contains(item)
            {
                let json_item = to_borrowed_value(&mut line)?;
                embeddings.add(&json_item, item)?;
                added += 1;
                if added % EMBEDDINGS_UPDATE_INTERVAL == 0 {
                    pb.inc(EMBEDDINGS_UPDATE_INTERVAL as u64);
                }
            }
        }
        embeddings.flush()?;
        pb.finish();
        Ok(embeddings)
    }

    fn add_all_to_ety_graph(&self, ety_graph: &mut EtyGraph) -> Result<()> {
        let pb = progress_bar(self.n, "Adding items to ety graph")?;
        for lang_map in self.term_map.values() {
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
