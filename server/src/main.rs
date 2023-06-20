use server::{
    get_item_expansion, get_item_head_progenitor_tree, get_item_search_matches,
    get_lang_search_matches, AppState, Environment,
};

use std::{env, net::SocketAddr, path::Path, str::FromStr, sync::Arc};

use anyhow::Result;
use axum::{
    http::{HeaderValue, Method},
    routing::get,
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::{AllowOrigin, CorsLayer},
};

#[tokio::main]
async fn main() -> Result<()> {
    env::set_var("RUST_BACKTRACE", "1");

    let environment = Environment::from_str(
        &env::var("WETY_ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
    )?;

    let origins: AllowOrigin = match environment {
        Environment::Development => tower_http::cors::Any.into(),
        Environment::Production => vec![
            "https://wety.org".parse::<HeaderValue>()?,
            "https://www.wety.org".parse::<HeaderValue>()?,
        ]
        .into(),
    };

    // make this configurable
    let data_path = Path::new("data/wety.json");
    let state = if data_path.exists() {
        Arc::new(AppState::new(data_path)?)
    } else {
        Arc::new(AppState::new(Path::new("data/wety.json.gz"))?)
    };

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

    match environment {
        Environment::Development => {
            let addr = SocketAddr::from_str("0.0.0.0:3000")?;
            println!("Running wety server at http://{}...", addr);
            axum_server::bind(addr)
                .serve(app.into_make_service())
                .await?;
        }
        Environment::Production => {
            let cert_path = env::var("WETY_CERT_PATH")
                .expect("WETY_CERT_PATH environment variable set in production");
            let key_path = env::var("WETY_KEY_PATH")
                .expect("WETY_KEY_PATH environment variable set in production");
            let config = RustlsConfig::from_pem_file(&cert_path, &key_path).await?;
            let addr = SocketAddr::from_str("0.0.0.0:3000")?;
            println!("Running wety server at https://{}...", addr);
            axum_server::bind_rustls(addr, config)
                .serve(app.into_make_service())
                .await?;
        }
    }

    Ok(())
}
