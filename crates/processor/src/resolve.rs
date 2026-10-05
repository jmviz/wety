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
//! the rest, embeddings decide (see embeddings::citation_score), using a gloss
//! the citation gives for its term (t=) where it gives one. If every candidate of an etymology's citation
//! is contradicted, the term meant is taken to be missing from the data.

use crate::{
    HashSet,
    embeddings::{self, Embeddings},
    etymology::ParsedRawEtyTemplate,
    items::Items,
};
use wety_core::{
    etymology_templates::EtyMode, items::Item, items::ItemId, langterm::LangTerm, pos::Pos,
    string_pool::StringPool,
};

use anyhow::{Ok, Result};

// how many etymology steps to follow when relating two terms
const DEPTH: usize = 3;

// the confidence of a link to the only candidate, where nothing was weighed
const SINGLE_CANDIDATE_CONFIDENCE: f32 = 0.5;
// added to the score of a candidate of the part of speech the citation gives
// the term, or else of the citing item's
const SAME_POS_BONUS: f32 = 0.05;

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
    // the cited term's part of speech, where the citation gives it
    pub(crate) pos: Option<Pos>,
    pub(crate) citer: ItemId,
    pub(crate) relation: Relation,
}

// How citations were resolved, for the build log: counts by deciding
// evidence, and a sample of the terms judged missing despite homographs.
#[derive(Default)]
pub(crate) struct ResolutionStats {
    counts: std::cell::RefCell<std::collections::BTreeMap<&'static str, u64>>,
    missing_sample: std::cell::RefCell<Vec<String>>,
}

impl ResolutionStats {
    fn count(&self, how: &'static str) {
        *self.counts.borrow_mut().entry(how).or_default() += 1;
    }

    pub(crate) fn report(&self) {
        for (how, n) in self.counts.borrow().iter() {
            println!("  citations resolved by {how}: {n}");
        }
        for line in self.missing_sample.borrow().iter() {
            println!("  missing: {line}");
        }
        self.counts.borrow_mut().clear();
        self.missing_sample.borrow_mut().clear();
    }
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

    // A key and, where it names an inflected or alternative form, its lemma.
    fn with_lemmas(&self, key: LangTerm) -> Vec<LangTerm> {
        let mut keys = vec![key];
        for &item in self.candidates(key).into_iter().flatten() {
            if let Some(ety) = self.graph.immediate_ety(item)
                && ety.mode == EtyMode::Form
            {
                for lemma in ety.items {
                    keys.extend(self.item_keys(lemma));
                }
            }
        }
        keys
    }

    // Whether two keys may name the same word or one comes from the other.
    fn related(&self, a: LangTerm, b: LangTerm) -> bool {
        let (a_keys, b_keys) = (self.with_lemmas(a), self.with_lemmas(b));
        a_keys.iter().any(|&a| {
            b_keys.iter().any(|&b| {
                a == b
                    || self
                        .candidates(a)
                        .zip(self.candidates(b))
                        .is_some_and(|(x, y)| x.iter().any(|i| y.contains(i)))
                    || self.reaches(a, b, DEPTH)
                    || self.reaches(b, a, DEPTH)
            })
        })
    }

    // Whether the items two keys name share a gloss word, as spelling
    // variants with separate entries do (e.g. Old English amānsumian and
    // amænsumian, both "to excommunicate").
    fn alike(&self, string_pool: &StringPool, a: LangTerm, b: LangTerm) -> bool {
        let words = |key: LangTerm| {
            gloss_words(
                self.candidates(key)
                    .into_iter()
                    .flatten()
                    .filter_map(|&item| match self.get(item) {
                        Item::Real(real) => Some(real.gloss.iter()),
                        Item::Imputed(_) => None,
                    })
                    .flatten()
                    .flat_map(|g| g.words(string_pool)),
            )
        };
        !words(a).is_disjoint(&words(b))
    }

