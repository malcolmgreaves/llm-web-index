#!/usr/bin/env bash
#
# Run all services locally (api, worker, cron) with proper health checks
#
# This script:
# 1. Starts PostgreSQL via docker compose
# 2. Waits for postgres to be healthy
# 3. Runs database migrations
# 4. Starts api, waits for health
# 5. Starts worker, waits for health
# 6. Starts cron
# 7. Cleans up all processes on exit/interrupt

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Configuration
MAX_RETRIES=30
RETRY_INTERVAL=1

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# PIDs for cleanup
API_PID=""
WORKER_PID=""
CRON_PID=""

cleanup() {
    echo -e "\n${YELLOW}Shutting down services...${NC}"

    # Kill processes in reverse order
    if [ -n "$CRON_PID" ] && kill -0 "$CRON_PID" 2>/dev/null; then
        echo "Stopping cron (PID: $CRON_PID)..."
        kill "$CRON_PID" 2>/dev/null || true
    fi

    if [ -n "$WORKER_PID" ] && kill -0 "$WORKER_PID" 2>/dev/null; then
        echo "Stopping worker (PID: $WORKER_PID)..."
        kill "$WORKER_PID" 2>/dev/null || true
    fi

    if [ -n "$API_PID" ] && kill -0 "$API_PID" 2>/dev/null; then
        echo "Stopping api (PID: $API_PID)..."
        kill "$API_PID" 2>/dev/null || true
    fi

    docker compose down postgres

    # Wait for processes to terminate
    sleep 1

    # Force kill if still running
    if [ -n "$CRON_PID" ] && kill -0 "$CRON_PID" 2>/dev/null; then
        kill -9 "$CRON_PID" 2>/dev/null || true
    fi
    if [ -n "$WORKER_PID" ] && kill -0 "$WORKER_PID" 2>/dev/null; then
        kill -9 "$WORKER_PID" 2>/dev/null || true
    fi
    if [ -n "$API_PID" ] && kill -0 "$API_PID" 2>/dev/null; then
        kill -9 "$API_PID" 2>/dev/null || true
    fi

    echo -e "${GREEN}All services stopped.${NC}"
}

# Set up trap for cleanup on exit/interrupt
trap cleanup EXIT INT TERM

wait_for_postgres() {
    echo -e "${YELLOW}Waiting for PostgreSQL to be ready...${NC}"
    local retries=0

    until pg_isready -h localhost -p 5432 -U ltx_user -d ltx_db >/dev/null 2>&1; do
        retries=$((retries + 1))
        if [ $retries -ge $MAX_RETRIES ]; then
            echo -e "${RED}ERROR: PostgreSQL did not become ready after ${MAX_RETRIES} attempts${NC}"
            exit 1
        fi
        echo "Waiting for PostgreSQL... ($retries/$MAX_RETRIES)"
        sleep $RETRY_INTERVAL
    done

    echo -e "${GREEN}PostgreSQL is ready!${NC}"
}

wait_for_api() {
    echo -e "${YELLOW}Waiting for API to be healthy...${NC}"
    local retries=0

    until curl -sf -k https://localhost:3000/health >/dev/null 2>&1; do
        retries=$((retries + 1))
        if [ $retries -ge $MAX_RETRIES ]; then
            echo -e "${RED}ERROR: API did not become healthy after ${MAX_RETRIES} attempts${NC}"
            exit 1
        fi

        # Check if api process is still running
        if ! kill -0 "$API_PID" 2>/dev/null; then
            echo -e "${RED}ERROR: API process died unexpectedly${NC}"
            exit 1
        fi

        echo "Waiting for API health check... ($retries/$MAX_RETRIES)"
        sleep $RETRY_INTERVAL
    done

    echo -e "${GREEN}API is healthy!${NC}"
}

