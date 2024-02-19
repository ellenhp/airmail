use airmail::index::AirmailIndex;
use clap::Parser;
use rustyline::DefaultEditor;

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
    let mut rl = DefaultEditor::new()?;
    loop {
        let query = rl.readline("query: ")?;
        rl.add_history_entry(query.as_str())?;
        let start = std::time::Instant::now();
        let query = query.trim().to_lowercase();

        let mut results = index.search(&query).await.unwrap();

        results.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());
        for (poi, score) in results.iter().take(10) {
            println!("{:?} {}", poi, score);
        }
        println!("{} results found in {:?}", results.len(), start.elapsed());
    }
}
