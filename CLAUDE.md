# Tunny Dashboard

A Rust egui desktop app for analyzing Optuna optimization results.

## Language

Code comments, commit messages, and PRs must always be written in English.

## Development Commands

### Build

```bash
# Build the entire workspace
cargo build --workspace

# Release build
cargo build --workspace --release
```

### Run Tests

```bash
# Run all tests
cargo test --workspace

# rust_core only
cargo test -p tunny-core

# egui-app only
cargo test -p tunny-desktop
```

### Static Analysis

Always run these before committing. Run them under the same conditions as CI (`--all-targets` is required so that test code is also covered by clippy).

```bash
cargo clippy --workspace --all-targets --locked -- -D warnings

cargo fmt --manifest-path rust_core/Cargo.toml --all -- --check
cargo fmt --manifest-path egui-app/Cargo.toml --all -- --check
cargo fmt -p tunny-mcp -- --check
```

### Run the Application

```bash
cargo run -p tunny-desktop
```

### Benchmark

```bash
cargo bench -p tunny-core
```
