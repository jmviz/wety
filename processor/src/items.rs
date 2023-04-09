use crate::{
    descendants::RawDescendants,
    embeddings::{self, EmbeddingComparand, Embeddings, EmbeddingsConfig, ItemEmbedding},
    ety_graph::{EtyGraph, ItemIndex},
    etymology::RawEtymology,
    gloss::Gloss,
    langterm::{Lang, LangTerm, Term},
    pos::Pos,
    progress_bar,
    redirects::Redirects,
    root::RawRoot,
    string_pool::StringPool,
    wiktextract_json::wiktextract_lines,
    HashMap, HashSet,
};

use std::{collections::hash_map::Entry, path::Path};

use anyhow::{Ok, Result};
use petgraph::stable_graph::NodeIndex;
use serde::{Deserialize, Serialize};
use simd_json::to_borrowed_value;

/// basic data read from a line in the wiktextract raw data
pub(crate) struct RawItem {
    pub(crate) ety_num: u8, // the nth numbered ety for this term-lang combo (1,2,...)
    pub(crate) lang: Lang,
    pub(crate) term: Term,
    pub(crate) page_term: Term, // i.e. the term stripped of diacritics etc. at the top of the page
    pub(crate) pos: Pos,        // e.g. "noun"
    pub(crate) gloss: Gloss,
}

impl RawItem {
    fn langterm(&self) -> LangTerm {
        LangTerm {
            lang: self.lang,
            term: self.term,
        }
    }
}

pub type ItemId = NodeIndex<ItemIndex>; // wiktionary has about ~10M items including imputations

/// An etymologically distinct item, which may have multiple (pos, gloss)'s
#[derive(Serialize, Deserialize)]
pub(crate) struct Item {
    pub(crate) is_imputed: bool,
    pub(crate) ety_num: u8, // the nth numbered ety for this term-lang combo (1,2,...)
    pub(crate) lang: Lang,
    pub(crate) term: Term,
    pub(crate) page_term: Option<Term>, // i.e. the term stripped of diacritics etc. at the top of the page
    pub(crate) pos: Option<Vec<Pos>>,   // e.g. "noun"
    pub(crate) gloss: Option<Vec<Gloss>>,
}

impl From<RawItem> for Item {
    fn from(raw_item: RawItem) -> Self {
        Item {
            is_imputed: false,
            ety_num: raw_item.ety_num,
            lang: raw_item.lang,
            term: raw_item.term,
            page_term: Some(raw_item.page_term),
            pos: Some(vec![raw_item.pos]),
            gloss: Some(vec![raw_item.gloss]),
        }
    }
}

impl Item {
    pub(crate) fn new_imputed(langterm: LangTerm) -> Self {
        Self {
            is_imputed: true,
            ety_num: 1,
            lang: langterm.lang,
            term: langterm.term,
            page_term: None,
            pos: None,
            gloss: None,
        }
    }

