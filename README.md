# llm-web-index
A system for generating llms.txt files from websites and keeping them up-to-date.


## Organization

- [`core-ltx`](src/core_ltx): Functional core: all llms.txt generation logic + CLI program for one-offs (generation + update).
- [`api-ltx`](src/api_ltx): API webserver + DB setup.
- [`cli-ltx`](src/cli_ltx): CLI client for users: interfaces with API.
- [`front-ltx`](src/front_ltx): Webapp frontend for users: interfaces with API.
- [`worker-ltx`](src/worker_ltx): Backend worker executing logic (generation + update) from API server into database.
- [`cron-ltx`](src/cron_ltx): Cron worker service to periodically update websites' llms.txt.
- [`common-ltx`](src/common_ltx): Catch-all for utilities common to all crates.


## Quick Start

### Running with Docker Compose (Recommended)

The fastest way to get started is using Docker Compose:

```bash
# Enable BuildKit for faster builds (recommended)
export DOCKER_BUILDKIT=1

# Start the API server and PostgreSQL database
docker compose up

# Or run in detached mode (background)
docker compose up -d
```

The API server will be available at `http://localhost:3000`. BuildKit enables cache mounts that significantly speed up Rust compilation.

See [src/api_ltx/SETUP.md](src/api_ltx/SETUP.md) for detailed setup instructions and API testing examples.

## Development

#### Pre-reqs
- [`cargo` & `rustc`](https://rustup.rs)
- [`just`](https://github.com/casey/just)
- [`pre-commit`](https://pre-commit.com)
- [Docker](https://docs.docker.com/get-docker/) & [Docker Compose](https://docs.docker.com/compose/install/) (for containerized setup)

Install [`just`](https://github.com/casey/just) to run project-specific commands.

Ensure all new code has tests and appropriate documentation.

Always run the [`pre-commit`](https://pre-commit.com) hooks before submitting PRs for review or using CI.

Use `just test` to run tests, `just check` to format & lint code, and `just bench` to run benchmarks.

Build release binaries (servers, frontend, CLI programs) with `just release`.

