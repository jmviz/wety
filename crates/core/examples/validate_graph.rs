fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("graph path required");
    let data = wety_core::processed::Data::deserialize(std::path::Path::new(&path))?;
    println!("Graph deserialized with matching language/POS reference fingerprint");
    // walks over every etymology (the etymology chart's) must end
    let graph = &data.graph.graph;
    let cycles = petgraph::algo::tarjan_scc(graph)
        .into_iter()
        .filter(|c| c.len() > 1 || graph.contains_edge(c[0], c[0]))
        .count();
    anyhow::ensure!(cycles == 0, "{cycles} cycles of etymologies remain");
    println!("No cycles of etymologies");
    Ok(())
}
