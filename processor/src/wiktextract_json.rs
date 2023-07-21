use crate::{
    descendants::RawDescendants,
    gloss::Gloss,
    items::{Items, RealItem},
    langterm::Term,
    languages::Lang,
    pos::Pos,
    redirects::WiktextractJsonRedirect,
    string_pool::StringPool,
};

use std::{
    fs::File,
    io::{BufReader, Read},
    mem,
    path::Path,
};

use anyhow::{Ok, Result};
use bytelines::ByteLines;
use flate2::read::GzDecoder;
use simd_json::{to_borrowed_value, ValueAccess};

/// Returns an iterator over the lines in the file at the given path.
///
/// # Errors
///
/// This function will return an error if the file at the given path cannot be opened.
pub fn wiktextract_lines(path: &Path) -> Result<impl Iterator<Item = Vec<u8>>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let is_gz_compressed = path.extension().is_some_and(|ext| ext == "gz");
    let uncompressed: Box<dyn Read> = if is_gz_compressed {
        Box::new(GzDecoder::new(reader))
    } else {
        Box::new(reader)
    };
    let reader = BufReader::new(uncompressed);
    let lines = ByteLines::new(reader);
    // We use into_iter() here and thereby allocate a Vec<u8> for each line, so
    // that we have the convenience of returning an iterator. These allocations
    // are not particularly a bottleneck relative to other things so it's fine.
    Ok(lines.into_iter().filter_map(Result::ok))
}

impl Items {
    pub(crate) fn process_wiktextract_lines(
        &mut self,
        string_pool: &mut StringPool,
        path: &Path,
    ) -> Result<()> {
        for (line_number, mut line) in wiktextract_lines(path)?.enumerate() {
            let json = to_borrowed_value(&mut line)?;
            self.total_ok_lines_in_file += 1;
            // Some wiktionary pages are redirects. These are actually used somewhat
            // heavily, so we need to take them into account
            // https://github.com/tatuylonen/wiktextract#format-of-extracted-redirects
            if json.contains_key("redirect") {
                let redirect = WiktextractJsonRedirect { json };
                self.process_redirect(string_pool, &redirect);
            } else {
                let item = WiktextractJsonItem { json };
                self.process_item(string_pool, &item, line_number);
            }
        }
        Ok(())
    }
}

pub(crate) type WiktextractJson<'a> = simd_json::value::borrowed::Value<'a>;

pub(crate) trait WiktextractJsonValidStr {
    fn get_valid_str(&self, key: &str) -> Option<&str>;
    fn get_valid_term(&self, key: &str) -> Option<&str>;
}

impl WiktextractJsonValidStr for WiktextractJson<'_> {
    /// return a cleaned version of the str if it exists
    fn get_valid_str(&self, key: &str) -> Option<&str> {
        self.get_str(key)
            .and_then(|s| (!s.is_empty() && s != "-").then_some(s))
    }

    /// A stricter version of `get_valid_str` for terms
    fn get_valid_term(&self, key: &str) -> Option<&str> {
        self.get_str(key)
            .map(clean_template_term)
            .and_then(|s| (!s.is_empty() && s != "-").then_some(s))
    }
}

pub(crate) struct WiktextractJsonItem<'a> {
    pub(crate) json: WiktextractJson<'a>,
}

