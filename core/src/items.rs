use crate::{
    ety_graph::ItemIndex,
    gloss::Gloss,
    langterm::Term,
    languages::Lang,
    pos::Pos,
    string_pool::StringPool,
};

use petgraph::stable_graph::NodeIndex;
use serde::{Deserialize, Serialize};

pub type ItemId = NodeIndex<ItemIndex>;

/// An etymologically distinct item, which may have multiple (pos, gloss)'s
#[derive(Serialize, Deserialize)]
pub struct RealItem {
    pub ety_num: u8,
    pub lang: Lang,
    pub term: Term,
    pub pos: Vec<Pos>,
    pub gloss: Vec<Gloss>,
    pub page_term: Option<Term>,
    pub romanization: Option<Term>,
    pub is_reconstructed: bool,
}

impl RealItem {
    #[must_use]
    pub fn url(&self, string_pool: &StringPool) -> String {
        let page_term = self.page_term.unwrap_or(self.term);
        let url_term = urlencoding::encode(page_term.resolve(string_pool));
        let url_lang_name = self.lang.ety2non().url_name();
        if self.is_reconstructed {
            return format!(
                "https://en.wiktionary.org/wiki/Reconstruction:{url_lang_name}/{url_term}"
            );
        }
        format!("https://en.wiktionary.org/wiki/{url_term}#{url_lang_name}")
    }
}

#[derive(Serialize, Deserialize)]
pub struct ImputedItem {
    pub ety_num: u8,
    pub lang: Lang,
    pub term: Term,
    pub romanization: Option<Term>,
    pub from: ItemId,
}

#[derive(Serialize, Deserialize)]
pub enum Item {
    Real(RealItem),
    Imputed(ImputedItem),
}

impl Item {
    #[must_use]
    pub fn is_imputed(&self) -> bool {
        match self {
            Item::Real(_) => false,
            Item::Imputed(_) => true,
        }
    }

    #[must_use]
    pub fn ety_num(&self) -> u8 {
        match self {
            Item::Real(real_item) => real_item.ety_num,
            Item::Imputed(imputed_item) => imputed_item.ety_num,
        }
    }

    #[must_use]
    pub fn lang(&self) -> Lang {
        match self {
            Item::Real(real_item) => real_item.lang,
            Item::Imputed(imputed_item) => imputed_item.lang,
        }
    }

    #[must_use]
    pub fn term(&self) -> Term {
        match self {
            Item::Real(real_item) => real_item.term,
            Item::Imputed(imputed_item) => imputed_item.term,
        }
    }

    #[must_use]
    pub fn page_term(&self) -> Option<Term> {
        match self {
            Item::Real(real_item) => real_item.page_term,
            Item::Imputed(_) => None,
        }
    }

    #[must_use]
    pub fn pos(&self) -> Option<&Vec<Pos>> {
        match self {
            Item::Real(real_item) => Some(&real_item.pos),
            Item::Imputed(_) => None,
        }
    }

    #[must_use]
    pub fn gloss(&self) -> Option<&Vec<Gloss>> {
        match self {
            Item::Real(real_item) => Some(&real_item.gloss),
            Item::Imputed(_) => None,
        }
    }

    #[must_use]
    pub fn romanization(&self) -> Option<Term> {
        match self {
            Item::Real(real_item) => real_item.romanization,
            Item::Imputed(imputed_item) => imputed_item.romanization,
        }
    }

    #[must_use]
    pub fn url(&self, string_pool: &StringPool) -> Option<String> {
        match self {
            Item::Real(real_item) => Some(real_item.url(string_pool)),
            Item::Imputed(_) => None,
        }
    }

    #[must_use]
    pub fn is_reconstructed(&self) -> bool {
        match self {
            Item::Real(real_item) => real_item.is_reconstructed,
            Item::Imputed(imputed_item) => imputed_item.lang.is_reconstructed(),
        }
    }
}
