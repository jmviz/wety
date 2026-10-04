use crate::{
    descendants::RawDescendants,
    embeddings::{self, Embeddings, ItemEmbedding},
    etymology::RawEtymology,
    progress_bar,
    redirects::Redirects,
    root::RawRoot,
    wiktextract_json::wiktextract_lines,
    HashMap, HashSet,
};
use wety_core::{
    ety_graph::EtyGraph,
    items::{ImputedItem, Item, ItemId, RealItem},
    langterm::{LangTerm, Term},
    languages::Lang,
    pos::Pos,
    string_pool::StringPool,
};

use std::{collections::hash_map::Entry, mem, path::Path};

use anyhow::{Ok, Result};
use simd_json::to_borrowed_value;

#[derive(Default)]
pub(crate) struct RawTemplates {
    pub(crate) ety: HashMap<ItemId, RawEtymology>,
    pub(crate) desc: HashMap<ItemId, RawDescendants>,
    pub(crate) root: HashMap<ItemId, Box<[RawRoot]>>,
}

type Dupes = HashMap<LangTerm, Vec<ItemId>>;
// (langterm, {{etymon}} id) -> item, e.g. (enm moder, "mother")
type EtymonIds = HashMap<(LangTerm, Box<str>), ItemId>;
// cited langterm -> the page title Wiktionary links it to, where different,
// e.g. la mātrīx -> matrix
pub(crate) type EntryNames = HashMap<LangTerm, Term>;
type Lines = HashMap<usize, ItemId>;

pub(crate) struct Items {
    pub(crate) graph: EtyGraph,
    pub(crate) dupes: Dupes,
    pub(crate) page_term_dupes: Dupes,
    pub(crate) imputed_dupes: Dupes,
    pub(crate) etymon_ids: EtymonIds,
    pub(crate) entry_names: EntryNames,
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
            imputed_dupes: Dupes::default(),
            etymon_ids: EtymonIds::default(),
            entry_names: EntryNames::default(),
            redirects: Redirects::default(),
            raw_templates: RawTemplates::default(),
            lines: Lines::default(),
            total_ok_lines_in_file: 0,
        })
    }
}

impl Items {
    // Read the entry names exported by packages/source_tools/entry_names.py:
    // {lang code: {cited term: page title}}.
    pub(crate) fn load_entry_names(&mut self, string_pool: &mut StringPool, path: &Path) -> Result<()> {
        let file = std::io::BufReader::new(std::fs::File::open(path)?);
        let entry_names: HashMap<String, HashMap<String, String>> = serde_json::from_reader(file)?;
        for (code, titles) in entry_names {
            let Some(lang) = code.parse::<Lang>().ok() else {
                continue;
            };
            for (term, title) in titles {
                self.entry_names.insert(
                    lang.new_langterm(string_pool, &term),
                    Term::new(string_pool, &title),
                );
            }
        }
        Ok(())
    }

    pub(crate) fn len(&self) -> usize {
        self.graph.len()
    }