impl Items {
    fn process_item(
        &mut self,
        string_pool: &mut StringPool,
        json_item: &WiktextractJsonItem,
        line_number: usize,
    ) {
        if let Some(page_term) = json_item.get_page_term(string_pool)
            && let Some(term) = json_item.get_canonical_term(string_pool)
            && let Some(lang) = json_item.get_lang()
            && let Some(pos) = json_item.get_pos()
            && let Some(gloss) = json_item.get_gloss(string_pool)
        {
            let item = RealItem {
                ety_num: json_item.get_ety_num(),
                lang,
                term,
                pos: vec![pos],
                gloss: vec![gloss],
                page_term: (page_term != term).then_some(page_term),
                romanization: json_item.get_romanization(string_pool),
                is_reconstructed: json_item.is_reconstructed(),
            };
            let (item_id, is_new_ety) = self.add_real(item);
            if is_new_ety { // a new item was added
                // This means that the glosses embedding for a multi-pos item
                // will be based on the glosses for whichever pos happens to
                // first in the wiktextract data. $$ This may be good enough or
                // may require better handling in the future...
                self.lines.insert(line_number, item_id);
                if let Some(raw_root) = json_item.get_root(string_pool, lang) {
                    self.raw_templates.root.insert(item_id, raw_root);
                }
                if let Some(raw_etymology) = json_item.get_etymology(string_pool, lang) {
                    self.raw_templates.ety.insert(item_id, raw_etymology);
                }
                if let Some(raw_descendants) = json_item.get_descendants(string_pool) {
                    self.raw_templates.desc.insert(item_id, raw_descendants);
                }
                return;
            }
            // This was a new pos of an existing item. 
            if let Some(mut raw_descendants) = json_item.get_descendants(string_pool) {
                // Sometimes multiple pos's under the same ety have different
                // Descendants sections. This handles that by simply joining the
                // lists into one. $$ This does assume that each list uses the
                // same base level of indentation though...
                if let Some(existing) = self.raw_templates.desc.get_mut(&item_id) {
                    let mut ex_lines = Vec::from(mem::take(&mut existing.lines));
                    let new_lines = Vec::from(mem::take(&mut raw_descendants.lines));
                    ex_lines.extend(new_lines);
                    let full = RawDescendants::from(ex_lines);
                    self.raw_templates.desc.insert(item_id, full);
                }
                self.raw_templates.desc.insert(item_id, raw_descendants);
            }
        }
    }
}

impl WiktextractJsonItem<'_> {
    fn get_lang(&self) -> Option<Lang> {
        let lang_code = self.json.get_valid_str("lang_code")?;
        lang_code.parse().ok()
    }

    // The form of the term used in the page url, e.g. "voco"
    fn get_page_term(&self, string_pool: &mut StringPool) -> Option<Term> {
        let term = self.json.get_valid_term("word")?;
        if !should_ignore_term(term) {
            return Some(Term::new(string_pool, term));
        }
        None
    }

    // The canonical form of the term, e.g. "vocō". This is the form generally
    // used in ety templates, which gets converted under the hood by wiktionary
    // Module:languages into the page_term "link" version. See notes.md for
    // more.
    fn get_canonical_term(&self, string_pool: &mut StringPool) -> Option<Term> {
        if let Some(forms) = self.json.get_array("forms") {
            let mut f = 0;
            while let Some(form) = forms.get(f) {
                if let Some(tags) = form.get_array("tags") {
                    let mut t = 0;
                    while let Some(tag) = tags.get(t).as_str() {
                        if tag == "canonical" {
                            // There are some
                            if let Some(term) = form.get_valid_term("form")
                                && !should_ignore_term(term)
                            {
                                return Some(Term::new(string_pool, term));
                            }
                        }
                        t += 1;
                    }
                }
                f += 1;
            }
        }
        self.get_page_term(string_pool)
    }

    fn get_pos(&self) -> Option<Pos> {
        let pos = self.json.get_valid_str("pos")?;
        if !should_ignore_pos(pos) {
            return pos.parse().ok();
        }
        None
    }

    fn get_ety_num(&self) -> u8 {
        // if langterm has multiple ety's, then 'etymology_number' is present
        // with range 1,2,... Otherwise, this key is missing. If it is missing,
        // then most likely there is a single unnumbered "Etymology" section.
        // Or, there could be no ety sections at all (e.g. in a PIE root page
        // where there are multiple "Root" sections, e.g. see "men-"). Or, there
        // could be multiple unnumbered ety sections (very rare defective page).
        // Whatever number is returned here might get changed in items.add()
        // When the item is compared with its dupes and potentially gets merged.
        self.json.get_u8("etymology_number").unwrap_or(1)
    }

    fn get_gloss(&self, string_pool: &mut StringPool) -> Option<Gloss> {
        // 'senses' key should always be present with non-empty value, but glosses
        // may be missing or empty.
        self.json
            .get_array("senses")
            .and_then(|senses| senses.get(0))
            .and_then(|sense| sense.get_array("glosses"))
            .and_then(|glosses| glosses.get(0))
            .and_then(|gloss| gloss.as_str())
            .and_then(|gloss| (!gloss.is_empty()).then(|| Gloss::new(string_pool, gloss)))
    }

    fn get_romanization(&self, string_pool: &mut StringPool) -> Option<Term> {
        for form in self.json.get_array("forms")? {
            if form.get_array("tags").is_some_and(|tags| {
                tags.iter()
                    .filter_map(|tag| tag.as_str())
                    .any(|tag| tag == "romanization")
            }) {
                return form
                    .get_valid_term("form")
                    .map(|romanization| Term::new(string_pool, romanization));
            }
        }
        None
    }

    fn is_reconstructed(&self) -> bool {
        self.json
            .get_array("senses")
            .into_iter()
            .flatten()
            .any(|sense| {
                sense
                    .get_array("tags")
                    .into_iter()
                    .flatten()
                    .any(|tag| tag.as_str().map_or(false, |s| s == "reconstruction"))
            })
    }
}

