check:
  cargo check --all-targets --workspace

build: front
  cargo build

front:
  #!/usr/bin/env bash
  set -e
  cd src/front-ltx
  echo "Building WASM frontend..."
  cargo build --target wasm32-unknown-unknown --release
  echo "Generating JS bindings..."
  wasm-bindgen \
    ../../target/wasm32-unknown-unknown/release/front_ltx.wasm \
    --out-dir www/pkg \
    --target web
  echo "Frontend built successfully in src/front-ltx/www/pkg/"

serve: front
  cargo run -p api-ltx

test_cov:
  cargo install cargo-llvm-cov || true
  cargo llvm-cov --all-targets --workspace --html

test:
  cargo test --all-targets --workspace

test_only test_name:
  cargo test {{test_name}} -- --exact --nocapture 

release: front
  #!/usr/bin/env bash
  set -e
  echo "Optimizing WASM..."
  wasm-opt -Oz src/front-ltx/www/pkg/front_ltx_bg.wasm -o src/front-ltx/www/pkg/front_ltx_bg.wasm
  cargo build --release --all-targets --workspace

bench:
  cargo bench --all-targets --workspace

fmt:
  cargo fmt --all

clean:
  # Remove Rust build artifacts
  rm -rf target/
  rm -rf src/*/target/
  # Remove WASM build artifacts
  rm -rf src/front-ltx/www/pkg/

tidy:
  cargo install cargo-machete || true
  [ "${CI_RELAX:-no}" != "yes" ] && cargo machete --with-metadata || true
  cargo clippy --all-targets --workspace --fix

ci: tidy check test_cov
  #!/usr/bin/env bash
  set -e
  CURRENT_BRANCH=$(git branch --show-current)
  if [ "$CURRENT_BRANCH" = "main" ]; then
    just bench
  else
    echo "Skipping benchmarks because current branch is '$CURRENT_BRANCH' (not 'main')"
  fi
  echo "Success!"
