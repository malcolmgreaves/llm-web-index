check:
  cargo check --all-targets --workspace

build:
  cargo build

test:
  cargo install cargo-llvm-cov || true
  cargo llvm-cov --all-targets --workspace --html

release:
  cargo build --release --all-targets --workspace

bench:
  cargo bench --all-targets --workspace

fmt:
  cargo fmt --all

clean:
  # Remove Rust build artifacts
  rm -rf target/
  rm -rf src/*/target/

tidy:
  cargo install cargo-machete || true
  [ "${CI_RELAX:-no}" != "yes" ] && cargo machete --with-metadata || true
  cargo clippy --all-targets --workspace --fix

ci: tidy check test bench
  # trims dependencies, formats & lints code, runs tests, runs benchmarks
  echo "Success!"
