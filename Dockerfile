# syntax=docker/dockerfile:1

# ---- builder ---------------------------------------------------------------
# Builds the REST server with the `rocks` feature so the same image can run
# in-memory (default) or durable (set CHRONOS_DATA_DIR to a mounted volume).
FROM rust:1.85-bookworm AS builder

# librocksdb-sys builds RocksDB from source via bindgen, which needs clang/llvm.
RUN apt-get update \
    && apt-get install -y --no-install-recommends clang libclang-dev llvm-dev cmake \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY . .

RUN cargo build --release --locked -p chronos-server --features rocks

# ---- runtime ---------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libstdc++6 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 chronos \
    && mkdir -p /data && chown chronos:chronos /data

COPY --from=builder /src/target/release/chronos-server /usr/local/bin/chronos-server

USER chronos
WORKDIR /data
VOLUME ["/data"]

# Listen on all interfaces inside the container; persist to the /data volume.
ENV CHRONOS_ADDR=0.0.0.0:8080 \
    CHRONOS_DATA_DIR=/data
EXPOSE 8080

ENTRYPOINT ["chronos-server"]
