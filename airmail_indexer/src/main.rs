use std::path::PathBuf;

use airmail::index::AirmailIndex;
use airmail_indexer::ImporterBuilder;
use anyhow::Result;
use clap::Parser;
use env_logger::Env;
use tokio::task::spawn_blocking;

mod openstreetmap;

#[derive(Debug, Parser)]
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

    // ============================ OSM-specific options ===================================
    /// Path to an OSMExpress file to import.
    #[clap(long, short)]
    osmx: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let args = Args::parse();

    // Create the index
    let index = AirmailIndex::create(&args.index)?;
    let mut import_builder = ImporterBuilder::new(&args.wof_db);
    if let Some(admin_cache) = args.admin_cache {
        import_builder = import_builder.admin_cache(&admin_cache);
    }
    let importer = import_builder.build().await?;

    let (poi_sender, poi_receiver) = crossbeam::channel::bounded(4096);

    // Load the OSM data and pass to channel
    let osm_loader = spawn_blocking(move || openstreetmap::parse_osm(&args.osmx, poi_sender));
    importer.run_import(index, "osm", poi_receiver).await?;
    osm_loader.await??;

    Ok(())
}
