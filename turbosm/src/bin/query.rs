use clap::Parser;
use turbosm::Turbosm;

#[derive(Debug, Parser)]
struct Args {
    #[clap(long, short)]
    db_path: String,
    #[clap(long, short)]
    nodes_path: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse();
    let osm = Turbosm::open(&args.db_path)?;

    let mut editor = rustyline::DefaultEditor::new()?;
    loop {
        let input =
            editor.readline("Enter an element type and id (e.g. 'n1234', 'w5678', 'r91011'): ")?;
        if input == "quit" || input == "q" || input.is_empty() {
            println!("Goodbye.");
            break;
        }
        match input.chars().next().unwrap() {
            'n' => {
                let id = input[1..].parse::<u64>()?;
                println!("Looking for node {}", id);
                if let Ok(node) = osm.node(id) {
                    println!("{:?}", node);
                } else {
                    println!("Node not found.");
                }
            }
            'w' => {
                let id = input[1..].parse::<u64>()?;
                println!("Looking for way {}", id);
                let way = osm.way(id);
                if let Ok(way) = way {
                    println!("{:?}", way);
                } else {
                    println!("Way not found.");
                }
            }
            'r' => {
                let id = input[1..].parse::<u64>()?;
                println!("Looking for relation {}", id);
                let rel = osm.relation(id);
                if let Ok(rel) = rel {
                    println!("{:?}", rel);
                } else {
                    println!("Relation not found.");
                }
            }
            _ => println!("Unknown element type. Use 'n', 'w', or 'r' followed by the id."),
        }
    }
    Ok(())
}
