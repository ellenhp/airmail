#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]

use airmail_indexer::{error::IndexerError, ImporterBuilder};
use anyhow::Result;
use clap::{Parser, Subcommand};
use env_logger::Env;
use futures_util::future::join_all;
use log::warn;
use openstreetmap::OSMExpressLoader;
use osm_pbf::OsmPbf;
use osmx::Database;
use std::path::PathBuf;
use tokio::{select, spawn, task::spawn_blocking};

mod openstreetmap;
mod osm_pbf;

#[derive(Debug, Parser)]
#[clap(version = env!("CARGO_PKG_VERSION"), author = env!("CARGO_PKG_AUTHORS"))]
struct Args {
    /// Path to the Who's On First Spatialite database. Used for populating
    /// administrative areas, which are often missing or wrong in OSM.
    #[clap(long, short)]
    wof_db: PathBuf,

    /// Path to the Airmail index to import into. This should be either an empty
    /// directory or a directory containing an existing index created with the
    /// same version of airmail (unless you really know what you're doing).
    #[clap(long, short)]
    index: PathBuf,

    /// Path to a administrative area cache db. This is a redb file that
    /// contains a cache of point-in-polygon lookups into the pelias spatial
    /// server. This is technically optional but we'll just create one in a
    /// temporary directory if you don't specify it. Keeping a cache around can
    /// speed up subsequent imports by like 5-10x so it's worth it.
    #[clap(long, short)]
    admin_cache: Option<PathBuf>,

    /// Path to `WhosOnFirst` spatial index for point-in-polygon lookups. If this is specified
    /// we'll use the spatial index instead of sqlite geospatial lookups. This will speed up imports,
    /// after the index is built. It'll be faster for planet scale imports, or frequent imports
    /// but will use 10GB of memory and takes a few minutes to build. `mod_spatialite` is not required
    /// if this is specified.
    #[clap(long, short)]
    pip_tree: Option<PathBuf>,

    /// The loader to use for importing data.
    #[clap(subcommand)]
    loader: Loader,
}

#[derive(Subcommand, Clone, Debug, Eq, PartialEq)]
#[command(arg_required_else_help = true)]
enum Loader {
    LoadOsmx {
        /// Path to an `OSMExpress` file to import.
        path: PathBuf,
    },

    /// Path to an OSM PBF file to import.
    LoadOsmPbf {
        /// Path to an OSM PBF file to import.
        path: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    let mut handles = vec![];

    // Setup the import pipeline
    let mut import_builder = ImporterBuilder::new(&args.index, &args.wof_db)?;
    if let Some(admin_cache) = args.admin_cache {
        import_builder = import_builder.admin_cache(&admin_cache);
    }
    if let Some(pip_tree) = args.pip_tree {
        import_builder = import_builder.pip_tree_cache(&pip_tree);
    }
    let importer = import_builder.build().await?;

    // Send POIs from the OSM parser to the importer.
    let (poi_sender, poi_receiver) = crossbeam::channel::bounded(16384);

    // Spawn the OSM parser
    let indexer_cache = importer.indexer_cache();
    handles.push(spawn_blocking(move || {
        match args.loader {
            Loader::LoadOsmx { path } => {
                // Setup OSM
                let osm_db = Database::open(path).map_err(IndexerError::from)?;
                let osm = OSMExpressLoader::new(&osm_db, poi_sender)?;
                osm.parse_osm().map_err(|e| {
                    warn!("Error parsing OSM: {}", e);
                    e
                })
            }
            Loader::LoadOsmPbf { path } => {
                let osm = OsmPbf::new(&path, poi_sender, indexer_cache);
                osm.parse_osm().map_err(|e| {
                    warn!("Error parsing OSM: {}", e);
                    e
                })
            }
        }
    }));

    // Spawn the importer
    handles.push(spawn(async move {
        importer.run_import("osm", poi_receiver).await
    }));

    // Wait for the first thing to finish
    select! {
        _ = join_all(handles) => {}
        _ = tokio::signal::ctrl_c() => {
            warn!("Received ctrl-c, shutting down");
        }
    }

    Ok(())
}
