fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("graph path required");
    let _data = wety_core::processed::Data::deserialize(std::path::Path::new(&path))?;
    println!("Graph deserialized with matching language/POS reference fingerprint");
    Ok(())
}
