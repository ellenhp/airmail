# How to index an OSM file

This is an extremely quick writeup mostly for my own use, so please pardon the dust.

These are the commands I use.

mkdir -p ~/index; rm ~/index/* ; RUST_LOG=info cargo run --release --bin airmail_import_osm -- --wof-db ~/Downloads/whosonfirst-data-admin-latest.spatial.db --index ~/index --admin-cache ~/admins --docker-socket /run/user/1000/podman/podman.sock --osmflat ~/Downloads/seattle-latest.osmflat

RUST_LOG=info cargo run --release --bin airmail_service -- --index ~/index