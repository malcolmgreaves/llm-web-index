#!/usr/bin/env bash
set -euo pipefail

# Query the job_state table from the PostgreSQL database
# Set DOCKER_EXEC=1 (or "yes" or "y") to use docker exec instead of DATABASE_URL

# Check if DOCKER_EXEC is enabled
if [[ "${DOCKER_EXEC:-}" == "1" || "${DOCKER_EXEC:-}" == "yes" || "${DOCKER_EXEC:-}" == "y" ]]; then
    CONTAINER_NAME="${POSTGRES_CONTAINER:-ltx_postgres}"
    DB_NAME="${POSTGRES_DB:-ltx_db}"
    DB_USER="${POSTGRES_USER:-ltx_user}"

    # Check if container is running
    if ! docker ps --filter "name=${CONTAINER_NAME}" --format "{{.Names}}" | grep -q "^${CONTAINER_NAME}$"; then
        echo "ERROR: PostgreSQL container '${CONTAINER_NAME}' is not running"
        echo "Start it with: docker compose up -d postgres"
        exit 1
    fi

    echo "Querying job_state table from container '${CONTAINER_NAME}'..."
    echo ""

    # Query the job_state table using docker exec
    docker exec "${CONTAINER_NAME}" psql -U "${DB_USER}" -d "${DB_NAME}" -c "SELECT * FROM job_state ORDER BY status, url;"
else
    # Original DATABASE_URL-based approach
    # Try to load DATABASE_URL from .env file if it exists and DATABASE_URL is not set
    if [ -z "${DATABASE_URL:-}" ] && [ -f "src/api-ltx/.env" ]; then
        export $(grep -v '^#' src/api-ltx/.env | grep DATABASE_URL | xargs)
    fi

    # Check if DATABASE_URL is set
    if [ -z "${DATABASE_URL:-}" ]; then
        echo "ERROR: DATABASE_URL environment variable is not set"
        echo "Please set it or create src/api-ltx/.env with DATABASE_URL"
        echo "Example: DATABASE_URL=postgres://ltx_user:ltx_password@localhost/ltx_db"
        echo ""
        echo "Tip: If you're using Docker, set DOCKER_EXEC=1 to connect via docker exec"
        exit 1
    fi

    echo "Querying job_state table..."
    echo "DATABASE_URL: ${DATABASE_URL}"
    echo ""

    # Query the full job_state table
    psql "${DATABASE_URL}" -c "SELECT * FROM job_state ORDER BY status, url;"
fi
