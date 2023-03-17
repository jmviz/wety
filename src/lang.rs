use crate::{
    lang_phf::{LANG_CODE2NAME, LANG_ETYCODE2CODE, LANG_RECONSTRUCTED},
    phf_ext::OrderedMapExt,
};

pub(crate) fn etylang2lang(lang: usize) -> usize {
    // If lang is an etymology-only language, we will not find any entries
    // for it in Items lang map, since such a language definitionally does
    // not have any entries itself. So we look for the actual lang that the
    // ety lang is associated with.
    LANG_CODE2NAME
        .get_index_key(lang)
        .and_then(|code| {
            LANG_ETYCODE2CODE
                .get(code)
                .and_then(|code| LANG_CODE2NAME.get_index(code))
        })
        .unwrap_or(lang)
}

pub(crate) fn is_reconstructed_lang(lang: usize) -> bool {
    LANG_CODE2NAME
        .get_index_key(lang)
        .map_or(false, |code| LANG_RECONSTRUCTED.contains(code))
}
