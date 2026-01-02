# Build stage
FROM rust:1.84-slim as builder

# Install PostgreSQL development libraries
RUN apt-get update && apt-get install -y \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Create a new empty project
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY src/api_ltx/Cargo.toml ./src/api_ltx/
COPY src/common_ltx/Cargo.toml ./src/common_ltx/

# Create dummy source files to cache dependencies
RUN mkdir -p src/api_ltx/src && \
    echo "fn main() {}" > src/api_ltx/src/main.rs && \
    mkdir -p src/common_ltx/src && \
    echo "" > src/common_ltx/src/lib.rs

# Build dependencies (this layer will be cached)
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release -p api-ltx

# Remove dummy files
RUN rm -rf src/api_ltx/src src/common_ltx/src

# Copy actual source code
COPY src/api_ltx/src ./src/api_ltx/src
COPY src/common_ltx/src ./src/common_ltx/src

# Build the application
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release -p api-ltx && \
    cp /app/target/release/api-ltx /app/api-ltx

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libpq5 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install diesel_cli for running migrations
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Rust (minimal) and diesel_cli
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
ENV PATH="/root/.cargo/bin:${PATH}"
RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
    cargo install diesel_cli --no-default-features --features postgres

# Clean up build tools to reduce image size
RUN apt-get remove -y curl build-essential && \
    apt-get autoremove -y && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the built binary
COPY --from=builder /app/api-ltx /usr/local/bin/api-ltx

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
