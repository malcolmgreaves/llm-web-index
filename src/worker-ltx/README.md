# worker-ltx

Backend worker service that processes llms.txt generation jobs. Continuously polls the database for pending jobs, executes the generation logic using the core library, and updates job status and results.

## Overview

The `worker-ltx` crate provides:

- **Job processing**: Picks up pending jobs from the database and processes them
- **llms.txt generation**: Uses `core-ltx` to generate llms.txt content from websites
- **Status updates**: Tracks job progress through queued → started → running → success/failure
- **Error handling**: Captures and stores error messages for failed jobs
- **Continuous operation**: Runs as a long-lived service, polling for new work
- **Graceful degradation**: Handles transient failures and continues processing

## Architecture

```
src/worker-ltx/
├── src/
│   ├── main.rs    # Service entry point, main polling loop
│   ├── lib.rs     # Library exports
│   ├── work.rs    # Core job processing logic
│   └── errors.rs  # Error types
└── Cargo.toml
```

## How It Works

The worker operates in a continuous loop:

1. **Poll for Jobs**: Queries database for jobs with status `queued`
2. **Claim Job**: Updates status to `started` to prevent duplicate processing
3. **Execute Generation**:
   - Updates status to `running`
   - Calls `core-ltx` to fetch website and generate llms.txt
   - Waits for LLM response (can take 10-60 seconds)
4. **Store Result**:
   - On success: Updates status to `success`, stores generated content
   - On failure: Updates status to `failure`, stores error message
5. **Sleep**: Waits briefly before checking for more jobs
6. **Repeat**: Continues indefinitely until stopped

## Job Lifecycle

```
API creates job
      ↓
   [queued]
      ↓
Worker claims job
      ↓
   [started]
      ↓
Worker begins processing
      ↓
   [running]
      ↓
    ┌─────────┐
    ↓         ↓
[success]  [failure]
    │         │
    └────┬────┘
         ↓
    Job complete
```

## Configuration

Configure via environment variables:

### Required Settings

- `DATABASE_URL`: PostgreSQL connection string (required)
  - Example: `postgres://ltx_user:ltx_password@postgres:5432/ltx_db`

- `OPENAI_API_KEY`: OpenAI API key for generation (required)
  - Used by `core-ltx` to call GPT models
  - Must have access to GPT-5.2, GPT-5-mini, or GPT-5-nano

### Worker Configuration

- `WORKER_POLL_INTERVAL_MS`: Polling interval in milliseconds (default: `1000`)
  - How often to check for new jobs
  - Lower values = faster job pickup = higher database load
  - Higher values = slower response = lower database load
  - Recommended: 500-2000ms

### Logging

- `RUST_LOG`: Logging level (default: `info`)
  - `info`: Basic job processing logs
  - `debug`: Detailed processing information
  - `trace`: Maximum verbosity including LLM prompts/responses

## Building

```bash
# Development build
cargo build -p worker-ltx

# Production build
cargo build -p worker-ltx --release
```

## Running

### Using Docker Compose (Recommended)

The worker service is automatically started with the full stack:

```bash
# Ensure OPENAI_API_KEY is set
export OPENAI_API_KEY='your_api_key_here'

# Start all services
docker compose up

# The worker will start processing jobs immediately
```

### Manual Execution

Requires PostgreSQL running and API key configured:

```bash
# Set environment variables
export DATABASE_URL='postgresql://ltx_user:ltx_password@localhost/ltx_db'
export OPENAI_API_KEY='your_api_key_here'
export WORKER_POLL_INTERVAL_MS='1000'
export RUST_LOG='info'

# Run the worker
cargo run -p worker-ltx
```

## Testing

```bash
# Run unit tests
cargo test -p worker-ltx

# Run integration tests (requires database and API key)
DATABASE_URL='postgres://...' OPENAI_API_KEY='...' \
  cargo test -p worker-ltx -- --ignored

# Run with coverage
just test
```

## Monitoring and Logs

The worker logs important events:

```
INFO worker_ltx: Worker service starting with poll interval: 1000ms
DEBUG worker_ltx: Polling for jobs...
DEBUG worker_ltx: Found 1 pending job(s)
INFO worker_ltx: Processing job abc-123 for URL: https://example.com
DEBUG worker_ltx: Job abc-123: Fetching website content
DEBUG worker_ltx: Job abc-123: Sending to LLM for generation
INFO worker_ltx: Job abc-123 completed successfully
INFO worker_ltx: Sleeping for 1000ms before next poll
```

Enable debug logging for more detail:

```bash
RUST_LOG=worker_ltx=debug cargo run -p worker-ltx
```

Enable trace logging to see LLM prompts and responses:

```bash
RUST_LOG=worker_ltx=trace,core_ltx=trace cargo run -p worker-ltx
```

## Error Handling

The worker handles various failure scenarios:

### Transient Errors

