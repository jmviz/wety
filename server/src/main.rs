use server::{
    item_expansion, item_head_progenitor_tree, item_search_matches, lang_search_matches, AppState,
    Environment,
};

use std::{env, net::SocketAddr, path::Path, str::FromStr, sync::Arc};

use anyhow::Result;
use axum::{
    error_handling::HandleErrorLayer,
    http::{HeaderValue, Method},
    routing::get,
    BoxError, Router,
};
use axum_server::tls_rustls::RustlsConfig;
use tower::ServiceBuilder;
use tower_governor::{errors::display_error, GovernorLayer};
use tower_http::{
    compression::CompressionLayer,
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};

#[tokio::main]
async fn main() -> Result<()> {
    env::set_var("RUST_BACKTRACE", "1");

    env::set_var("RUST_LOG", "tower_http=trace,tower_governor=trace");
    tracing_subscriber::fmt::init();

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

    // $$$ make this configurable
    let data_path = Path::new("data/wety.json");
    let state = if data_path.exists() {
        Arc::new(AppState::new(data_path)?)
    } else {
        Arc::new(AppState::new(Path::new("data/wety.json.gz"))?)
    };

    let app = Router::new()
        .route("/langs/:lang", get(lang_search_matches))
        .route("/items/:lang/:term", get(item_search_matches))
        .route("/expand/:item", get(item_expansion))
        .route("/headProgenitorTree/:item", get(item_head_progenitor_tree))
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(HandleErrorLayer::new(|e: BoxError| async move {
                    display_error(e)
                }))
                .layer(GovernorLayer {
                    config: Box::leak(Box::default()),
                })
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
                .serve(app.into_make_service_with_connect_info::<SocketAddr>())
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
                .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                .await?;
        }
    }

    Ok(())
}
