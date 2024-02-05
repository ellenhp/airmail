use std::{collections::HashMap, sync::Arc};

use airmail::{index::AirmailIndex, poi::AirmailPoi};
use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use clap::Parser;
use deunicode::deunicode;
use log::trace;
use serde_json::Value;

#[derive(Debug, Parser)]
struct Args {
    #[clap(long, short)]
    index_path: String,
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
    let results: Option<Vec<AirmailPoi>> = scenarios
        .iter()
        .take(10)
        .filter_map(|scenario| {
            let results = index.search(scenario).unwrap();
            if results.is_empty() {
                None
            } else {
                Some(results)
            }
        })
        .next();

    if let Some(results) = results {
        Json(serde_json::to_value(results).unwrap())
    } else {
        Json(serde_json::Value::Array(vec![]))
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Args::parse();
    let index = Arc::new(AirmailIndex::new(&args.index_path).unwrap());
    let app = Router::new().route("/search", get(search).with_state(index));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
