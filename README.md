# Tunny Dashboard

A desktop analytics dashboard for [Optuna](https://optuna.org/) optimization results, built with Rust/egui.

---

## Overview

Tunny Dashboard reads Optuna studies locally on your desktop — no server, no Python installation required.

Supported storage sources:

- **Journal log files** (`JournalFileBackend`)
- **SQLite** databases (`sqlite:///study.db`)
- **PostgreSQL / MySQL** RDB storage (connection URL, SQLAlchemy-style URLs accepted)
- **Flat CSV** import for non-Optuna data

All sources support **live update**: running studies are polled via a lightweight fingerprint check and reloaded automatically as new trials arrive.

<img width="800" height="516" alt="1782045917513" src="https://github.com/user-attachments/assets/ab008af2-0556-4c10-9ff7-1bc50f0a595f" />

### Key Features

- **High-performance data processing** — Journal/SQLite/RDB parsing and DataFrame operations run in native Rust
- **Interactive visualizations** — 30+ widgets: Pareto front (2D/3D), Parallel Coordinates, Scatter Matrix, learning curves, EDF/Timeline/Rank plots, and more
- **Multi-criteria decision making (MCDM)** — TOPSIS, VIKOR, PROMETHEE I/II rankings with equal or entropy-based objective weighting
- **Multi-objective convergence indicators** — Hypervolume, IGD+, ε-indicator, and R2 histories
- **Sensitivity analysis** — Spearman, Ridge, RF-ANOVA, MDI, SHAP, Permutation, Sobol (first-order / total-effect), GP-ARD
- **Surrogate models & optimization** — response surface (3D), candidate suggestion via CMA-ES / NSGA-II / EHVI, robustness (MC noise propagation) analysis
- **Clustering & projection** — k-means, hierarchical (dendrogram), PCA biplot, SOM map
- **Self-contained report export** — one-file HTML / Markdown / JSON reports with key findings, charts (SVG), statistics, and MCDM rankings; light/dark, English/Japanese
- **MCP server for AI agents** — `tunny-mcp` exposes studies, reports, and trial data to LLM agents (Claude Code, etc.) via the Model Context Protocol
- **Dark mode** — light/dark theme toggle, persisted with the session
- **Brushing & Linking** — cross-chart selection synchronized across all views in real time
- **Free layout** — drag-and-drop chart arrangement
- **Multi-study comparison** — overlay or side-by-side comparison of multiple studies
- **Artifacts gallery** — browse Optuna artifacts (images) attached to trials
- **CSV export** — export selected trials with customizable columns
- **Session persistence** — save and restore the full UI state as a JSON file
- **Bilingual help** — every widget links straight to its page on the online documentation site (English / 日本語), where each algorithm is documented and cross-validated against Python reference implementations

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
├── rust_core/          # Core library (headless analytics)
│   ├── src/
│   │   ├── clustering/     # k-means, hierarchical, PCA, SOM
│   │   ├── contour/        # Observed contour interpolation
│   │   ├── data/           # DataFrame operations
│   │   ├── gh/             # Grasshopper (.ghx) → Rhino.Compute execution
│   │   ├── io/             # Journal / SQLite / PostgreSQL / MySQL / CSV / artifacts
│   │   ├── mcdm/           # TOPSIS, VIKOR, PROMETHEE I·II, entropy weights
│   │   ├── process/        # Generic external-tool objective (command integration)
│   │   ├── runner/         # Self-contained optimization loop (no Python/Optuna)
│   │   ├── multi_objective/# Pareto ranking, Hypervolume, IGD+, ε, R2
│   │   ├── pdp/            # Partial Dependence Plots (1D/2D, GP-backed)
│   │   ├── report/         # Self-contained HTML/Markdown/JSON report export
│   │   ├── sensitivity/    # Spearman, Ridge, tree-based, SHAP, Sobol, ARD
│   │   ├── statistics/     # Histograms, quantiles, correlations
│   │   ├── surrogate_opt/  # Candidate suggestion, robustness analysis
│   │   └── gaussian_process.rs # Sparse GP (FITC/VFE) + MoE
│   └── Cargo.toml
├── egui-app/           # Desktop application (egui UI)
│   ├── src/
│   │   ├── io/         # File dialogs, storage readers, live update, export
│   │   ├── state/      # Application state & message handling
│   │   ├── theme/      # Light/dark color palette & colormaps
│   │   ├── ui/         # Panels, canvas, widgets, help system
│   │   ├── app.rs      # Main app logic
│   │   └── main.rs     # Entry point
│   └── Cargo.toml
├── mcp-server/         # MCP server (tunny-mcp) for LLM/agent integration
├── docs/               # User guides (execution / integration)
└── Cargo.toml          # Workspace configuration
```

---

## Running Optimizations

Beyond analyzing existing studies, the Dashboard can **drive** an optimization
itself: the samplers (Random / NSGA-II) run in Rust and the Dashboard writes an
Optuna-compatible journal, so a run needs only the Dashboard and the tool that
evaluates the objective — **no Python or Optuna at runtime**.

- **Grasshopper** — drop a Tunny-configured `.ghx` onto the Dashboard to run the
  optimization via Rhino.Compute. See [docs/tunny-plugin-integration.md](docs/tunny-plugin-integration.md).
- **Any external tool** — describe how a command receives parameters and how its
  output is parsed, and optimize it directly. See
  [docs/process-integration.md](docs/process-integration.md).

---

## Available Widgets

All widgets can be freely arranged on the dashboard canvas via drag-and-drop.

### Optimization results

| Widget                     | Description                                                                             |
| -------------------------- | ---------------------------------------------------------------------------------------- |
| **Pareto Scatter 2D**      | 2D scatter plot of the Pareto front. Supports brushing and GPU-accelerated rendering.    |
| **Pareto Scatter 3D**      | 3D scatter plot of the Pareto front with arcball camera rotation.                        |
| **Optimization History**   | Line/scatter chart of objective values over trial number.                                |
| **Convergence Indicators** | Multi-objective convergence histories: Hypervolume, IGD+, ε-indicator, R2.               |
| **Intermediate Values**    | Learning curves of intermediate values, including PRUNED trials (pruning analysis).      |
| **EDF Plot**               | Empirical distribution function of objective values.                                     |
| **Timeline**               | Trial states over wall-clock time (`datetime_start` → `datetime_complete`).              |
| **Rank Plot**              | Parameter vs. objective-rank scatter.                                                    |

### Exploration

| Widget                   | Description                                                                                                        |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------ |
| **Parallel Coordinates** | Multi-axis parallel coordinates chart for exploring parameter-objective relationships.                             |
| **Scatter Matrix**       | Pairwise scatter matrix covering all parameters and objectives.                                                    |
| **Observed Contour**     | Interpolated contour of observed objective values over a parameter pair.                                           |
| **Histogram**            | Distribution histogram for any column.                                                                             |
| **Box Plot**             | Per-column box plots, optionally grouped.                                                                          |
| **Correlation Matrix**   | Spearman correlation heatmap across parameters and objectives.                                                     |
| **Trial Table**          | Full trial data table with sortable columns and row selection.                                                     |
| **Artifact Gallery**     | Image artifacts attached to trials, linked to selection.                                                           |

### Sensitivity & surrogate models

| Widget                  | Description                                                                                                                            |
| ----------------------- | ---------------------------------------------------------------------------------------------------------------------------------------|
| **Importance Chart**    | Parameter importance bar chart. Metrics: Spearman, Ridge, RF-ANOVA, MDI, SHAP, Permutation, Sobol (First-order / Total-effect), GP-ARD. |
| **Sensitivity Heatmap** | Heatmap of pairwise sensitivities across all parameters and objectives.                                                                |
| **PDP Chart**           | 1-D Partial Dependence Plot for a selected parameter (GP-backed, with uncertainty).                                                    |
| **PDP Chart 2D**        | 2-D Partial Dependence Plot (heatmap/surface) for a pair of parameters.                                                                |
| **Slice Chart**         | Objective vs. single-parameter slice with trial ordering.                                                                              |
| **Response Surface 3D** | Surrogate-model response surface over a parameter pair.                                                                                |
| **Surrogate Optimizer** | Candidate suggestion on the surrogate (CMA-ES / NSGA-II / EHVI) with what-if exploration.                                              |
| **Robustness**          | Monte-Carlo noise propagation around a design point.                                                                                   |

### Clustering & projection

| Widget                 | Description                                                                                                          |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------------|
| **Cluster Scatter**    | k-means clustering projected to 2-D via PCA. Target space: Objective, Variable, or Combined.                         |
| **Cluster Scatter 3D** | 3-D projection of the clustering result.                                                                             |
| **Dendrogram**         | Hierarchical clustering dendrogram.                                                                                  |
| **PCA Biplot**         | PCA scores + loadings biplot.                                                                                        |
| **SOM Map**            | Self-Organizing Map (U-matrix / component planes).                                                                   |

### Decision making

| Widget                 | Description                                                                                        |
| ---------------------- | ---------------------------------------------------------------------------------------------------|
| **MCDM Ranking**       | Ranking bar chart. Methods: TOPSIS, VIKOR, PROMETHEE I/II; equal or entropy weights.               |
| **MCDM Scatter 2D/3D** | Ranked trials in objective space, colored by MCDM score.                                           |
| **Comparison Table**   | Side-by-side comparison of selected trials.                                                        |
| **Radar Comparison**   | Radar chart comparison of selected trials across objectives.                                       |

### Export & reporting

- **Report export** — self-contained HTML / Markdown / JSON report of the whole study (key findings, Pareto front with constraint handling, convergence, importance, statistics, correlations, MCDM consensus). English/Japanese, light/dark, no external resources.
- **CSV export** — selected trials with customizable columns.

---

## MCP Server (LLM integration)

`tunny-mcp` exposes the same headless analytics to AI agents via the
[Model Context Protocol](https://modelcontextprotocol.io/) (stdio transport,
tools capability). An agent can list studies, pull an LLM-optimized Markdown
report, and page through raw trial data — against journal files, SQLite, or
PostgreSQL/MySQL storage.

```bash
cargo build --release -p tunny-mcp

# Register with Claude Code:
claude mcp add tunny -- /path/to/target/release/tunny-mcp
```

| Tool            | Description                                                                  |
| --------------- | ---------------------------------------------------------------------------- |
| `list_studies`  | Studies in a storage: id, name, directions, parameters, trial counts         |
| `study_summary` | Compact JSON summary: overview + key findings + convergence status           |
| `study_report`  | Full analysis report (Markdown for LLM consumption, or structured JSON)      |
| `trials`        | Raw COMPLETE-trial rows (objectives / params / constraints) with pagination  |

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
