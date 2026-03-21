FROM rust:1.88-bookworm AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY src src

# par2cmdline-turbo is downloaded and bundled by the par2-sys crate at build time
RUN cargo build --release


FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        unrar-free \
        p7zip-full \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/rustnzbd /usr/local/bin/rustnzbd

RUN useradd -m -s /bin/bash rustnzbd \
    && mkdir -p /config /data /downloads/incomplete /downloads/complete \
    && chown -R rustnzbd:rustnzbd /config /data /downloads

USER rustnzbd
WORKDIR /app

EXPOSE 9090

VOLUME ["/config", "/data", "/downloads"]

ENTRYPOINT ["rustnzbd"]
CMD ["--config", "/config/config.toml", "--data-dir", "/data", "--port", "9090"]
