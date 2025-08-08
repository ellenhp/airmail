#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]

use airmail::{index::AirmailIndexBuilder, poi::ToIndexPoi};
use airmail_indexer::{cache::IndexerCache, ImporterBuilder};
use anyhow::Result;
use async_channel::Sender;
use clap::{Parser, Subcommand};
use env_logger::Env;
use log::{info, warn};
use osm_pbf::{OsmPbf, ParseOsmTypes};
use std::{path::PathBuf, sync::Arc};
use tokio::{
    sync::Mutex,
    task::{spawn_blocking, JoinHandle},
};

mod osm;
mod osm_pbf;

#[derive(Debug, Parser)]
#[clap(version = env!("CARGO_PKG_VERSION"), author = env!("CARGO_PKG_AUTHORS"))]
struct Cli {
    /// Path to the Who's On First Spatialite database. Used for populating
    /// administrative areas, which are often missing or wrong in OSM.
    #[clap(long, short)]
    wof_db: PathBuf,

    // Path to store the airmail index intermediate db.
    #[clap(long, short)]
    index_db: String,

    /// Path where an indexing cache will be stored. This is a redb file that
    /// contains a cache of expensive operations. It is technically optional but we'll just create one in a
    /// temporary directory if you don't specify it. Keeping a cache around can
    /// speed up imports by like 5-10x+ so it's worth it.
    #[clap(long, short)]
    admin_cache: Option<PathBuf>,

    /// Path to `WhosOnFirst` spatial index for point-in-polygon lookups. If this is specified
    /// we'll use the spatial index instead of sqlite geospatial lookups. This will speed up imports,
    /// after the index is first built. It'll be faster for planet scale imports, or frequent imports,
    /// but it will use 10GB of memory and takes a few minutes to build.
    #[clap(long, short)]
    pip_tree: Option<PathBuf>,

    /// The loader to use for importing data.
    #[clap(subcommand)]
    loader: Loader,
}

#[derive(Subcommand, Clone, Debug, Eq, PartialEq)]
#[command(arg_required_else_help = true)]
pub(crate) enum Loader {
    /// Path to an OSM PBF file to import.
    LoadOsmPbf {
        /// Path to an OSM PBF file to import.
        path: PathBuf,

        /// If the nodes are known to be present in the cache (after first run), don't re-add nor check.
        #[clap(long, short)]
        nodes_already_cached: bool,

        /// Types to ignore during parsing.
        #[clap(long)]
        ignore: Vec<ParseOsmTypes>,
    },
}

impl Cli {
    async fn spawn_pass(
        &self,
        poi_sender: Sender<ToIndexPoi>,
        indexer_cache: Arc<IndexerCache>,
    ) -> Result<JoinHandle<()>, anyhow::Error> {
        Ok(match &self.loader {
            Loader::LoadOsmPbf {
                path,
                nodes_already_cached,
                ignore,
            } => {
                let osm = OsmPbf::new(
                    path,
                    *nodes_already_cached,
                    ignore,
                    poi_sender,
                    indexer_cache,
                );
                spawn_blocking(|| {
                    osm.parse_osm().expect("Failed to process osmpbf.");
                })
            }
        })
    }

    async fn main(self) -> Result<(), anyhow::Error> {
        // Setup the import pipeline
        let mut import_builder = ImporterBuilder::new(&self.wof_db).await?;
        if let Some(admin_cache) = &self.admin_cache {
            import_builder = import_builder.admin_cache(&admin_cache);
        }
        if let Some(pip_tree) = &self.pip_tree {
            import_builder = import_builder.pip_tree_cache(&pip_tree);
        }
        let mut importer = import_builder.build().await?;
        let indexer_cache = importer.indexer_cache();
        let builder = Arc::new(Mutex::new(
            AirmailIndexBuilder::create(&self.index_db).await?,
        ));
        builder.lock().await.clear_db().await?;

        let (poi_sender, poi_receiver) = async_channel::bounded(16384);
        let pass_handle = {
            let indexer_cache = indexer_cache.clone();
            self.spawn_pass(poi_sender, indexer_cache).await?
        };

        {
            let builder = builder.clone();
            importer
                .run_import(poi_receiver, async move |poi| {
                    let mut attempts = 0;
                    while let Err(err) = builder.lock().await.collect_vocabulary(&poi).await {
                        attempts += 1;
                        if attempts >= 5 {
                            warn!("Failed to insert POI after {attempts} attempts: {err}");
                            break;
                        }
                    }
                })
                .await?;
        }
        pass_handle.await?;

        info!("Finished pass 1");

        info!("Beginning vocabulary processing...");

        builder.lock().await.process_vocabulary().await?;

        info!("Done");

        info!("Beginning pass 2");

        // Pass 2

        let (poi_sender, poi_receiver) = async_channel::bounded(16384);
        let pass_handle = self.spawn_pass(poi_sender, indexer_cache).await?;

        {
            let builder = builder.clone();
            importer
                .run_import(poi_receiver, async move |poi| {
                    let mut attempts = 0;
                    while let Err(err) = builder.lock().await.insert_poi(&poi).await {
                        attempts += 1;
                        if attempts >= 5 {
                            warn!("Failed to insert POI after {attempts} attempts: {err}");
                            break;
                        }
                    }
                })
                .await?;
        }
        pass_handle.await?;

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    cli.main().await?;

    Ok(())
}
