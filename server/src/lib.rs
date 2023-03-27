#![feature(is_some_and, let_chains)]

use flate2::read::GzDecoder;
use processor::ProcessedData;

use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
    time::Instant,
};

use anyhow::{Ok, Result};

/// .
///
/// # Errors
///
/// This function will return an error if deserialization goes wrong.
pub fn deserialize_data(path: &Path) -> Result<ProcessedData> {
    let t = Instant::now();
    println!("Deserializing processed data {}...", path.display());
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let is_gz_compressed = path.extension().is_some_and(|ext| ext == "gz");
    let uncompressed: Box<dyn Read> = if is_gz_compressed {
        Box::new(GzDecoder::new(reader))
    } else {
        Box::new(reader)
    };
    let data = serde_json::from_reader(uncompressed)?;
    // println!("Finished. Took {}.", HumanDuration(t.elapsed()));
    println!("Finished. Took {:#?}.", t.elapsed());
    Ok(data)
}
