#!/usr/bin/env bash

WAIT_S="${WAIT_S:-1}"
MAX_RETRIES="${MAX_RETRIES:-30}"

set -euo pipefail

# Verify DATABASE_URL is set
if [ -z "${DATABASE_URL:-}" ]; then
  echo "ERROR: DATABASE_URL environment variable is not set"
  exit 1
fi

echo "DATABASE_URL is set: ${DATABASE_URL}"

# Extract database host and port from DATABASE_URL
# Format: postgres://user:pass@host:port/database
DB_HOST=$(echo "$DATABASE_URL" | sed -E 's|.*@([^:/]+).*|\1|')
DB_PORT=$(echo "$DATABASE_URL" | sed -E 's|.*:([0-9]+)/.*|\1|')

echo "Parsed database connection: host=${DB_HOST}, port=${DB_PORT}"

# Wait for database to accept connections
echo "Waiting for database to be ready..."
RETRIES=0
until pg_isready -h "${DB_HOST}" -p "${DB_PORT}" -U ltx_user -d ltx_db >/dev/null 2>&1; do
  RETRIES=$((RETRIES + 1))
  if [ $RETRIES -ge $MAX_RETRIES ]; then
    echo "ERROR: Database did not become ready after ${MAX_RETRIES} attempts"
    echo "Attempting to show pg_isready output:"
    pg_isready -h "${DB_HOST}" -p "${DB_PORT}" -U ltx_user -d ltx_db || true
    exit 1
  fi
  echo "Database is unavailable - sleeping (${WAIT_S}s) [attempt ${RETRIES}/${MAX_RETRIES}]"
  sleep ${WAIT_S}
done

echo "Database is accepting connections!"

# Run sqlx migrations
echo "Running database migrations with sqlx..."
if sqlx migrate run; then
  echo "Migrations completed successfully"
else
  echo "ERROR: Failed to run migrations"
  echo "Attempting to show migration status:"
  sqlx migrate info || true
  exit 1
fi

echo "Database is ready!"
echo "Starting API server..."
exec api-ltx
