use std::sync::Arc;

use airmail::{index::AirmailIndex, poi::AirmailPoi};
use anyhow::Result;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use deunicode::deunicode;
use geo::{Coord, Rect};
use log::debug;
use serde::{Deserialize, Serialize};

use crate::error::AirmailServiceError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQueryParams {
    q: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    tags: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    leniency: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    bbox: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    metadata: MetadataResponse,
    features: Vec<AirmailPoi>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MetadataResponse {
    query: SearchQueryParams,
}

fn parse_bbox(s: &str) -> Option<Rect> {
    let mut parts = s.split(',');
    let min_lng: f64 = parts.next()?.parse().ok()?;
    let min_lat: f64 = parts.next()?.parse().ok()?;
    let max_lng: f64 = parts.next()?.parse().ok()?;
    let max_lat: f64 = parts.next()?.parse().ok()?;

    Some(Rect::new(
        Coord {
            y: min_lat,
            x: min_lng,
        },
        Coord {
            y: max_lat,
            x: max_lng,
        },
    ))
}

pub async fn search(
    Query(params): Query<SearchQueryParams>,
    State(index): State<Arc<AirmailIndex>>,
) -> Result<impl IntoResponse, AirmailServiceError> {
    let query = deunicode(params.q.trim()).to_lowercase();
    let tags: Option<Vec<String>> = params
        .tags
        .clone()
        .map(|s| s.split(',').map(std::string::ToString::to_string).collect());
    let leniency = params.leniency.unwrap_or_default();
    let bbox = params.bbox.clone().and_then(|s| parse_bbox(&s));

    let start = std::time::Instant::now();

    let results = index.search(&query, leniency, tags, bbox, &[]).await?;

    debug!(
        "Query: {:?} produced: {} results found in {:?}",
        params,
        results.len(),
        start.elapsed()
    );

    let response = Response {
        metadata: MetadataResponse { query: params },
        features: results
            .into_iter()
            .map(|(results, _)| results)
            .collect::<Vec<AirmailPoi>>(),
    };

    Ok(Json(serde_json::to_value(response)?))
}
