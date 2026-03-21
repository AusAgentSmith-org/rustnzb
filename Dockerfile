FROM rust:1.88-bookworm AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY src src

RUN cargo build --release

# Build par2cmdline-turbo (multi-threaded, SIMD-accelerated par2)
RUN apt-get update && apt-get install -y --no-install-recommends git cmake g++ && \
    git clone --depth 1 https://github.com/animetosho/par2cmdline-turbo.git /tmp/par2turbo && \
    cd /tmp/par2turbo && \
    cmake -DCMAKE_BUILD_TYPE=Release . && cmake --build . && \
    cp par2 /usr/local/bin/par2


FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        unrar-free \
        p7zip-full \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/bin/par2 /usr/local/bin/par2
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
