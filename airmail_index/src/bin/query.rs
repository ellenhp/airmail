use airmail::{index::AirmailIndex, poi::AirmailPoi};
use clap::Parser;
use futures_util::future::join_all;
use rustyline::DefaultEditor;
use tokio::spawn;

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
        let mut scaled_results: Vec<tokio::task::JoinHandle<Vec<(AirmailPoi, f32)>>> = Vec::new();
        for scenario in scenarios.into_iter().take(3) {
            let index = index.clone();
            scaled_results.push(spawn(async move {
                let docs = index.search(&scenario).await.unwrap();
                let docs = docs
                    .into_iter()
                    .map(|(poi, score)| (poi, scenario.penalty_mult() * score))
                    .collect::<Vec<_>>();
                docs
            }));
        }
        let mut results: Vec<(AirmailPoi, f32)> = join_all(scaled_results)
            .await
            .into_iter()
            .flatten()
            .flatten()
            .collect::<Vec<_>>();

        results.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());
        for (poi, score) in results.iter().take(10) {
            println!("{:?} {}", poi, score);
        }
        println!("{} results found in {:?}", results.len(), start.elapsed());
    }
}
