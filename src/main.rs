#[global_allocator]
static ALLOC: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;

use std::error::Error;
use std::env;

use wety::process_wiktextract_data;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env::set_var("RUST_BACKTRACE", "1");
    let _ = process_wiktextract_data().await?;
    Ok(())
}

