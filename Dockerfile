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
### Project build: all dependencies
###
FROM rust:1.92-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
RUN mkdir ./bin

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./

# Ensure that the first layers of builder image contain all of the Rust
# dependencies for the project. By keeping dependencies first, then
# project code, we can reuse layers when making code changes.
#
# NOTE: Keep up to date! Include **ALL** crates referenced in the top-level Cargo.toml!
#       All crates **MUST** have:
#         - a Cargo.toml file
#         - either a lib.rs or a main.rs file (depending on what its Cargo says)
#           + a "dummy" file ("" or "fn main(){}", respectively) works

COPY src/core-ltx/Cargo.toml ./src/core-ltx/
COPY src/core-ltx/build.rs ./src/core-ltx/
COPY src/data-model-ltx/Cargo.toml ./src/data-model-ltx/
COPY src/front-ltx/Cargo.toml ./src/front-ltx/
COPY src/api-ltx/Cargo.toml ./src/api-ltx/
COPY src/cli-ltx/Cargo.toml ./src/cli-ltx/
COPY src/cron-ltx/Cargo.toml ./src/cron-ltx/
COPY src/worker-ltx/Cargo.toml ./src/worker-ltx/

# NOTE: Keep up to date! Dummy file for library crates.
RUN for crate in core-ltx data-model-ltx front-ltx; do \
        mkdir -p src/${crate}/src && \
        echo "" > src/${crate}/src/lib.rs; \
    done

# NOTE: Keep up to date! Dummy file for binary crates.
RUN for crate in api-ltx cli-ltx cron-ltx worker-ltx core-ltx; do \
        mkdir -p src/${crate}/src && \
        echo "fn main() {}" > src/${crate}/src/main.rs; \
    done

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release

###
### WASM frontend
###
FROM builder AS frontend

COPY --from=tools /tools/bin/wasm-pack /usr/local/bin/wasm-pack

# NOTE: Keep up to date! Remove dummy files from crates that **ARE** used.
RUN rm -rf src/front-ltx/src

COPY src/front-ltx/src ./src/front-ltx/src
COPY src/front-ltx/www ./src/front-ltx/www
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    rm -rf /app/target/release/.fingerprint/front-ltx-* \
           /app/target/release/.fingerprint/front_ltx-* \
           /app/target/release/deps/libfront_ltx* \
           /app/target/release/deps/front_ltx* && \
    cd src/front-ltx && \
    wasm-pack build --target web --out-dir www/pkg --release

###
### Builder - API server
###
FROM builder AS api-build

# NOTE: Keep up to date! Remove dummy files from crates that **ARE** used.
RUN for crate in core-ltx data-model-ltx api-ltx ; do \
        rm -rf src/${crate}/src; \
    done

COPY src/core-ltx/src ./src/core-ltx/src
COPY src/data-model-ltx/src ./src/data-model-ltx/src
COPY src/api-ltx/src ./src/api-ltx/src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    rm -rf /app/target/release/.fingerprint/core-ltx-* \
           /app/target/release/.fingerprint/core_ltx-* \
           /app/target/release/.fingerprint/data-model-ltx-* \
           /app/target/release/.fingerprint/data_model_ltx-* \
           /app/target/release/.fingerprint/api-ltx-* \
           /app/target/release/.fingerprint/api_ltx-* \
           /app/target/release/deps/libcore_ltx* \
           /app/target/release/deps/libdata_model_ltx* \
           /app/target/release/deps/libapi_ltx* \
           /app/target/release/deps/api_ltx* && \
    cargo build --release -p api-ltx && \
    cp /app/target/release/api-ltx /app/bin/api-ltx

###
### Builder - Worker
###
FROM builder AS worker-build

# NOTE: Keep up to date! Remove dummy files from crates that **ARE** used.
RUN for crate in core-ltx data-model-ltx worker-ltx; do \
        rm -rf src/${crate}/src; \
    done

COPY src/core-ltx/src ./src/core-ltx/src
COPY src/data-model-ltx/src ./src/data-model-ltx/src
COPY src/worker-ltx/src ./src/worker-ltx/src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    rm -rf /app/target/release/.fingerprint/core-ltx-* \
           /app/target/release/.fingerprint/core_ltx-* \
           /app/target/release/.fingerprint/data-model-ltx-* \
           /app/target/release/.fingerprint/data_model_ltx-* \
           /app/target/release/.fingerprint/worker-ltx-* \
           /app/target/release/.fingerprint/worker_ltx-* \
           /app/target/release/deps/libcore_ltx* \
           /app/target/release/deps/libdata_model_ltx* \
           /app/target/release/deps/libworker_ltx* \
           /app/target/release/deps/worker_ltx* && \
    cargo build --release -p worker-ltx && \
    cp /app/target/release/worker-ltx /app/bin/worker-ltx

###
### Builder - Cron Updater
###
FROM builder AS cron-build

# NOTE: Keep up to date! Remove dummy files from crates that **ARE** used.
RUN for crate in core-ltx data-model-ltx cron-ltx; do \
        rm -rf src/${crate}/src; \
    done

COPY src/core-ltx/src ./src/core-ltx/src
COPY src/data-model-ltx/src ./src/data-model-ltx/src
COPY src/cron-ltx/src ./src/cron-ltx/src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    rm -rf /app/target/release/.fingerprint/core-ltx-* \
           /app/target/release/.fingerprint/core_ltx-* \
           /app/target/release/.fingerprint/data-model-ltx-* \
           /app/target/release/.fingerprint/data_model_ltx-* \
           /app/target/release/.fingerprint/cron-ltx-* \
           /app/target/release/.fingerprint/cron_ltx-* \
           /app/target/release/deps/libcore_ltx* \
           /app/target/release/deps/libdata_model_ltx* \
           /app/target/release/deps/libcron_ltx* \
           /app/target/release/deps/cron_ltx* && \
    cargo build --release -p cron-ltx && \
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
ENTRYPOINT ["/usr/local/bin/worker-ltx"]

###
### Runtime - Cron Updater
###
FROM runtime-base AS cron

WORKDIR /app
# cron updater binary
COPY --from=cron-build /app/bin/cron-ltx /usr/local/bin/cron-ltx
ENTRYPOINT ["/usr/local/bin/cron-ltx"]
