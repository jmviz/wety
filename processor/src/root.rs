use std::{mem, str::FromStr};

use crate::{
    embeddings::{EmbeddingComparand, Embeddings, ItemEmbedding},
    etymology::validate_ety_template_lang,
    etymology_templates::EtyMode,
    items::{ItemId, Items, Retrieval},
    langterm::{LangTerm, Term},
    languages::Lang,
    progress_bar,
    string_pool::{StringPool, Symbol},
    wiktextract_json::{WiktextractJson, WiktextractJsonItem, WiktextractJsonValidStr},
};

use anyhow::{Ok, Result};
use lazy_static::lazy_static;
use regex::Regex;
use simd_json::ValueAccess;

#[derive(Hash, Eq, PartialEq, Debug)]
pub(crate) struct RawRoot {
    pub(crate) langterm: LangTerm,
    pub(crate) sense_id: Option<Symbol>,
}

enum RootKind {
    Root,
    Word,
}

impl WiktextractJsonItem<'_> {
    // cf. https://en.wiktionary.org/wiki/Template:root. For now we skip
    // attempting to deal with multiple roots listed in a root template or
    // multiple root templates being listed. In both cases we just take the
    // first root term seen. If we discover it is common, we will handle it. We
    // also handle https://en.wiktionary.org/wiki/Template:PIE_word and
    // https://en.wiktionary.org/wiki/Template:word here. These are both used in
    // the same way as {{root}}; namely, they are placed at the top of the
    // etymology section before the body of the etymology where the "normal" ety
    // templates are found. And all three are used to indicate ultimate descent
    // from a term in a proto-language. For expedience, we gloss over the
    // distinction among them and categorize them all as "root" etys.
    pub(crate) fn get_root(&self, string_pool: &mut StringPool, lang: Lang) -> Option<RawRoot> {
        if let Some(templates) = self.json.get_array("etymology_templates") {
            for template in templates {
                if let Some(name) = template.get_valid_str("name")
                    && let Some(args) = template.get("args")
                {
                   match name {
                        "root" => {
                            return process_root_template(string_pool, args, lang, &RootKind::Root);
                        }
                        "word" => {
                            return process_root_template(string_pool, args, lang, &RootKind::Word);
                        }
                        "PIE word" => {
                            return process_pie_word_template(string_pool, args, lang);
                        }
                        _ => {}
                   }
                }
            }
        }

        // if no {root} found in ety section, look for a category of the form
        // e.g. "English terms derived from the Proto-Indo-European root *dʰeh₁-"
        // or "English terms derived from the Proto-Indo-European root *bʰel- (shiny)"
        if let Some(categories) = self.json.get_array("categories") {
            for category in categories.iter().filter_map(|c| c.as_str()) {
                if let Some(raw_root) = process_json_root_category(string_pool, category, lang) {
                    return Some(raw_root);
                }
            }
        }

        None
    }
}

// https://en.wiktionary.org/wiki/Template:root
// https://en.wiktionary.org/wiki/Template:word
// This handles both {{root}} and {{word}}. The templates work the same, with
// the only exception being that the arg for the proto-language uses the
// standard language code in the case of {{root}}, while "-pro" is clipped off
// in the case of {{word}}.
fn process_root_template(
    string_pool: &mut StringPool,
    args: &WiktextractJson,
    lang: Lang,
    kind: &RootKind,
) -> Option<RawRoot> {
    validate_ety_template_lang(args, lang).ok()?;
    let root_lang = args.get_valid_str("2")?;
    let root_lang = match kind {
        RootKind::Root => Lang::from_str(root_lang).ok()?,
        RootKind::Word => Lang::from_str(&format!("{root_lang}-pro")).ok()?,
    };
    let raw_root_term = args.get_valid_str("3")?;
    let root_term = args.get_valid_term("3")?;
    // we don't deal with multi-roots for now:
    args.get_valid_term("4").is_none().then_some(())?;

    let mut sense_id = "";
    // Sometimes a root's senseid is given in parentheses after the term in
    // the 3 arg slot, see e.g. https://en.wiktionary.org/wiki/blaze.
    if let Some(right_paren_idx) = raw_root_term.rfind(')')
        && let Some(left_paren_idx) = raw_root_term.rfind(" (")
    {
        sense_id = &raw_root_term[left_paren_idx + 2..right_paren_idx];
    } else if let Some(id) = args.get_valid_str("id") {
        sense_id = id;
    }
    let sense_id = (!sense_id.is_empty()).then(|| string_pool.get_or_intern(sense_id));
    let langterm = root_lang.new_langterm(string_pool, root_term);
    Some(RawRoot { langterm, sense_id })
}

