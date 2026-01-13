# api-ltx

API webserver for the llms.txt generation system. Provides a RESTful API for submitting website URLs, managing generation jobs, and serving the generated llms.txt files. Also includes the web frontend built with WASM.

## Overview

The `api-ltx` crate is the main entry point for the application, providing:

- **RESTful API**: Endpoints for creating and querying llms.txt generation jobs
- **Static file serving**: Serves the WASM-based frontend application
- **Database integration**: PostgreSQL connection pooling and schema migrations
- **Authentication**: Optional password-based authentication with session management
- **TLS/HTTPS**: Secure connections using rustls
- **Health checks**: Service readiness and liveness endpoints

## Architecture

```
src/api-ltx/
├── src/
│   ├── main.rs              # Application entry point, server initialization
│   ├── lib.rs               # Core library exports
│   ├── routes/              # API route handlers
│   │   ├── mod.rs           # Route definitions
│   │   ├── jobs.rs          # Job creation and status endpoints
│   │   └── health.rs        # Health check endpoints
│   ├── auth/                # Authentication system
│   │   ├── mod.rs           # Auth module exports
│   │   ├── handlers.rs      # Login/logout handlers
│   │   ├── middleware.rs    # Request authentication middleware
│   │   ├── password.rs      # Password hashing/verification
│   │   └── session.rs       # Session token management
│   ├── db.rs                # Database connection pooling
│   └── bin/                 # Utility binaries
│       ├── generate-password-hash.rs   # Generate bcrypt password hashes
│       └── generate-tls-cert.rs        # Generate self-signed TLS certificates
├── migrations/              # Database schema migrations
├── SETUP.md                 # Detailed setup instructions
└── Cargo.toml
```

## Key Features

### Authentication System

The API server supports optional password-based authentication:

- **Bcrypt password hashing**: Secure password storage with configurable cost
- **HMAC-signed sessions**: Tamper-proof session tokens using SHA-256
- **Configurable session duration**: Default 24 hours, customizable via env vars
- **Middleware protection**: Automatic authentication enforcement for protected routes

### TLS/HTTPS

All connections use TLS:

- **Rustls integration**: Modern, memory-safe TLS implementation
- **Self-signed certificates**: Built-in certificate generation for development
- **mkcert support**: Automatic browser trust via mkcert (optional)
- **Production-ready**: Supports proper CA-signed certificates

### Database

- **PostgreSQL**: Primary data store for jobs and generated content
- **Connection pooling**: Async connection pool via deadpool
- **Diesel ORM**: Type-safe database queries and migrations
- **Async operations**: Non-blocking database access with diesel-async

## Configuration

The API server is configured via environment variables. See `.env.example` in the project root for a complete reference.

### Core Settings

- `DATABASE_URL`: PostgreSQL connection string (required)
- `HOST`: Host to bind to (default: `0.0.0.0`)
- `PORT`: Port to listen on (default: `3000`)
- `RUST_LOG`: Logging level (default: `info`)

### Authentication Settings

Enable authentication by setting `ENABLE_AUTH=1`:

- `AUTH_PASSWORD_HASH`: Bcrypt hash of the password (required if auth enabled)
- `SESSION_SECRET`: Secret key for signing session tokens (required if auth enabled)
- `SESSION_DURATION_SECONDS`: Session lifetime (default: `86400` = 24 hours)

Generate these values using:
```bash
# Generate password hash
cargo run --bin generate-password-hash -- your_password_here

# Generate session secret
openssl rand -base64 32
```

Or use the convenience script:
```bash
source ./make_password_and_export_env.sh your_password_here
```

### TLS Settings

- `TLS_CERT_PATH`: Path to TLS certificate file (PEM format, required)
- `TLS_KEY_PATH`: Path to TLS private key file (PEM format, required)

Generate a self-signed certificate:
```bash
./make_tls_cert.sh ./certs
```

## Building

### Development Build

```bash
# Build the API server only
cargo build -p api-ltx

# Build with frontend (requires WASM toolchain)
just build
```

### Production Build

```bash
# Optimized build with WASM optimization
just release
```

## Running

### Using Docker Compose (Recommended)

```bash
# Development mode
export OPENAI_API_KEY='your_key_here'
docker compose up

# Production mode with authentication
export ENABLE_AUTH=1
export AUTH_PASSWORD='your_password'
source ./make_password_and_export_env.sh "$AUTH_PASSWORD"
docker compose up
```

