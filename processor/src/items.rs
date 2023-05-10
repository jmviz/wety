use crate::{
    descendants::RawDescendants,
    embeddings::{self, EmbeddingComparand, Embeddings, EmbeddingsConfig, ItemEmbedding},
    ety_graph::{EtyGraph, ItemIndex},
    etymology::RawEtymology,
    gloss::Gloss,
    langterm::{LangTerm, Term},
    languages::Lang,
    pos::Pos,
    progress_bar,
    redirects::Redirects,
    root::RawRoot,
    string_pool::StringPool,
    wiktextract_json::wiktextract_lines,
    HashMap, HashSet,
};

use std::{collections::hash_map::Entry, mem, path::Path};

use anyhow::{Ok, Result};
use petgraph::stable_graph::NodeIndex;
use serde::{Deserialize, Serialize};
use simd_json::to_borrowed_value;

pub type ItemId = NodeIndex<ItemIndex>; // wiktionary has about ~10M items including imputations

/// An etymologically distinct item, which may have multiple (pos, gloss)'s
#[derive(Serialize, Deserialize)]
pub(crate) struct RealItem {
    pub(crate) ety_num: u8, // the nth numbered ety for this term-lang combo (1,2,...)
    pub(crate) lang: Lang,
    pub(crate) term: Term,
    pub(crate) pos: Vec<Pos>, // e.g. "noun"
    pub(crate) gloss: Vec<Gloss>,
    pub(crate) page_term: Option<Term>, // i.e. the term stripped of diacritics etc. at the top of the page
    pub(crate) romanization: Option<Term>,
    pub(crate) is_reconstructed: bool,
}

