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


## Development

#### Pre-reqs
- [`cargo` & `rustc`](https://rustup.rs)
- [`just`](https://github.com/casey/just)
- [`pre-commit`](https://pre-commit.com)
- [`binaryen`](https://github.com/WebAssembly/binaryen) (required for `just release` to optimize WASM frontend)

Install [`just`](https://github.com/casey/just) to run project-specific commands. Install [`binaryen`](https://github.com/WebAssembly/binaryen) for WASM optimization (`brew install binaryen` on macOS).

Ensure all new code has tests and appropriate documentation.

Always run the [`pre-commit`](https://pre-commit.com) hooks before submitting PRs for review or using CI.

Use `just test` to run tests, `just check` to format & lint code, and `just bench` to run benchmarks.

Build release binaries (servers, frontend, CLI programs) with `just release`.