// https://en.wiktionary.org/wiki/Template:PIE_word
fn process_pie_word_template(
    string_pool: &mut StringPool,
    args: &WiktextractJson,
    lang: Lang,
) -> Option<RawRoot> {
    validate_ety_template_lang(args, lang).ok()?;
    let pie_lang = Lang::from_str("ine-pro").ok()?;
    let pie_word = args.get_valid_term("2")?;
    let pie_langterm = pie_lang.new_langterm(string_pool, pie_word);
    Some(RawRoot {
        langterm: pie_langterm,
        sense_id: None,
    })
}

fn process_json_root_category(
    string_pool: &mut StringPool,
    category: &str,
    lang: Lang,
) -> Option<RawRoot> {
    lazy_static! {
        static ref ROOT_CAT: Regex =
            Regex::new(r"^(.+) terms derived from the (.+) root \*([^ ]+)(?: \((.+)\))?$").unwrap();
    }
    let caps = ROOT_CAT.captures(category)?;
    let cat_term_lang_name = caps.get(1).map(|m| m.as_str())?;
    let cat_term_lang = Lang::from_name(cat_term_lang_name).ok()?;
    (cat_term_lang == lang).then_some(())?;
    let cat_root_lang_name = caps.get(2).map(|m| m.as_str())?;
    let cat_root_lang = Lang::from_name(cat_root_lang_name).ok()?;
    let cat_root_term = caps.get(3).map(|m| m.as_str())?;
    let cat_root_term = Term::new(string_pool, cat_root_term);
    let cat_root_sense_id = caps
        .get(4)
        .map(|cap| string_pool.get_or_intern(cap.as_str()));
    Some(RawRoot {
        langterm: LangTerm::new(cat_root_lang, cat_root_term),
        sense_id: cat_root_sense_id,
    })
}

impl Items {
    fn impute_item_root_ety(
        &mut self,
        embeddings: &Embeddings,
        embedding: &ItemEmbedding,
        item_id: ItemId,
        raw_root: &RawRoot,
    ) -> Result<()> {
        let Retrieval {
            item_id: root_item_id,
            confidence,
        } = self.get_or_impute_item(embeddings, embedding, item_id, raw_root.langterm)?;

        let root_lang = self.get(root_item_id).lang();

        match self.graph.progenitors(item_id) {
            None => {
                let item = self.get(item_id);
                let item_lang = item.lang();
                if item_lang.strictly_descends_from(root_lang)
                || item.is_imputed() && item_lang.descends_from(root_lang)
                {
                    self.graph.add_ety(
                        item_id,
                        EtyMode::Root,
                        Some(0u8),
                        &[root_item_id],
                        &[confidence],
                    );
                }
            }
            Some(progenitors) => {
                if let Some(head_progenitor_id) = progenitors.head
                    && let head_progenitor = self.get(head_progenitor_id)
                    && !progenitors.items.contains(&root_item_id)
                    && let head_progenitor_lang = head_progenitor.lang()
                    && (
                        head_progenitor_lang.strictly_descends_from(root_lang)
                        || head_progenitor.is_imputed() && head_progenitor_lang.descends_from(root_lang)
                    )
                {
                    let root_embedding = embeddings.get(self.get(root_item_id), root_item_id)?;
                    let hp_embedding = embeddings.get(head_progenitor, head_progenitor_id)?;
                    let similarity = hp_embedding.cosine_similarity(&root_embedding);
                    self.graph.add_ety(
                        head_progenitor_id,
                        EtyMode::Root,
                        Some(0u8),
                        &[root_item_id],
                        &[similarity],
                    );
                }
            }
        }

        Ok(())
    }

    pub(crate) fn impute_root_etys(&mut self, embeddings: &Embeddings) -> Result<()> {
        let n = self.raw_templates.root.len();
        let pb = progress_bar(n, "Imputing root etys")?;
        let raw_templates_root = mem::take(&mut self.raw_templates.root);
        for (item_id, root) in raw_templates_root {
            let embedding = embeddings.get(self.get(item_id), item_id)?;
            self.impute_item_root_ety(embeddings, &embedding, item_id, &root)?;
            pb.inc(1);
        }
        pb.finish();
        Ok(())
    }
}
