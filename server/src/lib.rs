#![allow(clippy::unused_async)]

use processor::{Data, ItemId, Lang, Search};

use std::{str::FromStr, sync::Arc};

use anyhow::Result;
use axum::{
    extract::{Path, State},
    response::Json,
};
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

pub async fn get_lang_search_matches(
    Path(lang): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let matches = state.search.langs(&lang);
    Json(matches)
}

pub async fn get_item_search_matches(
    Path((lang, term)): Path<(Lang, String)>,
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let matches = state.search.items(&state.data, lang, &term);
    Json(matches)
}

pub async fn get_item_expansion(
    Path((item_id, filter_lang)): Path<(ItemId, Lang)>,
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    Json(state.data.expanded_item_json(item_id, filter_lang))
}

pub async fn get_item_head_progenitor_tree(
    Path((item_id, filter_lang)): Path<(ItemId, Lang)>,
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    Json(state.data.head_progenitor_tree(item_id, filter_lang))
}
