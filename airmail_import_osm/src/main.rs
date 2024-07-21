use airmail::index::AirmailIndex;
use airmail_indexer::ImporterBuilder;
use clap::Parser;
use reqwest::Url;
use tokio::task::spawn_blocking;

mod openstreetmap;

#[derive(Debug, Parser)]
struct Args {
    /// Path to the Docker socket. This is used to run the Pelias spatial server
    /// container and perform point-in-polygon queries for administrative area
    /// population.
    #[clap(long, short)]
    docker_socket: Option<String>,

    /// Url to the Pelias spatial server. 
    #[clap(long, short, default_value = "http://localhost:3000")]
    pelias_url: Url,

    /// Path to the Who's On First Spatialite database. Used for populating
    /// administrative areas, which are often missing or wrong in OSM.
    #[clap(long, short)]
    wof_db: String,

    /// Whether to forcefully recreate the WOF spatial server container. Default
    /// false.
    #[clap(long, short, default_value = "false")]
    recreate: bool,

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
        let mut builder = ImporterBuilder::new(&args.wof_db, &args.pelias_url);

        if let Some(docker_socket) = args.docker_socket {
            builder = builder.docker_socket(&docker_socket);
        }

        if let Some(admin_cache) = args.admin_cache {
            builder = builder.admin_cache(&admin_cache);
        }

        if args.recreate {
            builder = builder.recreate_containers(true);
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
