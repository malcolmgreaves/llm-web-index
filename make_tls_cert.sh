#!/usr/bin/env bash

set -euo pipefail

# Default certificate directory
CERT_DIR="${1:-./certs}"

# Create directory if it doesn't exist
echo "Creating certificate directory: $CERT_DIR"
mkdir -p "$CERT_DIR"

# Generate certificate with default SANs
echo "Generating self-signed TLS certificate..."
CERT_OUTPUT=$(cargo run --bin generate-tls-cert -- "$CERT_DIR" "localhost" "127.0.0.1" "0.0.0.0")

if [ $? -ne 0 ]; then
    echo "Error: Failed to generate TLS certificate" >&2
    return 1 2>/dev/null || exit 1
fi

# Parse output paths (last two lines from stdout)
TLS_CERT_PATH=$(echo "$CERT_OUTPUT" | head -n 1)
TLS_KEY_PATH=$(echo "$CERT_OUTPUT" | tail -n 1)

if [ -z "$TLS_CERT_PATH" ] || [ -z "$TLS_KEY_PATH" ]; then
    echo "Error: Failed to parse certificate paths" >&2
    return 1 2>/dev/null || exit 1
fi

# Export the environment variables
export TLS_CERT_PATH
export TLS_KEY_PATH

echo ""
echo "✓ Environment variables set successfully:"
echo ""
echo "TLS_CERT_PATH='${TLS_CERT_PATH}'"
echo "TLS_KEY_PATH='${TLS_KEY_PATH}'"
echo ""
echo "These variables are now available in your current shell session."
echo "Add them to your .env file for persistent configuration."
