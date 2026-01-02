# API-LTX Setup Guide

This guide will walk you through setting up the API web server with PostgreSQL database integration.

## Quick Start with Docker Compose (Recommended)

The easiest way to run the API server and database is using Docker Compose.

### Prerequisites

- [Docker](https://docs.docker.com/get-docker/)
- [Docker Compose](https://docs.docker.com/compose/install/) (usually included with Docker Desktop)

### Running with Docker Compose

From the **workspace root directory** (not `src/api_ltx`), run:

```bash
# Enable BuildKit for faster builds with cache mounts (recommended)
export DOCKER_BUILDKIT=1

# Start both the database and API server
docker compose up

# Or run in detached mode (background)
docker compose up -d

# View logs
docker compose logs -f

# Stop the services
docker compose down

# Stop and remove volumes (resets database)
docker compose down -v
```

The API server will be available at `http://localhost:3000`.

**Note:** Setting `DOCKER_BUILDKIT=1` enables BuildKit, which uses cache mounts to significantly speed up Rust compilation by caching the Cargo registry, Git dependencies, and build artifacts across builds.

### What Docker Compose Does

The `docker-compose.yml` file automatically:
- Starts a PostgreSQL 15 database container
- Creates the database, user, and password
- Builds and starts the API server container
- Uses BuildKit cache mounts for faster Rust compilation (caches Cargo registry, Git deps, and build artifacts)
- Runs database migrations automatically on startup
- Sets up networking between the containers
- Persists database data in a Docker volume

### Testing the API

Once the services are running, you can test the endpoints:

```bash
# Test the Hello endpoint
curl http://localhost:3000/hello

# Add a name to the database
curl -X POST http://localhost:3000/add \
  -H "Content-Type: application/json" \
  -d '{"name": "Alice"}'

# Fetch all names from the database
curl http://localhost:3000/fetch
```

### Rebuilding After Code Changes

If you make changes to the code, rebuild the API container:

```bash
# Rebuild and restart the API service (with BuildKit cache for faster builds)
DOCKER_BUILDKIT=1 docker compose up --build api

# Or rebuild all services
DOCKER_BUILDKIT=1 docker compose up --build
```

The BuildKit cache mounts will significantly speed up subsequent builds by reusing downloaded crates and compiled dependencies.

### Accessing the Database

To connect to the PostgreSQL database running in Docker:

```bash
# Using docker compose exec
docker compose exec postgres psql -U ltx_user -d ltx_db

# Or using psql on your host (if installed)
psql -h localhost -U ltx_user -d ltx_db
# Password: ltx_password
```

---

## Manual Setup (Alternative)

If you prefer to run services manually without Docker, follow the instructions below.

### Prerequisites

- Rust toolchain (cargo, rustc)
- PostgreSQL 12 or higher
- diesel_cli (installation covered below)

### 1. Install PostgreSQL

#### macOS (using Homebrew)
```bash
brew install postgresql@15
brew services start postgresql@15
```

#### Ubuntu/Debian
```bash
sudo apt update
sudo apt install postgresql postgresql-contrib
sudo systemctl start postgresql
sudo systemctl enable postgresql
```

#### Arch Linux
```bash
sudo pacman -S postgresql
sudo systemctl start postgresql
sudo systemctl enable postgresql
```

### 2. Set Up PostgreSQL Database

Create the database and user:

```bash
# Connect to PostgreSQL as the postgres user
sudo -u postgres psql

# Run these commands in the PostgreSQL prompt:
CREATE USER ltx_user WITH PASSWORD 'ltx_password';
CREATE DATABASE ltx_db OWNER ltx_user;
GRANT ALL PRIVILEGES ON DATABASE ltx_db TO ltx_user;
\q
```

**Note:** For production environments, use a strong password and consider using environment-specific credentials.

### 3. Install Diesel CLI

Install the Diesel CLI tool with PostgreSQL support:

```bash
cargo install diesel_cli --no-default-features --features postgres
```

If you encounter linking errors on macOS, you may need to set the PostgreSQL library path:

```bash
# For Homebrew PostgreSQL
export PQ_LIB_DIR="$(brew --prefix postgresql@15)/lib"
cargo install diesel_cli --no-default-features --features postgres
```

### 4. Configure Environment Variables

The `.env` file is already created in the `src/api_ltx/` directory with default values:

```env
DATABASE_URL=postgres://ltx_user:ltx_password@localhost/ltx_db
```

If you used different credentials in step 2, update the `.env` file accordingly.

### 5. Run Database Migrations

Navigate to the api_ltx directory and run migrations:

```bash
cd src/api_ltx
diesel migration run
```

This will create the `names` table with a single `name` column.

To verify the migration worked:

```bash
diesel migration list
```

### 6. Build and Run the Server

From the `src/api_ltx` directory:

```bash
cargo run
```

Or from the workspace root:

```bash
cargo run -p api-ltx
```

The server will start on `http://127.0.0.1:3000`

You should see output like:
```
2024-01-02T12:00:00.000000Z  INFO api_ltx: Listening on 127.0.0.1:3000
```

### 7. Test the API Endpoints

#### Test the Hello endpoint

```bash
curl http://localhost:3000/hello
```

Expected output:
```
Hello world!
```

#### Add a name to the database

```bash
curl -X POST http://localhost:3000/add \
  -H "Content-Type: application/json" \
  -d '{"name": "Alice"}'
```

Expected output:
```json
{
  "id": 1,
  "name": "Alice",
  "message": "Name added successfully"
}
```

Add more names:
```bash
curl -X POST http://localhost:3000/add \
  -H "Content-Type: application/json" \
  -d '{"name": "Bob"}'

curl -X POST http://localhost:3000/add \
  -H "Content-Type: application/json" \
  -d '{"name": "Charlie"}'
```

#### Fetch all names from the database

```bash
curl http://localhost:3000/fetch
```

Expected output:
```json
{
  "names": [
    {"id": 1, "name": "Alice"},
    {"id": 2, "name": "Bob"},
    {"id": 3, "name": "Charlie"}
  ],
  "count": 3
}
```

---

## Project Structure

```
src/api_ltx/
├── Cargo.toml              # Dependencies configuration
├── diesel.toml             # Diesel configuration
├── .env                    # Environment variables (DATABASE_URL)
├── SETUP.md               # This file
├── migrations/            # Database migrations
│   ├── 00000000000000_diesel_initial_setup/
│   │   ├── up.sql
│   │   └── down.sql
│   └── 2024-01-01-000000_create_names/
│       ├── up.sql         # CREATE TABLE names
│       └── down.sql       # DROP TABLE names
└── src/
    ├── main.rs            # Web server and route handlers
    ├── db.rs              # Database connection pool
    ├── models.rs          # Diesel models
    └── schema.rs          # Database schema (auto-generated)
```

---

## Configuration

The API server can be configured using environment variables:

- `DATABASE_URL`: PostgreSQL connection string (required)
- `HOST`: Host to bind to (default: `127.0.0.1`, use `0.0.0.0` for Docker)
- `PORT`: Port to listen on (default: `3000`)
- `RUST_LOG`: Logging level (default: `api_ltx=debug,tower_http=debug`)

---

## Troubleshooting

### Docker Compose Issues

#### Port already in use
If port 3000 or 5432 is already in use, you can change them in `docker-compose.yml`:

```yaml
services:
  postgres:
    ports:
      - "5433:5432"  # Change host port to 5433
  api:
    ports:
      - "8080:3000"  # Change host port to 8080
```

#### View container logs
```bash
docker compose logs api
docker compose logs postgres
```

#### Rebuild from scratch
```bash
docker compose down -v
docker compose build --no-cache
docker compose up
```

### Manual Setup Issues

#### Connection refused errors

Ensure PostgreSQL is running:
```bash
# macOS
brew services list

# Linux
sudo systemctl status postgresql
```

#### Migration errors

If migrations fail, you can reset the database:
```bash
diesel migration redo
```

#### Diesel CLI not found

Ensure `~/.cargo/bin` is in your PATH:
```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

---

## Development Tips

### Running with auto-reload

Install `cargo-watch` for automatic recompilation:
```bash
cargo install cargo-watch
cargo watch -x 'run -p api-ltx'
```

### Viewing database contents directly

Using Docker:
```bash
docker compose exec postgres psql -U ltx_user -d ltx_db
\dt              # List tables
SELECT * FROM names;  # View all names
\q              # Quit
```

Using local PostgreSQL:
```bash
psql -U ltx_user -d ltx_db
\dt              # List tables
SELECT * FROM names;  # View all names
\q              # Quit
```

### Resetting the database

Using Docker:
```bash
docker compose down -v  # Removes volumes
docker compose up
```

Using manual setup:
```bash
diesel migration revert --all
diesel migration run
```
