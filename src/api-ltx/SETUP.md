# API Server

This guide will walk you through setting up the API web server with PostgreSQL database integration.

## Quick Start with Docker Compose (Recommended)

The easiest way to run the API server and database is using Docker Compose.

### Prerequisites

- [Docker](https://docs.docker.com/get-docker/)
- [Docker Compose](https://docs.docker.com/compose/install/) (usually included with Docker Desktop)

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

### Install PostgreSQL

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

### Set Up PostgreSQL Database

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

### Install Diesel CLI

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

### Run Database Migrations

Navigate to the api-ltx directory and run migrations:

```bash
cd src/api-ltx
diesel migration run
```

This will create the `names` table with a single `name` column.

To verify the migration worked:

```bash
diesel migration list
```


## Configuration

The API server can be configured using environment variables:

- `DATABASE_URL`: PostgreSQL connection string (required)
- `HOST`: Host to bind to (default: `127.0.0.1`, use `0.0.0.0` for Docker)
- `PORT`: Port to listen on (default: `3000`)
- `RUST_LOG`: Logging level (default: `api-ltx=debug,tower_http=debug`)

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
