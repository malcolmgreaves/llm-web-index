#!/usr/bin/env bash

set -euo pipefail

# Check if password argument is provided
if [ $# -ne 1 ]; then
    echo "Error: Please provide exactly one argument (the password)" >&2
    return 1 2>/dev/null || exit 1
fi

# Get password and remove all whitespace
PASSWORD="${1//[[:space:]]/}"

# Check if password is empty after removing whitespace
if [ -z "$PASSWORD" ]; then
    echo "Error: Password cannot be empty" >&2
    return 1 2>/dev/null || exit 1
fi

# Check if password is forbidden value
if [ "$PASSWORD" = "password" ] || [ "$PASSWORD" = "test_password" ]; then
    echo "Error: Password cannot be 'password' or 'test_password'" >&2
    return 1 2>/dev/null || exit 1
fi

# Generate password hash
echo "Generating password hash..."
AUTH_PASSWORD_HASH=$(cargo run --bin generate-password-hash -- "$PASSWORD")

if [ -z "$AUTH_PASSWORD_HASH" ]; then
    echo "Error: Failed to generate password hash" >&2
    return 1 2>/dev/null || exit 1
fi

# Generate session secret
echo "Generating session secret..."
SESSION_SECRET=$(openssl rand -base64 32)

if [ -z "$SESSION_SECRET" ]; then
    echo "Error: Failed to generate session secret" >&2
    return 1 2>/dev/null || exit 1
fi

# Export the environment variables
export AUTH_PASSWORD_HASH
export SESSION_SECRET

echo ""
echo "✓ Environment variables set successfully:"
echo "  AUTH_PASSWORD_HASH=$AUTH_PASSWORD_HASH"
echo "  SESSION_SECRET=$SESSION_SECRET"
echo ""
echo "These variables are now available in your current shell session."
