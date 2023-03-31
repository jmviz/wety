#![allow(clippy::unused_async)]

use processor::{Data, ItemId, Lang, LangId};

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::Json,
};
use serde_json::Value;

pub async fn get_item_expansion(
    Path((item_id, filter_lang_id)): Path<(ItemId, LangId)>,
    State(data): State<Arc<Data>>,
) -> Json<Value> {
    let filter_lang = Lang::from(filter_lang_id);
    Json(data.expand(item_id, filter_lang))
}
