use crate::{
    items::Items,
    langterm::{LangTerm, Term},
    languages::Lang,
    string_pool::StringPool,
    wiktextract_json::{WiktextractJson, WiktextractJsonValidStr},
    HashMap,
};

use phf::{phf_set, Set};

#[derive(Default)]
pub(crate) struct Redirects {
    reconstruction: HashMap<LangTerm, LangTerm>,
    regular: HashMap<Term, Term>,
}

impl Redirects {
    // If a redirect page exists for given lang + term combo, get the redirect.
    // If not, just return back the original lang + term.
    fn get(&self, langterm: LangTerm) -> LangTerm {
        if let Some(&redirect) = self.reconstruction.get(&langterm) {
            return redirect;
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
        let non_ety_lang = langterm.lang.ety2non();
        // Then we also check if there is a redirect for this lang term combo.
        self.get(LangTerm::new(non_ety_lang, langterm.term))
    }
}

static IGNORED_REDIRECTS: Set<&'static str> = phf_set! {
    "Index", "Help", "MediaWiki", "Citations", "Concordance", "Rhymes",
    "Thread", "Summary", "File", "Transwiki", "Category", "Appendix",
    "Wiktionary", "Thesaurus", "Module", "Template"
};

pub(crate) struct WiktextractJsonRedirect<'a> {
    pub(crate) json: WiktextractJson<'a>,
}

impl Items {
    pub(crate) fn process_redirect(
        &mut self,
        string_pool: &mut StringPool,
        redirect: &WiktextractJsonRedirect,
    ) {
        // cf. https://github.com/tatuylonen/wiktextract/blob/master/wiktwords

        if let Some(from_title) = redirect.json.get_valid_str("title")
            && let Some(to_title) = redirect.json.get_valid_str("redirect")
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
                    self.redirects.reconstruction.insert(from_title, to_title);
                }
                return;
            }
            // otherwise, this is a simple term-to-term redirect
            let from_title = Term::new(string_pool, from_title);
            let to_title = Term::new(string_pool, to_title);
            self.redirects.regular.insert(from_title, to_title);
        }
    }
}

fn process_reconstruction_title(string_pool: &mut StringPool, title: &str) -> Option<LangTerm> {
    // e.g. Reconstruction:Proto-Germanic/pīpǭ
    let title = title.strip_prefix("Reconstruction:")?;
    let slash = title.find('/')?;
    let lang_name = title.get(..slash)?;
    let term = title.get(slash + 1..)?;
    let lang = Lang::from_name(lang_name).ok()?;
    Some(lang.new_langterm(string_pool, term))
}
