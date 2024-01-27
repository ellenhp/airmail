use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    /// The GeoJSON file to index.
    #[clap(short, long)]
    geojson: Option<String>,
    /// The directory to output index tiles into.
    #[clap(short, long)]
    index_dir: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