    pub(crate) fn url(&self, string_pool: &StringPool) -> Option<String> {
        let page_term = urlencoding::encode(self.page_term?.resolve(string_pool));
        let page_lang = self.lang.ety2main();
        let page_lang_name = urlencoding::encode(page_lang.name());
        Some(if page_lang.is_reconstructed() {
            format!("https://en.wiktionary.org/wiki/Reconstruction:{page_lang_name}/{page_term}")
        } else {
            format!("https://en.wiktionary.org/wiki/{page_term}#{page_lang_name}")
        })
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

pub(crate) struct Retrieval {
    pub(crate) item_id: ItemId,
    pub(crate) confidence: f32,
    pub(crate) is_newly_imputed: bool,
}

impl Items {
    pub(crate) fn len(&self) -> usize {
        self.graph.len()
    }

    /// get previously added item
    pub(crate) fn get(&self, id: ItemId) -> &Item {
        self.graph.get(id)
    }

    /// get previously added item mutably
    pub(crate) fn get_mut(&mut self, id: ItemId) -> &mut Item {
        self.graph.get_mut(id)
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

    fn add_dupe(
        &mut self,
        mut raw_item: RawItem,
        max_ety: u8,
        langterm: LangTerm,
        page_langterm: LangTerm,
    ) -> (ItemId, bool) {
        raw_item.ety_num = max_ety + 1;
        let id = self.add(raw_item.into());
        self.dupes
            .get_mut(&langterm)
            .expect("only called when dupes already found for langterm")
            .push(id);
        if langterm != page_langterm {
            self.add_page_term_dupe(page_langterm, id);
        }
        (id, true)
    }

    // the returned bool is true if the ItemId is new, false if the RawItem
    // got merged into an existing item and hence the ItemId is old
    pub(crate) fn add_raw(&mut self, raw_item: RawItem) -> (ItemId, bool) {
        let langterm = raw_item.langterm();
        let page_langterm = LangTerm::new(raw_item.lang, raw_item.page_term);
        // If we've seen this langterm before...
        if let Some(ids) = self.dupes.get(&langterm) {
            let mut max_ety = 0;
            let mut same_ety = None;
            for &id in ids {
                let other = self.get(id);
                if other.ety_num == raw_item.ety_num {
                    same_ety = Some(id);
                }
                max_ety = other.ety_num.max(max_ety);
            }
            // If it shares an ety with an already stored item...
            if let Some(same_ety) = same_ety {
                // If the pos is "root" and the already-stored item already has
                // another "root", then we need to make a new item for this.
                // This to handle the special but important case of PIE root
                // pages where there are several "Root" sections with no
                // Etymology sections (and hence here they will all have ety_num
                // == 1 in the raw_item), but they really are etymologically
                // distinct items.
                if raw_item.pos == Pos::root_pos()
                    && self
                        .get(same_ety)
                        .pos
                        .as_ref()
                        .expect("at least one pos")
                        .iter()
                        .any(|&p| p == raw_item.pos)
                {
                    return self.add_dupe(raw_item, max_ety, langterm, page_langterm);
                }
                // Otherwise, we simply append this pos and gloss to the
                // existing item.
                let same = self.get_mut(same_ety);
                same.pos
                    .as_mut()
                    .expect("at least one pos")
                    .push(raw_item.pos);
                same.gloss
                    .as_mut()
                    .expect("at least one gloss")
                    .push(raw_item.gloss);
                return (same_ety, false);
            }
            // A new ety_num for an already seen langterm
            return self.add_dupe(raw_item, max_ety, langterm, page_langterm);
        }

        // A langterm that hasn't been seen yet
        let id = self.add(raw_item.into());
        self.dupes.insert(langterm, vec![id]);
        if langterm != page_langterm {
            self.add_page_term_dupe(page_langterm, id);
        }
        (id, true)
    }

    pub(crate) fn add_imputed(&mut self, langterm: LangTerm) -> ItemId {
        self.add(Item::new_imputed(langterm))
    }

    // returns all items that share the same lang and term
    pub(crate) fn get_dupes(&self, langterm: LangTerm) -> Option<&Vec<ItemId>> {
        self.dupes.get(&langterm)
    }
}

fn get_max_similarity_candidate(
    embeddings: &Embeddings,
    embedding_comp: &impl EmbeddingComparand<ItemEmbedding>,
    candidates: &[ItemId],
) -> Result<Option<(ItemId, f32)>> {
    let mut max_similarity = 0f32;
    let mut best_candidate = 0usize;
    for (i, &candidate) in candidates.iter().enumerate() {
        let candidate_embedding = embeddings.get(candidate)?;
        let similarity = embedding_comp.cosine_similarity(&candidate_embedding);
        let old_max_similarity = max_similarity;
        max_similarity = max_similarity.max(similarity);
        if max_similarity > old_max_similarity {
            best_candidate = i;
        }
    }
    if max_similarity > embeddings::SIMILARITY_THRESHOLD {
        return Ok(Some((candidates[best_candidate], max_similarity)));
    }
    Ok(None)
}

impl Items {
    pub(crate) fn get_disambiguated_item_id(
        &self,
        embeddings: &Embeddings,
        embedding_comp: &impl EmbeddingComparand<ItemEmbedding>,
        langterm: LangTerm,
    ) -> Result<Option<(ItemId, f32)>> {
        let langterm = self.redirects.rectify_langterm(langterm);
        if let Some(candidates) = self.get_dupes(langterm)
            && let Some((item_id, similarity)) = get_max_similarity_candidate(embeddings, embedding_comp, candidates)? {
            return Ok(Some((item_id, similarity)));
        }
        if let Some(candidates) = self.page_term_dupes.get(&langterm)
            && let Some((item_id, similarity)) = get_max_similarity_candidate(embeddings, embedding_comp, candidates)? {
            return Ok(Some((item_id, similarity)));
        }
        Ok(None)
    }

    pub(crate) fn get_or_impute_item(
        &mut self,
        embeddings: &Embeddings,
        embedding_comp: &impl EmbeddingComparand<ItemEmbedding>,
        langterm: LangTerm,
    ) -> Result<Retrieval> {
        if let Some((item_id, confidence)) =
            self.get_disambiguated_item_id(embeddings, embedding_comp, langterm)?
        {
            return Ok(Retrieval {
                item_id,
                confidence,
                is_newly_imputed: false,
            });
        }
        let item_id = self.add_imputed(langterm);
        Ok(Retrieval {
            item_id,
            confidence: 0.0,
            is_newly_imputed: true,
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
                let lang_name = item.lang.name();
                let term = item.term.resolve(string_pool);
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
