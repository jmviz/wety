use server::deserialize_data;

use std::{env, path::Path};

use anyhow::{Ok, Result};

fn main() -> Result<()> {
    env::set_var("RUST_BACKTRACE", "1");
    deserialize_data(Path::new("data/wety.json.gz"))?;
    Ok(())
}
