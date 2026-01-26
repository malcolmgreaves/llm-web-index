#!/usr/bin/env bash

set -euo pipefail

echo "Stopping test database..."
docker stop ltx-test-db

echo "Removing test database..."
docker remove ltx-test-db

echo "Success! Test database deleted."
