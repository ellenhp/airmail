#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]

use std::sync::Arc;

use airmail::index::AirmailIndex;
use anyhow::Result;
use api::search;
use axum::{http::HeaderValue, routing::get, Router};
use clap::Parser;
use env_logger::Env;
use log::{debug, info};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

mod api;
mod error;

#[derive(Debug, Parser)]
struct Args {
    /// The path to the index to load
    #[arg(short, long, env = "AIRMAIL_INDEX")]
    index: String,

    /// The address to bind to
    #[arg(short, long, env = "AIRMAIL_BIND", default_value = "127.0.0.1:3000")]
    bind: String,

    /// Cors origins to allow
    #[arg(
        short,
        long,
        env = "AIRMAIL_CORS",
        default_value = "http://localhost:5173"
    )]
    cors: Option<Vec<String>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let args = Args::parse();

    debug!("Loading index from {}", args.index);
    let index = if args.index.starts_with("http") {
        Arc::new(AirmailIndex::new_remote(&args.index)?)
    } else {
        Arc::new(AirmailIndex::new(&args.index)?)
    };

    let mut cors = CorsLayer::new();
    for origin in args.cors.unwrap_or_default() {
        cors = cors.allow_origin(origin.parse::<HeaderValue>()?);
    }

    info!("Loaded {} docs from index", index.num_docs().await?);
    let app = Router::new()
        .route("/search", get(search).with_state(index))
        .layer(cors);

    info!("Listening at: {}/search?q=query", args.bind);
    let listener = TcpListener::bind(args.bind).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
