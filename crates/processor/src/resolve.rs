//! Which item a citation names.
//!
//! A cited lang+term may match several items (homographs), or one item that is
//! not the word meant, while the word meant is missing from the data (e.g. an
//! English etymology cites Middle English "bern" (child), but the only Middle
//! English "bern" entries are other words). Before falling back on gloss
//! similarity, we weigh what the data says about the citation:
//!
//! - an id the citation gives for its term, declared by one of the items;
//! - agreement: the candidate's Descendants section or Derived terms list the
//!   citing item, or its own etymology names (or leads to) what the citing etymology gives as
//!   the cited term's own source (the continuation, "from enm m, from ang o");
//!   for a Descendants line, the candidate's etymology names (or leads to) the
//!   item whose list it is on;
//! - contradiction: the candidate's own etymology names a different existing
//!   term in the language of that continuation (or of that list's item),
//!   unrelated to it.
//!
//! An agreeing candidate is taken; contradicted candidates are dropped; among
//! the rest, a gloss the citation gives for its term (t=) decides by shared
//! words, else gloss embeddings. If every candidate of an etymology's citation
//! is contradicted, the term meant is taken to be missing from the data.

use crate::{
    HashSet,
    embeddings::{self, Embeddings, ItemEmbedding},
    etymology::ParsedRawEtyTemplate,
    items::Items,
};
use wety_core::{items::Item, items::ItemId, langterm::LangTerm, string_pool::StringPool};

use anyhow::{Ok, Result};

// how many etymology steps to follow when relating two terms
const DEPTH: usize = 3;

pub(crate) enum Relation {
    // the cited term is the citing item's parent; `next` is what the citing
    // etymology gives as the cited term's own source
    Parent { next: Option<LangTerm> },
    // the cited term is listed in the citing item's Descendants section
    Child,
}

pub(crate) struct Citation<'a> {
    pub(crate) langterm: LangTerm,
    pub(crate) id: Option<&'a str>,
    pub(crate) gloss: Option<&'a str>,
    pub(crate) citer: ItemId,
    pub(crate) relation: Relation,
}

pub(crate) enum Resolution {
    Found { item: ItemId, confidence: f32 },
    // no item is the term meant
    Missing,
}

// The terms an item's etymology and Descendants section name, as keys.
#[derive(Default)]
pub(crate) struct Mentions {
    ety: crate::HashMap<ItemId, Box<[LangTerm]>>,
    desc: crate::HashMap<ItemId, Box<[LangTerm]>>,
}

const STOPWORDS: [&str; 24] = [
    "a", "an", "the", "to", "of", "or", "and", "in", "on", "for", "with", "by", "from", "as",
    "at", "be", "is", "one", "someone", "something", "that", "which", "who", "its",
];

fn gloss_words<'a>(text: impl Iterator<Item = &'a str>) -> HashSet<String> {
    text.flat_map(|t| t.split(|c: char| !c.is_alphanumeric()))
        .map(str::to_lowercase)
        .filter(|w| w.chars().count() > 1 && !STOPWORDS.contains(&w.as_str()))
        .map(|w| match w.strip_suffix('s') {
            Some(stem) if stem.chars().count() > 2 => stem.to_owned(),
            _ => w,
        })
        .collect()
}

impl Items {
    /// A cited langterm as the page it names: full language, page title.
    pub(crate) fn key(&self, langterm: LangTerm) -> LangTerm {
        let title = self
            .entry_names
            .get(&langterm)
            .copied()
            .unwrap_or(langterm.term);
        LangTerm::new(langterm.lang.ety2non(), title)
    }

    fn item_keys(&self, item: ItemId) -> Vec<LangTerm> {
        let item_ref = self.get(item);
        let mut keys = vec![self.key(LangTerm::new(item_ref.lang(), item_ref.term()))];
        if let Item::Real(real) = item_ref
            && let Some(page_term) = real.page_term
        {
            keys.push(LangTerm::new(item_ref.lang().ety2non(), page_term));
        }
        keys
    }

