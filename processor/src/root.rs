use std::{mem, str::FromStr};

use crate::{
    embeddings::{EmbeddingComparand, Embeddings, ItemEmbedding},
    ety_graph::EtyGraph,
    etymology::validate_ety_template_lang,
    etymology_templates::EtyMode,
    items::{ItemId, RawItems, Retrieval},
    langterm::{Lang, LangTerm, Language, Term},
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

impl WiktextractJsonItem<'_> {
    // cf. https://en.wiktionary.org/wiki/Template:root. For now we skip
    // attempting to deal with multiple roots listed in a root template or
    // multiple root templates being listed. In both cases we just take the
    // first root term seen. If we discover it is common, we will handle it.
    pub(crate) fn get_root(&self, string_pool: &mut StringPool, lang: Lang) -> Option<RawRoot> {
        if let Some(templates) = self.json.get_array("etymology_templates") {
            for template in templates {
                if let Some(name) = template.get_valid_str("name")
                    && name == "root"
                    && let Some(args) = template.get("args")
                    && let Some(raw_root) = process_root_template(string_pool, args, lang)
                {
                   return Some(raw_root);
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

fn process_root_template(
    string_pool: &mut StringPool,
    args: &WiktextractJson,
    lang: Lang,
) -> Option<RawRoot> {
    validate_ety_template_lang(args, lang).ok()?;
    let root_lang = args.get_valid_str("2")?;
    let root_lang = Lang::from_str(root_lang).ok()?;
    let mut root_term = args.get_valid_str("3")?;
    // we don't deal with multi-roots for now:
    args.get_valid_str("4").is_none().then_some(())?;

    let mut sense_id = "";
    // Sometimes a root's senseid is given in parentheses after the term in
    // the 3 arg slot, see e.g. https://en.wiktionary.org/wiki/blaze.
    if let Some(right_paren_idx) = root_term.rfind(')')
        && let Some(left_paren_idx) = root_term.rfind(" (")
    {
        sense_id = &root_term[left_paren_idx + 2..right_paren_idx];
        root_term = &root_term[..left_paren_idx];
    } else if let Some(id) = args.get_valid_str("id") {
        sense_id = id;
    }
    let sense_id = (!sense_id.is_empty()).then_some(string_pool.get_or_intern(sense_id));
    let langterm = root_lang.new_langterm(string_pool, root_term);
    Some(RawRoot { langterm, sense_id })
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
    let cat_term_lang = Lang::from(Language::from_str(cat_term_lang_name).ok()?);
    (cat_term_lang == lang).then_some(())?;
    let cat_root_lang_name = caps.get(2).map(|m| m.as_str())?;
    let cat_root_lang = Lang::from(Language::from_str(cat_root_lang_name).ok()?);
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

impl RawItems {
    fn impute_item_root_ety(
        &self,
        ety_graph: &mut EtyGraph,
        embeddings: &Embeddings,
        embedding: &ItemEmbedding,
        item_id: ItemId,
        raw_root: &RawRoot,
    ) -> Result<()> {
        let Retrieval {
            item_id: root_item_id,
            ..
        } = self.get_or_impute_item(ety_graph, embeddings, embedding, raw_root.langterm)?;

        if ety_graph.get_immediate_ety(item_id).is_some() {
            return Ok(());
        }

        let confidence = embedding.cosine_similarity(&embeddings.get(root_item_id)?);
        ety_graph.add_ety(
            item_id,
            EtyMode::Root,
            Some(0u8),
            &[root_item_id],
            &[confidence],
        );
        Ok(())
    }

    pub(crate) fn impute_root_etys(
        &mut self,
        embeddings: &Embeddings,
        ety_graph: &mut EtyGraph,
    ) -> Result<()> {
        let n = self.raw_templates.root.len();
        let pb = progress_bar(n, "Imputing root etys")?;
        let raw_templates_root = mem::take(&mut self.raw_templates.root);
        for (item_id, root) in raw_templates_root {
            let embedding = embeddings.get(item_id)?;
            self.impute_item_root_ety(ety_graph, embeddings, &embedding, item_id, &root)?;
            pb.inc(1);
        }
        pb.finish();
        Ok(())
    }
}
