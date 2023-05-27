use processor::Data;
use server::{
    get_item_expansion, get_item_head_progenitor_tree, get_item_search_matches,
    get_lang_search_matches, AppState,
};
use tower::ServiceBuilder;
use tower_http::{compression::CompressionLayer, cors::CorsLayer};

use std::{env, path::Path, sync::Arc};

use anyhow::{Ok, Result};
use axum::{
    http::{HeaderValue, Method},
    routing::get,
    Router, Server,
};

#[tokio::main]
async fn main() -> Result<()> {
    env::set_var("RUST_BACKTRACE", "1");

    // let data = Data::deserialize(Path::new("data/test_output/wety.json.gz"))?;
    let data = Data::deserialize(Path::new("data/wety.json"))?;
    let search = data.build_search();
    let state = Arc::new(AppState { data, search });

    println!("Running wety server...");

    let origins = [
        "http://localhost".parse::<HeaderValue>()?,
        "http://localhost:8000".parse::<HeaderValue>()?,
        "http://wety.org".parse::<HeaderValue>()?,
    ];

    let app = Router::new()
        .route("/langs/:lang", get(get_lang_search_matches))
        .route("/items/:lang/:term", get(get_item_search_matches))
        .route("/expand/:item/filter/:lang", get(get_item_expansion))
        .route(
            "/headProgenitorTree/:item/filter/:lang",
            get(get_item_head_progenitor_tree),
        )
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                // ?
                // https://docs.rs/tower-http/0.4.0/tower_http/trace/index.html
                // https://docs.rs/tower/0.4.13/tower/limit/struct.RateLimitLayer.html
                // https://docs.rs/tower/0.4.13/tower/limit/struct.ConcurrencyLimitLayer.html
                // https://docs.rs/tower/0.4.13/tower/timeout/struct.TimeoutLayer.html
                // https://docs.rs/axum/latest/axum/error_handling/struct.HandleErrorLayer.html
                .layer(CompressionLayer::new())
                .layer(
                    CorsLayer::new()
                        .allow_methods([Method::GET])
                        .allow_origin(origins),
                ),
        );

    Server::bind(&"0.0.0.0:3000".parse()?)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}
