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

# Install cargo-chef for dependency caching
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo install cargo-chef --locked --root /tools

###
### Planner - Analyze dependencies
###
FROM rust:1.92-slim-bookworm AS planner

RUN apt-get update && apt-get install -y \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=tools /tools/bin/cargo-chef /usr/local/bin/cargo-chef

# Copy entire source to analyze dependencies
COPY . .

# Generate recipe.json with all dependency information
RUN cargo chef prepare --recipe-path recipe.json

###
### Dependencies - Build all workspace dependencies
###
FROM rust:1.92-slim-bookworm AS dependencies

RUN apt-get update && apt-get install -y \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=tools /tools/bin/cargo-chef /usr/local/bin/cargo-chef

# Copy the recipe
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies - this is the cached layer
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo chef cook --release --recipe-path recipe.json

###
### WASM frontend
###
FROM dependencies AS frontend

COPY --from=tools /tools/bin/wasm-pack /usr/local/bin/wasm-pack

# Copy entire source
COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cd src/front-ltx && \
    wasm-pack build --target web --out-dir www/pkg --release

###
### Builder - API server
###
FROM dependencies AS api-build

# Copy entire source (cargo needs workspace structure)
COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release -p api-ltx && \
    mkdir -p ./bin && \
    cp /app/target/release/api-ltx /app/bin/api-ltx

###
### Builder - Worker
###
FROM dependencies AS worker-build

# Copy entire source
COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release -p worker-ltx && \
    mkdir -p ./bin && \
    cp /app/target/release/worker-ltx /app/bin/worker-ltx

###
### Builder - Cron Updater
###
FROM dependencies AS cron-build

# Copy entire source
COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release -p cron-ltx && \
    mkdir -p ./bin && \
    cp /app/target/release/cron-ltx /app/bin/cron-ltx


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

# API server
COPY --from=api-build /app/bin/api-ltx /usr/local/bin/api-ltx
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
COPY --from=worker-build /app/bin/worker-ltx /usr/local/bin/worker-ltx
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/worker-ltx"]

###
### Runtime - Cron Updater
###
FROM runtime-base AS cron

WORKDIR /app
# cron updater binary
COPY --from=cron-build /app/bin/cron-ltx /usr/local/bin/cron-ltx
ENTRYPOINT ["/usr/local/bin/cron-ltx"]
