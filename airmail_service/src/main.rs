use std::{collections::HashMap, sync::Arc};

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
    let mut results: Vec<(AirmailPoi, f32, QueryScenario)> = scenarios
        .iter()
        .take(10)
        .filter_map(|scenario| {
            let results = index.search(scenario).unwrap();
            if results.is_empty() {
                None
            } else {
                Some(
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
                )
            }
        })
        .take(2)
        .flatten()
        .collect();

    results.sort_by(|(_, a, _), (_, b, _)| b.partial_cmp(a).unwrap());

    println!("{} results found in {:?}", results.len(), start.elapsed());

    let mut response = Response {
        metadata: HashMap::new(),
        features: results
            .clone()
            .iter()
            .map(|(results, _, _)| results.clone())
            .collect::<Vec<AirmailPoi>>(),
    };

    response
        .metadata
        .insert("query".to_string(), Value::String(query));
    if params.get("debug").is_some() {
        // response.metadata.insert(
        //     "parsed".to_string(),
        //     results
        //         .iter()
        //         .map(|(_, scenario)| {
        //             Value::Array(
        //                 scenario
        //                     .as_vec()
        //                     .iter()
        //                     .map(|component| {
        //                         let text = Value::String(component.text().to_string());
        //                         let component = Value::String(component.debug_name().to_string());
        //                         json!({"text": text, "component": component})
        //                     })
        //                     .collect(),
        //             )
        //         })
        //         .unwrap_or(Value::Null),
        // );
    }

    Json(serde_json::to_value(response).unwrap())
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Args::parse();
    let index = Arc::new(AirmailIndex::new(&args.index).unwrap());
    let app = Router::new().route("/search", get(search).with_state(index));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
