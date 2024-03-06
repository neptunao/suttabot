FROM debian:bookworm-slim as run

RUN apt-get update -y && apt-get install -y ca-certificates

FROM rust:1.76 as builder

WORKDIR /usr/src/app

COPY . .

RUN cargo install --path .

FROM run

COPY --from=builder /usr/local/cargo/bin/suttabot /usr/local/bin/suttabot

ENTRYPOINT ["suttabot"]
