use processor::Data;
use server::{
    get_item_expansion, get_item_head_progenitor_tree, get_item_search_matches,
    get_lang_search_matches, AppState,
};

use std::{env, net::SocketAddr, path::Path, sync::Arc};

use anyhow::{Ok, Result};
use axum::{
    http::{HeaderValue, Method},
    routing::get,
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use tower::ServiceBuilder;
use tower_http::{compression::CompressionLayer, cors::CorsLayer};

#[tokio::main]
async fn main() -> Result<()> {
    env::set_var("RUST_BACKTRACE", "1");

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));

    let config = RustlsConfig::from_pem_file(
        "/home/ubuntu/certs/fullchain.pem",
        "/home/ubuntu/certs/privkey.pem",
    )
    .await?;

    let server = axum_server::bind_rustls(addr, config);

    let origins = [
        "https://localhost".parse::<HeaderValue>()?,
        "https://localhost:8000".parse::<HeaderValue>()?,
        "https://wety.org".parse::<HeaderValue>()?,
        "https://www.wety.org".parse::<HeaderValue>()?,
    ];

    let data = Data::deserialize(Path::new("data/wety.json"))?;
    let search = data.build_search();
    let state = Arc::new(AppState { data, search });

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

    println!("Running wety server...");
    server.serve(app.into_make_service()).await?;
    Ok(())
}
