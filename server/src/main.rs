use processor::processed::Data;

use std::{env, path::Path};

use anyhow::{Ok, Result};

fn main() -> Result<()> {
    env::set_var("RUST_BACKTRACE", "1");
    let _data = Data::deserialize(Path::new("data/wety.json.gz"))?;
    Ok(())
}
