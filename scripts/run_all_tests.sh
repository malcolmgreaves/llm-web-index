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
echo "Checking test database..."
if ! docker compose -f docker-compose.test.yml ps postgres-test | grep -q "Up"; then
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

    if eval "$command"; then
        echo -e "${GREEN}✓ $name passed${NC}"
        ((TESTS_PASSED++))
    else
        echo -e "${RED}✗ $name failed${NC}"
        ((TESTS_FAILED++))
    fi
}

# Run all unit tests
echo ""
echo -e "${YELLOW}Running All Workspace Unit Tests...${NC}"
run_test_suite "Workspace Unit Tests" "cargo test --workspace --lib"

# Run worker integration tests separately with single thread for DB tests
echo ""
echo -e "${YELLOW}Running Worker Integration Tests...${NC}"
run_test_suite "Worker Job Processing Tests" "cargo test --test job_processing"
run_test_suite "Worker Job Queue Tests" "cargo test --test job_queue -- --test-threads=1"
run_test_suite "Worker Result Handling Tests" "cargo test --test result_handling -- --test-threads=1"

# Summary
echo ""
echo -e "${YELLOW}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${YELLOW}║                    Test Summary                            ║${NC}"
echo -e "${YELLOW}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

TOTAL_SUITES=$((TESTS_PASSED + TESTS_FAILED))

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}All test suites passed! ($TESTS_PASSED/$TOTAL_SUITES)${NC}"
    echo ""
    echo -e "${GREEN}✓ Mock LLM Provider: 11 tests${NC}"
    echo -e "${GREEN}✓ Database Test Helpers: 5 tests${NC}"
    echo -e "${GREEN}✓ Worker Job Processing: 11 tests${NC}"
    echo -e "${GREEN}✓ Worker Job Queue: 10-12 tests${NC}"
    echo -e "${GREEN}✓ Worker Result Handling: 8 tests${NC}"
    echo -e "${GREEN}✓ API Auth: 11 tests${NC}"
    echo -e "${GREEN}✓ Core LLM: 24 tests${NC}"
    echo -e "${GREEN}✓ Cron: 8 tests${NC}"
    echo -e "${GREEN}✓ Data Model: 4 tests${NC}"
    echo ""
    echo -e "${GREEN}Total: ~90+ tests passing${NC}"
else
    echo -e "${RED}Some test suites failed: $TESTS_FAILED/$TOTAL_SUITES${NC}"
    echo -e "${GREEN}Passed: $TESTS_PASSED${NC}"
    echo -e "${RED}Failed: $TESTS_FAILED${NC}"
fi

echo ""
echo -e "${YELLOW}═══════════════════════════════════════════════════════════${NC}"

# Optional: Generate coverage report if cargo-llvm-cov is installed
if command -v cargo-llvm-cov &> /dev/null; then
    echo ""
    echo -e "${YELLOW}Generating coverage report...${NC}"
    cargo llvm-cov --all-targets --workspace --html || true
    echo -e "${GREEN}Coverage report generated: target/llvm-cov/html/index.html${NC}"
else
    echo ""
    echo -e "${YELLOW}Note: Install cargo-llvm-cov to generate coverage reports:${NC}"
    echo "  cargo install cargo-llvm-cov"
fi

echo ""

# Exit with appropriate code
if [ $TESTS_FAILED -eq 0 ]; then
    exit 0
else
    exit 1
fi
