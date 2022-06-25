use std::io;
use std::env;

use wety::process_download;

#[tokio::main]
async fn main() -> io::Result<()> {
    env::set_var("RUST_BACKTRACE", "1");
    return process_download().await;
}

