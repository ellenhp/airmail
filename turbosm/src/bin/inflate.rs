use clap::Parser;
use turbosm::Turbosm;

#[derive(Debug, Parser)]
struct Args {
    #[clap(long, short)]
    pbf_path: String,
    #[clap(long, short)]
    db_path: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse();
    Turbosm::create_from_pbf(&args.pbf_path, &args.db_path)?;
    Ok(())
}
