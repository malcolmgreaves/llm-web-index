# llm-web-index

A comprehensive system for generating and maintaining [llms.txt](http://llmstxt.org) files from websites. Provides automated generation via LLM models with a full-stack web application, periodic updates, and production-grade authentication.

## Overview

llm-web-index is a complete, production-ready system built entirely in Rust that:

- **Generates llms.txt files** from any website using OpenAI's GPT-5.2 model
- **Provides a web interface** for submitting URLs and viewing results
- **Automatically updates** llms.txt files on a configurable schedule
- **Handles authentication** with password-based sessions for production deployments
- **Uses HTTPS/TLS** for all connections with automatic certificate generation
- **Scales efficiently** with async Rust, connection pooling, and worker architecture

The system consists of:
- **WASM Frontend**: Rust-compiled WebAssembly UI served as a static web app
- **API Server**: RESTful API with authentication, database integration, and TLS
- **Worker Service**: Background processor for llms.txt generation
- **Cron Service**: Periodic updater for keeping llms.txt files current
- **Core Library**: Shared business logic and utilities

All components run in Docker containers for easy deployment, or can be run manually for development.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         User's Browser                           │
│                    (WASM Frontend - Rust)                        │
└────────────────────────┬────────────────────────────────────────┘
                         │ HTTPS/TLS
                         ↓
┌─────────────────────────────────────────────────────────────────┐
│                       API Server (api-ltx)                       │
│  - RESTful API (Axum)                                           │
│  - Authentication & Sessions                                     │
│  - Static file serving (WASM assets)                            │
│  - TLS/HTTPS (rustls)                                           │
└───────┬─────────────────────────────────────┬───────────────────┘
        │                                     │
        │ PostgreSQL                          │ PostgreSQL
        ↓                                     ↓
┌──────────────────┐                  ┌──────────────────┐
│   Worker Service │                  │   Cron Service   │
│   (worker-ltx)   │                  │   (cron-ltx)     │
│                  │                  │                  │
│ - Polls for jobs │                  │ - Checks for     │
│ - Generates      │                  │   updates needed │
│   llms.txt       │                  │ - Creates jobs   │
│ - Updates DB     │                  │   periodically   │
└────────┬─────────┘                  └──────────────────┘
         │
         │ Uses core-ltx
         ↓
┌─────────────────────────────────────────────────────────────────┐
│                     Core Library (core-ltx)                      │
│  - Web content fetching and parsing                             │
│  - LLM integration (OpenAI GPT)                                 │
│  - llms.txt generation and validation                           │
│  - Common utilities (auth, TLS, config)                         │
└─────────────────────────────────────────────────────────────────┘
```

## Project Structure

- [`api-ltx`](src/api-ltx): API webserver with authentication, TLS, and database integration
- [`core-ltx`](src/core-ltx): Functional core containing all llms.txt generation logic + CLI tool
- [`cron-ltx`](src/cron-ltx): Updater service to periodically update websites' llms.txt files
- [`data-model-ltx`](src/data-model-ltx): Database models, schema, and CRUD operations
- [`front-ltx`](src/front-ltx): WASM frontend for browser-based user interface
- [`worker-ltx`](src/worker-ltx): Background worker service for processing generation jobs

Each crate has its own detailed README with specific documentation.

## Quick Start

### Prerequisites

Before you begin, ensure you have these installed:

#### Required for Docker Deployment

- **Docker**: Container runtime
  - macOS: [Docker Desktop](https://docs.docker.com/desktop/install/mac-install/)
  - Linux: [Docker Engine](https://docs.docker.com/engine/install/)
  - Windows: [Docker Desktop](https://docs.docker.com/desktop/install/windows-install/)

- **Docker Compose**: Multi-container orchestration (usually included with Docker Desktop)
  - Verify: `docker compose version`

- **OpenAI API Key**: Required for llms.txt generation
  - Get one at: https://platform.openai.com/api-keys
  - Requires access to GPT-5.2, GPT-5-mini, or GPT-5-nano

#### Required for Local Development

All of the above, plus:

- **Rust Toolchain**: Latest stable Rust compiler and cargo
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
  - Verify: `cargo --version` (should be 1.70.0 or higher)

- **WASM Target**: For compiling the frontend
  ```bash
  rustup target add wasm32-unknown-unknown
  ```

- **wasm-bindgen-cli**: For generating JavaScript bindings
  ```bash
  cargo install wasm-bindgen-cli
  ```

- **just**: Command runner for project tasks
  ```bash
  # macOS
  brew install just

  # Linux
  cargo install just

  # Windows
  cargo install just
  ```
  - Verify: `just --version`

- **PostgreSQL 15+**: Database server
  ```bash
  # macOS
  brew install postgresql@15
  brew services start postgresql@15

  # Linux (Ubuntu/Debian)
  sudo apt update && sudo apt install postgresql postgresql-contrib
  sudo systemctl start postgresql

  # Linux (Arch)
  sudo pacman -S postgresql
  sudo systemctl start postgresql
  ```

- **diesel_cli**: Database migration tool
  ```bash
  cargo install diesel_cli --no-default-features --features postgres
  ```

- **pre-commit**: Git hooks for code quality (optional but recommended)
  ```bash
  # macOS
  brew install pre-commit

  # Linux/macOS with pip
  pip install pre-commit

  # Setup hooks
  pre-commit install
  ```

#### Optional but Recommended

- **binaryen**: For optimizing WASM in production builds
  ```bash
  # macOS
  brew install binaryen

  # Linux
  sudo apt install binaryen    # Ubuntu/Debian
  sudo pacman -S binaryen      # Arch
  ```

- **mkcert**: For locally-trusted TLS certificates (avoids browser warnings)
  ```bash
  # macOS
  brew install mkcert
  mkcert -install

  # Linux
  # See: https://github.com/FiloSottile/mkcert#installation
  ```

- **cargo-watch**: Auto-rebuild on file changes
  ```bash
  cargo install cargo-watch
  ```

### Running with Docker Compose (Recommended for First-Time Users)

This is the fastest way to get the entire system running:

```bash
# 1. Clone the repository
git clone <repository-url>
cd llm-web-index

# 2. Set your OpenAI API key
export OPENAI_API_KEY='sk-...'

# 3. Start all services
docker compose up

# Or run in detached mode (background)
docker compose up -d
```

The system will be available at **https://localhost:3000**

Services started:
- PostgreSQL database (port 5432)
- API server with frontend (port 3000, HTTPS)
- Worker service (processing jobs)
- Cron service (scheduling updates)

To view logs:
```bash
# All services
docker compose logs -f

# Specific service
docker compose logs -f api
docker compose logs -f worker
docker compose logs -f cron
```

To stop:
```bash
docker compose down

# To also remove database volumes (fresh start)
docker compose down -v
```

### Enabling Authentication (Production Mode)

For production deployments, enable password-based authentication:

```bash
# 1. Generate authentication credentials
export AUTH_PASSWORD='your_secure_password_here'
source ./make_password_and_export_env.sh "$AUTH_PASSWORD"

# This sets:
#   - AUTH_PASSWORD (plain text, for cron service)
#   - AUTH_PASSWORD_HASH (bcrypt hash, for API server)
#   - SESSION_SECRET (HMAC key for session tokens)

# 2. Enable authentication
export ENABLE_AUTH=1

# 3. Generate TLS certificates (if not already done)
./make_tls_cert.sh ./certs

# 4. Start with authentication enabled
docker compose up
```

Now accessing https://localhost:3000 will show a login page.

### Setting Up TLS Certificates

The system requires HTTPS. You have two options:

#### Option 1: Self-Signed Certificate (Development)

```bash
# Generate self-signed certificate
./make_tls_cert.sh ./certs

# Or use the Rust binary directly
cargo run --bin generate-tls-cert -- ./certs

# For development, accept invalid certs
export ACCEPT_INVALID_CERTS=true
```

Your browser will show a security warning. Click "Advanced" → "Proceed" to continue.

#### Option 2: Locally-Trusted Certificate (Development, Recommended)

```bash
# Install mkcert (see prerequisites above)
brew install mkcert
mkcert -install

# Generate locally-trusted certificate
./make_tls_cert.sh ./certs

# This automatically uses mkcert if available
# No browser warnings!
```

#### Option 3: CA-Signed Certificate (Production)

For production, use a certificate from a trusted CA (Let's Encrypt, etc.):

```bash
# Place your certificate files in ./certs/
cp /path/to/your/cert.pem ./certs/
cp /path/to/your/key.pem ./certs/

# Update environment variables
export TLS_CERT_PATH=./certs/cert.pem
export TLS_KEY_PATH=./certs/key.pem

# Make sure to unset or set to false
export ACCEPT_INVALID_CERTS=false
```

## Development

### First-Time Setup

```bash
# 1. Clone and enter the repository
git clone <repository-url>
cd llm-web-index

# 2. Copy environment template
cp .env.example .env

# 3. Edit .env with your settings
# At minimum, set OPENAI_API_KEY
vim .env  # or nano, code, etc.

# 4. Set up pre-commit hooks (recommended)
pre-commit install

# 5. Set up the database
# Create PostgreSQL database and user
createdb ltx_db
psql ltx_db -c "CREATE USER ltx_user WITH PASSWORD 'ltx_password';"
psql ltx_db -c "GRANT ALL PRIVILEGES ON DATABASE ltx_db TO ltx_user;"

# Run database migrations
cd src/api-ltx
diesel migration run
cd ../..

# 6. Generate TLS certificates
./make_tls_cert.sh ./certs

# 7. Build the frontend
just front

# 8. Build all services
just build
```

### Development Workflow

#### Running All Services

```bash
# Option 1: Use Docker Compose (recommended)
docker compose up

# Option 2: Run manually with cargo
# Terminal 1: API server
just serve

# Terminal 2: Worker
cargo run -p worker-ltx

# Terminal 3: Cron service
cargo run -p cron-ltx
```

#### Frontend Development

```bash
# Build frontend once
just front

# Or watch for changes (in separate terminal)
cd src/front-ltx
cargo watch -i www/ -s "just front"

# Or use the manual commands
cargo build --target wasm32-unknown-unknown --release
wasm-bindgen ../../target/wasm32-unknown-unknown/release/front_ltx.wasm \
  --out-dir www/pkg --target web
```

#### Backend Development

```bash
# Auto-rebuild and restart on changes
cargo watch -x 'run -p api-ltx'
cargo watch -x 'run -p worker-ltx'
cargo watch -x 'run -p cron-ltx'
```

### Just Commands Reference

Use `just -l` to see all available commands. Key commands:

```bash
# Development
just check          # Check all code compiles
just build          # Build all services + frontend
just front          # Build WASM frontend only
just serve          # Build and run API server

# Testing
just test           # Run tests with coverage report (generates HTML)
just bench          # Run benchmarks

# Code Quality
just fmt            # Format all code with rustfmt
just tidy           # Run clippy with auto-fixes + cargo machete
just ci             # Run full CI suite (tidy + check + test + bench)

# Cleanup
just clean          # Remove all build artifacts and WASM output

# Production
just release        # Optimized build with WASM optimization
```

### Cargo Commands Reference

Direct cargo commands for specific crates:

```bash
# Build specific crates
cargo build -p api-ltx
cargo build -p worker-ltx
cargo build -p cron-ltx
cargo build -p core-ltx
cargo build -p front-ltx --target wasm32-unknown-unknown

# Run specific services
cargo run -p api-ltx
cargo run -p worker-ltx
cargo run -p cron-ltx
cargo run -p core-ltx -- generate https://example.com

# Test specific crates
cargo test -p api-ltx
cargo test -p worker-ltx
cargo test -p core-ltx

# Test workspace
cargo test --workspace

# Generate test coverage (HTML report in target/llvm-cov/html/)
cargo llvm-cov --all-targets --workspace --html

# Run benchmarks
cargo bench --all-targets --workspace

# Check without building
cargo check --all-targets --workspace

# Clippy linting
cargo clippy --all-targets --workspace

# Format code
cargo fmt --all

# Build for production
cargo build --release --all-targets --workspace
```

### Database Management

```bash
# Create a new migration
cd src/api-ltx
diesel migration generate migration_name

# Run pending migrations
diesel migration run

# Revert last migration
diesel migration revert

# List migration status
diesel migration list

# Regenerate schema.rs after migrations
diesel print-schema > ../data-model-ltx/src/schema.rs

# Connect to database
psql -h localhost -U ltx_user -d ltx_db

# Or via Docker
docker compose exec postgres psql -U ltx_user -d ltx_db
```

### Using the CLI Tool

The `core-ltx` crate includes a CLI for standalone generation:

```bash
# Generate llms.txt for a website
cargo run -p core-ltx -- generate https://example.com

# Save to file
cargo run -p core-ltx -- generate https://example.com --output example.txt

# Use a different model
cargo run -p core-ltx -- generate https://example.com --model gpt-5-mini

# Update an existing file
cargo run -p core-ltx -- update https://example.com --existing old.txt

# View help
cargo run -p core-ltx -- --help
```

## Authentication Setup (Detailed)

### Password and Session Token Generation

The system uses three authentication components:

1. **AUTH_PASSWORD**: Plain text password (used by cron service)
2. **AUTH_PASSWORD_HASH**: Bcrypt hash (used by API server)
3. **SESSION_SECRET**: HMAC signing key (used for session tokens)

#### Automatic Generation (Recommended)

```bash
# Generate all three at once
source ./make_password_and_export_env.sh 'your_password_here'

# This exports:
#   AUTH_PASSWORD='your_password_here'
#   AUTH_PASSWORD_HASH='$2b$12$...'
#   SESSION_SECRET='base64-encoded-random-bytes'
```

The script:
- Rejects weak passwords ('password', 'test_password')
- Uses bcrypt with cost=12 for password hashing
- Generates a cryptographically secure 32-byte session secret
- Exports all variables to your current shell

#### Manual Generation

```bash
# 1. Generate password hash
cargo run --bin generate-password-hash -- 'your_password_here'
# Output: $2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYWNGZqKzRu

# 2. Generate session secret
openssl rand -base64 32
# Output: dGhpc2lzYXNlY3JldGtleWZvcnNpZ25pbmdz

# 3. Set environment variables
export AUTH_PASSWORD='your_password_here'
export AUTH_PASSWORD_HASH='$2b$12$...'
export SESSION_SECRET='dGhpc2lzYXNlY3JldGtleWZvcnNpZ25pbmdz'
```

#### Adding to .env File

```bash
# Add to .env for persistence
echo "ENABLE_AUTH=1" >> .env
echo "AUTH_PASSWORD='your_password_here'" >> .env
echo "AUTH_PASSWORD_HASH='$2b$12$...'" >> .env
echo "SESSION_SECRET='dGhpc2lzYXNlY3JldGtleWZvcnNpZ25pbmdz'" >> .env
```

### TLS Certificate Generation (Detailed)

#### Using the Convenience Script

```bash
# Generate in ./certs directory
./make_tls_cert.sh ./certs

# Generate in custom directory
./make_tls_cert.sh /path/to/certs

# The script automatically:
# - Uses mkcert if available (trusted by browser)
# - Falls back to self-signed certificate
# - Creates cert.pem and key.pem
# - Exports TLS_CERT_PATH and TLS_KEY_PATH
```

#### Using the Rust Binary

```bash
# With default SANs (localhost, 127.0.0.1, 0.0.0.0)
cargo run --bin generate-tls-cert -- ./certs

# With custom SANs
cargo run --bin generate-tls-cert -- ./certs localhost 127.0.0.1 myapp.local

# Output:
#   ./certs/cert.pem
#   ./certs/key.pem
```

#### Using mkcert (Recommended for Development)

```bash
# Install and setup mkcert
brew install mkcert
mkcert -install

# Generate certificate
cd certs
mkcert -cert-file cert.pem -key-file key.pem localhost 127.0.0.1 ::1 0.0.0.0
cd ..

# Set environment variables
export TLS_CERT_PATH=./certs/cert.pem
export TLS_KEY_PATH=./certs/key.pem
```

#### Production Certificates (Let's Encrypt)

For production with a real domain:

```bash
# Install certbot
brew install certbot  # macOS
sudo apt install certbot  # Linux

# Get certificate (requires domain pointed to your server)
sudo certbot certonly --standalone -d yourdomain.com

# Certificates will be in:
#   /etc/letsencrypt/live/yourdomain.com/fullchain.pem
#   /etc/letsencrypt/live/yourdomain.com/privkey.pem

# Configure in .env or docker-compose.yml
export TLS_CERT_PATH=/etc/letsencrypt/live/yourdomain.com/fullchain.pem
export TLS_KEY_PATH=/etc/letsencrypt/live/yourdomain.com/privkey.pem
```

### Session Configuration

```bash
# Session duration (default: 24 hours)
export SESSION_DURATION_SECONDS=86400

# For shorter sessions (1 hour)
export SESSION_DURATION_SECONDS=3600

# For longer sessions (7 days)
export SESSION_DURATION_SECONDS=604800
```

## Docker Compose Commands

### Development

```bash
# Start all services (attached, see logs)
docker compose up

# Start in background (detached)
docker compose up -d

# Start specific services
docker compose up api worker

# View logs
docker compose logs -f           # All services
docker compose logs -f api       # API server only
docker compose logs -f worker    # Worker only
docker compose logs -f cron      # Cron only

# Stop services (keep volumes)
docker compose stop

# Stop and remove containers (keep volumes)
docker compose down

# Stop and remove everything including volumes (fresh start)
docker compose down -v

# Rebuild containers after code changes
docker compose build

# Rebuild without cache (clean build)
docker compose build --no-cache

# Rebuild and restart
docker compose up --build

# Run a command in a running container
docker compose exec api sh
docker compose exec postgres psql -U ltx_user -d ltx_db

# View resource usage
docker compose stats
```

### Production Deployment

```bash
# 1. Set production environment variables
export ENABLE_AUTH=1
export OPENAI_API_KEY='sk-...'
source ./make_password_and_export_env.sh 'strong_password'

# 2. Use production TLS certificates
export TLS_CERT_PATH=/etc/letsencrypt/live/yourdomain.com/fullchain.pem
export TLS_KEY_PATH=/etc/letsencrypt/live/yourdomain.com/privkey.pem

# 3. Build for production
docker compose build

# 4. Start services in background
docker compose up -d

# 5. Monitor logs
docker compose logs -f

# 6. Check service health
docker compose ps
curl https://yourdomain.com/health
```

### Database Queries via Docker

```bash
# Using the provided scripts (recommended)
./query_job_state.sh <job_id>
./query_llms_txt.sh <job_id>

# Or run SQL directly
docker compose exec postgres psql -U ltx_user -d ltx_db -c "SELECT * FROM jobs;"

# Interactive psql session
docker compose exec postgres psql -U ltx_user -d ltx_db
```

## Testing

### Running Tests

```bash
# All tests with coverage (HTML report)
just test

# All tests (simple)
cargo test --workspace

# Specific crate
cargo test -p api-ltx
cargo test -p worker-ltx
cargo test -p core-ltx

# Integration tests only
cargo test --test '*'

# Unit tests only
cargo test --lib

# Specific test
cargo test test_name

# Show test output (don't capture)
cargo test -- --nocapture

# Run ignored tests (e.g., requiring API keys)
OPENAI_API_KEY=sk-... cargo test -- --ignored
```

### Coverage Report

```bash
# Generate HTML coverage report
just test

# Or manually
cargo llvm-cov --all-targets --workspace --html

# View report
open target/llvm-cov/html/index.html  # macOS
xdg-open target/llvm-cov/html/index.html  # Linux
```

### Benchmarks

```bash
# Run all benchmarks
just bench

# Or manually
cargo bench --all-targets --workspace

# Specific benchmark
cargo bench --bench benchmark_name

# Save baseline for comparison
cargo bench -- --save-baseline main
```

## Troubleshooting

### Common Issues

#### Port Already in Use

```bash
# Check what's using port 3000
lsof -i :3000  # macOS/Linux
netstat -ano | findstr :3000  # Windows

# Change ports in docker-compose.yml or .env
PORT=3001
```

#### Database Connection Failed

```bash
# Check PostgreSQL is running
brew services list  # macOS
sudo systemctl status postgresql  # Linux

# Check connection string
echo $DATABASE_URL

# Test connection
psql -h localhost -U ltx_user -d ltx_db

# Reset database
docker compose down -v
docker compose up -d postgres
cd src/api-ltx && diesel migration run && cd ../..
```

#### TLS Certificate Errors

```bash
# For development, accept invalid certificates
export ACCEPT_INVALID_CERTS=true

# Or regenerate certificates
./make_tls_cert.sh ./certs

# Or use mkcert for trusted certificates
brew install mkcert
mkcert -install
./make_tls_cert.sh ./certs
```

#### OpenAI API Key Issues

```bash
# Verify key is set
echo $OPENAI_API_KEY

# Test key validity
curl https://api.openai.com/v1/models \
  -H "Authorization: Bearer $OPENAI_API_KEY"

# Check worker logs for errors
docker compose logs worker
```

#### WASM Build Failures

```bash
# Install WASM target
rustup target add wasm32-unknown-unknown

# Install wasm-bindgen-cli
cargo install wasm-bindgen-cli

# Clean and rebuild
just clean
just front
```

#### Authentication Not Working

```bash
# Verify auth is enabled
echo $ENABLE_AUTH  # Should be 1

# Check password hash
echo $AUTH_PASSWORD_HASH  # Should start with $2b$

# Check session secret
echo $SESSION_SECRET  # Should be base64 string

# Regenerate if needed
source ./make_password_and_export_env.sh 'your_password'
```

### Getting Help

- Check crate-specific READMEs in `src/*/README.md`
- Review application logs for error messages
- Check GitHub issues (if project is on GitHub)
- Enable debug logging: `export RUST_LOG=debug`

## Production Deployment Checklist

- [ ] Set `ENABLE_AUTH=1`
- [ ] Use strong password (not 'password' or 'test_password')
- [ ] Generate secure session secret (`openssl rand -base64 32`)
- [ ] Use CA-signed TLS certificates (Let's Encrypt)
- [ ] Set `ACCEPT_INVALID_CERTS=false` or unset it
- [ ] Use `RUST_LOG=info` (not debug/trace)
- [ ] Configure firewall to only allow ports 80, 443, 5432 (if needed)
- [ ] Set up database backups
- [ ] Configure log rotation
- [ ] Set up monitoring and alerting
- [ ] Run multiple worker instances for redundancy
- [ ] Use a reverse proxy (nginx, Caddy) if needed
- [ ] Set appropriate session duration
- [ ] Review and harden PostgreSQL configuration
- [ ] Regularly update dependencies and Docker images

## Design Philosophy

This project follows the "functional core, effectful shell" pattern:

- **Functional core**: Pure business logic in `core-ltx`
- **Effectful shell**: I/O and state in service crates (api, worker, cron)

Key principles:
- Immutability by default
- Explicit error handling with `Result` types
- No `.unwrap()` or `.expect()` (except in tests)
- Comprehensive error enums with context
- Type-safe database operations via Diesel
- Async-first with tokio
- Extensive testing with coverage reports

See [CLAUDE.md](CLAUDE.md) for detailed coding standards.

## Contributing

### Code Quality Standards

Before submitting PRs:

```bash
# Format code
just fmt

# Run linter with auto-fixes
just tidy

# Run full CI suite
just ci
```

### Pull Request Process

1. Create a feature branch from `main`
2. Make your changes following the coding standards
3. Add tests for new functionality
4. Run `just ci` and ensure it passes
5. Update relevant documentation
6. Submit PR with clear description

### Git Hooks

Set up pre-commit hooks:

```bash
pre-commit install
```

This automatically runs formatting, linting, and basic checks before each commit.

## License

Mozilla Public License 2.0

See [LICENSE](LICENSE) file for details.

## Related Documentation

- [api-ltx](src/api-ltx/README.md) - API server documentation
- [core-ltx](src/core-ltx/README.md) - Core library and CLI tool
- [worker-ltx](src/worker-ltx/README.md) - Worker service documentation
- [cron-ltx](src/cron-ltx/README.md) - Cron service documentation
- [front-ltx](src/front-ltx/README.md) - WASM frontend documentation
- [data-model-ltx](src/data-model-ltx/README.md) - Database models documentation
- [llmstxt.org](https://llmstxt.org) - llms.txt specification

## Additional Resources

- **Rust**: https://www.rust-lang.org/
- **Axum**: https://github.com/tokio-rs/axum
- **Diesel**: https://diesel.rs/
- **WebAssembly**: https://webassembly.org/
- **Docker**: https://docs.docker.com/
- **PostgreSQL**: https://www.postgresql.org/