- **Database connection failures**: Logs error, waits, retries on next poll
- **Network timeouts**: Marks job as failed, continues to next job
- **LLM API rate limits**: Logs error, job remains queued for retry
- **Temporary API outages**: Worker continues polling, picks up jobs when API recovers

### Permanent Errors

- **Invalid URLs**: Marks job as failed with descriptive error
- **LLM generation failures**: Marks job as failed, stores error message
- **Validation failures**: Multiple retry attempts, then fails with error details
- **Missing API key**: Worker exits immediately (configuration error)

All errors are logged and stored in the database for debugging.

## Performance Considerations

### Processing Time

- Web fetching: 1-5 seconds
- LLM generation: 10-60 seconds (varies by model and content size)
- Total per job: 15-65 seconds typically

### Throughput

Single worker instance:
- Best case: ~50-60 jobs/hour (using fast models)
- Typical case: ~30-40 jobs/hour (using GPT-5.2)
- Worst case: ~10-20 jobs/hour (large websites, slow networks)

### Scaling

To increase throughput:

1. **Run multiple workers**: Each instance processes jobs independently
   - Ensure proper job locking to prevent duplicate processing
   - Database handles coordination automatically via atomic updates

2. **Use faster models**: GPT-5-mini or GPT-5-nano for faster generation
   - Trade-off: Lower quality but higher throughput

3. **Optimize polling interval**: Balance responsiveness vs database load
   - Lower interval = faster job pickup
   - Higher interval = reduced database queries

4. **Increase connection pool**: Allow more concurrent database operations
   - Configure in `DATABASE_URL` query parameters

## Deployment Considerations

### Production Setup

1. Use a process supervisor (systemd, supervisord, Docker) to ensure worker stays running
2. Set `RUST_LOG=info` for production (avoid debug/trace in production)
3. Monitor logs for repeated failures or API errors
4. Set up alerting on worker crashes or high failure rates
5. Consider running 2-4 worker instances for redundancy and throughput

### Resource Requirements

Per worker instance:
- **Memory**: 100-500 MB (varies with job size)
- **CPU**: Low (mostly I/O-bound, waiting for network/LLM)
- **Network**: Moderate (fetching websites, calling LLM API)
- **Database connections**: 1-2 per instance

### High Availability

For production deployments:

1. Run multiple worker instances across different hosts
2. Use Docker healthchecks or process supervisors
3. Monitor database connection health
4. Set up alerting for worker downtime
5. Consider load balancing multiple workers

## Troubleshooting

### Worker not processing jobs

Check:
1. Database connection is working
2. `OPENAI_API_KEY` is set correctly
3. Jobs exist in database with status `queued`
4. Worker logs for errors
5. Database migrations are up to date

### Jobs failing repeatedly

Check:
1. Website URL is valid and accessible
2. OpenAI API key has sufficient quota/credits
3. LLM API is responding (check OpenAI status page)
4. Website content is parseable (not requiring JavaScript)
5. Error messages in database for specific failure reasons

### High memory usage

Possible causes:
1. Processing very large websites
2. Memory leak in LLM client
3. Too many concurrent operations

Solutions:
- Restart worker periodically
- Reduce poll frequency
- Implement website size limits

### Slow processing

Check:
1. Network latency to target websites
2. LLM API response times (may vary by load)
3. Model selection (use faster models if appropriate)
4. Database query performance

## Development

### Adding Support for New LLM Models

1. Add model variant in `core-ltx::llms::LlmModel`
2. Implement generation function in `core-ltx`
3. Worker automatically uses the new model
4. Configure via job metadata or environment variable

### Customizing Job Processing

To modify job processing logic:

1. Edit `src/work.rs`
2. Modify the `process_job` function
3. Add custom validation, preprocessing, or post-processing
4. Update error handling as needed
5. Add tests for new behavior

### Local Development

```bash
# Set up local environment
cp .env.example .env
# Edit .env with your settings

# Run database migrations
cd src/api-ltx
diesel migration run
cd ../..

# Start worker
cargo run -p worker-ltx
```

## Dependencies

Key dependencies:

- `tokio`: Async runtime and timers
- `diesel` + `diesel-async`: Database operations
- `core-ltx`: llms.txt generation logic
- `data-model-ltx`: Database models and schema
- `tracing`: Structured logging
- `uuid`: Job ID handling
- `chrono`: Timestamp management

See [Cargo.toml](Cargo.toml) for the complete dependency list.

## Related Documentation

- [Project Root README](../../README.md) - Overall project documentation
- [core-ltx README](../core-ltx/README.md) - Generation logic used by worker
- [data-model-ltx README](../data-model-ltx/README.md) - Database models and operations
- [api-ltx README](../api-ltx/README.md) - API that creates jobs for worker
- [cron-ltx README](../cron-ltx/README.md) - Service that creates recurring jobs
