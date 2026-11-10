// An entry's References and footnotes, as credits. The wiktionary_references
// workflow reads each References section of the entries, keyed by page,
// language heading and etymology label, each footnote citing a template, keyed
// by its section and note id too, and the works they cite (its bibliography). A section
// under "Etymology N" covers that etymology; one beside the etymologies, with
// no label, every etymology of its language. Which items those are is ours to
// say: an item takes the credits of the sections covering a line it was read
// from.

use crate::{
    entries::EntryPos,
    items::{Items, intern},
};

use wety_core::{
    HashMap, HashSet,
    assertions::{Credit, Role, Scope, Work},
    items::ItemId,
};

use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use serde::Deserialize;

#[derive(Deserialize, Clone, PartialEq)]
struct SectionCredit {
    work: Box<str>,
    role: Role,
    scope: Scope,
    locator: Option<Box<str>>,
}

#[derive(Deserialize)]
struct Section {
    page: Box<str>,
    lang: Box<str>,
    ety: Option<Box<str>>,
    // a footnote's id; none for a References section
    note: Option<Box<str>>,
    credits: Box<[SectionCredit]>,
}

// a footnote: its page, language heading, etymology label and note id
type NoteKey = (Box<str>, Box<str>, Option<Box<str>>, Box<str>);

#[derive(Deserialize)]
struct BibliographyObject {
    #[serde(rename = "type")]
    kind: Box<str>,
    id: Box<str>,
    citation: Option<Box<str>>,
    year: Option<u16>,
}

// a section of a page: its language heading, etymology label and credits
type PageSection = (Box<str>, Option<Box<str>>, Box<[SectionCredit]>);

#[derive(Default)]
pub(crate) struct References {
    // page -> its sections
    sections: HashMap<Box<str>, Vec<PageSection>>,
    // the works each footnote citing a template cites
    notes: HashMap<NoteKey, Box<[SectionCredit]>>,
    bibliography: HashMap<Box<str>, Work>,
    // each item's credits, from the sections covering it
    items: HashMap<ItemId, Vec<SectionCredit>>,
    // (page, section index) of the sections covering an item
    covering: HashSet<(Box<str>, usize)>,
}

impl References {
    pub(crate) fn load(sections_path: &Path, bibliography_path: &Path) -> Result<Self> {
        let mut refs = Self::default();
        let file =
            File::open(sections_path).with_context(|| sections_path.display().to_string())?;
        for line in BufReader::new(GzDecoder::new(file)).lines() {
            let s: Section = serde_json::from_str(&line?)?;
            match s.note {
                Some(note) => {
                    refs.notes.insert((s.page, s.lang, s.ety, note), s.credits);
                }
                None => refs
                    .sections
                    .entry(s.page)
                    .or_default()
                    .push((s.lang, s.ety, s.credits)),
            }
        }
        let file = File::open(bibliography_path)
            .with_context(|| bibliography_path.display().to_string())?;
        for line in BufReader::new(file).lines() {
            let o: BibliographyObject = serde_json::from_str(&line?)?;
            if &*o.kind == "work" {
                let citation = o.citation.context("a work's citation")?;
                refs.bibliography.insert(
                    o.id.clone(),
                    Work {
                        id: o.id,
                        citation,
                        year: o.year,
                    },
                );
            }
        }
        Ok(refs)
    }

    pub(crate) fn sections(&self) -> usize {
        self.sections.values().map(Vec::len).sum()
    }

    /// References knowing one footnote's works, for tests.
    #[cfg(test)]
    pub(crate) fn with_note(
        key: (&str, &str, Option<&str>, &str),
        works: &[(Work, Option<&str>)],
    ) -> Self {
        let mut refs = Self::default();
        let credits = works
            .iter()
            .map(|(work, locator)| SectionCredit {
                work: work.id.clone(),
                role: Role::Follows,
                scope: Scope::Note,
                locator: locator.map(Into::into),
            })
            .collect();
        for (work, _) in works {
            refs.bibliography.insert(work.id.clone(), work.clone());
        }
        refs.notes.insert(
            (
                key.0.into(),
                key.1.into(),
                key.2.map(Into::into),
                key.3.into(),
            ),
            credits,
        );
        refs
    }

    pub(crate) fn notes(&self) -> usize {
        self.notes.len()
    }

    /// The works a footnote cites through templates, each with its locator, or
    /// None when it cites none (it is read from its text).
    pub(crate) fn note_works(
        &self,
        page: &str,
        lang: &str,
        ety: Option<&str>,
        note: &str,
    ) -> Option<Vec<(&Work, Option<&str>)>> {
        let key: NoteKey = (page.into(), lang.into(), ety.map(Into::into), note.into());
        let credits = self.notes.get(&key)?;
        Some(
            credits
                .iter()
                .filter_map(|c| Some((self.bibliography.get(&c.work)?, c.locator.as_deref())))
                .collect(),
        )
    }

    pub(crate) fn covering(&self) -> usize {
        self.covering.len()
    }

    pub(crate) fn items(&self) -> usize {
        self.items.len()
    }

    // The sections covering the entry an item was read from: those of its
    // page and language, under its etymology or beside them all.
    pub(crate) fn cover(&mut self, item: ItemId, entry: &EntryPos) {
        let (Some(page), Some(lang)) = (entry.page(), entry.lang_heading()) else {
            return;
        };
        let Some(sections) = self.sections.get(page) else {
            return;
        };
        let label = entry.get_ety_label();
        for (n, (heading, ety, credits)) in sections.iter().enumerate() {
            if &**heading == lang && (ety.is_none() || ety.as_deref() == label.as_deref()) {
                let found = self.items.entry(item).or_default();
                for credit in credits {
                    if !found.contains(credit) {
                        found.push(credit.clone());
                    }
                }
                self.covering.insert((page.into(), n));
            }
        }
    }
}

impl Items {
    /// An item's entry credits, each work counted once in the graph.
    pub(crate) fn entry_credits(&mut self, item: ItemId) -> Result<Vec<Credit>> {
        let Some(credits) = self.references.items.get(&item) else {
            return Ok(vec![]);
        };
        credits
            .iter()
            .map(|c| {
                let work = self
                    .references
                    .bibliography
                    .get(&c.work)
                    .with_context(|| format!("{} is not in the bibliography", c.work))?;
                Ok(Credit {
                    work: intern(&mut self.works, &mut self.graph.works, work),
                    role: c.role,
                    scope: c.scope,
                    locator: c.locator.clone(),
                })
            })
            .collect()
    }
}
