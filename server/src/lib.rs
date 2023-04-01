#![allow(clippy::unused_async)]

use processor::{Data, ItemId, Lang, LangId, Search};

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::Json,
};
use serde_json::Value;

pub struct AppState {
    pub data: Data,
    pub search: Search,
}

pub async fn get_item_expansion(
    Path((item_id, filter_lang_id)): Path<(ItemId, LangId)>,
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let filter_lang = Lang::from(filter_lang_id);
    Json(state.data.expand(item_id, filter_lang))
}

pub async fn get_lang_search_matches(
    Path(lang): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    Json(state.search.langs(&lang))
}

pub async fn get_item_search_matches(
    Path((lang, term)): Path<(LangId, String)>,
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    Json(state.search.items(lang, &term))
}
