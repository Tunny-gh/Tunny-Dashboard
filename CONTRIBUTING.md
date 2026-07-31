# Contributing to Tunny Dashboard

## Building from Source

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable 1.97.1 or later — the `libsqlite3-sys` dependency requires Rust stable 1.97.1+, so run `rustup update` if your toolchain is older)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
- [Node.js](https://nodejs.org/) 18+

### LightGBM Library Setup

The sensitivity analysis features (SHAP, MDI, RF-ANOVA) and 2D PDP use LightGBM via a pre-built native library. These binaries are **not included in the repository** and must be placed manually before building.

#### Required files

| File | Platform | Description |
|---|---|---|
| `libs/lib_lightgbm.dll` | Windows | Shared library |
| `libs/lib_lightgbm.lib` | Windows | Import library for the linker |
| `libs/lib_lightgbm.dylib` | macOS | Shared library |

Place the files directly under the `libs/` directory at the project root:

```
Tunny-Dashboard/
└── libs/
    ├── lib_lightgbm.dll   (Windows)
    ├── lib_lightgbm.lib   (Windows)
    └── lib_lightgbm.dylib (macOS)
```

#### Where to download

Download the pre-built binaries from the [LightGBM GitHub Releases](https://github.com/microsoft/LightGBM/releases):

**Windows**

1. From the release assets, download `lib_lightgbm.zip`
2. Extract and copy `lib_lightgbm.dll` and `lib_lightgbm.lib` into `libs/`

**macOS**

The recommended way is to install via Homebrew, which provides the correct arm64 binary:

```bash
brew install lightgbm
cp $(brew --prefix lightgbm)/lib/lib_lightgbm.dylib libs/
# Bundle libomp (a LightGBM runtime dependency) alongside it so libs/ is
# self-contained and does not depend on Homebrew staying installed.
cp $(brew --prefix libomp)/lib/libomp.dylib libs/
chmod u+w libs/lib_lightgbm.dylib libs/libomp.dylib
```

After copying, fix the install name so the dynamic linker can find the library relative to the binary (this step is required once per copy):

```bash
install_name_tool -id "@rpath/lib_lightgbm.dylib" libs/lib_lightgbm.dylib
# Point the libomp dependency at the sibling copy via @loader_path.
install_name_tool -change \
  "@rpath/libomp.dylib" \
  "@loader_path/libomp.dylib" \
  libs/lib_lightgbm.dylib

# install_name_tool invalidates the code signature; re-sign ad-hoc so macOS
# will load the dylibs (otherwise binaries linking them are killed with SIGKILL).
# The Homebrew bottles are already signed, so --force is needed to re-sign.
codesign --force -s - libs/libomp.dylib
codesign --force -s - libs/lib_lightgbm.dylib
```

Alternatively, download from [LightGBM GitHub Releases](https://github.com/microsoft/LightGBM/releases) — make sure to pick the **arm64** asset (e.g. `LightGBM-*-macos-arm64.tar.gz`) and apply the same commands above.

> **Why are these commands needed?**
> The Homebrew-installed dylib embeds its Homebrew install path as its install name; the build system looks for the library via `@rpath`, so the name must be rewritten once after copying into `libs/`. Rewriting the binary invalidates its code signature, and macOS refuses to load (and SIGKILLs) a process that links an improperly signed dylib — `codesign -s -` re-signs it ad-hoc to fix this.

> The `lib_lightgbm.def` and `lib_lightgbm.exp` files sometimes bundled in LightGBM releases are not required.

### Build

```bash
# Run tests (requires LightGBM libs to be in place)
cargo test -p tunny-core

# Build the egui desktop app
cargo build -p egui-wgpu

# Build the WebAssembly package
wasm-pack build rust_core --target web --out-dir ../frontend/src/wasm/pkg
```

### Runtime (Windows)

On Windows, `lib_lightgbm.dll` must be discoverable at runtime. The `build.rs` script automatically copies it into the Cargo target directory during build, so `cargo test` and `cargo run` work out of the box. For a standalone binary, place `lib_lightgbm.dll` alongside the executable.

## Development Commands

CI is defined in [`.github/workflows/ci.yml`](.github/workflows/ci.yml). Commands marked **CI** are run by CI with the exact flags shown below; unmarked commands are local-only conveniences.

### Build

```bash
# Build the entire workspace
cargo build --workspace

# Release build
cargo build --workspace --release
```

### Run Tests

**CI** (see the `build-test` job; runs on Windows/macOS with the `ci-test` profile):

```bash
cargo test --workspace --locked --profile ci-test
```

Local only (quicker feedback, uses the default dev profile):

```bash
cargo test -p tunny-core
cargo test -p tunny-desktop
```

### Static Analysis

Always run these before committing. **CI** (see the `lint` job; runs on Linux, `--all-targets` is required so that test code is also covered by clippy):

```bash
cargo clippy --workspace --all-targets --locked -- -D warnings

cargo fmt --manifest-path rust_core/Cargo.toml --all -- --check
cargo fmt --manifest-path egui-app/Cargo.toml --all -- --check
```

Local only:

```bash
cargo fmt -p tunny-mcp -- --check
```

### Security Audit

**CI** (see the `audit` job; requires `cargo-audit` to be installed locally):

```bash
cargo audit --ignore RUSTSEC-2026-0194 --ignore RUSTSEC-2026-0195
```

### Run the Application

```bash
cargo run -p tunny-desktop
```

### Benchmark

```bash
cargo bench -p tunny-core
```
