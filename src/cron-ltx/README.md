# cron-ltx

Periodic updater service that automatically regenerates llms.txt files for websites on a configurable schedule. Acts as a cron-like daemon that monitors the database and triggers updates via the API.

## Overview

The `cron-ltx` crate provides:

- **Periodic polling**: Checks the database at regular intervals for websites needing updates
- **Automatic job creation**: Submits update jobs to the API server
- **Authenticated requests**: Handles authentication when the API requires it
- **TLS support**: Makes secure HTTPS requests to the API
- **Configurable scheduling**: Adjustable poll intervals via environment variables
- **Graceful operation**: Handles failures and retries appropriately

## Architecture

```
src/cron-ltx/
├── src/
│   ├── main.rs          # Service entry point, main polling loop
│   ├── lib.rs           # Library exports
│   ├── process.rs       # Core update scheduling logic
│   ├── auth_client.rs   # HTTP client with authentication support
│   └── errors.rs        # Error types
└── Cargo.toml
```

## How It Works

The cron service operates in a continuous loop:

1. **Poll Database**: Queries for websites that need updating
   - Checks for sites that haven't been updated recently
   - Respects configurable update intervals
   - Prioritizes older entries

2. **Create Update Jobs**: For each website needing an update:
   - Makes authenticated POST request to `/api/jobs`
   - Provides the website URL
   - Receives job ID confirmation

3. **Sleep**: Waits for the configured poll interval before next cycle

4. **Repeat**: Continues indefinitely until stopped

## Configuration

Configure via environment variables:

### Required Settings

- `DATABASE_URL`: PostgreSQL connection string (required)
  - Example: `postgres://ltx_user:ltx_password@postgres:5432/ltx_db`

- `HOST`: API server hostname (default: `localhost`)
- `PORT`: API server port (default: `3000`)

### Scheduling Configuration

- `CRON_POLL_INTERVAL_S`: Polling interval in seconds (default: `300` = 5 minutes)
  - How often to check for websites needing updates
  - Adjust based on update frequency requirements
  - Lower values = more frequent checks = higher load

### Authentication Configuration (when API requires auth)

When `ENABLE_AUTH=1` on the API server, the cron service must authenticate:

- `ENABLE_AUTH`: Set to `1` to enable authentication
- `AUTH_PASSWORD`: Plain text password for authentication (required if auth enabled)
- `AUTH_PASSWORD_HASH`: Password hash (used for verification)
- `SESSION_SECRET`: Secret for session validation

### TLS Configuration

- `ACCEPT_INVALID_CERTS`: Set to `true` for development with self-signed certificates
  - **Must be `false` or unset in production**
  - Only use when working with locally-generated certificates

### Logging

- `RUST_LOG`: Logging level (default: `info`)
  - Use `debug` for detailed operational logs
  - Use `trace` for maximum verbosity

## Building

```bash
# Development build
cargo build -p cron-ltx

# Production build
cargo build -p cron-ltx --release
```

## Running

### Using Docker Compose (Recommended)

The cron service is automatically started with the full stack:

```bash
# Development mode
docker compose up

# The cron service will start polling every 5 minutes (default)
```

Configure via environment variables in docker-compose.yml or .env file.

### Manual Execution

Requires PostgreSQL and API server running:

```bash
# Set environment variables
export DATABASE_URL='postgresql://ltx_user:ltx_password@localhost/ltx_db'
export HOST='localhost'
export PORT='3000'
export CRON_POLL_INTERVAL_S='300'

# If authentication is enabled on the API
export ENABLE_AUTH='1'
export AUTH_PASSWORD='your_password'
source ./make_password_and_export_env.sh "$AUTH_PASSWORD"

# If using self-signed certificates (development only)
export ACCEPT_INVALID_CERTS='true'

# Run the service
cargo run -p cron-ltx
```

## Testing

```bash
# Run unit tests
cargo test -p cron-ltx

# Run with coverage
just test
```

## Authentication Flow

When the API server has authentication enabled:

1. **Login Request**: On startup, sends POST to `/auth/login` with password
2. **Session Cookie**: Receives and stores session cookie
3. **Authenticated Requests**: Includes session cookie in all subsequent API calls
4. **Session Renewal**: Automatically handles session expiration and re-authenticates

