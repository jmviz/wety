use server::{
    item_cognates, item_descendants, item_etymology, item_search_matches, lang_search_matches,
    AppState, Environment,
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
        .route("/search/lang", get(lang_search_matches))
        .route("/search/item/:lang", get(item_search_matches))
        .route("/cognates/:item", get(item_cognates))
        .route("/etymology/:item", get(item_etymology))
        .route("/descendants/:item", get(item_descendants))
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

    let addr = SocketAddr::from_str("0.0.0.0:3000")?;
    println!("Running wety server at http://{}...", addr);

    match environment {
        Environment::Development => {
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
            axum_server::bind_rustls(addr, config)
                .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                .await?;
        }
    }

    Ok(())
}
