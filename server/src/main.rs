use processor::Data;
use server::{get_item_expansion, get_item_search_matches, get_lang_search_matches, AppState};
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;

use std::{env, path::Path, sync::Arc};

use anyhow::{Ok, Result};
use axum::{routing::get, Router, Server};

#[tokio::main]
async fn main() -> Result<()> {
    env::set_var("RUST_BACKTRACE", "1");

    let data = Data::deserialize(Path::new("data/test_output/wety.json.gz"))?;
    let search = data.build_search();
    let state = Arc::new(AppState { data, search });

    println!("Running wety server...");

    let app = Router::new()
        .route("/expand/:item/filter/:lang", get(get_item_expansion))
        .route("/langs/:lang", get(get_lang_search_matches))
        .route("/items/:lang/:term", get(get_item_search_matches))
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                // ?
                // https://docs.rs/tower-http/0.4.0/tower_http/trace/index.html
                // https://docs.rs/tower/0.4.13/tower/limit/struct.RateLimitLayer.html
                // https://docs.rs/tower/0.4.13/tower/limit/struct.ConcurrencyLimitLayer.html
                // https://docs.rs/tower/0.4.13/tower/timeout/struct.TimeoutLayer.html
                // https://docs.rs/axum/latest/axum/error_handling/struct.HandleErrorLayer.html
                .layer(CompressionLayer::new()),
        );

    Server::bind(&"0.0.0.0:3000".parse()?)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}
