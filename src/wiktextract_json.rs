use crate::{
    gloss::Gloss,
    items::{Item, RawItems},
    langterm::{Lang, LangTerm, Term},
    pos::Pos,
    pos_phf::POS,
    string_pool::StringPool,
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
use simd_json::{to_borrowed_value, ValueAccess};

pub(crate) type WiktextractJson<'a> = simd_json::value::borrowed::Value<'a>;

pub(crate) trait WiktextractJsonAccess {
    fn get_valid_str(&self, key: &str) -> Option<&str>;
    fn get_lang(&self) -> Option<Lang>;
    fn get_page_term(&self, string_pool: &mut StringPool) -> Option<Term>;
    fn get_canonical_term(&self, string_pool: &mut StringPool) -> Option<Term>;
    fn get_langterm(&self, string_pool: &mut StringPool) -> Option<LangTerm>;
    fn get_pos(&self) -> Option<Pos>;
    fn get_ety_num(&self) -> Option<u8>;
    fn get_gloss(&self, string_pool: &mut StringPool) -> Option<Gloss>;
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

    fn get_lang(&self) -> Option<Lang> {
        let lang_code = self.get_valid_str("lang_code")?;
        lang_code.try_into().ok()
    }

    // The form of the term used in the page url, e.g. "voco"
    fn get_page_term(&self, string_pool: &mut StringPool) -> Option<Term> {
        if let Some(term) = self.get_valid_str("word")
            && !should_ignore_term(term)
        {
            return Some(Term::new(string_pool, term));
    }
        None
    }

    // The canonical form of the term, e.g. "vocō". This is the form generally
    // used in ety templates, which gets converted under the hood by wiktionary
    // Module:languages into the page_term "link" version. See notes.md for
    // more.
    fn get_canonical_term(&self, string_pool: &mut StringPool) -> Option<Term> {
        if let Some(forms) = self.get_array("forms") {
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

    fn get_langterm(&self, string_pool: &mut StringPool) -> Option<LangTerm> {
        let lang = self.get_lang()?;
        let term = self.get_canonical_term(string_pool)?;
        Some(LangTerm { lang, term })
    }

    fn get_pos(&self) -> Option<Pos> {
        let pos = self.get_valid_str("pos")?;
        if !should_ignore_pos(pos) {
            return pos.try_into().ok();
        }
        None
    }

    fn get_ety_num(&self) -> Option<u8> {
        // if term-lang combo has multiple ety's, then 'etymology_number' is
        // present with range 1,2,... Otherwise, this key is missing.
        self.get_u8("etymology_number")
    }

    fn get_gloss(&self, string_pool: &mut StringPool) -> Option<Gloss> {
        // 'senses' key should always be present with non-empty value, but glosses
        // may be missing or empty.
        self.get_array("senses")
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
        if let Some(page_term) = json_item.get_page_term(&mut self.string_pool)
            && let Some(term) = json_item.get_canonical_term(&mut self.string_pool)
            && let Some(lang) = json_item.get_lang()
        {
            let pos = json_item.get_pos();
            let ety_num = json_item.get_ety_num();
            let gloss = json_item.get_gloss(&mut self.string_pool);

            let raw_root = self.process_json_root(json_item, lang.code());
            let raw_etymology = self.process_json_ety(json_item, lang.code());
            let raw_descendants = self.process_json_descendants(json_item);

            let item = Item {
                line: Some(line_number),
                is_imputed: false,
                // $$ This will not catch all reconstructed terms, since some terms
                // in attested languages are reconstructed. Some better inference
                // should be done based on "*" prefix for terms. 
                is_reconstructed: lang.is_reconstructed(),
                i: items.n,
                lang: lang.id(),
                term,
                page_term,
                ety_num,
                pos,
                gloss,
                raw_etymology,
                raw_root,
                raw_descendants,
            };
            if let Some(item) = items.add(item)? {
                items.lines.insert(line_number, item);
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