    pub(crate) fn index_mentions(&mut self) {
        let mut mentions = Mentions::default();
        for (&item, ety) in &self.raw_templates.ety {
            let keys: Vec<LangTerm> = ety
                .templates
                .iter()
                .filter_map(|t| match t {
                    ParsedRawEtyTemplate::Parsed(t) => Some(t.langterms.iter()),
                    ParsedRawEtyTemplate::Skipped => None,
                })
                .flatten()
                .map(|&lt| self.key(lt))
                .collect();
            if !keys.is_empty() {
                mentions.ety.insert(item, keys.into_boxed_slice());
            }
        }
        // Descendants sections and "Derived terms" lists
        let mut desc: crate::HashMap<ItemId, Vec<LangTerm>> = crate::HashMap::default();
        for (&item, raw) in &self.raw_templates.desc {
            desc.entry(item).or_default().extend(raw.langterms().map(|lt| self.key(lt)));
        }
        for (&item, derived) in &self.raw_templates.derived {
            desc.entry(item).or_default().extend(derived.iter().map(|&lt| self.key(lt)));
        }
        mentions.desc = desc
            .into_iter()
            .filter(|(_, keys)| !keys.is_empty())
            .map(|(item, keys)| (item, keys.into_boxed_slice()))
            .collect();
        self.mentions = mentions;
    }

    fn ety_mentions(&self, item: ItemId) -> &[LangTerm] {
        self.mentions.ety.get(&item).map_or(&[], |m| m)
    }

    // The terms an item is said to come from: those its etymology names and
    // its parents so far (e.g. an imputed item's, given by a citing chain).
    fn source_keys(&self, item: ItemId) -> Vec<LangTerm> {
        let mut keys = self.ety_mentions(item).to_vec();
        if let Some(ety) = self.graph.immediate_ety(item) {
            for parent in ety.items {
                keys.extend(self.item_keys(parent));
            }
        }
        keys
    }

    // Whether the etymologies of the items `from` may name lead to `to`
    // within `depth` steps.
    fn reaches(&self, from: LangTerm, to: LangTerm, depth: usize) -> bool {
        if from == to {
            return true;
        }
        if depth == 0 {
            return false;
        }
        self.candidates(from).is_some_and(|items| {
            items.iter().any(|&item| {
                self.ety_mentions(item)
                    .iter()
                    .any(|&m| self.reaches(m, to, depth - 1))
                    // and the links already made (e.g. from Descendants lists)
                    || self.graph.immediate_ety(item).is_some_and(|ety| {
                        ety.items.iter().any(|&parent| {
                            self.item_keys(parent)
                                .into_iter()
                                .any(|k| self.reaches(k, to, depth - 1))
                        })
                    })
            })
        })
    }

    // Whether two keys may name the same word or one comes from the other.
    fn related(&self, a: LangTerm, b: LangTerm) -> bool {
        a == b
            || self.candidates(a).zip(self.candidates(b)).is_some_and(|(x, y)| x.iter().any(|i| y.contains(i)))
            || self.reaches(a, b, DEPTH)
            || self.reaches(b, a, DEPTH)
    }

    // A candidate contradicts the citation if its etymology names terms in the
    // language of `expected`, all of them existing and unrelated to it.
    fn contradicts(&self, candidate: ItemId, expected: LangTerm) -> bool {
        let same_lang: Vec<LangTerm> = self
            .source_keys(candidate)
            .into_iter()
            .filter(|m| m.lang == expected.lang)
            .collect();
        !same_lang.is_empty()
            && same_lang
                .iter()
                .all(|&m| self.candidates(m).is_some() && !self.related(m, expected))
    }

    fn agrees(&self, candidate: ItemId, citation: &Citation, citer_keys: &[LangTerm]) -> bool {
        let linked = |child: ItemId, parent: ItemId| {
            self.graph
                .immediate_ety(child)
                .is_some_and(|ety| ety.items.contains(&parent))
        };
        match citation.relation {
            Relation::Parent { next } => {
                linked(citation.citer, candidate)
                    || self
                        .mentions
                        .desc
                        .get(&candidate)
                        .is_some_and(|desc| desc.iter().any(|k| citer_keys.contains(k)))
                    || next.is_some_and(|next| {
                        let next = self.key(next);
                        self.source_keys(candidate)
                            .into_iter()
                            .any(|m| self.related(m, next))
                    })
            }
            Relation::Child => {
                linked(candidate, citation.citer)
                    || self.source_keys(candidate).into_iter().any(|m| {
                        citer_keys
                            .iter()
                            .any(|&k| m == k || self.reaches(m, k, DEPTH))
                    })
            }
        }
    }

