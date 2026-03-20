FROM rust:1-bookworm AS builder
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src/ src/
RUN cargo build --release && \
    ./target/release/benchnzb --help > /dev/null

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl par2 p7zip-full \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/benchnzb /usr/local/bin/benchnzb
ENTRYPOINT ["benchnzb"]
