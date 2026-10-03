#![allow(clippy::unused_async)]

use serde::{Deserialize, Serialize};
use wety_core::{Data, ItemId, Lang, Search};

use std::{str::FromStr, sync::Arc};

use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Json, Response},
};
use axum_extra::extract::Query as ExtraQuery;
use bytes::Bytes;
use moka::sync::Cache;
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

/// Everything that determines a tree response. The data is immutable for the
/// life of the process, so this is a complete cache key.
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub enum TreeKey {
    Etymology {
        item: ItemId,
    },
    Descendants {
        item: ItemId,
        dist_lang: Lang,
        /// sorted and deduplicated, so equivalent queries share an entry
        desc_langs: Vec<Lang>,
    },
    Cognates {
        item: ItemId,
        dist_lang: Lang,
        desc_langs: Vec<Lang>,
    },
}

/// Total serialized (uncompressed) bytes the tree cache may hold. Responses
/// for the big Proto-Indo-European roots run to a couple of MB each, so this
/// is on the order of a hundred of those or thousands of typical ones.
const TREE_CACHE_BYTES: u64 = 256 * 1024 * 1024;

pub struct AppState {
    pub data: Data,
    pub search: Search,
    /// Serialized JSON for previously requested trees. Compression happens
    /// downstream in the `CompressionLayer`, so one entry serves every
    /// `Accept-Encoding`.
    tree_cache: Cache<TreeKey, Bytes>,
}

impl AppState {
    /// # Errors
    ///
    /// Will return `Err` if deserializing the data file fails.
    pub fn new(data_path: &std::path::Path) -> Result<Self> {
        let data = Data::deserialize_with_legacy(data_path, std::env::var("WETY_ALLOW_LEGACY_GRAPH").as_deref() == Ok("1"))?;
        let search = data.build_search();
        let tree_cache = Cache::builder()
            .weigher(|_: &TreeKey, bytes: &Bytes| u32::try_from(bytes.len()).unwrap_or(u32::MAX))
            .max_capacity(TREE_CACHE_BYTES)
            .build();
        Ok(Self {
            data,
            search,
            tree_cache,
        })
    }

    fn build_tree(&self, key: &TreeKey) -> serde_json::Result<Bytes> {
        let bytes = match key {
            TreeKey::Etymology { item } => {
                let lang = self.data.lang(*item);
                serde_json::to_vec(&self.data.item_etymology_json(*item, 0, lang))?
            }
            TreeKey::Descendants {
                item,
                dist_lang,
                desc_langs,
            } => {
                let ancestors = self.data.ancestors_in_langs(*item, desc_langs);
                serde_json::to_vec(
                    &self
                        .data
                        .item_descendants_json(*item, *dist_lang, desc_langs, &ancestors),
                )?
            }
            TreeKey::Cognates {
                item,
                dist_lang,
                desc_langs,
            } => {
                let ancestors = self.data.ancestors_in_langs(*item, desc_langs);
                serde_json::to_vec(
                    &self
                        .data
                        .item_cognates_json(*item, *dist_lang, desc_langs, &ancestors),
                )?
            }
        };
        Ok(Bytes::from(bytes))
    }
}

/// Serve `key` from the tree cache, building it on a blocking thread on a
/// miss so a large tree doesn't stall the async runtime. Concurrent misses on
/// the same key share one build.
async fn tree_response(state: Arc<AppState>, key: TreeKey) -> Response {
    let bytes = if let Some(bytes) = state.tree_cache.get(&key) {
        bytes
    } else {
        let built = tokio::task::spawn_blocking(move || {
            state
                .tree_cache
                .try_get_with(key.clone(), || state.build_tree(&key))
                .map_err(|e| format!("building {key:?}: {e}"))
        })
        .await;
        match built {
            Ok(Ok(bytes)) => bytes,
            Ok(Err(e)) => {
                tracing::error!("{e}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
            Err(e) => {
                tracing::error!("tree build task failed: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        }
    };
    ([(header::CONTENT_TYPE, "application/json")], bytes).into_response()
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

pub async fn item_etymology(
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<ItemId>,
) -> Response {
    if !state.data.has_item(item_id) {
        return StatusCode::NOT_FOUND.into_response();
    }
    tree_response(state, TreeKey::Etymology { item: item_id }).await
}

#[derive(Deserialize, Serialize)]
pub struct TreeQueries {
    #[serde(rename = "descLang")]
    desc_langs: Vec<Lang>,
    #[serde(rename = "distLang")]
    dist_lang: Option<Lang>,
}

impl TreeQueries {
    /// `(dist_lang, desc_langs)` in canonical form for `TreeKey`.
    fn canonical(mut self, data: &Data, item_id: ItemId) -> (Lang, Vec<Lang>) {
        let dist_lang = self.dist_lang.unwrap_or(data.lang(item_id));
        self.desc_langs.sort_unstable_by_key(|l| l.id());
        self.desc_langs.dedup();
        (dist_lang, self.desc_langs)
    }
}

pub async fn item_descendants(
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<ItemId>,
    ExtraQuery(tree_queries): ExtraQuery<TreeQueries>,
) -> Response {
    if !state.data.has_item(item_id) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let (dist_lang, desc_langs) = tree_queries.canonical(&state.data, item_id);
    tree_response(
        state,
        TreeKey::Descendants {
            item: item_id,
            dist_lang,
            desc_langs,
        },
    )
    .await
}

pub async fn item_cognates(
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<ItemId>,
    ExtraQuery(tree_queries): ExtraQuery<TreeQueries>,
) -> Response {
    if !state.data.has_item(item_id) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let (dist_lang, desc_langs) = tree_queries.canonical(&state.data, item_id);
    tree_response(
        state,
        TreeKey::Cognates {
            item: item_id,
            dist_lang,
            desc_langs,
        },
    )
    .await
}