The `auth_client` module encapsulates this logic, providing a simple interface for authenticated HTTP requests.

## Monitoring and Logs

The service logs important events:

```
INFO cron_ltx: Starting cron service with poll interval: 300s
DEBUG cron_ltx: Found 3 websites needing updates
DEBUG cron_ltx: Creating update job for https://example.com
INFO cron_ltx: Successfully created job abc-123 for https://example.com
INFO cron_ltx: Sleeping for 300 seconds until next poll
```

Enable debug logging for more detail:

```bash
RUST_LOG=cron_ltx=debug cargo run -p cron-ltx
```

## Update Scheduling Logic

The service determines which websites need updating based on:

1. **Last update timestamp**: Sites not updated recently are prioritized
2. **Update interval**: Configurable per-site update frequency
3. **Job status**: Only creates new jobs if no pending job exists
4. **Failure handling**: Backs off on repeated failures

The exact scheduling logic is implemented in `src/process.rs`.

## Error Handling

The service handles various failure scenarios:

- **Database connection failures**: Retries on next poll cycle
- **API unavailable**: Logs error and continues to next cycle
- **Authentication failures**: Attempts to re-authenticate
- **Network errors**: Logs and retries later
- **Invalid responses**: Logs detailed error information

The service is designed to be resilient and continue operating despite transient failures.

## Performance Considerations

- **Poll interval**: Balance between update freshness and system load
  - Too frequent: Unnecessary database queries and API requests
  - Too infrequent: Outdated llms.txt files
  - Recommended: 5-15 minutes for most use cases

- **Database queries**: Optimized with indexes on update timestamps
- **Concurrent jobs**: Currently sequential, could be parallelized
- **Memory usage**: Minimal, processes sites one at a time

## Deployment Considerations

### Production Setup

1. Use a process supervisor (systemd, supervisord, Docker) to ensure the service stays running
2. Set `ACCEPT_INVALID_CERTS=false` (or unset)
3. Use proper CA-signed certificates for the API server
4. Configure appropriate poll intervals for your scale
5. Monitor logs for authentication or connection issues
6. Consider alerting on repeated failures

### Scaling

For high-volume deployments:

- Run multiple cron service instances with distributed locking
- Implement job deduplication in the database
- Consider using a proper job queue (Redis, RabbitMQ) instead of polling
- Add metrics and monitoring (Prometheus, Grafana)

## Dependencies

Key dependencies:

- `tokio`: Async runtime and timers
- `diesel` + `diesel-async`: Database queries
- `reqwest`: HTTP client for API requests
- `serde`: JSON serialization
- `tracing`: Structured logging
- `core-ltx`: Common utilities (auth config, TLS, logging)

See [Cargo.toml](Cargo.toml) for the complete dependency list.

## Troubleshooting

### Service not creating jobs

Check:
1. Database connection is working (`diesel migration run`)
2. API server is running and reachable
3. Authentication credentials are correct (if enabled)
4. Websites in database actually need updates
5. Check logs with `RUST_LOG=debug`

### Authentication failures

Ensure:
1. `AUTH_PASSWORD` matches the password used by the API server
2. `SESSION_SECRET` is consistent across services
3. API server has `ENABLE_AUTH=1` set
4. Session hasn't expired (check `SESSION_DURATION_SECONDS`)

### TLS certificate errors

For development:
- Set `ACCEPT_INVALID_CERTS=true`
- Use self-signed certificates generated by `./make_tls_cert.sh`

For production:
- Use proper CA-signed certificates
- Set `ACCEPT_INVALID_CERTS=false` or unset it
- Verify certificate paths in API server configuration

## Related Documentation

- [Project Root README](../../README.md) - Overall project documentation
- [api-ltx README](../api-ltx/README.md) - API server that cron interacts with
- [worker-ltx README](../worker-ltx/README.md) - Worker that processes the jobs cron creates
- [core-ltx README](../core-ltx/README.md) - Common utilities used by cron
- [data-model-ltx README](../data-model-ltx/README.md) - Database models used by cron