The API will be available at `https://localhost:3000`.

### Manual Execution

Requires PostgreSQL running and configured. See [SETUP.md](SETUP.md) for database setup.

```bash
# Set required environment variables
export DATABASE_URL='postgresql://ltx_user:ltx_password@localhost/ltx_db'
export TLS_CERT_PATH='./certs/cert.pem'
export TLS_KEY_PATH='./certs/key.pem'

# Optional: Enable authentication
export ENABLE_AUTH=1
source ./make_password_and_export_env.sh your_password

# Run the server
cargo run -p api-ltx
```

Or use the just command:
```bash
just serve
```

## API Endpoints

### Public Endpoints (no authentication required)

- `GET /health` - Health check endpoint, returns 200 OK
- `GET /` - Serves the frontend application (index.html)
- `GET /pkg/*` - Serves WASM and JS assets

### Protected Endpoints (authentication required if enabled)

- `POST /api/jobs` - Create a new llms.txt generation job
  - Body: `{"url": "https://example.com"}`
  - Returns: Job ID and initial status

- `GET /api/jobs/:id` - Get job status and result
  - Returns: Job status (pending, in_progress, completed, failed) and generated content

- `GET /api/jobs/:id/llms-txt` - Download the generated llms.txt file
  - Returns: Plain text llms.txt content

### Authentication Endpoints (only available when auth is enabled)

- `POST /auth/login` - Login with password
  - Body: `{"password": "your_password"}`
  - Returns: Sets session cookie

- `POST /auth/logout` - Logout and invalidate session
  - Clears session cookie

## Testing

```bash
# Run unit tests
cargo test -p api-ltx

# Test with coverage report
just test
```

### Manual API Testing

```bash
# Health check
curl https://localhost:3000/health

# Create a job (without auth)
curl -X POST https://localhost:3000/api/jobs \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com"}'

# Login (with auth enabled)
curl -X POST https://localhost:3000/auth/login \
  -H "Content-Type: application/json" \
  -d '{"password": "your_password"}' \
  -c cookies.txt

# Create a job (with auth)
curl -X POST https://localhost:3000/api/jobs \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com"}' \
  -b cookies.txt

# Check job status
curl https://localhost:3000/api/jobs/{job_id} -b cookies.txt
```

## Database Migrations

Migrations are managed with Diesel CLI:

```bash
# Install diesel CLI
cargo install diesel_cli --no-default-features --features postgres

# Run pending migrations
cd src/api-ltx
diesel migration run

# Create a new migration
diesel migration generate migration_name

# Revert last migration
diesel migration revert
```

In Docker, migrations run automatically on container startup via the entrypoint script.

## Utility Binaries

### generate-password-hash

Generate a bcrypt hash for password authentication:

```bash
cargo run --bin generate-password-hash -- your_password_here
```

Output can be used as the `AUTH_PASSWORD_HASH` environment variable.

### generate-tls-cert

Generate a self-signed TLS certificate for development:

```bash
cargo run --bin generate-tls-cert -- ./certs
cargo run --bin generate-tls-cert -- ./certs localhost 127.0.0.1 myapp.local
```

Creates `cert.pem` and `key.pem` in the specified directory.

## Dependencies

Key dependencies:

- `axum`: Web framework
- `axum-server`: HTTPS server with rustls
- `tokio`: Async runtime
- `diesel` + `diesel-async`: Database ORM with async support
- `deadpool`: Connection pooling
- `bcrypt`: Password hashing
- `hmac` + `sha2`: Session token signing
- `rustls`: TLS implementation
- `tower-http`: HTTP middleware (tracing, CORS, static files)

See [Cargo.toml](Cargo.toml) for the complete dependency list.

## Troubleshooting

See [SETUP.md](SETUP.md) for detailed troubleshooting guides including:

- Database connection issues
- Port conflicts
- TLS certificate problems
- Docker Compose issues
- Migration failures

## Related Documentation

- [SETUP.md](SETUP.md) - Detailed database and development environment setup
- [Project Root README](../../README.md) - Overall project documentation
- [core-ltx README](../core-ltx/README.md) - llms.txt generation logic
- [front-ltx README](../front-ltx/README.md) - WASM frontend documentation
