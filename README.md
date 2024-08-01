# 📫 Airmail 📫

Airmail is an extremely lightweight geocoder[^1] written in pure Rust. Built on top of [tantivy](https://github.com/quickwit-oss/tantivy), it offers a low memory footprint and fast indexing (index the planet in under 3 hours!). Airmail aims to support international queries in several languages, but in practice it's still very early days and there are definitely bugs preventing correct behavior.

[^1]: A geocoder is a search engine for places. When you type in "vegan donut shop" into your maps app of choice, a geocoder is what shows you nearby places that fit your query.

## Features

Airmail's killer feature is the ability to query remote indices, e.g. on S3. This lets you keep your index hosting costs fixed while you scale horizontally. The baseline cost of a global Airmail deployment is about $5 per month.

## Roadmap

- [x] English/North American query parser for addresses, place names, and place name queries with locality or neighborhood.
- [x] Index OpenStreetMap data, from osmx or pbf file.
- [ ] Index OpenAddresses data (not currently used in demo).
- [ ] Index WhosOnFirst data.
- [x] API server.
- [x] Address queries.
- [x] Named POI queries.
- [x] Prefix queries.
- [x] Query remote indices.
- [x] Support and test planet-scale indices.
- [x] International address queries.
- [x] Categorical search, e.g. "coffee shop seattle".
- [x] Typo tolerance (limited to >=8 character input tokens)
- [x] Bounding box restriction.
- [ ] Focus point queries.
- [ ] Systematic/automatic quality testing in CI.

## Quickstart

This guide will create an index with a chosen geographical region (or the planet!) and run Airmail.

### Requirements

- Rust environment, or Docker with Docker Compose.
- ~16GB memory and 10-100GB of free space.

### Clone the Repo

```bash
git clone git@github.com:ellenhp/airmail.git

cd airmail

mkdir ./data
```

### Fetch Data

It's likely a good idea to build a smaller region first, and then planet if you have the need and space. This guide references Australia, but you can use any region.

1. Download OSM probuf file (.pbf file) for the target region of interest. See: <https://download.geofabrik.de> or <https://download.bbbike.org/osm/planet/> and place into `./data` folder.
2. Download Who's On First (SpatiaLite format). For planet see: <https://geocode.earth/data/whosonfirst/combined/> and <https://data.geocode.earth/wof/dist/spatial/whosonfirst-data-admin-latest.spatial.db.bz2>
3. Ensure files are present and decompressed in the `./data/` directory.

### Option 1 - Docker

```bash
# Build the images
docker compose build

# Build the index (from a pbf)
docker compose run indexer \
indexer --wof-db /data/whosonfirst-data-admin-latest.spatial.db \
--index /data/index/ \
load-osm-pbf /data/australia-oceania-latest.osm.pbf

# Launch the service
docker compose up airmail_service
```

### Option 2 - From Source

```bash
# Install deps
apt-get install -y libssl-dev capnproto clang pkg-config libzstd-dev libsqlite3-mod-spatialite

# Run indexer
cargo run --bin indexer \
--wof-db /data/whosonfirst-data-admin-latest.spatial.db \
--index /data/index/ \
load-osm-pbf /data/australia-oceania-latest.osm.pbf

# Run service
cargo run --bin airmail_service \
--index /data/index/
```

## License

Dual MIT/Apache 2 license, at your option.
