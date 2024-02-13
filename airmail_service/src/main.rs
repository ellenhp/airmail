use std::{collections::HashMap, error::Error, sync::Arc};

use airmail::{index::AirmailIndex, poi::AirmailPoi};
use airmail_parser::query::QueryScenario;
use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use clap::Parser;
use deunicode::deunicode;
use log::trace;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::task::spawn_blocking;

#[derive(Debug, Parser)]
struct Args {
    #[clap(long, short)]
    index: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Response {
    metadata: HashMap<String, Value>,
    features: Vec<AirmailPoi>,
}

async fn search(
    Query(params): Query<HashMap<String, String>>,
    State(index): State<Arc<AirmailIndex>>,
) -> Json<Value> {
    let query = params.get("q").unwrap();
    trace!("searching for {:?}", query);
    let query = deunicode(query.trim()).to_lowercase();
    let parsed = airmail_parser::query::Query::parse(&query);

    let scenarios = parsed.scenarios();
    let start = std::time::Instant::now();
    let mut all_results: Vec<(AirmailPoi, f32, QueryScenario)> = vec![];
    for scenario in scenarios.iter().take(3) {
        if all_results.len() > 20 {
            break;
        }
        let results = {
            let scenario = scenario.clone();
            let index = index.clone();
            spawn_blocking(move || index.search(&scenario).unwrap())
                .await
                .unwrap()
        };
        if results.is_empty() {
            continue;
        } else {
            all_results.extend(
                results
                    .iter()
                    .map(|(poi, score)| {
                        (
                            poi.clone(),
                            *score * scenario.penalty_mult(),
                            scenario.clone(),
                        )
                    })
                    .collect::<Vec<_>>(),
            );
        }
    }

    all_results.sort_by(|(_, a, _), (_, b, _)| b.partial_cmp(a).unwrap());

    println!(
        "{} results found in {:?}",
        all_results.len(),
        start.elapsed()
    );

    let mut response = Response {
        metadata: HashMap::new(),
        features: all_results
            .clone()
            .iter()
            .map(|(results, _, _)| results.clone())
            .collect::<Vec<AirmailPoi>>(),
    };

    response
        .metadata
        .insert("query".to_string(), Value::String(query));

    Json(serde_json::to_value(response).unwrap())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let args = Args::parse();
    let index_path = args.index.clone();

    let index = spawn_blocking(move || {
        if index_path.starts_with("http") {
            Arc::new(AirmailIndex::new_remote(&index_path).unwrap())
        } else {
            Arc::new(AirmailIndex::new(&index_path).unwrap())
        }
    })
    .await
    .unwrap();
    println!("Have {} docs", index.num_docs()?);
    let app = Router::new().route("/search", get(search).with_state(index));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}
