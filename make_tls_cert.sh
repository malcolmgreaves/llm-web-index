#!/usr/bin/env bash

set -euo pipefail

# Default certificate directory
CERT_DIR="${1:-./certs}"

# Create directory if it doesn't exist
echo "Creating certificate directory: $CERT_DIR"
mkdir -p "$CERT_DIR"

# Check if mkcert is available
if command -v mkcert &> /dev/null; then
    echo "Using mkcert to generate locally-trusted certificate..."
    echo "This certificate will be automatically trusted by your browsers!"

    # Generate certificate with mkcert
    cd "$CERT_DIR"
    mkcert -cert-file cert.pem -key-file key.pem localhost 127.0.0.1 ::1 0.0.0.0
    cd - > /dev/null

    TLS_CERT_PATH="${CERT_DIR}/cert.pem"
    TLS_KEY_PATH="${CERT_DIR}/key.pem"

    echo ""
    echo "✓ Locally-trusted certificate generated successfully!"
    echo "  Your browsers will trust this certificate without warnings."

else
    echo "mkcert not found, generating self-signed certificate..."
    echo "Note: Browsers will show security warnings for self-signed certificates."
    echo ""
    echo "To avoid warnings, install mkcert:"
    echo "  macOS:   brew install mkcert && mkcert -install"
    echo "  Linux:   See https://github.com/FiloSottile/mkcert#installation"
    echo ""

    # Generate certificate with default SANs using our Rust binary
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

    echo ""
    echo "✓ Self-signed certificate generated."
    echo ""
    echo "To trust this certificate on macOS:"
    echo "  1. Open Keychain Access"
    echo "  2. File → Import Items → Select ${CERT_DIR}/cert.pem"
    echo "  3. Double-click the certificate → Trust → Always Trust"
    echo ""
fi

# Export the environment variables
export TLS_CERT_PATH
export TLS_KEY_PATH

echo ""
echo "Environment variables:"
echo ""
echo "TLS_CERT_PATH='${TLS_CERT_PATH}'"
echo "TLS_KEY_PATH='${TLS_KEY_PATH}'"
echo ""
echo "Add these to your .env file for persistent configuration."
