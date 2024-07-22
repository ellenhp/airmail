use airmail::index::AirmailIndex;
use airmail_indexer::ImporterBuilder;
use clap::Parser;
use reqwest::Url;
use tokio::task::spawn_blocking;

mod openstreetmap;

#[derive(Debug, Parser)]
struct Args {
    /// Url to the spatial server.
    #[clap(long, short, default_value = "http://localhost:3000")]
    spatial_url: Url,

    /// Path to the Who's On First Spatialite database. Used for populating
    /// administrative areas, which are often missing or wrong in OSM.
    #[clap(long, short)]
    wof_db: String,

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
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Args::parse();

    let mut index = AirmailIndex::create(&args.index).expect("Failed to open index");
    let importer = {
        let mut builder = ImporterBuilder::new(&args.wof_db, &args.spatial_url);

        if let Some(admin_cache) = args.admin_cache {
            builder = builder.admin_cache(&admin_cache);
        }

        builder.build().await
    };

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