/// Clean a term that appears as a template arg
fn clean_template_term(mut term: &str) -> &str {
    // Reconstructed terms (e.g. PIE) are supposed to start with "*" when cited
    // in etymologies but their entry titles (and hence wiktextract "word"
    // field) do not. This is done by
    // https://en.wiktionary.org/wiki/Module:links. Sometimes reconstructed
    // terms are missing this *, and sometimes non-reconstructed terms start
    // with * incorrectly. So we strip the * in every case. This will break
    // terms that actually start with *, but there are almost none of these, and
    // none of them are particularly relevant for our purposes AFAIK ($$).
    term = term.strip_prefix('*').unwrap_or(term);
    // Occasionally a term is linked in an ety or descendants section like e.g.
    // on page tuig: {{desc|bor=1|en|twig#Etymology_2}}. Or even more rarely,
    // using #X to link to some other subsection (e.g. language). We just take
    // everything before the first # ($$ do any languages use # that should be
    // exempt from this?)
    term = term.split_once('#').map_or(term, |(t, _)| t);
    // Sometimes senseid is given in parentheses like e.g.
    // {{root|en|ine-pro|*bʰel- (shiny)}}.
    term = term.split_once(" (").map_or(term, |(t, _)| t);
    term
}

// $$ These two functions needs revisiting depending on results.

// We would generally like to ignore phrases, and potentially other things.
//  Barring all phrases may be both too strict and not strict enough. Too
// strict because certain phrases may be relevant for etymologies (i.e. a
// phrase became one word in a daughter language). Not strict enough because
// many phrases are categorized as other pos. See e.g.
// https://en.wiktionary.org/wiki/this,_that,_or_the_other. Ignoring terms
// that contain any ascii punctuation is too strict, as this would ingore
// e.g. affixes with -. Ignoring terms with any ascii whitespace is too
// strict as well, as this would ignore e.g. circumfixes (e.g. "ver- -en").
fn should_ignore_term(term: &str) -> bool {
    term.contains(|c: char| c == ',')
}

fn should_ignore_pos(pos: &str) -> bool {
    pos.contains("phrase")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_template_terms() {
        assert_eq!("gaberaną", clean_template_term("*gaberaną"));
        assert_eq!("bʰel-", clean_template_term("*bʰel- (shiny)"));
        assert_eq!("twig", clean_template_term("twig#Etymology_2"));
    }
}
