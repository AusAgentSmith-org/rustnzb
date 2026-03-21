FROM rust:1.88-bookworm AS builder

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY src src

RUN cargo build --release


FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        unrar-free \
        p7zip-full \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/rustnzb /usr/local/bin/rustnzb

RUN useradd -m -s /bin/bash rustnzb \
    && mkdir -p /config /data /downloads/incomplete /downloads/complete \
    && chown -R rustnzb:rustnzb /config /data /downloads

USER rustnzb
WORKDIR /app

EXPOSE 9090

VOLUME ["/config", "/data", "/downloads"]

ENTRYPOINT ["rustnzb"]
CMD ["--config", "/config/config.toml", "--data-dir", "/data", "--port", "9090"]