wait_for_worker() {
    echo -e "${YELLOW}Waiting for Worker to be healthy...${NC}"
    local retries=0

    until curl -sf http://localhost:8080/health >/dev/null 2>&1; do
        retries=$((retries + 1))
        if [ $retries -ge $MAX_RETRIES ]; then
            echo -e "${RED}ERROR: Worker did not become healthy after ${MAX_RETRIES} attempts${NC}"
            exit 1
        fi

        # Check if worker process is still running
        if ! kill -0 "$WORKER_PID" 2>/dev/null; then
            echo -e "${RED}ERROR: Worker process died unexpectedly${NC}"
            exit 1
        fi

        echo "Waiting for Worker health check... ($retries/$MAX_RETRIES)"
        sleep $RETRY_INTERVAL
    done

    echo -e "${GREEN}Worker is healthy!${NC}"
}

# Navigate to project root
cd "$PROJECT_ROOT"

# Build the frontend first
echo -e "${YELLOW}Building frontend...${NC}"
just front

# Start PostgreSQL container
echo -e "${YELLOW}Starting PostgreSQL...${NC}"
docker compose up -d postgres

# Wait for PostgreSQL
wait_for_postgres

# Run migrations
echo -e "${YELLOW}Running database migrations...${NC}"
export DATABASE_URL="postgres://ltx_user:ltx_password@localhost:5432/ltx_db"
cd src/api-ltx
if diesel database setup --locked-schema; then
    echo -e "${GREEN}Database setup completed!${NC}"
else
    echo "Database setup failed, attempting migrations directly..."
    diesel migration run
fi
cd "$PROJECT_ROOT"

# Build all binaries
echo -e "${YELLOW}Building services...${NC}"
cargo build -p api-ltx -p worker-ltx -p cron-ltx

# Start API server
echo -e "${YELLOW}Starting API server...${NC}"
DATABASE_URL="${DATABASE_URL}" \
HOST=0.0.0.0 \
PORT=3000 \
RUST_LOG="${RUST_LOG:-info}" \
TLS_CERT_PATH="./certs/cert.pem" \
TLS_KEY_PATH="./certs/key.pem" \
cargo run -p api-ltx --bin api-ltx &
API_PID=$!
echo "API started with PID: $API_PID"

# Wait for API to be healthy
wait_for_api

# Start Worker
echo -e "${YELLOW}Starting Worker...${NC}"
DATABASE_URL="${DATABASE_URL}" \
WORKER_POLL_INTERVAL_MS="${WORKER_POLL_INTERVAL_MS:-600}" \
RUST_LOG="${RUST_LOG:-info}" \
WORKER_MAX_CONCURRENCY="${WORKER_MAX_CONCURRENCY:-1000}" \
cargo run -p worker-ltx &
WORKER_PID=$!
echo "Worker started with PID: $WORKER_PID"

# Wait for Worker to be healthy
wait_for_worker

# Start Cron
echo -e "${YELLOW}Starting Cron...${NC}"
# DATABASE_URL="${DATABASE_URL}" \
DATABASE_URL="postgres://ltx_user:ltx_password@localhost:5432/ltx_db" \
CRON_POLL_INTERVAL_S="${CRON_POLL_INTERVAL_S:-86400}" \
RUST_LOG="${RUST_LOG:-info}" \
HOST='0.0.0.0' \
PORT=3000 \
ACCEPT_INVALID_CERTS="${ACCEPT_INVALID_CERTS:-true}" \
cargo run -p cron-ltx &
CRON_PID=$!
echo "Cron started with PID: $CRON_PID"

echo ""
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}All services are running!${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "  API:    https://localhost:3000 (PID: $API_PID)"
echo "  Worker: http://localhost:8080  (PID: $WORKER_PID)"
echo "  Cron:   running                (PID: $CRON_PID)"
echo ""
echo "Press Ctrl+C to stop all services"
echo ""

# Wait for any process to exit
wait $API_PID $WORKER_PID $CRON_PID 2>/dev/null || true

# If we get here, one of the processes exited
echo -e "${YELLOW}A service exited, shutting down...${NC}"
