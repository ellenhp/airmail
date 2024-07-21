# Bootstrap Airmail from Scratch

This guide will create an index with a chosen geographical region (or planet). And run Airmail.

## Requirements

- Docker with Docker Compose.
- Local clone of the repo.
- You'll require 10-100GBish of free datas.

```bash
# Clone repo
git clone git@github.com:ellenhp/airmail.git

cd airmail

# Pull submodules, required for custom spatial build
git submodule update --init --recursive

# Create index folder
mkdir -p ./data/index
```

## Fetching Data

1. Download OSM probuf file (.pbf file) for target region. See: <https://download.geofabrik.de> and place into `./data` folder.
2. Download Who's On First (SpatiaLite format). For planet see: <https://geocode.earth/data/whosonfirst/combined/> and <https://data.geocode.earth/wof/dist/spatial/whosonfirst-data-admin-latest.spatial.db.bz2>
3. Ensure files are present and decompressed in the `./data/` directory.

## Build Images

```bash
# Build the images necessary for creating an index.
docker compose --profile index build

# Build the airmail container, for running the http listener
docker compose build
```

## Create the Index

It's likely a good idea to build a smaller region first and then planet if you have a need/space. This guide references Australia.

**Note:** The `/data/` path within containers refers to `./data/`, as configured in the `docker-compose.yaml` file.

```bash

# Convert the .pbf to .osmx interactively
docker compose run osmx \
expand /data/australia-latest.osm.pbf /data/australia-latest.osm.osmx

# Check the files we need exist, you should see the two files
ls -lh ./data/whosonfirst-data-admin-latest.spatial.db ./data/australia-latest.osm.osmx

# Run pelias-spatial (in another window). Make sure it stays up/listening.
docker compose up spatial

# Build the index
docker compose --profile index run build-index \
airmail_import_osm --wof-db /data/whosonfirst-data-admin-latest.spatial.db \
--index /data/index \
--admin-cache /data/admin-cache \
--osmx /data/australia-latest.osm.osmx \
--spatial-url http://host.docker.internal:3000

# If the index built ok, stop pelias
```

## Run Airmail

```bash
# Launch the service
docker compose up airmail_service
```

Run a query: <http://localhost:3000/search?q=lilydale>
