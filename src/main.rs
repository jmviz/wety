#[global_allocator]
static ALLOC: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_BACKTRACE", "1");
    wety::process_wiktextract_data().await?;
    Ok(())
}
