# API-LTX Setup Guide

This guide will walk you through setting up the API web server with PostgreSQL database integration.

## Prerequisites

- Rust toolchain (cargo, rustc)
- PostgreSQL 12 or higher
- diesel_cli (installation covered below)

## 1. Install PostgreSQL

### macOS (using Homebrew)
```bash
brew install postgresql@15
brew services start postgresql@15
```

### Ubuntu/Debian
```bash
sudo apt update
sudo apt install postgresql postgresql-contrib
sudo systemctl start postgresql
sudo systemctl enable postgresql
```

### Arch Linux
```bash
sudo pacman -S postgresql
sudo systemctl start postgresql
sudo systemctl enable postgresql
```

## 2. Set Up PostgreSQL Database

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

## 3. Install Diesel CLI

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

## 4. Configure Environment Variables

The `.env` file is already created in the `src/api_ltx/` directory with default values:

```env
DATABASE_URL=postgres://ltx_user:ltx_password@localhost/ltx_db
```

If you used different credentials in step 2, update the `.env` file accordingly.

## 5. Run Database Migrations

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

## 6. Build and Run the Server

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

## 7. Test the API Endpoints

### Test the Hello endpoint

```bash
curl http://localhost:3000/hello
```

Expected output:
```
Hello world!
```

### Add a name to the database

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

### Fetch all names from the database

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

## Troubleshooting

### Connection refused errors

Ensure PostgreSQL is running:
```bash
# macOS
brew services list

# Linux
sudo systemctl status postgresql
```

### Migration errors

If migrations fail, you can reset the database:
```bash
diesel migration redo
```

### Diesel CLI not found

Ensure `~/.cargo/bin` is in your PATH:
```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

### Port already in use

If port 3000 is already in use, you can change it in `src/main.rs`:
```rust
let addr = SocketAddr::from(([127, 0, 0, 1], 8080)); // Change 3000 to 8080
```

## Development Tips

### Running with auto-reload

Install `cargo-watch` for automatic recompilation:
```bash
cargo install cargo-watch
cargo watch -x 'run -p api-ltx'
```

### Viewing database contents directly

```bash
psql -U ltx_user -d ltx_db
\dt              # List tables
SELECT * FROM names;  # View all names
\q              # Quit
```

### Resetting the database

To start fresh:
```bash
diesel migration revert --all
diesel migration run
```
