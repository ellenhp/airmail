# Airmail build docs

These are extremely barebones but it's what I'm able to throw together with the amount of free time I have available. Contributions and feedback welcome. You will need to modify paths in the commands I've given.

You have two options for indexing:
1. Build the indexer with cargo.
2. Build the indexer with Podman.

Option 1 is recommended, but if you have a weird configuration on your system or some of the dependencies are hard to find, you can use podman (or docker) for everything, which should be a little more foolproof.

## Option 1: Install dependencies for a host-side build of the indexer.

Install capnp, clang, pkg-config, zstd's development packages and openssl's development packages.

## Download data

Put data files into ./data

Create ./index with mkdir.

Get OSM file. BBBike and Geofabrik have extracts. Attempt the process with an extract before building the planet.

Download a WhosOnFirst Spatialite database for the country of your selected extract. Use the planet download if you're doing a planet build or your extract covers multiple countries.

## Building the necessary containers

Option 2 only: `podman build -f Dockerfile.build -t airmail_builder`

`podman build -f airmail_indexer/Dockerfile.osmx -t airmail_osmx`

`podman build -f Dockerfile -t airmail_service`

`podman build -f spatial/Dockerfile -t spatial_custom`

## Expanding the OSM data to OSMExpress

Substitute paths for your extract. `podman run --rm -v ./data:/var/osm:Z airmail_osmx expand Seattle.osm.pbf Seattle.osmx`

## Indexing

Option 1: `cargo run --release --bin airmail_import_osm -- --wof-db $PWD/whosonfirst-data-admin-us-latest.spatial.db --index ./index --admin-cache ./admin-cache --osmx $PWD/data/Seattle.osmx --docker-socket /run/user/1000/podman/podman.sock`

OR

Option 2: `chcon -t container_file_t ./data/whosonfirst-data-admin-us-latest.spatial.db; podman run --security-opt label=disable --net=host -v /run/user/1000/podman/podman.sock:/run/podman/podman.sock:z -v ./data:/var/airmail/data:Z -v ./index:/var/airmail/index:Z --rm airmail_build airmail_import_osm --wof-db $PWD/data/whosonfirst-data-admin-us-latest.spatial.db --index /var/airmail/index --admin-cache /var/airmail/data/admin-cache --osmx /var/airmail/data/Seattle.osmx --docker-socket /run/podman/podman.sock --recreate`

## Running the service

`podman run --rm --name airmail-service -p 3000:3000 -v $PWD/index:/var/airmail/index:Z airmail_service --index /var/airmail/index`


