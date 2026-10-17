#![allow(clippy::unused_async)]

pub mod client_ip;
pub mod periods;

use periods::Periods;
use serde::{Deserialize, Deserializer, de::Error as _};
use wety_core::{
    Data, ItemId, Lang, Search,
    keys::{Keys, Resolution},
    processed::graph_version,
};

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
use serde_json::{Value, json};

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
        lean: bool,
    },
    Cognates {
        item: ItemId,
        dist_lang: Lang,
        desc_langs: Vec<Lang>,
        lean: bool,
    },
}

/// Total serialized (uncompressed) bytes the tree cache may hold. Responses
/// for the big Proto-Indo-European roots run to a couple of MB each, so this
/// is on the order of a hundred of those or thousands of typical ones.
const TREE_CACHE_BYTES: u64 = 256 * 1024 * 1024;

pub struct AppState {
    pub data: Data,
    pub search: Search,
    pub keys: Keys,
    pub periods: Periods,
    /// Identity of the loaded graph; clients send it back with numeric ids.
    pub graph: String,
    /// Serialized JSON for previously requested trees. Compression happens
    /// downstream in the `CompressionLayer`, so one entry serves every
    /// `Accept-Encoding`.
    tree_cache: Cache<TreeKey, Bytes>,
}

impl AppState {
    /// # Errors
    ///
    /// Will return `Err` if deserializing the data file fails, or if the
    /// periods file exists but is invalid.
    pub fn new(data_path: &std::path::Path, periods_path: &std::path::Path) -> Result<Self> {
        let periods = Periods::load(periods_path)?;
        println!(
            "Loaded {} language periods from {}.",
            periods.languages,
            periods_path.display()
        );
        if !periods.unknown_codes.is_empty() {
            tracing::warn!("periods for unknown lang codes ignored: {:?}", periods.unknown_codes);
        }
        let data = Data::deserialize_with_legacy(data_path, std::env::var("WETY_ALLOW_LEGACY_GRAPH").as_deref() == Ok("1"))?;
        let graph = graph_version(data_path)?;
        let search = data.build_search();
        let keys = data.build_keys();
        let tree_cache = Cache::builder()
            .weigher(|_: &TreeKey, bytes: &Bytes| u32::try_from(bytes.len()).unwrap_or(u32::MAX))
            .max_capacity(TREE_CACHE_BYTES)
            .build();
        Ok(Self {
            data,
            search,
            keys,
            periods,
            graph,
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
                lean,
            } => {
                let ancestors = self.data.ancestors_in_langs(*item, desc_langs);
                serde_json::to_vec(
                    &self
                        .data
                        .item_descendants_json(*item, *dist_lang, desc_langs, &ancestors, *lean),
                )?
            }
            TreeKey::Cognates {
                item,
                dist_lang,
                desc_langs,
                lean,
            } => {
                let ancestors = self.data.ancestors_in_langs(*item, desc_langs);
                serde_json::to_vec(
                    &self
                        .data
                        .item_cognates_json(*item, *dist_lang, desc_langs, &ancestors, *lean),
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

/// A language named by its Wiktionary code in a URL. Retired codes are
/// accepted and mapped to their successors.
#[derive(Clone, Copy)]
pub struct LangCode(Lang);

impl<'de> Deserialize<'de> for LangCode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let code = String::deserialize(deserializer)?;
        Lang::from_code(&code).map(LangCode).map_err(D::Error::custom)
    }
}

/// The graph a client's numeric ids came from, if it says.
#[derive(Deserialize)]
pub struct GraphQuery {
    graph: Option<String>,
}

/// Numeric ids from another graph would name other items: refuse them, so
/// the client re-resolves its URL instead of showing the wrong word.
fn stale_graph(state: &AppState, graph: Option<&str>) -> Option<Response> {
    graph.filter(|&g| g != state.graph).map(|_| {
        (StatusCode::CONFLICT, Json(json!({ "graph": state.graph }))).into_response()
    })
}

#[derive(Deserialize)]
pub struct ResolveQuery {
    lang: LangCode,
    term: String,
    ety: Option<String>,
}

/// What a link names: `{graph, item, homographs}`. `item` is null when
/// several items share the name and none is singled out; 404 when none has it.
pub async fn resolve(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ResolveQuery>,
) -> Response {
    let data = &state.data;
    let (status, item, homographs) =
        match data.resolve(&state.keys, query.lang.0, &query.term, query.ety.as_deref()) {
            Resolution::Found(item, all) => (StatusCode::OK, Some(item), all),
            Resolution::Ambiguous(all) => (StatusCode::OK, None, all),
            Resolution::NotFound => (StatusCode::NOT_FOUND, None, vec![]),
        };
    let body = json!({
        "graph": state.graph,
        "item": item.map(|id| data.item_json(id)),
        "homographs": homographs.iter().map(|&id| data.item_json(id)).collect::<Vec<_>>(),
    });
    (status, Json(body)).into_response()
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
    Path(LangCode(lang)): Path<LangCode>,
    Query(item_search): Query<ItemSearch>,
) -> Json<Value> {
    let matches = state.search.items(&state.data, lang, &item_search.term);
    Json(matches)
}

pub async fn lang(Path(LangCode(lang)): Path<LangCode>) -> Response {
    Json(lang.json()).into_response()
}

/// When each language was spoken, where the dating registry has adopted a
/// cited period (`language_periods.json`). Empty when no export is present.
pub async fn periods(State(state): State<Arc<AppState>>) -> Response {
    (
        [(header::CONTENT_TYPE, "application/json")],
        state.periods.bytes.clone(),
    )
        .into_response()
}

pub async fn item(
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<ItemId>,
    Query(query): Query<GraphQuery>,
) -> Response {
    if let Some(stale) = stale_graph(&state, query.graph.as_deref()) {
        return stale;
    }
    if !state.data.has_item(item_id) {
        return StatusCode::NOT_FOUND.into_response();
    }
    Json(state.data.item_json(item_id)).into_response()
}

pub async fn item_etymology(
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<ItemId>,
    Query(query): Query<GraphQuery>,
) -> Response {
    if let Some(stale) = stale_graph(&state, query.graph.as_deref()) {
        return stale;
    }
    if !state.data.has_item(item_id) {
        return StatusCode::NOT_FOUND.into_response();
    }
    tree_response(state, TreeKey::Etymology { item: item_id }).await
}

#[derive(Deserialize)]
pub struct TreeQueries {
    #[serde(rename = "descLang", default)]
    desc_langs: Vec<LangCode>,
    #[serde(rename = "distLang")]
    dist_lang: Option<LangCode>,
    graph: Option<String>,
    /// Omit glosses from every node (they dominate payload size); fetch them
    /// on demand from `/item/:id` instead.
    #[serde(default)]
    lean: bool,
}

impl TreeQueries {
    /// `(dist_lang, desc_langs, lean)` in canonical form for `TreeKey`.
    fn canonical(self, data: &Data, item_id: ItemId) -> (Lang, Vec<Lang>, bool) {
        let dist_lang = self.dist_lang.map_or(data.lang(item_id), |l| l.0);
        let mut desc_langs: Vec<Lang> = self.desc_langs.into_iter().map(|l| l.0).collect();
        desc_langs.sort_unstable_by_key(|l| l.id());
        desc_langs.dedup();
        (dist_lang, desc_langs, self.lean)
    }
}

pub async fn item_descendants(
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<ItemId>,
    ExtraQuery(tree_queries): ExtraQuery<TreeQueries>,
) -> Response {
    if let Some(stale) = stale_graph(&state, tree_queries.graph.as_deref()) {
        return stale;
    }
    if !state.data.has_item(item_id) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let (dist_lang, desc_langs, lean) = tree_queries.canonical(&state.data, item_id);
    tree_response(
        state,
        TreeKey::Descendants {
            item: item_id,
            dist_lang,
            desc_langs,
            lean,
        },
    )
    .await
}

pub async fn item_cognates(
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<ItemId>,
    ExtraQuery(tree_queries): ExtraQuery<TreeQueries>,
) -> Response {
    if let Some(stale) = stale_graph(&state, tree_queries.graph.as_deref()) {
        return stale;
    }
    if !state.data.has_item(item_id) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let (dist_lang, desc_langs, lean) = tree_queries.canonical(&state.data, item_id);
    tree_response(
        state,
        TreeKey::Cognates {
            item: item_id,
            dist_lang,
            desc_langs,
            lean,
        },
    )
    .await
}
