#[global_allocator]
static ALLOC: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;

fn main() -> anyhow::Result<()> {
    wety::process_wiktextract_data()?;
    Ok(())
}
