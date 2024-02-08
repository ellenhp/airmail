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
        let parsed = airmail_parser::query::Query::parse(&query);

        let scenarios = parsed.scenarios();
        let results: Option<Vec<_>> = scenarios
            .iter()
            .take(10)
            .filter_map(|scenario| {
                let results = index.search(scenario).unwrap();
                if results.is_empty() {
                    None
                } else {
                    dbg!(scenario);
                    Some(results)
                }
            })
            .next();

        println!();
        if let Some(results) = results {
            for result in &results {
                println!("  - {:?}", result);
            }
            println!("{} results found in {:?}", results.len(), start.elapsed());
        } else {
            println!("No results found in {:?}.", start.elapsed());
        }
        println!();
    }
}
