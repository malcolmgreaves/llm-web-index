###
### Supporting tools
###
FROM rust:1.92-slim-bookworm as tools

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
FROM rust:1.92-slim-bookworm as builder

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

COPY src/common_ltx/Cargo.toml ./src/common_ltx/
COPY src/core_ltx/Cargo.toml ./src/core_ltx/
COPY src/front_ltx/Cargo.toml ./src/front_ltx/
COPY src/api_ltx/Cargo.toml ./src/api_ltx/
COPY src/cli_ltx/Cargo.toml ./src/cli_ltx/
COPY src/cron_ltx/Cargo.toml ./src/cron_ltx/
COPY src/worker_ltx/Cargo.toml ./src/worker_ltx/

# NOTE: Keep up to date! Library crates
RUN for crate in common_ltx core_ltx front_ltx; do \
        mkdir -p src/${crate}/src && \
        echo "" > src/${crate}/src/lib.rs; \
    done

# NOTE: Keep up to date! Binary crates
RUN for crate in api_ltx cli_ltx cron_ltx worker_ltx; do \
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
FROM builder as frontend

COPY --from=tools /tools/bin/wasm-pack /usr/local/bin/wasm-pack

# remove dummy files
RUN rm -rf src/front_ltx/src

COPY src/front_ltx/src ./src/front_ltx/src
COPY src/front_ltx/www ./src/front_ltx/www
RUN cd src/front_ltx && \
    wasm-pack build --target web --out-dir www/pkg --release

###
### API server
###
FROM builder as api

# NOTE: Keep up to date! Remove dummy files from crates that **ARE NOT** used.
RUN for crate in common_ltx core_ltx api_ltx; do \
        rm -rf src/${crate}/src; \
    done

COPY src/common_ltx/src ./src/common_ltx/src
COPY src/core_ltx/src ./src/core_ltx/src
COPY src/api_ltx/src ./src/api_ltx/src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release -p api-ltx && \
    cp /app/target/release/api-ltx /app/bin/api-ltx

###
### Runtime
###
FROM debian:bookworm-slim

# NOTE: Keep these runtime dependencies in-sync with the system dependencies from the builder image.
RUN apt-get update && apt-get install -y \
    libpq5 \
    postgresql-client \
    libc6 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=tools /tools/bin/diesel /usr/local/bin/diesel

WORKDIR /app

# API server
COPY --from=api /app/bin/api-ltx /usr/local/bin/api-ltx
# DB migrations
COPY src/api_ltx/migrations ./migrations
COPY src/api_ltx/diesel.toml ./diesel.toml
# WASM frontend
COPY --from=frontend /app/src/front_ltx/www/index.html ./src/front_ltx/www/index.html
COPY --from=frontend /app/src/front_ltx/www/pkg ./src/front_ltx/www/pkg

EXPOSE 3000
COPY src/api_ltx/entrypoint.sh /usr/local/bin/docker-entrypoint.sh
ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]
