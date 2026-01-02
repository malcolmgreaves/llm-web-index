#!/usr/bin/env bash

WAIT_S="${WAIT_S:-1}"

set -euo pipefail

echo "Waiting for database to be ready..."
until diesel database setup --locked-schema 2>/dev/null || diesel migration run 2>/dev/null; do
  echo "Database is unavailable - sleeping (${WAIT_S}s)"
  sleep ${WAIT_S}
done

echo "Database is ready - running migrations"
diesel migration run

echo "Starting API server"
exec api-ltx
