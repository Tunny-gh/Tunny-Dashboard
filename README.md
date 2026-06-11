# Tunny Dashboard

A desktop analytics dashboard for [Optuna](https://optuna.org/) optimization results, built with Rust/egui.

---

## Overview

Tunny Dashboard parses Optuna Journal log files locally on your desktop — no server, no Python installation required.

![Tunny-Dashboard](https://github.com/user-attachments/assets/d85a722f-ee7c-4217-a3a9-75ab2a20734c)

### Key Features

- **High-performance data processing** — Journal parsing and DataFrame operations run in native Rust
- **Interactive visualizations** — Pareto front (3D/2D), Parallel Coordinates, Scatter Matrix, Hypervolume history, Sensitivity analysis, and more
- **Brushing & Linking** — cross-chart selection synchronized across all views in real time
- **Free layout** — drag-and-drop chart arrangement
- **Multi-study comparison** — overlay or side-by-side comparison of multiple studies
- **CSV export** — export selected trials with customizable columns
- **Session persistence** — save and restore the full UI state as a JSON file

### Technology Stack

| Layer            | Technology                                                              |
| ---------------- | ----------------------------------------------------------------------- |
| Core processing  | Rust + [ndarray](https://docs.rs/ndarray/)                              |
| UI framework     | [eframe](https://docs.rs/eframe/) + [egui](https://docs.rs/egui/)       |
| GPU rendering    | [wgpu](https://docs.rs/wgpu/) + [egui-wgpu](https://docs.rs/egui-wgpu/) |
| Charts           | [egui_plot](https://docs.rs/egui_plot/)                                 |
| Machine learning | [linfa](https://linfa-rs.github.io/)                                    |
| Build tool       | [cargo](https://doc.rust-lang.org/cargo/)                               |
| Testing          | [cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html)  |

### Repository Structure

```
tunny-dashboard/
├── rust_core/          # Core library (Journal parser, DataFrame, analytics)
│   ├── src/
│   │   ├── clustering/     # Clustering algorithms (k-means)
│   │   ├── core/           # Core data structures
│   │   ├── data/           # DataFrame operations
│   │   ├── io/             # File I/O (Optuna Journal)
│   │   ├── mcdm/           # Multi-criteria decision making (TOPSIS, VIKOR, PROMETHEE)
│   │   ├── multi_objective/# Multi-objective optimization
│   │   ├── pdp/            # Partial Dependence Plots
│   │   ├── sensitivity/    # Sensitivity analysis
│   │   └── tests/          # Unit tests
│   └── Cargo.toml
├── egui-app/           # Desktop application (egui UI)
│   ├── src/
│   │   ├── io/         # File dialogs, clipboard
│   │   ├── render/     # Chart rendering
│   │   ├── state/      # Application state
│   │   ├── ui/         # UI components
│   │   ├── app.rs      # Main app logic
│   │   ├── main.rs     # Entry point
│   │   └── theme.rs    # Theming
│   └── Cargo.toml
└── Cargo.toml          # Workspace configuration
```

---

## Available Widgets

All widgets can be freely arranged on the dashboard canvas via drag-and-drop.

| Widget                   | Description                                                                                                                  |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------- |
| **Pareto Scatter 2D**    | 2D scatter plot of the Pareto front. Supports brushing and GPU-accelerated rendering.                                        |
| **Pareto Scatter 3D**    | 3D scatter plot of the Pareto front with arcball camera rotation.                                                            |
| **Optimization History** | Line/scatter chart of objective values over trial number.                                                                    |
| **Hypervolume History**  | Hypervolume indicator history across trials (multi-objective).                                                               |
| **Parallel Coordinates** | Multi-axis parallel coordinates chart for exploring parameter-objective relationships.                                       |
| **Scatter Matrix**       | Pairwise scatter matrix covering all parameters and objectives.                                                              |
| **Importance Chart**     | Parameter importance bar chart. Supported metrics: Spearman, Ridge, RF-Anova, MDI, SHAP, Sobol (First-order / Total-effect). |
| **Sensitivity Heatmap**  | Heatmap of pairwise sensitivities across all parameters and objectives.                                                      |
| **PDP Chart**            | 1-D Partial Dependence Plot for a selected parameter.                                                                        |
| **PDP Chart 2D**         | 2-D Partial Dependence Plot (heatmap/surface) for a pair of parameters.                                                      |
| **Cluster Scatter**      | k-means clustering projected to 2-D via PCA. Target space can be switched between Objective, Variable, or Combined.          |
| **MCDM Ranking**         | Multi-criteria decision making ranking bar chart. Supported methods: TOPSIS, VIKOR.                                          |
| **MCDM Table**           | Sortable ranking table produced by MCDM analysis (TOPSIS / VIKOR).                                                           |
| **Trial Table**          | Full trial data table with sortable columns and row selection.                                                               |

---

## Quick Start

Build and run the desktop application:

```bash
cargo run --release
```

---

## Prerequisites

| Tool                                            | Version        | Purpose               |
| ----------------------------------------------- | -------------- | --------------------- |
| [Rust](https://www.rust-lang.org/tools/install) | stable (1.70+) | Build the application |

Install Rust if not already present:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

---

## Build

### Development Build

```bash
cargo run
```

Builds and runs the application in debug mode.

### Production Build

```bash
cargo build --release
```

Builds an optimized release binary. The executable will be at:

- Windows: `target/release/TunnyDashboard.exe`
- Linux/macOS: `target/release/TunnyDashboard`

### Build specific workspace members

```bash
# Build only the core library
cargo build -p tunny-core

# Build only the desktop app
cargo build -p tunny-desktop
```

---

## Testing

Run tests for the entire workspace:

```bash
cargo test
```

Run tests with output:

```bash
cargo test -- --nocapture
```

Run tests for a specific module:

```bash
cargo test -p tunny-core
```

---

## Formatting

Format all Rust code:

```bash
cargo fmt
```

Check formatting without applying changes:

```bash
cargo fmt -- --check
```

Configuration can be added in `rustfmt.toml` if needed.

---

## License

[MIT](LICENSE)
