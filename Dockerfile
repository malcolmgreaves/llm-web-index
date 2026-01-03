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

COPY src/common-ltx/Cargo.toml ./src/common-ltx/
COPY src/core-ltx/Cargo.toml ./src/core-ltx/
COPY src/core-ltx/build.rs ./src/core-ltx/
COPY src/front-ltx/Cargo.toml ./src/front-ltx/
COPY src/api-ltx/Cargo.toml ./src/api-ltx/
COPY src/cli-ltx/Cargo.toml ./src/cli-ltx/
COPY src/cron-ltx/Cargo.toml ./src/cron-ltx/
COPY src/worker-ltx/Cargo.toml ./src/worker-ltx/

# NOTE: Keep up to date! Library crates
RUN for crate in common-ltx core-ltx front-ltx; do \
        mkdir -p src/${crate}/src && \
        echo "" > src/${crate}/src/lib.rs; \
    done

# NOTE: Keep up to date! Binary crates
RUN for crate in api-ltx cli-ltx cron-ltx worker-ltx; do \
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

# remove dummy files
RUN rm -rf src/front-ltx/src

COPY src/front-ltx/src ./src/front-ltx/src
COPY src/front-ltx/www ./src/front-ltx/www
RUN cd src/front-ltx && \
    wasm-pack build --target web --out-dir www/pkg --release

###
### API server
###
FROM builder AS api

# NOTE: Keep up to date! Remove dummy files from crates that **ARE NOT** used.
RUN for crate in common-ltx core-ltx api-ltx; do \
        rm -rf src/${crate}/src; \
    done

COPY src/common-ltx/src ./src/common-ltx/src
COPY src/core-ltx/src ./src/core-ltx/src
COPY src/api-ltx/src ./src/api-ltx/src

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
COPY src/api-ltx/migrations ./migrations
COPY src/api-ltx/diesel.toml ./diesel.toml
# WASM frontend
COPY --from=frontend /app/src/front-ltx/www/index.html ./src/front-ltx/www/index.html
COPY --from=frontend /app/src/front-ltx/www/pkg ./src/front-ltx/www/pkg

EXPOSE 3000
COPY src/api-ltx/entrypoint.sh /usr/local/bin/docker-entrypoint.sh
ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]
