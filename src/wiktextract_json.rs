use crate::{
    lang::is_reconstructed_lang,
    lang_phf::LANG_CODE2NAME,
    pos_phf::POS,
    raw_items::{RawItem, RawItems},
    RawDataProcessor,
};

use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use anyhow::{Ok, Result};
use bytelines::ByteLines;
use flate2::read::GzDecoder;
use simd_json::{to_borrowed_value, value::borrowed::Value, ValueAccess};

pub(crate) type WiktextractJson<'a> = Value<'a>;

pub(crate) trait WiktextractJsonAccess {
    fn get_valid_str(&self, key: &str) -> Option<&str>;
}

impl WiktextractJsonAccess for WiktextractJson<'_> {
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

impl RawDataProcessor {
    fn process_json_item(
        &mut self,
        items: &mut RawItems,
        json_item: &WiktextractJson,
        line_number: usize,
    ) -> Result<()> {
        // Some wiktionary pages are redirects. These are actually used somewhat
        // heavily, so we need to take them into account
        // https://github.com/tatuylonen/wiktextract#format-of-extracted-redirects
        if json_item.contains_key("redirect") {
            self.process_redirect(items, json_item);
            return Ok(());
        }
        if let Some(page_title) = json_item.get_valid_str("word")
        && let Some(pos) = json_item.get_valid_str("pos")
        && let Some(pos_index) = POS.get_index(pos)
        && !should_ignore_term(page_title, pos)
        && let Some(lang) = json_item.get_valid_str("lang_code")
        && let Some(lang_index) = LANG_CODE2NAME.get_index(lang)
    {
        let term = get_term_canonical_form(json_item).unwrap_or(page_title);
        let term = self.string_pool.get_or_intern(term);
        let page_title = Some(self.string_pool.get_or_intern(page_title));
        // if term-lang combo has multiple ety's, then 'etymology_number' is
        // present with range 1,2,... Otherwise, this key is missing.
        let ety_num = json_item.get_u8("etymology_number");
        // 'senses' key should always be present with non-empty value, but glosses
        // may be missing or empty.
        let gloss = json_item
            .get_array("senses")
            .and_then(|senses| senses.get(0))
            .and_then(|sense| sense.get_array("glosses"))
            .and_then(|glosses| glosses.get(0))
            .and_then(|gloss| gloss.as_str())
            .and_then(|s| (!s.is_empty()).then(|| self.string_pool.get_or_intern(s)));

        let raw_root = self.process_json_root(json_item, lang);
        let raw_etymology = self.process_json_ety(json_item, lang);
        let raw_descendants = self.process_json_descendants(json_item);

        let item = RawItem {
            line: Some(line_number),
            is_imputed: false,
            // $$ This will not catch all reconstructed terms, since some terms
            // in attested languages are reconstructed. Some better inference
            // should be done based on "*" prefix for terms. 
            is_reconstructed: is_reconstructed_lang(lang_index),
            i: items.n,
            lang: lang_index,
            term,
            page_title,
            ety_num,
            pos: Some(pos_index),
            gloss,
            gloss_num: 0, // temp value to be changed if need be in add()
            raw_etymology,
            raw_root,
            raw_descendants,
        };
        if let Some(item) = items.add_to_term_map(item)? {
            items.line_map.insert(line_number, item);
        }
    }
        Ok(())
    }

    pub(crate) fn process_json_items(&mut self, path: &Path) -> Result<RawItems> {
        let mut items = RawItems::default();
        for (line_number, mut line) in wiktextract_lines(path)?.enumerate() {
            let json_item = to_borrowed_value(&mut line)?;
            self.process_json_item(&mut items, &json_item, line_number)?;
            items.total_ok_lines_in_file += 1;
        }
        Ok(items)
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

fn should_ignore_term(term: &str, pos: &str) -> bool {
    // This function needs revisiting depending on results.

    // We would generally like to ignore phrases, and potentially other things.
    //  Barring all phrases may be both too strict and not strict enough. Too
    // strict because certain phrases may be relevant for etymologies (i.e. a
    // phrase became one word in a daughter language). Not strict enough because
    // many phrases are categorized as other pos. See e.g.
    // https://en.wiktionary.org/wiki/this,_that,_or_the_other. Ignoring terms
    // that contain any ascii punctuation is too strict, as this would ingore
    // e.g. affixes with -. Ignoring terms with any ascii whitespace is too
    // strict as well, as this would ignore e.g. circumfixes (e.g. "ver- -en").
    pos.contains("phrase") || term.contains(|c: char| c == ',')
}

// We look for a canonical form, otherwise we take the "word" field.
// See notes.md for motivation.
fn get_term_canonical_form<'a>(json_item: &'a Value) -> Option<&'a str> {
    let forms = json_item.get_array("forms")?;
    let mut f = 0;
    while let Some(form) = forms.get(f) {
        if let Some(tags) = form.get_array("tags") {
            let mut t = 0;
            while let Some(tag) = tags.get(t).as_str() {
                if tag == "canonical" {
                    // There are some
                    if let Some(term) = form.get_valid_str("form") {
                        return Some(term);
                    }
                }
                t += 1;
            }
        }
        f += 1;
    }
    None
}