    /// get previously added item
    pub(crate) fn get(&self, id: ItemId) -> &Item {
        self.graph.item(id)
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
            let mut ety_num_taken = false;
            let mut same_ety_id = None;
            for &id in dupes {
                let other = self.graph.item(id);
                if other.ety_label() == item.ety_label.as_deref() {
                    same_ety_id = Some(id);
                }
                ety_num_taken |= other.ety_num() == item.ety_num;
                max_ety = other.ety_num().max(max_ety);
            }
            // If it shares an ety with an already stored real item...
            if let Some(same_ety_id) = same_ety_id
                && let Item::Real(same_ety) = self.graph.item_mut(same_ety_id)
                && !(item.pos[0] == Pos::root_pos()
                    && same_ety.pos.iter().any(|&p| p == item.pos[0]))
            {
                // If the pos is "root" and the already-stored item already has
                // another "root", then we need to make a new item for this.
                // This to handle the special but important case of PIE root
                // pages where there are several "Root" sections with no
                // Etymology sections (and hence here they will all have no
                // ety_label in the raw_item), but they really are etymologically
                // distinct items.
                //
                // Otherwise, we simply append this pos and gloss to the
                // existing item.
                same_ety.pos.push(item.pos[0]);
                same_ety.gloss.push(mem::take(&mut item.gloss[0]));
                return (same_ety_id, false);
            }
            // A new ety for an already seen langterm: keep Wiktionary's number
            // where it is free, so that it usually matches the page.
            if ety_num_taken {
                item.ety_num = max_ety.saturating_add(1);
            }
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
        if let Some(dupes) = self.imputed_dupes.get(&langterm) {
            item.ety_num = dupes
                .iter()
                .map(|&id| self.get(id).ety_num())
                .max()
                .expect("at least one")
                .saturating_add(1);

            let id = self.add(Item::Imputed(item));
            self.imputed_dupes
                .get_mut(&langterm)
                .expect("already found")
                .push(id);
            return id;
        }
        // A langterm that hasn't been seen yet
        let id = self.add(Item::Imputed(item));
        self.imputed_dupes.insert(langterm, vec![id]);
        id
    }

    // Imputed items sharing a (lang, term) are numbered in the order they
    // were imputed, which follows hash-map iteration and so differs between
    // builds. Renumber them by the item that cited them, so that a link to one
    // keeps naming the same imputation while its citing item persists.
    pub(crate) fn renumber_imputed_dupes(&mut self, string_pool: &StringPool) {
        let groups: Vec<Vec<ItemId>> = self
            .imputed_dupes
            .values()
            .filter(|ids| ids.len() > 1)
            .cloned()
            .collect();
        for mut ids in groups {
            ids.sort_by_cached_key(|&id| {
                let item = self.get(id);
                let Item::Imputed(imputed) = item else {
                    unreachable!("imputed_dupes holds imputed items")
                };
                let from = self.get(imputed.from);
                (
                    from.lang().code(),
                    from.term().resolve(string_pool).to_owned(),
                    from.ety_num(),
                    item.ety_num(),
                )
            });
            for (n, id) in ids.into_iter().enumerate() {
                if let Item::Imputed(imputed) = self.graph.item_mut(id) {
                    imputed.ety_num = u8::try_from(n + 1).unwrap_or(u8::MAX);
                }
            }
        }
    }

    // Record the id that an item's {{etymon}} gives its etymology, under both
    // its canonical and its page term. If two items claim the same id (a
    // Wiktionary error), the first is kept.
    pub(crate) fn add_etymon_id(&mut self, item_id: ItemId, id: &str) {
        let item = self.get(item_id);
        let lang = item.lang();
        let mut terms = vec![item.term()];
        if let Item::Real(real) = item
            && let Some(page_term) = real.page_term
        {
            terms.push(page_term);
        }
        for term in terms {
            self.etymon_ids
                .entry((LangTerm::new(lang, term), Box::from(id)))
                .or_insert(item_id);
        }
    }

    // The langterms under which a cited langterm may be found, in order, as
    // Wiktionary links it: as written (matching a canonical form or a page
    // title), then under the page title its language's entry-name rules give
    // (la mātrīx -> matrix), then through a redirect. Etymology-only languages
    // are looked up under their full language.
    fn lookup_langterms(&self, langterm: LangTerm) -> impl Iterator<Item = LangTerm> {
        let lang = langterm.lang.ety2non();
        let title = self
            .entry_names
            .get(&langterm)
            .copied()
            .unwrap_or(langterm.term);
        let redirect = self.redirects.rectify_langterm(LangTerm::new(lang, title));
        [
            LangTerm::new(lang, langterm.term),
            LangTerm::new(lang, title),
            redirect,
        ]
        .into_iter()
    }

