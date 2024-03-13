use airmail::index::AirmailIndex;
use clap::Parser;
use geo::Coord;
use rustyline::DefaultEditor;

#[derive(Debug, Parser)]
struct Args {
    #[clap(long, short)]
    index: String,
    #[clap(long, short)]
    bbox: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let index = AirmailIndex::new(&args.index)?;
    let mut rl = DefaultEditor::new()?;

    let bbox = args.bbox.map(|s| {
        let mut parts = s.split(',');
        let min_lng = parts
            .next()
            .expect("Invalid bbox format. Need: `lng1,lat1,lng2,lat2`")
            .parse()
            .unwrap();
        let min_lat = parts
            .next()
            .expect("Invalid bbox format. Need: `lng1,lat1,lng2,lat2`")
            .parse()
            .unwrap();
        let max_lng = parts
            .next()
            .expect("Invalid bbox format. Need: `lng1,lat1,lng2,lat2`")
            .parse()
            .unwrap();
        let max_lat = parts
            .next()
            .expect("Invalid bbox format. Need: `lng1,lat1,lng2,lat2`")
            .parse()
            .unwrap();
        geo::Rect::new(
            Coord {
                y: min_lat,
                x: min_lng,
            },
            Coord {
                y: max_lat,
                x: max_lng,
            },
        )
    });

    loop {
        let query = rl.readline("query: ")?;
        rl.add_history_entry(query.as_str())?;
        let start = std::time::Instant::now();
        let query = query.trim().to_lowercase();

        let mut results = index.search(&query, true, None, bbox, &[]).await.unwrap();

        results.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());
        for (poi, score) in results.iter().take(10) {
            println!("{:?} {}", poi, score);
        }
        println!("{} results found in {:?}", results.len(), start.elapsed());
    }
}
