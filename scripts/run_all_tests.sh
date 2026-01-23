#!/usr/bin/env bash
#
# Master test runner for the llm-web-index project
#
# This script runs all tests (unit tests for all crates + integration tests)
# and generates a coverage report.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Set test database URL
export TEST_DATABASE_URL="postgres://ltx_test_user:ltx_test_password@localhost:5433/ltx_test_db"

echo -e "${YELLOW}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${YELLOW}║         llm-web-index Test Suite Runner                   ║${NC}"
echo -e "${YELLOW}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check if test database is running
STARTED_DB=0
echo "Checking test database..."
if ! docker compose -f docker-compose.test.yml ps postgres-test | grep -q "Up"; then
    STARTED_DB=1
    echo -e "${YELLOW}Test database not running. Starting it now...${NC}"
    ./scripts/setup_test_db.sh
fi

cd "$PROJECT_ROOT"

# Test counters
TESTS_PASSED=0
TESTS_FAILED=0

# Function to run tests and track results
run_test_suite() {
    local name=$1
    local command=$2

    echo ""
    echo -e "${YELLOW}═══════════════════════════════════════════════════════════${NC}"
    echo -e "${YELLOW}Running: $name${NC}"
    echo -e "${YELLOW}═══════════════════════════════════════════════════════════${NC}"

    set +e
    if eval "$command"; then
        echo -e "${GREEN}✓ $name passed${NC}"
        ((TESTS_PASSED++))
    else
        echo -e "${RED}✗ $name failed${NC}"
        ((TESTS_FAILED++))
    fi
    set -e
}

# Run all unit tests with code coverage
cargo install cargo-llvm-cov || true
echo ""
echo -e "${YELLOW}Running All Workspace Unit Tests...${NC}"
run_test_suite "Workspace Unit Tests with Code Coverage" "cargo llvm-cov --all-targets --workspace --html"

echo -e "${GREEN}Coverage report generated: target/llvm-cov/html/index.html${NC}"

echo ""

if [ $STARTED_DB -eq 1 ]; then
    echo "Started test DB, stopping & removing."
    ./scripts/remove_test_db.sh
fi

# Exit with appropriate code
if [ $TESTS_FAILED -eq 0 ]; then
    exit 0
else
    exit 1
fi