    // A candidate contradicts the citation if its etymology names terms in the
    // language of `expected`, all of them existing, unrelated to it and unlike
    // it in meaning.
    fn contradicts(&self, string_pool: &StringPool, candidate: ItemId, expected: LangTerm) -> bool {
        let same_lang: Vec<LangTerm> = self
            .source_keys(candidate)
            .into_iter()
            .filter(|m| m.lang == expected.lang)
            .collect();
        !same_lang.is_empty()
            && same_lang
                .iter()
                .all(|&m| {
                    self.candidates(m).is_some()
                        && !self.related(m, expected)
                        && !self.alike(string_pool, m, expected)
                })
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

    // Among candidates, the one that fits the citation best by meaning
    // (embeddings::citation_score), preferring one of the citing item's part of
    // speech. A single candidate is taken without comparison.
    fn best_candidate(
        &self,
        embeddings: &Embeddings,
        citation: &Citation,
        candidates: &[ItemId],
    ) -> Result<Option<(ItemId, f32)>> {
        if let [only] = candidates {
            return Ok(Some((*only, SINGLE_CANDIDATE_CONFIDENCE)));
        }
        let citer = embeddings.get(self.get(citation.citer), citation.citer)?;
        let gloss = match citation.gloss {
            Some(gloss) => embeddings.text(gloss)?,
            None => None,
        };
        let citer_is_child = matches!(citation.relation, Relation::Parent { .. });
        let citer_pos = self.get(citation.citer).pos();
        let mut best: Option<(ItemId, f32)> = None;
        for &candidate in candidates {
            let embedding = embeddings.get(self.get(candidate), candidate)?;
            let candidate_pos = self.get(candidate).pos();
            let same_pos = match citation.pos {
                Some(pos) => candidate_pos.is_some_and(|c| c.contains(&pos)),
                None => citer_pos
                    .zip(candidate_pos)
                    .is_some_and(|(a, b)| a.iter().any(|p| b.contains(p))),
            };
            let score = embeddings::citation_score(&citer, &embedding, citer_is_child, gloss.as_ref())
                + if same_pos { SAME_POS_BONUS } else { 0.0 };
            if best.is_none_or(|(_, s)| score > s) {
                best = Some((candidate, score));
            }
        }
        Ok(best)
    }

    pub(crate) fn resolve(
        &self,
        string_pool: &StringPool,
        embeddings: &Embeddings,
        citation: &Citation,
    ) -> Result<Resolution> {
        if let Some(id) = citation.id
            && let Some(item) = self.get_id_item(citation.langterm, id)
        {
            self.resolution_stats.count("id");
            return Ok(Resolution::Found {
                item,
                confidence: 1.0,
            });
        }
        let Some(real) = self.candidates(citation.langterm) else {
            self.resolution_stats.count("none found");
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
            self.resolution_stats.count("single candidate");
            let best = self.best_candidate(embeddings, citation, candidates)?;
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
            self.resolution_stats.count("agreement");
            let best = self.best_candidate(embeddings, citation, &agreeing)?;
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
                .filter(|&c| !self.contradicts(string_pool, c, expected))
                .collect(),
            None => candidates.to_vec(),
        };
        let pool = match (&citation.relation, open.is_empty()) {
            // every candidate is some other word: the one meant is missing
            (Relation::Parent { .. }, true) => {
                self.resolution_stats.count("missing (all contradicted)");
                // a deterministic sample for review
                let citer = self.get(citation.citer);
                let line = format!(
                    "{} {} cites {} {} (from {:?})",
                    citer.lang().code(),
                    citer.term().resolve(string_pool),
                    citation.langterm.lang.code(),
                    citation.langterm.term.resolve(string_pool),
                    expected.map(|e| format!("{} {}", e.lang.code(), e.term.resolve(string_pool))),
                );
                if xxhash_rust::xxh3::xxh3_64(line.as_bytes()) % 20 == 0 {
                    self.resolution_stats.missing_sample.borrow_mut().push(line);
                }
                return Ok(Resolution::Missing);
            }
            // Descendants lists often skip steps; keep to the list
            (Relation::Child, true) => candidates,
            _ => &open,
        };
        self.resolution_stats.count(match (pool.len(), open.len() < candidates.len()) {
            (1, true) => "contradiction of the others",
            (1, false) => "single candidate",
            _ => "gloss or embedding",
        });
        let best = self.best_candidate(embeddings, citation, pool)?;
        Ok(best.map_or(Resolution::Missing, |(item, confidence)| Resolution::Found {
            item,
            confidence,
        }))
    }
}
