use std::io;
use std::env;

use wety::{process_wiktextract_data, print_all_items};

#[tokio::main]
async fn main() -> io::Result<()> {
    env::set_var("RUST_BACKTRACE", "1");
    let terms = process_wiktextract_data().await.unwrap();
    print_all_items(&terms);
    Ok(())
}

