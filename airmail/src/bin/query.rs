use airmail::index::AirmailIndex;
use clap::Parser;

#[derive(Debug, Parser)]
struct Args {
    #[clap(long, short)]
    index: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    println!("{:?}", args);
    let index = AirmailIndex::new(&args.index)?;

    let query = "425 harvard ave, seattle";
    let parsed = airmail_parser::query::Query::parse(query);

    let scenarios = parsed.scenarios();
    let top = scenarios.first().unwrap();

    let _results = index.search(top);

    Ok(())
}
