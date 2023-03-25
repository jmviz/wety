use crate::{
    items::RawItems,
    langterm::{LangTerm, Language, LanguageTerm, Term},
    string_pool::StringPool,
    wiktextract_json::{WiktextractJson, WiktextractJsonValidStr},
};

use hashbrown::HashMap;
use phf::{phf_set, Set};

#[derive(Default)]
pub(crate) struct Redirects {
    reconstruction: HashMap<LanguageTerm, LanguageTerm>,
    regular: HashMap<Term, Term>,
}

impl Redirects {
    // If a redirect page exists for given lang + term combo, get the redirect.
    // If not, just return back the original lang + term.
    fn get(&self, langterm: LangTerm) -> LangTerm {
        if let Some(&redirect) = self.reconstruction.get(&LanguageTerm::from(langterm)) {
            return redirect.into();
        } else if let Some(&redirect_term) = self.regular.get(&langterm.term) {
            return LangTerm::new(langterm.lang, redirect_term);
        }
        langterm
    }
    pub(crate) fn rectify_langterm(&self, langterm: LangTerm) -> LangTerm {
        // If lang is an etymology-only language, we will not find any entries
        // for it in Items lang map, since such a language definitionally does
        // not have any entries itself. So we look for the actual lang that the
        // ety lang is associated with.
        let main_lang = langterm.lang.ety2main();
        // Then we also check if there is a redirect for this lang term combo.
        self.get(LangTerm::new(main_lang, langterm.term))
    }
}

static IGNORED_REDIRECTS: Set<&'static str> = phf_set! {
    "Index", "Help", "MediaWiki", "Citations", "Concordance", "Rhymes",
    "Thread", "Summary", "File", "Transwiki", "Category", "Appendix",
    "Wiktionary", "Thesaurus", "Module", "Template"
};

pub(crate) struct WiktextractJsonRedirect<'a> {
    json: WiktextractJson<'a>,
}

impl WiktextractJsonRedirect<'_> {
    pub(crate) fn process(&self, string_pool: &mut StringPool, items: &mut RawItems) {
        // cf. https://github.com/tatuylonen/wiktextract/blob/master/wiktwords

        if let Some(from_title) = self.json.get_valid_str("title")
            && let Some(to_title) = self.json.get_valid_str("redirect")
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
            if let Some(from_title) = process_reconstruction_title(string_pool, from_title) {
                // e.g. "Reconstruction:Proto-West Germanic/pīpā"
                if let Some(to_title) = process_reconstruction_title(string_pool, to_title) {
                    items.redirects.reconstruction.insert(from_title, to_title);
                }
                return;
            }
            // otherwise, this is a simple term-to-term redirect
            let from_title = Term::new(string_pool, from_title);
            let to_title = Term::new(string_pool, to_title);
            items.redirects.regular.insert(from_title, to_title);
        }
    }
}

fn process_reconstruction_title(string_pool: &mut StringPool, title: &str) -> Option<LanguageTerm> {
    // e.g. Reconstruction:Proto-Germanic/pīpǭ
    let title = title.strip_prefix("Reconstruction:")?;
    let slash = title.find('/')?;
    let language = title.get(..slash)?;
    let term = title.get(slash + 1..)?;
    let language = Language::try_from(language).ok()?;
    let term = Term::new(string_pool, term);
    Some(LanguageTerm { language, term })
}
