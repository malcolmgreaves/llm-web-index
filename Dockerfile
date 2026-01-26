###
### Supporting tools
###
FROM rust:1.92-slim-bookworm AS tools

RUN apt-get update && apt-get install -y \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /tools

# Need diesel to run migrations in runtime image build.
RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
    cargo install diesel_cli --no-default-features --features postgres --root /tools

# Install wasm-pack for building the WASM frontend
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo install wasm-pack --root /tools

###
### Dependencies - Build and cache all external dependencies
###
### This stage ONLY changes when Cargo.toml, Cargo.lock, or build.rs files change.
### Source code changes (*.rs) do NOT invalidate this layer.
###
FROM rust:1.92-slim-bookworm AS dependencies

RUN apt-get update && apt-get install -y \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Install wasm32 target for frontend builds (cached in this layer)
RUN rustup target add wasm32-unknown-unknown

# Copy ONLY dependency-related files (Cargo manifests and build scripts)
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY src/core-ltx/Cargo.toml src/core-ltx/build.rs ./src/core-ltx/
COPY src/data-model-ltx/Cargo.toml ./src/data-model-ltx/
COPY src/front-ltx/Cargo.toml ./src/front-ltx/
COPY src/api-ltx/Cargo.toml ./src/api-ltx/
COPY src/cron-ltx/Cargo.toml ./src/cron-ltx/
COPY src/worker-ltx/Cargo.toml ./src/worker-ltx/

# Create minimal dummy source files for each workspace crate.
# This allows cargo to compile all EXTERNAL dependencies without any project source code.
# - Library crates need lib.rs
# - Binary crates need main.rs
# - core-ltx has both lib.rs and main.rs
# - api-ltx has additional binaries in src/bin/
RUN mkdir -p src/core-ltx/src src/data-model-ltx/src src/front-ltx/src \
             src/api-ltx/src src/api-ltx/src/bin src/cron-ltx/src src/worker-ltx/src && \
    echo "pub fn _dummy() {}" > src/core-ltx/src/lib.rs && \
    echo "fn main() {}" > src/core-ltx/src/main.rs && \
    echo "pub fn _dummy() {}" > src/data-model-ltx/src/lib.rs && \
    echo "pub fn _dummy() {}" > src/front-ltx/src/lib.rs && \
    echo "fn main() {}" > src/api-ltx/src/main.rs && \
    echo "fn main() {}" > src/api-ltx/src/bin/generate-password-hash.rs && \
    echo "fn main() {}" > src/api-ltx/src/bin/generate-tls-cert.rs && \
    echo "fn main() {}" > src/cron-ltx/src/main.rs && \
    echo "fn main() {}" > src/worker-ltx/src/main.rs

# Build all dependencies.
# IMPORTANT: Do NOT use --mount=type=cache for target directory here!
# The compiled dependencies must be part of the image layer so they're inherited by later stages.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --release --workspace

###
### Builder - Copy all project source code
###
### Inherits the compiled dependencies from the dependencies stage.
###
FROM dependencies AS builder

# Remove dummy source files (keep target/ with compiled deps)
RUN rm -rf src/*/src

# Copy all actual project source code
COPY src/ ./src/

###
### Binaries - Build all release binaries
###
### Inherits compiled dependencies from builder. Only workspace crates are recompiled.
###
FROM builder AS binaries

# Remove fingerprints for workspace crates to force them to rebuild.
# External dependencies remain cached and won't be recompiled.
RUN rm -rf target/release/.fingerprint/core-ltx-* \
           target/release/.fingerprint/core_ltx-* \
           target/release/.fingerprint/data-model-ltx-* \
           target/release/.fingerprint/data_model_ltx-* \
           target/release/.fingerprint/front-ltx-* \
           target/release/.fingerprint/front_ltx-* \
           target/release/.fingerprint/api-ltx-* \
           target/release/.fingerprint/api_ltx-* \
           target/release/.fingerprint/cron-ltx-* \
           target/release/.fingerprint/cron_ltx-* \
           target/release/.fingerprint/worker-ltx-* \
           target/release/.fingerprint/worker_ltx-* && \
    rm -rf target/release/deps/libcore_ltx* \
           target/release/deps/libdata_model_ltx* \
           target/release/deps/libfront_ltx* \
           target/release/deps/api_ltx* \
           target/release/deps/libapi_ltx* \
           target/release/deps/cron_ltx* \
           target/release/deps/libcron_ltx* \
           target/release/deps/worker_ltx* \
           target/release/deps/libworker_ltx*

# Build all workspace binaries (dependencies are already compiled)
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --release --workspace && \
    mkdir -p ./bin && \
    cp target/release/api-ltx ./bin/ && \
    cp target/release/worker-ltx ./bin/ && \
    cp target/release/cron-ltx ./bin/

###
### WASM frontend
###
### Builds the front-ltx WASM package for the web UI.
###
FROM builder AS frontend

COPY --from=tools /tools/bin/wasm-pack /usr/local/bin/wasm-pack

# Remove fingerprints for front-ltx to force rebuild
RUN rm -rf target/release/.fingerprint/front-ltx-* \
           target/release/.fingerprint/front_ltx-* \
           target/release/deps/libfront_ltx* \
           target/release/deps/front_ltx*

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cd src/front-ltx && \
    wasm-pack build --target web --out-dir www/pkg --release

###
### Runtime - Base
###
FROM debian:bookworm-slim AS runtime-base

# NOTE: Keep these runtime dependencies in-sync with the system dependencies from the builder image.
RUN apt-get update && apt-get install -y \
    libpq5 \
    postgresql-client \
    libc6 \
    ca-certificates \
    wget \
    && rm -rf /var/lib/apt/lists/*

###
### Runtime - API
###
FROM runtime-base AS api

COPY --from=tools /tools/bin/diesel /usr/local/bin/diesel

WORKDIR /app

# API server binary
COPY --from=binaries /app/bin/api-ltx /usr/local/bin/api-ltx
# DB migrations
COPY src/api-ltx/migrations ./migrations
COPY src/api-ltx/diesel.toml ./diesel.toml
# WASM frontend
COPY --from=frontend /app/src/front-ltx/www/index.html ./src/front-ltx/www/index.html
COPY --from=frontend /app/src/front-ltx/www/pkg ./src/front-ltx/www/pkg

EXPOSE 3000
COPY src/api-ltx/entrypoint.sh /usr/local/bin/docker-entrypoint.sh
ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]

###
### Runtime - Worker
###
FROM runtime-base AS worker

WORKDIR /app
# Worker binary
COPY --from=binaries /app/bin/worker-ltx /usr/local/bin/worker-ltx
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/worker-ltx"]

###
### Runtime - Cron Updater
###
FROM runtime-base AS cron

WORKDIR /app
# cron updater binary
COPY --from=binaries /app/bin/cron-ltx /usr/local/bin/cron-ltx
ENTRYPOINT ["/usr/local/bin/cron-ltx"]