    fn gloss_score(&self, string_pool: &StringPool, item: ItemId, words: &HashSet<String>) -> usize {
        let Item::Real(real) = self.get(item) else {
            return 0;
        };
        let item_words = gloss_words(real.gloss.iter().flat_map(|g| g.words(string_pool)));
        words.intersection(&item_words).count()
    }

    // Among candidates, the one whose glosses share most words with the
    // citation's gloss, if one does uniquely; else the one most similar by
    // gloss embeddings.
    fn best_candidate(
        &self,
        string_pool: &StringPool,
        embeddings: &Embeddings,
        embedding_comp: &impl embeddings::Comparand<ItemEmbedding>,
        candidates: &[ItemId],
        gloss: Option<&str>,
    ) -> Result<Option<(ItemId, f32)>> {
        if let [only] = candidates {
            let similarity = self
                .get_max_similarity_candidate(embeddings, embedding_comp, candidates)?
                .map_or(embeddings::SIMILARITY_THRESHOLD, |(_, s)| s);
            return Ok(Some((*only, similarity)));
        }
        if let Some(gloss) = gloss {
            let words = gloss_words(std::iter::once(gloss));
            let mut scores: Vec<(usize, ItemId)> = candidates
                .iter()
                .map(|&c| (self.gloss_score(string_pool, c, &words), c))
                .collect();
            scores.sort_unstable_by(|a, b| b.0.cmp(&a.0));
            if scores[0].0 > 0 && scores[0].0 > scores[1].0 {
                return Ok(Some((scores[0].1, 0.9)));
            }
        }
        self.get_max_similarity_candidate(embeddings, embedding_comp, candidates)
    }

    pub(crate) fn resolve(
        &self,
        string_pool: &StringPool,
        embeddings: &Embeddings,
        embedding_comp: &impl embeddings::Comparand<ItemEmbedding>,
        citation: &Citation,
    ) -> Result<Resolution> {
        if let Some(id) = citation.id
            && let Some(item) = self.get_id_item(citation.langterm, id)
        {
            return Ok(Resolution::Found {
                item,
                confidence: 1.0,
            });
        }
        let Some(real) = self.candidates(citation.langterm) else {
            return Ok(Resolution::Missing);
        };
        // A term found missing for an earlier citation (though homographs
        // exist) is a candidate too.
        let mut candidates = real.clone();
        if let Some(&imputed) = self.imputed_titles.get(&LangTerm::new(
            citation.langterm.lang,
            self.key(citation.langterm).term,
        )) {
            candidates.push(imputed);
        }
        let candidates = &candidates;
        // A single candidate can only be rejected for an etymology citation
        // with a continuation to check it against.
        if candidates.len() == 1
            && !matches!(citation.relation, Relation::Parent { next: Some(_) })
        {
            let best = self.best_candidate(string_pool, embeddings, embedding_comp, candidates, None)?;
            return Ok(best.map_or(Resolution::Missing, |(item, confidence)| Resolution::Found {
                item,
                confidence,
            }));
        }
        let citer_keys = self.item_keys(citation.citer);
        let agreeing: Vec<ItemId> = candidates
            .iter()
            .copied()
            .filter(|&c| self.agrees(c, citation, &citer_keys))
            .collect();
        if !agreeing.is_empty() {
            let best = self.best_candidate(string_pool, embeddings, embedding_comp, &agreeing, citation.gloss)?;
            return Ok(best.map_or(Resolution::Missing, |(item, _)| Resolution::Found {
                item,
                confidence: 1.0,
            }));
        }
        let expected = match citation.relation {
            Relation::Parent { next } => next
                .map(|next| self.key(next))
                .filter(|&next| self.candidates(next).is_some()),
            Relation::Child => citer_keys.first().copied(),
        };
        let open: Vec<ItemId> = match expected {
            Some(expected) => candidates
                .iter()
                .copied()
                .filter(|&c| !self.contradicts(c, expected))
                .collect(),
            None => candidates.to_vec(),
        };
        let pool = match (&citation.relation, open.is_empty()) {
            // every candidate is some other word: the one meant is missing
            (Relation::Parent { .. }, true) => return Ok(Resolution::Missing),
            // Descendants lists often skip steps; keep to the list
            (Relation::Child, true) => candidates,
            _ => &open,
        };
        let best = self.best_candidate(string_pool, embeddings, embedding_comp, pool, citation.gloss)?;
        Ok(best.map_or(Resolution::Missing, |(item, confidence)| Resolution::Found {
            item,
            confidence,
        }))
    }
}
