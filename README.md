# Tunny Dashboard

An in-browser analytics dashboard for [Optuna](https://optuna.org/) optimization results, built with Rust/WebAssembly and React.

> **Japanese documentation**: [README_ja.md](README_ja.md)

---

## Overview

Tunny Dashboard parses Optuna Journal log files entirely inside the browser — no server, no Python installation required. The output is a single self-contained `index.html` file that can be opened directly in any modern browser.

### Key Features

- **Zero-dependency deployment** — distributes as a single `index.html` with all assets inlined
- **High-performance data processing** — Journal parsing and DataFrame operations run in Rust/WASM via a dedicated Web Worker
- **Interactive visualizations** — Pareto front (3D/2D), Parallel Coordinates, Scatter Matrix, Hypervolume history, Sensitivity analysis, and more
- **Brushing & Linking** — cross-chart selection synchronized across all views in real time
- **Free layout (Mode D)** — drag-and-drop chart arrangement on a 4×4 grid
- **Multi-study comparison** — overlay or side-by-side comparison of up to 4 studies
- **CSV / HTML report export** — export selected trials with customizable columns
- **Session persistence** — save and restore the full UI state as a JSON file

### Technology Stack

| Layer | Technology |
|---|---|
| Core processing | Rust + [wasm-pack](https://rustwasm.github.io/wasm-pack/) (WebAssembly) |
| Frontend framework | React 19 + TypeScript |
| State management | [Zustand](https://zustand-demo.pmnd.rs/) |
| 3D/2D charts | [deck.gl](https://deck.gl/) |
| Statistical charts | [Apache ECharts](https://echarts.apache.org/) |
| Build tool | [Vite](https://vite.dev/) + [vite-plugin-singlefile](https://github.com/richardtallent/vite-plugin-singlefile) |
| Testing | [Vitest](https://vitest.dev/) + [Playwright](https://playwright.dev/) |

### Repository Structure

```
tunny-dashboard/
├── rust_core/          # Rust/WASM core (Journal parser, DataFrame, analytics)
│   ├── src/
│   └── Cargo.toml
└── frontend/           # React/TypeScript frontend
    ├── src/
    │   ├── components/ # UI components (charts, panels, layout)
    │   ├── stores/     # Zustand state stores
    │   ├── wasm/       # WASM loader, GPU buffer, workers
    │   └── types/      # Shared TypeScript type definitions
    ├── e2e/            # Playwright E2E tests
    └── package.json
```

---

## Quick Start (Windows)

Double-click any of the batch files in the project root:

| File | Action |
|---|---|
| `build.bat` | Full production build → outputs `frontend/dist/index.html` |
| `dev.bat` | Start the development server at `http://localhost:5173` |
| `format.bat` | Format all Rust and TypeScript/TSX source files |

---

## Prerequisites

| Tool | Version | Purpose |
|---|---|---|
| [Node.js](https://nodejs.org/) | ≥ 20 | Frontend build |
| [Rust](https://www.rust-lang.org/tools/install) | stable (1.70+) | WASM core build |
| [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) | ≥ 0.12 | Compile Rust to WASM |

Install `wasm-pack` if not already present:

```bash
cargo install wasm-pack
```

---

## Build

All frontend commands are run from the `frontend/` directory.

```bash
cd frontend
npm install
```

### Development Server

```bash
npm run dev
```

Opens `http://localhost:5173`. The development server sets the required `Cross-Origin-Opener-Policy` and `Cross-Origin-Embedder-Policy` headers needed for `SharedArrayBuffer` support.

### Build WASM Core

```bash
npm run build:wasm
```

Compiles `rust_core/` with `wasm-pack` and outputs the WASM package to `frontend/src/wasm/pkg/`.

### Full Production Build

```bash
npm run build:all
```

Runs `build:wasm` then the Vite production build. Output is a single file at `frontend/dist/index.html` with all JavaScript, CSS, and WASM inlined as base64.

To preview the production build locally:

```bash
npm run preview
```

### Build Steps Separately

```bash
# 1. Compile Rust core to WASM
npm run build:wasm

# 2. Type-check and bundle frontend
npm run build
```

---

## Testing

### Unit & Integration Tests (Vitest)

```bash
cd frontend
npm test
```

Runs all Vitest tests (unit, integration, and performance benchmarks). Tests run in `jsdom` environment without requiring a browser or WASM runtime.

### E2E Tests (Playwright)

```bash
cd frontend

# First time only: install browser binaries (~200 MB)
npx playwright install chromium

# Run E2E tests (dev server starts automatically)
npm run test:e2e

# Interactive UI mode
npm run test:e2e:ui
```

---

## Formatting

### Rust (rust_core)

```bash
cd rust_core
cargo fmt
```

Uses the standard `rustfmt` formatter. Configuration can be added in `rustfmt.toml` if needed.

To check formatting without applying changes:

```bash
cargo fmt -- --check
```

### TypeScript / TSX (frontend)

The project uses [ESLint](https://eslint.org/) for linting and [Prettier](https://prettier.io/) for code formatting.

**Lint (ESLint):**

```bash
cd frontend
npm run lint
```

**Format (Prettier):**

Prettier is configured via `frontend/.prettierrc`. Install and run it with:

```bash
cd frontend
npm install --save-dev prettier
npx prettier --write "src/**/*.{ts,tsx}"
```

Prettier settings (`.prettierrc`):

| Option | Value |
|---|---|
| `semi` | `false` |
| `singleQuote` | `true` |
| `tabWidth` | `2` |
| `trailingComma` | `"all"` |
| `printWidth` | `100` |

Most editors (VS Code, JetBrains) apply Prettier automatically on save when the [Prettier extension](https://marketplace.visualstudio.com/items?itemName=esbenp.prettier-vscode) is installed and `.prettierrc` is present.

---

## License

[MIT](LICENSE)
