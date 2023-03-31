use processor::Data;
use server::get_item_expansion;

use std::{env, path::Path, sync::Arc};

use anyhow::{Ok, Result};
use axum::{routing::get, Router, Server};

#[tokio::main]
async fn main() -> Result<()> {
    env::set_var("RUST_BACKTRACE", "1");

    let data = Data::deserialize(Path::new("data/test_output/wety.json.gz"))?;
    let data = Arc::new(data);

    println!("Running wety server...");

    let app = Router::new()
        .route("/expand/:item/filter/:lang", get(get_item_expansion))
        .with_state(data);

    Server::bind(&"0.0.0.0:3000".parse()?)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}
