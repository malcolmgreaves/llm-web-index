#!/usr/bin/env bash
set -euo pipefail

# Query the job_state table from the PostgreSQL database
# This script reads DATABASE_URL from environment or .env file

# Try to load DATABASE_URL from .env file if it exists and DATABASE_URL is not set
if [ -z "${DATABASE_URL:-}" ] && [ -f "src/api-ltx/.env" ]; then
    export $(grep -v '^#' src/api-ltx/.env | grep DATABASE_URL | xargs)
fi

# Check if DATABASE_URL is set
if [ -z "${DATABASE_URL:-}" ]; then
    echo "ERROR: DATABASE_URL environment variable is not set"
    echo "Please set it or create src/api-ltx/.env with DATABASE_URL"
    echo "Example: DATABASE_URL=postgres://ltx_user:ltx_password@localhost/ltx_db"
    exit 1
fi

echo "Querying job_state table..."
echo "DATABASE_URL: ${DATABASE_URL}"
echo ""

# Query the full job_state table
psql "${DATABASE_URL}" -c "SELECT * FROM job_state ORDER BY status, url;"
