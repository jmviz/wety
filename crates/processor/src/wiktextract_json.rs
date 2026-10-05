use crate::{
    descendants::RawDescendants,
    items::Items,
    redirects::WiktextractJsonRedirect,
};
use wety_core::{
    gloss::Gloss,
    items::RealItem,
    langterm::Term,
    languages::Lang,
    pos::Pos,
    string_pool::StringPool,
};

use std::{
    borrow::Cow,
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

/// The most common affix kinds.
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub(crate) enum Affix {
    Prefix,
    Suffix,
    Infix,
    Base, // i.e., not an affix
}

pub(crate) trait WiktextractJsonValidStr<'a> {
    fn get_valid_str(&self, key: &str) -> Option<&str>;
    fn get_valid_term(&self, key: &str) -> Option<&str>;
    fn get_affix_term(&'a self, key: &str, affix_kind: &Affix) -> Option<Cow<'a, str>>;
}

impl<'a> WiktextractJsonValidStr<'a> for WiktextractJson<'a> {
    /// Return a cleaned version of the str if it exists.
    fn get_valid_str(&self, key: &str) -> Option<&str> {
        self.get_str(key)
            .and_then(|s| (!s.is_empty() && s != "-").then_some(s))
    }

    /// A stricter version of `get_valid_str` for terms.
    fn get_valid_term(&self, key: &str) -> Option<&str> {
        self.get_str(key)
            .map(clean_template_term)
            .and_then(|s| (!s.is_empty() && s != "-").then_some(s))
    }

    // Used for prefix (suffix) templates to handle the case of a proto-root
    // that ends (begins) with a hyphen being placed in the prefix (suffix) arg
    // position. Usually the hyphen is not included for a prefix (suffix) term
    // in the template, so we want to add it. But in this case the hyphen is
    // already there, correctly, so we don't want to add another.
    /// Get a valid affix term and add hyphen(s) if needed.
    fn get_affix_term(&'a self, key: &str, affix_kind: &Affix) -> Option<Cow<'a, str>> {
        let mut term = Cow::from(self.get_valid_term(key)?);
        match affix_kind {
            Affix::Prefix => {
                if !term.ends_with('-') {
                    term.to_mut().push('-');
                }
            }
            Affix::Suffix => {
                if !term.starts_with('-') {
                    term.to_mut().insert(0, '-');
                }
            }
            Affix::Infix => {
                if !term.starts_with('-') {
                    term.to_mut().insert(0, '-');
                }
                if !term.ends_with('-') {
                    term.to_mut().push('-');
                }
            }
            Affix::Base => {}
        }
        Some(term)
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
            let ety_label = json_item.get_ety_label();
            let item = RealItem {
                ety_num: ety_label
                    .as_deref()
                    .and_then(|label| label.parse().ok())
                    .unwrap_or(1),
                ety_label,
                lang,
                term,
                pos: vec![pos],
                gloss: vec![gloss],
                page_term: (page_term != term).then_some(page_term),
                romanization: json_item.get_romanization(string_pool),
                is_reconstructed: json_item.is_reconstructed(),
            };
            let (item_id, is_new_ety) = self.add_real(item);
            for id in json_item.get_ids(lang) {
                self.add_id(item_id, id);
            }
            if is_new_ety { // a new item was added
                // This means that the glosses embedding for a multi-pos item
                // will be based on the glosses for whichever pos happens to
                // first in the wiktextract data. $$ This may be good enough or
                // may require better handling in the future...
                self.lines.insert(line_number, item_id);
                let raw_roots = json_item.get_roots(string_pool, lang);
                if !raw_roots.is_empty() {
                    self.raw_templates.root.insert(item_id, raw_roots.into_boxed_slice());
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
                            if let Some(term) = form.get_valid_term("form")
                                && !should_ignore_term(term)
                                && self
                                    .json
                                    .get_valid_term("word")
                                    .is_some_and(|word| is_respelling(term, word))
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

    fn get_ety_label(&self) -> Option<Box<str>> {
        // If langterm has multiple ety's, then 'etymology_number' is present:
        // "1", "2", ... or, on some Semitic pages, "1.1", "1.2", ... (older
        // dumps gave plain integers). Otherwise this key is missing: most
        // likely there is a single unnumbered "Etymology" section, or none at
        // all (e.g. a PIE root page with several "Root" sections, see "men-"),
        // or several unnumbered ones (very rare defective page).
        let value = self.json.get("etymology_number")?;
        let label = match value.as_str() {
            Some(s) => s.trim().into(),
            None => value.as_u64()?.to_string().into_boxed_str(),
        };
        (!label.is_empty()).then_some(label)
    }

    fn get_gloss(&self, string_pool: &mut StringPool) -> Option<Gloss> {
        // 'senses' key should always be present with non-empty value, but glosses
        // may be missing or empty.
        self.json
            .get_array("senses")
            .and_then(|senses| senses.first())
            .and_then(|sense| sense.get_array("glosses"))
            .and_then(|glosses| glosses.first())
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
                    .any(|tag| tag.as_str().is_some_and(|s| s == "reconstruction"))
            })
    }
}

/// Split `text<a:1><b<c>>` into `text` and the contents of its top-level
/// angle-bracket groups (`a:1`, `b<c>`). An unbalanced tail is dropped.
pub(crate) fn split_modifiers(s: &str) -> (&str, Vec<&str>) {
    let Some(start) = s.find('<') else {
        return (s, vec![]);
    };
    let mut modifiers = vec![];
    let mut depth = 0usize;
    let mut open = start;
    for (i, c) in s.char_indices().skip_while(|&(i, _)| i < start) {
        match c {
            '<' => {
                if depth == 0 {
                    open = i;
                }
                depth += 1;
            }
            '>' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    modifiers.push(&s[open + 1..i]);
                }
            }
            _ => {}
        }
    }
    (&s[..start], modifiers)
}

