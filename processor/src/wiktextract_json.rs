use crate::{
    gloss::Gloss,
    items::{Item, RawItems},
    langterm::{Lang, Term},
    pos::Pos,
    redirects::WiktextractJsonRedirect,
    string_pool::StringPool,
};

use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use anyhow::{Ok, Result};
use bytelines::ByteLines;
use flate2::read::GzDecoder;
use simd_json::{to_borrowed_value, ValueAccess};

pub(crate) fn wiktextract_lines(path: &Path) -> Result<impl Iterator<Item = Vec<u8>>> {
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

pub(crate) fn process_wiktextract_lines(
    string_pool: &mut StringPool,
    path: &Path,
) -> Result<RawItems> {
    let mut items = RawItems::default();
    for (line_number, mut line) in wiktextract_lines(path)?.enumerate() {
        let json = to_borrowed_value(&mut line)?;
        items.total_ok_lines_in_file += 1;
        // Some wiktionary pages are redirects. These are actually used somewhat
        // heavily, so we need to take them into account
        // https://github.com/tatuylonen/wiktextract#format-of-extracted-redirects
        if json.contains_key("redirect") {
            let redirect = WiktextractJsonRedirect { json };
            redirect.process(string_pool, &mut items);
        } else {
            let item = WiktextractJsonItem { json };
            item.process(string_pool, &mut items, line_number);
        }
    }
    Ok(items)
}

pub(crate) type WiktextractJson<'a> = simd_json::value::borrowed::Value<'a>;

pub(crate) trait WiktextractJsonValidStr {
    fn get_valid_str(&self, key: &str) -> Option<&str>;
}

impl WiktextractJsonValidStr for WiktextractJson<'_> {
    // return a cleaned version of the str if it exists
    fn get_valid_str(&self, key: &str) -> Option<&str> {
        self.get_str(key)
            // even though get_valid_str is called on other bits of wiktextract
            // json such as template lang args, clean_ety_term should never
            // effect them unless they're degenerate anyway, so we always call
            // this
            .map(clean_ety_term)
            .and_then(|s| (!s.is_empty() && s != "-").then_some(s))
    }
}

pub(crate) struct WiktextractJsonItem<'a> {
    pub(crate) json: WiktextractJson<'a>,
}

impl WiktextractJsonItem<'_> {
    fn process(&self, string_pool: &mut StringPool, items: &mut RawItems, line_number: usize) {
        if let Some(page_term) = self.get_page_term(string_pool)
            && let Some(term) = self.get_canonical_term(string_pool)
            && let Some(lang) = self.get_lang()
            && let Some(pos) = self.get_pos()
        {
            let ety_num = self.get_ety_num();
            let gloss = self.get_gloss(string_pool);

            let item = Item {
                is_imputed: false,
                i: 0, // temp value that will be changed in items.add()
                lang,
                term,
                page_term: Some(page_term),
                ety_num,
                pos: Some(pos),
                gloss,
            };
            let item_id = items.add(item);
            items.lines.insert(line_number, item_id);

            if let Some(raw_root) = self.get_root(string_pool, lang) {
                items.raw_templates.root.insert(item_id, raw_root);
            }
            if let Some(raw_etymology) = self.get_etymology(string_pool, lang) {
                items.raw_templates.ety.insert(item_id, raw_etymology);
            }
            if let Some(raw_descendants) = self.get_descendants(string_pool) {
                items.raw_templates.desc.insert(item_id, raw_descendants);
            }
        }
    }

    fn get_lang(&self) -> Option<Lang> {
        let lang_code = self.json.get_valid_str("lang_code")?;
        lang_code.try_into().ok()
    }

    // The form of the term used in the page url, e.g. "voco"
    fn get_page_term(&self, string_pool: &mut StringPool) -> Option<Term> {
        let term = self.json.get_valid_str("word")?;
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
                            if let Some(term) = form.get_valid_str("form")
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
            return pos.try_into().ok();
        }
        None
    }

    fn get_ety_num(&self) -> Option<u8> {
        // if langterm has multiple ety's, then 'etymology_number' is
        // present with range 1,2,... Otherwise, this key is missing.
        let ety_num = self.json.get_u8("etymology_number");
        let ety_text = self.json.get_valid_str("etymology_text");
        if ety_num.is_none() && ety_text.is_some() {
            // Most likely there is a single unnumbered "Etymology" section
            return Some(1);
        }
        // will be None when there are no ety sections at all (e.g. in a PIE
        // root page where there are multiple "Root" sections, e.g. see "men-")
        // or when there one or more blank unnumbered Etymology section(s) (very
        // rare). None values will possibly be updated during ProcessedData
        // finalization
        ety_num
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
}

fn clean_ety_term(term: &str) -> &str {
    // Reconstructed terms (e.g. PIE) are supposed to start with "*" when cited
    // in etymologies but their entry titles (and hence wiktextract "word"
    // field) do not. This is done by
    // https://en.wiktionary.org/wiki/Module:links. Sometimes reconstructed
    // terms are missing this *, and sometimes non-reconstructed terms start
    // with * incorrectly. So we strip the * in every case. This will break
    // terms that actually start with *, but there are almost none of these, and
    // none of them are particularly relevant for our purposes AFAIK.
    term.strip_prefix('*').unwrap_or(term)
}

// These two functions needs revisiting depending on results.

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
