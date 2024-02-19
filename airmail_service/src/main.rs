use std::{collections::HashMap, error::Error, sync::Arc};

use airmail::{index::AirmailIndex, poi::AirmailPoi};
use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use clap::Parser;
use deunicode::deunicode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::task::spawn_blocking;

#[derive(Debug, Parser)]
struct Args {
    #[arg(short, long, env = "AIRMAIL_INDEX")]
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
    let query = deunicode(query.trim()).to_lowercase();

    let start = std::time::Instant::now();

    let results = index.search(&query).await.unwrap();

    println!("{} results found in {:?}", results.len(), start.elapsed());

    let mut response = Response {
        metadata: HashMap::new(),
        features: results
            .clone()
            .into_iter()
            .map(|(results, _)| results.clone())
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
    println!("Have {} docs", index.num_docs().await?);
    let app = Router::new().route("/search", get(search).with_state(index));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}
