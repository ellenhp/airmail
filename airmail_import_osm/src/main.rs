use std::path::PathBuf;

use airmail::index::AirmailIndex;
use airmail_indexer::ImporterBuilder;
use clap::Parser;
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
    index: String,

    /// Path to a administrative area cache db. This is a redb file that
    /// contains a cache of point-in-polygon lookups into the pelias spatial
    /// server. This is technically optional but we'll just create one in a
    /// temporary directory if you don't specify it. Keeping a cache around can
    /// speed up subsequent imports by like 5-10x so it's worth it.
    #[clap(long, short)]
    admin_cache: Option<String>,

    // ============================ OSM-specific options ===================================
    /// Path to an OSMExpress file to import.
    #[clap(long, short)]
    osmx: String,

    /// Path or reference to libspatialite, the SQLite extension that provides
    /// spatial functions. If not provided, the default system library will be
    /// used (mod_spatialite) provided by libsqlite3-mod-spatialite on Debian/Ubuntu.
    /// On Windows, mod_spatialite.dll needs to be within your PATH, or you can specify
    /// the full path to the DLL.
    #[clap(long, short)]
    libspatialite_path: Option<String>,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Args::parse();

    let mut index = AirmailIndex::create(&args.index).expect("Failed to open index");
    let importer = {
        let mut builder = ImporterBuilder::new(&args.wof_db);

        if let Some(admin_cache) = args.admin_cache {
            builder = builder.admin_cache(&admin_cache);
        }

        if let Some(libspatialite_path) = args.libspatialite_path {
            builder = builder.libspatialite_path(&libspatialite_path);
        }

        builder.build().await
    }
    .expect("Failed to create importer");

    let (poi_sender, poi_receiver) = crossbeam::channel::bounded(1024);

    let handle = spawn_blocking(move || {
        openstreetmap::parse_osm(&args.osmx, &move |poi| {
            poi_sender.send(poi)?;
            Ok(())
        })
        .expect("Failed to parse OSM");
    });

    importer
        .run_import(&mut index, "osm", poi_receiver)
        .await
        .expect("Failed to import");

    handle.await.expect("Failed to join OSM parsing task");
}
