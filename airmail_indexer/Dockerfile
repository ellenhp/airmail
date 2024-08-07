FROM rust:1.76

RUN apt-get update && apt-get install -y libssl-dev capnproto clang pkg-config libzstd-dev libsqlite3-mod-spatialite && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/airmail
COPY ./airmail ./airmail
COPY ./airmail_indexer ./airmail_indexer
COPY ./airmail_service ./airmail_service
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock

RUN cargo install --path ./airmail_indexer

WORKDIR /var/airmail
