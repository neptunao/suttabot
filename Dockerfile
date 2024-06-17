FROM rust:1.79 as builder

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

FROM gcr.io/distroless/cc-debian12

ENV RUST_LOG=info
ENV DATABASE_URL=sqlite://db/suttabot.db
ENV DATA_DIR=/data

COPY --from=builder /usr/src/app/target/release/suttabot /

CMD ["./suttabot"]