impl RealItem {
    pub(crate) fn url(&self, string_pool: &StringPool) -> String {
        let page_term = self.page_term.unwrap_or(self.term);
        let url_term = urlencoding::encode(page_term.resolve(string_pool));
        let page_lang = self.lang.ety2non();
        let url_lang_name = urlencoding::encode(page_lang.name());
        if self.is_reconstructed {
            return format!(
                "https://en.wiktionary.org/wiki/Reconstruction:{url_lang_name}/{url_term}"
            );
        }
        format!("https://en.wiktionary.org/wiki/{url_term}#{url_lang_name}")
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ImputedItem {
    pub(crate) ety_num: u8,
    pub(crate) lang: Lang,
    pub(crate) term: Term,
    pub(crate) romanization: Option<Term>,
    pub(crate) from: ItemId, // during the processing of which Item was this imputed?
}

#[derive(Serialize, Deserialize)]
pub(crate) enum Item {
    Real(RealItem),
    Imputed(ImputedItem),
}

impl Item {
    pub(crate) fn is_imputed(&self) -> bool {
        match self {
            Item::Real(_) => false,
            Item::Imputed(_) => true,
        }
    }

    pub(crate) fn ety_num(&self) -> u8 {
        match self {
            Item::Real(real_item) => real_item.ety_num,
            Item::Imputed(imputed_item) => imputed_item.ety_num,
        }
    }

    pub(crate) fn lang(&self) -> Lang {
        match self {
            Item::Real(real_item) => real_item.lang,
            Item::Imputed(imputed_item) => imputed_item.lang,
        }
    }

    pub(crate) fn term(&self) -> Term {
        match self {
            Item::Real(real_item) => real_item.term,
            Item::Imputed(imputed_item) => imputed_item.term,
        }
    }

    pub(crate) fn pos(&self) -> Option<&Vec<Pos>> {
        match self {
            Item::Real(real_item) => Some(&real_item.pos),
            Item::Imputed(_) => None,
        }
    }

    pub(crate) fn gloss(&self) -> Option<&Vec<Gloss>> {
        match self {
            Item::Real(real_item) => Some(&real_item.gloss),
            Item::Imputed(_) => None,
        }
    }

    pub(crate) fn romanization(&self) -> Option<Term> {
        match self {
            Item::Real(real_item) => real_item.romanization,
            Item::Imputed(imputed_item) => imputed_item.romanization,
        }
    }

    pub(crate) fn url(&self, string_pool: &StringPool) -> Option<String> {
        match self {
            Item::Real(real_item) => Some(real_item.url(string_pool)),
            Item::Imputed(_) => None,
        }
    }

    pub(crate) fn is_reconstructed(&self) -> bool {
        match self {
            Item::Real(real_item) => real_item.is_reconstructed,
            Item::Imputed(imputed_item) => imputed_item.lang.is_reconstructed(),
        }
    }
}

#[derive(Default)]
pub(crate) struct RawTemplates {
    pub(crate) ety: HashMap<ItemId, RawEtymology>,
    pub(crate) desc: HashMap<ItemId, RawDescendants>,
    pub(crate) root: HashMap<ItemId, RawRoot>,
}

type Dupes = HashMap<LangTerm, Vec<ItemId>>;
type Lines = HashMap<usize, ItemId>;

pub(crate) struct Items {
    pub(crate) graph: EtyGraph,
    pub(crate) dupes: Dupes,
    pub(crate) page_term_dupes: Dupes,
    pub(crate) redirects: Redirects,
    pub(crate) raw_templates: RawTemplates,
    pub(crate) lines: Lines,
    pub(crate) total_ok_lines_in_file: usize,
}

impl Items {
    pub(crate) fn new() -> Result<Self> {
        Ok(Self {
            graph: EtyGraph::default(),
            dupes: Dupes::default(),
            page_term_dupes: Dupes::default(),
            redirects: Redirects::default(),
            raw_templates: RawTemplates::default(),
            lines: Lines::default(),
            total_ok_lines_in_file: 0,
        })
    }
}

impl Items {
    pub(crate) fn len(&self) -> usize {
        self.graph.len()
    }

    /// get previously added item
    pub(crate) fn get(&self, id: ItemId) -> &Item {
        self.graph.get(id)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (ItemId, &Item)> {
        self.graph.iter()
    }

    pub(crate) fn add(&mut self, item: Item) -> ItemId {
        self.graph.add(item)
    }

    fn add_page_term_dupe(&mut self, page_langterm: LangTerm, id: ItemId) {
        match self.page_term_dupes.entry(page_langterm) {
            Entry::Occupied(mut e) => e.get_mut().push(id),
            Entry::Vacant(e) => {
                e.insert(vec![id]);
            }
        }
    }

    // the returned bool is true if the ItemId is new, false if the RawItem
    // got merged into an existing item and hence the ItemId is old
    pub(crate) fn add_real(&mut self, mut item: RealItem) -> (ItemId, bool) {
        let langterm = LangTerm::new(item.lang, item.term);
        let page_langterm = item.page_term.map(|pt| LangTerm::new(item.lang, pt));
        // If we've seen this langterm before...
        if let Some(dupes) = self.dupes.get(&langterm) {
            let mut max_ety = 0;
            let mut same_ety_id = None;
            for &id in dupes {
                let other = self.graph.get(id);
                if other.ety_num() == item.ety_num {
                    same_ety_id = Some(id);
                }
                max_ety = other.ety_num().max(max_ety);
            }
            // If it shares an ety with an already stored real item...
            if let Some(same_ety_id) = same_ety_id
                    && let Item::Real(same_ety) = self.graph.get_mut(same_ety_id)
                    && !(item.pos[0] == Pos::root_pos() && same_ety.pos.iter().any(|&p| p == item.pos[0]))
                {
                    // If the pos is "root" and the already-stored item already has
                    // another "root", then we need to make a new item for this.
                    // This to handle the special but important case of PIE root
                    // pages where there are several "Root" sections with no
                    // Etymology sections (and hence here they will all have ety_num
                    // == 1 in the raw_item), but they really are etymologically
                    // distinct items.
                    // 
                    // Otherwise, we simply append this pos and gloss to the
                    // existing item.
                    same_ety.pos.push(item.pos[0]);
                    same_ety.gloss.push(mem::take(&mut item.gloss[0]));
                    return (same_ety_id, false);
                }
            // A new ety_num for an already seen langterm
            item.ety_num = max_ety + 1;
            let id = self.add(Item::Real(item));
            self.dupes
                .get_mut(&langterm)
                .expect("already found")
                .push(id);
            if let Some(page_langterm) = page_langterm {
                self.add_page_term_dupe(page_langterm, id);
            }
            return (id, true);
        }
        // A langterm that hasn't been seen yet
        let id = self.add(Item::Real(item));
        self.dupes.insert(langterm, vec![id]);
        if let Some(page_langterm) = page_langterm {
            self.add_page_term_dupe(page_langterm, id);
        }
        (id, true)
    }

    pub(crate) fn add_imputed(&mut self, mut item: ImputedItem) -> ItemId {
        let langterm = LangTerm::new(item.lang, item.term);
        // If we've seen this langterm before...
        if let Some(dupes) = self.dupes.get(&langterm) {
            item.ety_num = dupes
                .iter()
                .map(|&id| self.get(id).ety_num())
                .max()
                .expect("at least one")
                + 1;

            let id = self.add(Item::Imputed(item));
            self.dupes
                .get_mut(&langterm)
                .expect("already found")
                .push(id);
            return id;
        }
        // A langterm that hasn't been seen yet
        let id = self.add(Item::Imputed(item));
        self.dupes.insert(langterm, vec![id]);
        id
    }

    // returns all items that share the same lang and term
    pub(crate) fn get_dupes(&self, langterm: LangTerm) -> Option<&Vec<ItemId>> {
        self.dupes
            .get(&langterm)
            .or_else(|| self.page_term_dupes.get(&langterm))
    }

    fn get_max_similarity_candidate(
        &self,
        embeddings: &Embeddings,
        embedding_comp: &impl EmbeddingComparand<ItemEmbedding>,
        candidates: &[ItemId],
    ) -> Result<Option<(ItemId, f32)>> {
        let mut max_similarity = 0f32;
        let mut best_candidate = 0usize;
        for (i, &candidate) in candidates.iter().enumerate() {
            let candidate_embedding = embeddings.get(self.get(candidate), candidate)?;
            let similarity = embedding_comp.cosine_similarity(&candidate_embedding);
            let old_max_similarity = max_similarity;
            max_similarity = max_similarity.max(similarity);
            if max_similarity > old_max_similarity {
                best_candidate = i;
            }
        }
        if max_similarity >= embeddings::SIMILARITY_THRESHOLD {
            return Ok(Some((candidates[best_candidate], max_similarity)));
        }
        Ok(None)
    }

    pub(crate) fn get_disambiguated_item_id(
        &self,
        embeddings: &Embeddings,
        embedding_comp: &impl EmbeddingComparand<ItemEmbedding>,
        langterm: LangTerm,
    ) -> Result<Option<(ItemId, f32)>> {
        let langterm = self.redirects.rectify_langterm(langterm);
        if let Some(candidates) = self.get_dupes(langterm)
            && let Some((item_id, similarity)) = self.get_max_similarity_candidate(embeddings, embedding_comp, candidates)? {
            return Ok(Some((item_id, similarity)));
        }
        if let Some(candidates) = self.page_term_dupes.get(&langterm)
            && let Some((item_id, similarity)) = self.get_max_similarity_candidate(embeddings, embedding_comp, candidates)? {
            return Ok(Some((item_id, similarity)));
        }
        Ok(None)
    }
}

pub(crate) struct Retrieval {
    pub(crate) item_id: ItemId,
    pub(crate) confidence: f32,
    // pub(crate) is_newly_imputed: bool,
}

impl Items {
    pub(crate) fn get_or_impute_item(
        &mut self,
        embeddings: &Embeddings,
        embedding_comp: &impl EmbeddingComparand<ItemEmbedding>,
        from_item: ItemId,
        langterm: LangTerm,
    ) -> Result<Retrieval> {
        if let Some((item_id, confidence)) =
            self.get_disambiguated_item_id(embeddings, embedding_comp, langterm)?
        {
            return Ok(Retrieval {
                item_id,
                confidence,
                // is_newly_imputed: false,
            });
        }
        let imputed = ImputedItem {
            ety_num: 1, // may get changed in add_imputed
            lang: langterm.lang,
            term: langterm.term,
            romanization: None, // $$ implement getting this from template
            from: from_item,
        };
        let item_id = self.add_imputed(imputed);
        Ok(Retrieval {
            item_id,
            confidence: embeddings::SIMILARITY_THRESHOLD,
            // is_newly_imputed: true,
        })
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
        let mut items_needing_embedding = HashSet::default();
        if let Some(raw_etymology) = self.raw_templates.ety.get(&item_id) {
            items_needing_embedding
                .extend(self.get_ety_items_needing_embedding(item_id, raw_etymology));
        }
        if let Some(raw_descendants) = self.raw_templates.desc.get(&item_id) {
            items_needing_embedding
                .extend(self.get_desc_items_needing_embedding(item_id, raw_descendants));
        }
        if let Some(raw_root) = self.raw_templates.root.get(&item_id)
            && let Some(root_items) = self.get_dupes(raw_root.langterm)
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
        let mut items_needing_embedding = HashSet::default();
        for (item_id, _) in self.iter() {
            let items_to_embed = self.get_items_needing_embedding(item_id);
            for &item_to_embed in &items_to_embed {
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
        let mut embeddings = Embeddings::new(embeddings_config)?;
        let mut added = 0;
        let items_needing_embedding = self.get_all_items_needing_embedding()?;
        let pb = progress_bar(items_needing_embedding.len(), "Generating embeddings")?;
        let update_interval = embeddings_config.progress_update_interval;
        pb.inc(0);
        for (line_number, mut line) in wiktextract_lines(wiktextract_path)?.enumerate() {
            // Items were only inserted into the line map if they were added to
            // the term_map in process_json_item.
            if let Some(&item_id) = self.lines.get(&line_number)
                && items_needing_embedding.contains(&item_id)
            {
                let json_item = to_borrowed_value(&mut line)?;
                let item = self.get(item_id);
                let lang_name = item.lang().name();
                let term = item.term().resolve(string_pool);
                embeddings.add(&json_item, lang_name, term, item_id)?;
                added += 1;
                if added % update_interval == 0 {
                    pb.inc(update_interval as u64);
                }
            }
        }
        embeddings.flush()?;
        pb.finish();
        Ok(embeddings)
    }

    pub(crate) fn generate_ety_graph(&mut self, embeddings: &Embeddings) -> Result<()> {
        self.process_raw_descendants(embeddings)?;
        self.graph.remove_cycles()?;
        self.process_raw_etymologies(embeddings)?;
        self.graph.remove_cycles()?;
        self.impute_root_etys(embeddings)?;
        self.graph.remove_cycles()?;
        Ok(())
    }
}
