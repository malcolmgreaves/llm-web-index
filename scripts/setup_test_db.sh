#!/usr/bin/env bash
#
# Setup test database for integration and unit tests
#
# This script:
# 1. Starts a PostgreSQL test database in Docker
# 2. Waits for it to be ready
# 3. Runs database migrations
# 4. Confirms the database is ready for tests

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Setting up test database...${NC}"

# Navigate to project root
cd "$PROJECT_ROOT"

# Start test PostgreSQL container
echo "Starting PostgreSQL test container..."
docker compose -f docker-compose.test.yml up -d postgres-test

# Wait for PostgreSQL to be ready
echo "Waiting for PostgreSQL to be ready..."
MAX_RETRIES=30
RETRY_COUNT=0

until docker compose -f docker-compose.test.yml exec -T postgres-test pg_isready -U ltx_test_user > /dev/null 2>&1; do
    RETRY_COUNT=$((RETRY_COUNT + 1))
    if [ $RETRY_COUNT -ge $MAX_RETRIES ]; then
        echo "Error: PostgreSQL did not become ready in time"
        exit 1
    fi
    echo "Waiting for PostgreSQL... ($RETRY_COUNT/$MAX_RETRIES)"
    sleep 1
done

echo -e "${GREEN}PostgreSQL is ready!${NC}"

# Export database URL for migrations
export DATABASE_URL="postgres://ltx_test_user:ltx_test_password@localhost:5433/ltx_test_db"

# Run migrations
echo "Running database migrations..."
cd src/api-ltx
diesel migration run

echo -e "${GREEN}✓ Test database is ready!${NC}"
echo ""
echo "Database URL: $DATABASE_URL"
echo "Container: ltx-test-db"
echo ""
echo "To stop the test database:"
echo "  docker compose -f docker-compose.test.yml down"
echo ""
echo "To view logs:"
echo "  docker compose -f docker-compose.test.yml logs postgres-test"
