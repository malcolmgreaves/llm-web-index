# llm-web-index
A system for generating llms.txt files from websites and keeping them up-to-date.

Contains a full web application, 100% implemented in Rust:
- WASM frontend
- API server
- backend worker service to generate llms.txt files
- cron-like updater service
- production-grade authentication (_simple: no users, single password_)

The [llms.txt](http://llmstxt.org) file generation is performed via prompting to OpenAI's GPT 5.2 model.
This application requires a valid `OPENAI_API_KEY` set as an environment variable at runtime (_specifically within the worker service_).

## Organization

- [`api-ltx`](src/api-ltx): API webserver + DB setup.
- [`core-ltx`](src/core-ltx): Functional core: all llms.txt generation logic + CLI program for one-offs (generation + update). Also includes common utilities shared amongst the other crates.
- [`cron-ltx`](src/cron-ltx): Updater service to periodically update websites' llms.txt.
- [`data-model-ltx`](src/data-model-ltx): Catch-all for utilities common to all crates.
- [`front-ltx`](src/front-ltx): Webapp frontend for users: interfaces with API.
- [`worker-ltx`](src/worker-ltx): The application's data model.


## Quick Start

### Running with Docker Compose (Recommended)

The fastest way to get started is using Docker Compose:

```bash
# Enable BuildKit for faster builds (recommended)
export DOCKER_BUILDKIT=1
export OPENAI_API_KEY='<your open AI API key here>'

# Start the API server and PostgreSQL database
docker compose up

# Or run in detached mode (background)
docker compose up -d
```

The API server will be available at `http://localhost:3000`. BuildKit enables cache mounts that significantly speed up Rust compilation.

See [src/api-ltx/SETUP.md](src/api-ltx/SETUP.md) for detailed setup instructions and API testing examples.

## Development

#### Pre-reqs
- [`cargo` & `rustc`](https://rustup.rs)
- [`just`](https://github.com/casey/just)
- [`pre-commit`](https://pre-commit.com)
- [`binaryen`](https://github.com/WebAssembly/binaryen) (required for `just release` to optimize WASM frontend)
- [Docker](https://docs.docker.com/get-docker/) & [Docker Compose](https://docs.docker.com/compose/install/)

Setup the `pre-commit` hooks before submitting PRs for review or using CI.

Use `just` to run project-specific commands (run `just -l` to see them).

### Design Ethos

**Ensure all new code has tests and appropriate documentation.**

Follow the "functional core, effectful shell" code pattern. Implement logic as functions and push state 
and all user interaction into components that use the functional core.

Strive to have all logic implemented by pure functions. Minimize the use of mutable state (only use 
it _if_ it is necessary for performance) in interfaces and designs.

Use `Result` types and use clear `Error` enum variants whenever an operation could fail.
Only use `Option` if `None` naturally maps to the domain.

**Never use `.unwrap()` nor `.expect()`** except in tests.
