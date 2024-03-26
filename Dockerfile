FROM debian:bookworm-slim as run

RUN apt-get update -y && apt-get install -y ca-certificates

FROM rust:1.77 as builder

WORKDIR /usr/src

# create a new empty shell project
RUN USER=root cargo new --bin app

WORKDIR /usr/src/app

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN cargo build --release
RUN rm src/*.rs

COPY ./src ./src
COPY ./db ./db

RUN rm ./target/release/deps/suttabot*
RUN cargo build --release

FROM run

ENV RUST_LOG=info
ENV DATABASE_URL=sqlite://db/suttabot.db
ENV DATA_DIR=/data

COPY --from=builder /usr/src/app/target/release/suttabot /usr/local/bin/suttabot

ENTRYPOINT ["suttabot"]
