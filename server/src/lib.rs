#![allow(clippy::unused_async)]

use processor::{Data, ItemId};

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::Json,
};
use serde_json::Value;

pub async fn get_item_expansion(
    Path(id): Path<ItemId>,
    State(data): State<Arc<Data>>,
) -> Json<Value> {
    Json(data.expand(id))
}
