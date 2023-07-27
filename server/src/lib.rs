#![allow(clippy::unused_async)]

use processor::{Data, ItemId, Lang, Search};
use serde::Deserialize;

use std::{str::FromStr, sync::Arc};

use anyhow::Result;
use axum::{
    extract::{Path, State},
    response::Json,
};
use axum_extra::extract::Query;
use serde_json::Value;

pub enum Environment {
    Development,
    Production,
}

impl FromStr for Environment {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "production" => Ok(Self::Production),
            _ => Ok(Self::Development),
        }
    }
}

pub struct AppState {
    pub data: Data,
    pub search: Search,
}

impl AppState {
    /// # Errors
    ///
    /// Will return `Err` if deserializing the data file fails.
    pub fn new(data_path: &std::path::Path) -> Result<Self> {
        let data = Data::deserialize(data_path)?;
        let search = data.build_search();
        Ok(Self { data, search })
    }
}

pub async fn lang_search_matches(
    State(state): State<Arc<AppState>>,
    Path(lang): Path<String>,
) -> Json<Value> {
    let matches = state.search.langs(&lang);
    Json(matches)
}

pub async fn item_search_matches(
    State(state): State<Arc<AppState>>,
    Path((lang, term)): Path<(Lang, String)>,
) -> Json<Value> {
    let matches = state.search.items(&state.data, lang, &term);
    Json(matches)
}

#[derive(Deserialize)]
pub struct FilterLangs {
    #[serde(rename = "filterLang")]
    langs: Vec<Lang>,
}

pub async fn item_expansion(
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<ItemId>,
    Query(filter_langs): Query<FilterLangs>,
) -> Json<Value> {
    let query_lang = state.data.lang(item_id);
    Json(
        state
            .data
            .expanded_item_json(item_id, query_lang, &filter_langs.langs),
    )
}

pub async fn item_head_progenitor_tree(
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<ItemId>,
    Query(filter_langs): Query<FilterLangs>,
) -> Json<Value> {
    Json(
        state
            .data
            .head_progenitor_tree(item_id, &filter_langs.langs),
    )
}
