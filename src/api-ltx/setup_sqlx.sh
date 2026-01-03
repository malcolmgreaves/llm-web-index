#!/bin/bash
set -e

echo "Starting PostgreSQL if not running..."
if ! pg_isready -h localhost -p 5432 >/dev/null 2>&1; then
    echo "Please start PostgreSQL first."
    exit 1
fi

echo "Creating database user and database..."
psql postgres -c "CREATE USER ltx_user WITH PASSWORD 'ltx_password';" 2>/dev/null || echo "User already exists"
psql postgres -c "CREATE DATABASE ltx_db OWNER ltx_user;" 2>/dev/null || echo "Database already exists"

echo "Running migrations..."
cd /Users/malcolmgreaves/.superset/worktrees/llm-web-index/sqlx
sqlx migrate run

echo "Generating query cache for offline builds..."
cargo sqlx prepare --workspace

echo "Done! The .sqlx directory has been created for offline Docker builds."