    // The item that an {{etymon}} reference "lang:term<id:...>" names.
    pub(crate) fn get_etymon_item(&self, langterm: LangTerm, id: &str) -> Option<ItemId> {
        self.lookup_langterms(langterm)
            .find_map(|lt| self.etymon_ids.get(&(lt, Box::from(id))).copied())
    }

    // The items a cited langterm may name (several for homographs).
    pub(crate) fn candidates(&self, langterm: LangTerm) -> Option<&Vec<ItemId>> {
        self.lookup_langterms(langterm).find_map(|lt| {
            self.dupes
                .get(&lt)
                .or_else(|| self.page_term_dupes.get(&lt))
        })
    }

    fn get_max_similarity_candidate(
        &self,
        embeddings: &Embeddings,
        embedding_comp: &impl embeddings::Comparand<ItemEmbedding>,
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
        embedding_comp: &impl embeddings::Comparand<ItemEmbedding>,
        langterm: LangTerm,
    ) -> Result<Option<(ItemId, f32)>> {
        match self.candidates(langterm) {
            Some(candidates) => {
                self.get_max_similarity_candidate(embeddings, embedding_comp, candidates)
            }
            None => Ok(None),
        }
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
        embedding_comp: &impl embeddings::Comparand<ItemEmbedding>,
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
        for raw_root in self.raw_templates.root.get(&item_id).into_iter().flatten() {
            if let Some(root_items) = self.candidates(raw_root.langterm)
                && root_items.len() > 1
            {
                items_needing_embedding.insert(item_id);
                items_needing_embedding.extend(root_items);
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
        embeddings_config: &embeddings::Config,
    ) -> Result<Embeddings> {
        let mut embeddings = Embeddings::new(embeddings_config)?;
        let mut added = 0;
        let items_needing_embedding = self.get_all_items_needing_embedding()?;
        let pb = progress_bar(items_needing_embedding.len(), "Generating embeddings")?;
        let update_interval = embeddings_config.batch_size;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use wety_core::gloss::Gloss;

    fn add(items: &mut Items, sp: &mut StringPool, lang: &str, term: &str, page: Option<&str>) -> ItemId {
        let item = RealItem {
            ety_num: 1,
            ety_label: None,
            lang: Lang::from_str(lang).unwrap(),
            term: Term::new(sp, term),
            pos: vec![Pos::from_str("noun").unwrap()],
            gloss: vec![Gloss::new(sp, "x")],
            page_term: page.map(|p| Term::new(sp, p)),
            romanization: None,
            is_reconstructed: false,
        };
        items.add_real(item).0
    }

    #[test]
    fn candidates_follow_wiktionary_links() {
        let mut sp = StringPool::new();
        let mut items = Items::new().unwrap();
        let la = Lang::from_str("la").unwrap();
        let grc = Lang::from_str("grc").unwrap();
        // Latin matrix, canonically mātrīx; Greek περι-, with no canonical form
        let matrix = add(&mut items, &mut sp, "la", "mātrīx", Some("matrix"));
        let peri = add(&mut items, &mut sp, "grc", "περι-", None);
        let cited_peri = grc.new_langterm(&mut sp, "περῐ-");
        assert_eq!(items.candidates(cited_peri), None);
        items.entry_names.insert(cited_peri, Term::new(&mut sp, "περι-"));
        assert_eq!(items.candidates(cited_peri), Some(&vec![peri]));
        // as written, by canonical form or by page title
        assert_eq!(items.candidates(la.new_langterm(&mut sp, "mātrīx")), Some(&vec![matrix]));
        assert_eq!(items.candidates(la.new_langterm(&mut sp, "matrix")), Some(&vec![matrix]));
        // an etymology-only language is looked up under its full language
        let la_new = Lang::from_str("la-new").unwrap();
        assert_eq!(items.candidates(la_new.new_langterm(&mut sp, "mātrīx")), Some(&vec![matrix]));
    }
}
