#![allow(clippy::unused_async)]

use processor::{Data, ItemId, Lang, Search};
use serde::Deserialize;

use std::{str::FromStr, sync::Arc};

use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use axum_extra::extract::Query as ExtraQuery;
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

#[derive(Deserialize)]
pub struct LangSearch {
    name: String,
}

pub async fn lang_search_matches(
    State(state): State<Arc<AppState>>,
    Query(lang_search): Query<LangSearch>,
) -> Json<Value> {
    let matches = state.search.langs(&lang_search.name);
    Json(matches)
}

#[derive(Deserialize)]
pub struct ItemSearch {
    term: String,
}

pub async fn item_search_matches(
    State(state): State<Arc<AppState>>,
    Path(lang): Path<Lang>,
    Query(item_search): Query<ItemSearch>,
) -> Json<Value> {
    let matches = state.search.items(&state.data, lang, &item_search.term);
    Json(matches)
}

#[derive(Deserialize)]
pub struct TreeQueries {
    #[serde(rename = "descLang")]
    desc_langs: Vec<Lang>,
    #[serde(rename = "distLang")]
    dist_lang: Option<Lang>,
}

pub async fn item_etymology(
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<ItemId>,
) -> Json<Value> {
    let lang = state.data.lang(item_id);
    Json(state.data.etymology_json(item_id, 0, lang))
}

pub async fn item_head_descendants(
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<ItemId>,
    ExtraQuery(tree_queries): ExtraQuery<TreeQueries>,
) -> Json<Value> {
    let dist_lang = tree_queries.dist_lang.unwrap_or(state.data.lang(item_id));
    let head_ancestors_within_lang = state
        .data
        .ancestors_in_langs(item_id, &tree_queries.desc_langs);
    Json(state.data.item_descendants_json(
        item_id,
        dist_lang,
        &tree_queries.desc_langs,
        &head_ancestors_within_lang,
        None,
        None,
    ))
}

pub async fn item_head_progenitor_tree(
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<ItemId>,
    ExtraQuery(tree_queries): ExtraQuery<TreeQueries>,
) -> Json<Value> {
    Json(
        state
            .data
            .head_progenitor_tree(item_id, &tree_queries.desc_langs),
    )
}
