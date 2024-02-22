use airmail::index::AirmailIndex;
use clap::Parser;

#[derive(Debug, Parser)]
struct Args {
    #[clap(long, short)]
    index: String,
    #[clap(long, short)]
    merge: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut index = AirmailIndex::new(&args.index)?;
    if args.merge {
        index.merge().await?;
    } else {
        println!("Pass the --merge arg if you're sure");
    }
    Ok(())
}