pub(crate) fn modifier<'a>(modifiers: &[&'a str], name: &str) -> Option<&'a str> {
    modifiers.iter().find_map(|m| {
        m.strip_prefix(name)
            .and_then(|rest| rest.strip_prefix(':'))
            .map(str::trim)
    })
}

/// Split a `lang:term` prefix off a term arg, as Module:links and Module:affix
/// accept it (e.g. {{af|pt|a-|grc:χρῶμα|-ia}}). Only checks the syntax of the
/// code; callers decide what an unknown code means.
pub(crate) fn split_lang_prefix(s: &str) -> Option<(&str, &str)> {
    let (code, term) = s.split_once(':')?;
    (code.starts_with(|c: char| c.is_ascii_alphabetic())
        && code
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')))
    .then_some((code, term))
}

/// An explicit affix type, as given by {{af}}'s typeN=/<type:> or
/// {{etymon}}'s <aftype:>.
pub(crate) fn affix_type(name: &str) -> Option<Affix> {
    Some(match name {
        "prefix" | "pre" => Affix::Prefix,
        "suffix" | "suf" => Affix::Suffix,
        "infix" | "in" | "interfix" | "inter" => Affix::Infix,
        "non-affix" | "naf" | "root" => Affix::Base,
        _ => return None,
    })
}

/// The affix type implied by hyphens, as {{af}} infers it.
pub(crate) fn affix_by_hyphens(term: &str) -> Affix {
    match (term.starts_with('-'), term.ends_with('-')) {
        (true, true) => Affix::Infix,
        (true, false) => Affix::Suffix,
        (false, true) => Affix::Prefix,
        (false, false) => Affix::Base,
    }
}

/// Clean a term that appears as a template arg
pub(crate) fn clean_template_term(mut term: &str) -> &str {
    // A term may be a link whose target is the lemma, e.g.
    // {{af|it|la:[[lītus|lītoris]]|-aneo}}.
    if let Some(link) = term.strip_prefix("[[") {
        term = link.split(['|', ']']).next().unwrap_or(link);
    }
    // Reconstructed terms (e.g. PIE) are supposed to start with "*" when cited
    // in etymologies but their entry titles (and hence wiktextract "word"
    // field) do not. This is done by
    // https://en.wiktionary.org/wiki/Module:links. Sometimes reconstructed
    // terms are missing this *, and sometimes non-reconstructed terms start
    // with * incorrectly. So we strip the * in every case. This will break
    // terms that actually start with *, but there are almost none of these, and
    // none of them are particularly relevant for our purposes AFAIK ($$).
    term = term.strip_prefix('*').unwrap_or(term);
    // Terms in templates may carry inline modifiers, e.g.
    // {{af|en|non-<id:not>|sense}} or {{slbor|fro|la|līber<id:free>}}. Page
    // titles cannot contain "<", so we cut there.
    term = term.split_once('<').map_or(term, |(t, _)| t);
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

/// Whether a "canonical" form plausibly respells the page title (adding
/// diacritics, niqqud, Old Irish "·", etc.): it must have the same number of
/// words. Wiktextract sometimes takes headword-line labels or articles into
/// the canonical form, e.g. "octobre oblique singular or" for Old French
/// octobre ({{fro-noun}}), "the pond", "a abate 3rd conjugation" or
/// "kun # second-person singular imperative of kien".
fn is_respelling(canonical: &str, title: &str) -> bool {
    canonical.split_whitespace().count() == title.split_whitespace().count()
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
    term.contains(',')
}

fn should_ignore_pos(pos: &str) -> bool {
    pos.contains("phrase")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ety_label(json: &str) -> Option<Box<str>> {
        let mut bytes = json.as_bytes().to_vec();
        let json = to_borrowed_value(&mut bytes).unwrap();
        WiktextractJsonItem { json }.get_ety_label()
    }

    #[test]
    fn ety_labels() {
        assert_eq!(ety_label(r#"{"etymology_number": "3"}"#).as_deref(), Some("3"));
        assert_eq!(ety_label(r#"{"etymology_number": 2}"#).as_deref(), Some("2"));
        assert_eq!(ety_label(r#"{"etymology_number": "1.1"}"#).as_deref(), Some("1.1"));
        assert_eq!(ety_label(r#"{"word": "lime"}"#), None);
    }

    fn canonical_term(json: &str) -> String {
        let mut bytes = json.as_bytes().to_vec();
        let json = to_borrowed_value(&mut bytes).unwrap();
        let mut string_pool = StringPool::new();
        WiktextractJsonItem { json }
            .get_canonical_term(&mut string_pool)
            .unwrap()
            .resolve(&string_pool)
            .to_owned()
    }

    #[test]
    fn canonical_terms() {
        let form = |word: &str, canonical: &str| {
            canonical_term(&format!(
                r#"{{"word": "{word}", "forms": [{{"form": "{canonical}", "tags": ["canonical"]}}]}}"#
            ))
        };
        assert_eq!(form("matrix", "mātrīx"), "mātrīx");
        assert_eq!(form("מילה", "מִלָּה"), "מִלָּה");
        assert_eq!(form("dorat", "do·rat"), "do·rat");
        assert_eq!(form("qui docet", "quī docet"), "quī docet");
        assert_eq!(form("octobre", "octobre oblique singular or"), "octobre");
        assert_eq!(form("pond", "the pond"), "pond");
        assert_eq!(form("abate", "a abate 3rd conjugation"), "abate");
    }

    #[test]
    fn clean_template_terms() {
        assert_eq!("gaberaną", clean_template_term("*gaberaną"));
        assert_eq!("bʰel-", clean_template_term("*bʰel- (shiny)"));
        assert_eq!("twig", clean_template_term("twig#Etymology_2"));
        assert_eq!("līber", clean_template_term("līber<id:free>"));
        assert_eq!("non-", clean_template_term("non-<id:not><t:no>"));
        assert_eq!("lītus", clean_template_term("[[lītus|lītoris]]"));
        assert_eq!("lītus", clean_template_term("[[lītus]]"));
    }
}
