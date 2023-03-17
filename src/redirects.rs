use crate::{
    lang::etylang2lang,
    lang_phf::{LANG_CODE2NAME, LANG_NAME2CODE},
    phf_ext::OrderedMapExt,
    raw_items::RawItems,
    string_pool::Symbol,
    wiktextract_json::{WiktextractJson, WiktextractJsonAccess},
    RawDataProcessor,
};

use hashbrown::HashMap;
use phf::{phf_set, Set};

#[derive(Hash, Eq, PartialEq, Debug)]
struct ReconstructionTitle {
    language: usize,
    term: Symbol,
}

#[derive(Default)]
pub(crate) struct Redirects {
    reconstruction: HashMap<ReconstructionTitle, ReconstructionTitle>,
    regular: HashMap<Symbol, Symbol>,
}

impl Redirects {
    // If a redirect page exists for given lang + term combo, get the redirect.
    // If not, just return back the original lang + term.
    fn get(&self, lang: usize, term: Symbol) -> (usize, Symbol) {
        if let Some(language) = LANG_CODE2NAME.get_index_value(lang)
            && let Some(language_index) = LANG_NAME2CODE.get_index(language)
            && let Some(redirect) = self.reconstruction.get(&ReconstructionTitle {
                language: language_index,
                term,
            })
            && let Some(redirect_lang) = LANG_NAME2CODE.get_index_value(redirect.language)
            && let Some(redirect_lang_index) = LANG_CODE2NAME.get_index(redirect_lang)
        {
            return (redirect_lang_index, redirect.term);
        } else if let Some(&redirect_term) = self.regular.get(&term) {
                return (lang, redirect_term);
        }
        (lang, term)
    }
    pub(crate) fn rectify_lang_term(&self, lang: usize, term: Symbol) -> (usize, Symbol) {
        // If lang is an etymology-only language, we will not find any entries
        // for it in Items lang map, since such a language definitionally does
        // not have any entries itself. So we look for the actual lang that the
        // ety lang is associated with.
        let lang = etylang2lang(lang);
        // Then we also check if there is a redirect for this lang term combo.
        self.get(lang, term)
    }
}

static IGNORED_REDIRECTS: Set<&'static str> = phf_set! {
    "Index", "Help", "MediaWiki", "Citations", "Concordance", "Rhymes",
    "Thread", "Summary", "File", "Transwiki", "Category", "Appendix",
    "Wiktionary", "Thesaurus", "Module", "Template"
};

impl RawDataProcessor {
    pub(crate) fn process_redirect(&mut self, items: &mut RawItems, json_item: &WiktextractJson) {
        // cf. https://github.com/tatuylonen/wiktextract/blob/master/wiktwords

        if let Some(from_title) = json_item.get_valid_str("title")
            && let Some(to_title) = json_item.get_valid_str("redirect")
        {
            for title in [from_title, to_title] {
                if let Some(colon) = title.find(':')
                    && let Some(namespace) = title.get(..colon)
                    && IGNORED_REDIRECTS.contains(namespace)
                {
                    return;
                }
            }
            // e.g. Reconstruction:Proto-Germanic/pīpǭ
            if let Some(from_title) = self.process_reconstruction_title(from_title) {
                // e.g. "Reconstruction:Proto-West Germanic/pīpā"
                if let Some(to_title) = self.process_reconstruction_title(to_title) {
                    items.redirects.reconstruction.insert(from_title, to_title);
                }
                return;
            }
            // otherwise, this is a simple term-to-term redirect
            let from_title = self.string_pool.get_or_intern(from_title);
            let to_title = self.string_pool.get_or_intern(to_title);
            items.redirects.regular.insert(from_title, to_title);
        }
    }

    fn process_reconstruction_title(&mut self, title: &str) -> Option<ReconstructionTitle> {
        // e.g. Reconstruction:Proto-Germanic/pīpǭ
        let title = title.strip_prefix("Reconstruction:")?;
        let slash = title.find('/')?;
        let language = &title.get(..slash)?;
        let term = title.get(slash + 1..)?;
        let language_index = LANG_NAME2CODE.get_index(language)?;

        Some(ReconstructionTitle {
            language: language_index,
            term: self.string_pool.get_or_intern(term),
        })
    }
}
