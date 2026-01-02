###
### Supporting tools
###
FROM rust:1.92-slim as tools

RUN apt-get update && apt-get install -y \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Need diesel to run migrations in runtime image build.
WORKDIR /tools
RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
    cargo install diesel_cli --no-default-features --features postgres --root /tools

###
### Project build
###
FROM rust:1.92-slim as builder

## system dependencies
##
RUN apt-get update && apt-get install -y \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

## project dependencies
##
WORKDIR /app
RUN mkdir ./bin

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./

# NOTICE: Keep up to date!
#  - For every crate that is used!
#    - Include the `src/$crate/Cargo.toml` file
#      + These provide the crate structure & all dependencies.
#    - Create an empty file in the crate's `src/`
#
# This provides the crate structure, with the absolute minimum of
# unchanging code, and all dependencies of the entire project.
#
# Doing this allows us to reuse the build cache for dependencies
# without invalidating it on code changes.
COPY src/common_ltx/Cargo.toml ./src/common_ltx/
COPY src/core_ltx/Cargo.toml ./src/core_ltx/
COPY src/api_ltx/Cargo.toml ./src/api_ltx/

RUN mkdir -p src/api_ltx/src && \
    echo "fn main() {}" > src/api_ltx/src/main.rs

RUN mkdir -p src/common_ltx/src && \
    echo "" > src/common_ltx/src/lib.rs && \
    mkdir -p src/core_ltx/src && \
    echo "" > src/core_ltx/src/lib.rs

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release -p api-ltx

# only removes dummy files
RUN rm -rf src/common_ltx/src src/core_ltx/src src/api_ltx/src

## project code & configuration
##
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

COPY --from=tools /tools/bin/diesel /usr/local/bin/diesel

# Install runtime dependencies
# NOTE: Keep these in-sync with the system dependencies from the builder image.
RUN apt-get update && apt-get install -y \
    libpq5 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/bin/api-ltx /usr/local/bin/api-ltx
# Copy migrations and diesel config
COPY src/api_ltx/migrations ./migrations
COPY src/api_ltx/diesel.toml ./diesel.toml

# Create entrypoint script
RUN echo '#!/bin/bash\n\
set -e\n\
\n\
# Wait for database to be ready\n\
echo "Waiting for database to be ready..."\n\
until diesel database setup --locked-schema 2>/dev/null || diesel migration run 2>/dev/null; do\n\
  echo "Database is unavailable - sleeping"\n\
  sleep 1\n\
done\n\
\n\
echo "Database is ready - running migrations"\n\
diesel migration run\n\
\n\
echo "Starting API server"\n\
exec api-ltx\n\
' > /usr/local/bin/docker-entrypoint.sh && \
    chmod +x /usr/local/bin/docker-entrypoint.sh

EXPOSE 3000

ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]
