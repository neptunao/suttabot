FROM debian:bookworm-slim as run

RUN apt-get update -y && apt-get install -y ca-certificates

FROM rust:1.76 as builder

WORKDIR /usr/src/app

COPY . .

RUN cargo install --path .

FROM run

ENV RUST_LOG=info
ENV DATABASE_URL=sqlite://db/suttabot.db
ENV DATA_DIR=/data

COPY --from=builder /usr/local/cargo/bin/suttabot /usr/local/bin/suttabot

ENTRYPOINT ["suttabot"]
