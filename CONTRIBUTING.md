# Contributing to Tunny Dashboard

## Building from Source

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable)
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

1. From the release assets, download the macOS archive (e.g. `LightGBM-macos.zip`)
2. Extract and copy `lib_lightgbm.dylib` into `libs/`

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
