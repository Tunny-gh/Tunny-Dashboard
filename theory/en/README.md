# Tunny Dashboard Analysis Methods

Theoretical reference for the statistical, sensitivity, and multi-criteria decision-making methods provided by Tunny Dashboard.

---

## Parameter Importance Methods

The 9 parameter sensitivity metrics used by `ImportanceChart` / the Sensitivity Heatmap.

| Display name | Method | Sign | Notes |
| ------------ | ------ | ---- | ----- |
| Spearman | Spearman rank correlation | signed | Non-parametric; handles monotone non-linearity |
| Ridge | Ridge regression coefficient | signed | Assumes a linear relationship; intuitive to read |
| RF-Anova | Random Forest hold-out accuracy drop | non-negative | Close to real-world contribution; unstable under correlated features |
| MDI | Mean Decrease in Impurity | non-negative | Split contribution during training; over-rates high-cardinality features |
| Sobol First $S_i$ | First-order Sobol index | non-negative | Main effect only; excludes interactions |
| Sobol Total $ST_i$ | Total-effect Sobol index | non-negative | Total influence including interactions |
| SHAP | Shapley-value contribution | non-negative (mean abs) | Theoretically consistent; strong explainability |
| Permutation | Repeated-average permutation importance (PFI) | non-negative | Lower variance than RF-ANOVA; higher cost |
| ARD | GP length-scale relevance | non-negative | Global sensitivity from a trained GP surrogate's length scales |

> Only Spearman / Ridge are signed (can be negative). Tree-based, Sobol, SHAP, Permutation, and ARD are non-negative. Only Spearman / Ridge are cheap; the rest require model training or Sobol sampling.

### Method details

- [Spearman rank correlation](sensitivity-analysis/spearman.md)
- [Ridge regression coefficient](sensitivity-analysis/ridge.md)
- [RF-ANOVA](sensitivity-analysis/rfanova.md)
- [MDI (Mean Decrease in Impurity)](sensitivity-analysis/mdi.md)
- [Sobol sensitivity indices](sensitivity-analysis/sobol.md)
- [SHAP](sensitivity-analysis/shap.md)
- [Permutation importance](sensitivity-analysis/permutation.md)
- [ARD importance](sensitivity-analysis/ard-importance.md)
- [Sensitivity overview](sensitivity-analysis/overview.md)

### How to choose

```
The relationship with the objective is...

  close to linear ───────────────────→ Ridge
  monotone but non-linear ───────────→ Spearman
  tree-model feature importance ─────→ MDI (cheap) / RF-ANOVA / Permutation (stable)
  accountability / explainability ───→ SHAP
  non-linear with interactions ──────→ Sobol ST_i
  pure main effect (no interaction) ─→ Sobol S_i
  free reuse of a trained GP ────────→ ARD
```

When the number of parameters $p$ is large ($p \ge 20$), Sobol and tree-based methods become expensive — screen first with Spearman/Ridge, then apply the heavier methods.

---

## Multi-Criteria Decision Making

MCDM methods that aggregate multiple objectives into a single score to rank trials.

| Method | Range | Notes |
| ------ | ----- | ----- |
| [TOPSIS](mcdm/topsis.md) | $[0,1]$ | Ranks trials by the distance ratio to the ideal / anti-ideal solution |
| [VIKOR](mcdm/vikor.md) | $[0,1]$ (Q) | Compromise solution balancing utility and regret; ranked by ascending Q |
| [PROMETHEE I/II](mcdm/promethee.md) | Φnet ∈ [-1, 1] | Pairwise preference comparison. I: partial ranking; II: full ranking by descending Φnet |
| [Entropy Weight](mcdm/entropy-weight.md) | — | Objectively derives objective weights from the data's dispersion |

### Method details

- [TOPSIS](mcdm/topsis.md)
- [VIKOR](mcdm/vikor.md)
- [PROMETHEE I / II](mcdm/promethee.md)
- [Entropy Weight](mcdm/entropy-weight.md)
- [MCDM overview](mcdm/overview.md)

### How to choose

```
You want to rank trials holistically in multi-objective optimization
  ↓
How are the weights decided?
├─ objectively → Entropy Weight derives them automatically
└─ manually    → set weights with the manual sliders

Which ranking method?
├─ fast, intuitive score      → TOPSIS ([0,1] score)
├─ balance + worst-case       → VIKOR (tune with the v parameter)
└─ detailed pairwise ordering → PROMETHEE I/II

To see every solution on the Pareto front, use the
Pareto Front chart alongside MCDM.
```

---

## Response Surface / Partial Dependence Plots

Surrogate-model-based visualization used by `PdpChart2DState` (2D PDP) and `PdpChart` (1D PDP).

### Surrogate model options

| Model | Speed (release) | Non-linear | Few samples | Use case |
| ----- | --------------- | ---------- | ----------- | -------- |
| Ridge regression | < 100 ms | ✗ (linear only) | ○ | Linear response |
| Random Forest | < 2,000 ms | ✓ (incl. discontinuous) | △ | Non-linear / noisy |
| GP-FITC | < 10,000 ms | ✓ (smooth) | ◎ | Smooth non-linear; default GP |
| GP-VFE | < 10,000 ms | ✓ (smooth) | ◎ | Smooth non-linear; when GP-FITC overfits |
| GP-MOE | < 30,000 ms | ✓ (discontinuous / multi-region) | ○ | Discontinuous / regime-switching |

Random Forest uses LightGBM's RF mode (`boosting_type=rf`) as its backend (not a separate model). The GP variants (GP-FITC / GP-VFE / GP-MOE) all use the egobox-gp / egobox-moe (Apache-2.0) backend with M = min(N, 100) inducing points. Training always uses all N points (no data subsampling).

### Method details

- [Surrogate models overview](surrogate-models/overview.md)
- [Partial dependence plots (PDP) response surface](sensitivity-analysis/pdp.md)
- [Ridge surrogate model](surrogate-models/ridge.md)
- [Random Forest surrogate model (LightGBM RF)](surrogate-models/random-forest.md)
- [Gaussian Process (GP-FITC / GP-VFE) surrogate model](surrogate-models/gaussian-process.md)
- [Gaussian Process Mixture-of-Experts (GP-MOE) surrogate model](surrogate-models/gaussian-process-moe.md)

### How to choose

```
You want to see the "shape" of a parameter's effect on the objective
  ↓
1 parameter of interest → PdpChart (1D)
2 parameters of interest → PdpChart2DState (2D)
  ↓
Surrogate model choice:
  fast check                 → Ridge regression
  non-linear / discontinuous → Random Forest (LightGBM RF)
  smooth, default            → GP-FITC (trains on all N points)
  GP-FITC overfits           → GP-VFE (smoother / conservative)
  discontinuous, multi-region→ GP-MOE

If $R^2$ is low (< 0.5), the relationship is strongly non-linear → check with Random Forest / GP-FITC / Sobol.
```

---

## Optimization

Algorithms used to optimize on the surrogate model.

- [L-BFGS (Limited-memory BFGS)](optimization/lbfgs.md)
- [**Acquisition functions (Expected Improvement / Lower Confidence Bound)**](optimization/acquisition-functions.md)
- [**Expected Hypervolume Improvement (EHVI)**](optimization/ehvi.md)
- [**Hypervolume (WFG algorithm)**](optimization/hypervolume.md)
- [**Robustness analysis (Monte Carlo noise propagation)**](optimization/robustness-analysis.md)

---

## Clustering

Clustering-related methods used by the `ClusterScatter` widget.

| Method | Role | Details |
|--------|------|---------|
| k-means | Partitions data into $k$ clusters (Lloyd's algorithm) | [clustering/kmeans.md](clustering/kmeans.md) |
| Elbow method | Auto-estimates the optimal cluster count $k$ (WCSS second difference) | [clustering/elbow.md](clustering/elbow.md) |
| [Overview](clustering/overview.md) | Clustering pipeline summary | [clustering/overview.md](clustering/overview.md) |

---

## Foundational Statistics

Basic statistical measures referenced in common by several widgets and analysis methods.

| Method | Role | Details |
|--------|------|---------|
| Pearson product-moment correlation | Linear correlation of two variables (scatter-matrix correlation; internal to Spearman) | [statistics/pearson-correlation.md](statistics/pearson-correlation.md) |
| Histogram | Univariate distribution summary via binning (skewness, multimodality, outliers) | [statistics/histogram.md](statistics/histogram.md) |
| Box Plot | Five-number-summary distribution comparison across variables/clusters | [statistics/box-plot.md](statistics/box-plot.md) |
| Correlation Matrix | Heatmap overview of pairwise correlation across all variables | [statistics/correlation-matrix.md](statistics/correlation-matrix.md) |

---

## Data I/O

Optuna storage formats this app can read and how it interprets them.

| Topic | Role | Details |
|--------|------|---------|
| Optuna storage formats | Journal / RDB (SQLite) storage structure and this app's schema-interpretation rules | [io/optuna-storages.md](io/optuna-storages.md) |
| Session files | What a `.tunny` session persists (view state), what it excludes (data / derived state), and schema-evolution policy | [io/session-files.md](io/session-files.md) |

---

## Widgets

UI charts/panels and the quantities they display.

- [Pareto 2D](widgets/pareto-2d.md)
- [Pareto 3D](widgets/pareto-3d.md)
- [Parallel coordinates](widgets/parallel-coords.md)
- [Optimization history](widgets/optimization-history.md)
- [Convergence](widgets/convergence.md)
- [Trial table](widgets/trial-table.md)
- [Scatter matrix](widgets/scatter-matrix.md)
- [Observed contour](widgets/observed-contour.md)
- [Slice chart](widgets/slice-chart.md)
- [Artifact gallery](widgets/artifact-gallery.md)
- [Surrogate optimizer](widgets/surrogate-optimizer.md)

---

## Overall method map

```
I want to analyze optimization results
  │
  ├── Parameter importance
  │    ├── quick check    → Spearman / Ridge (ImportanceChart)
  │    └── accurate check → Sobol / tree-based / SHAP (ImportanceChart, higher cost)
  │
  ├── Pick good trials
  │    ├── holistic multi-objective score → TOPSIS / VIKOR / PROMETHEE (MCDM chart)
  │    └── full trade-off                 → Pareto Front (ParetoFront chart)
  │
  └── Visualize parameter–objective relationships
       ├── 1 parameter → 1D PDP (PdpChart)
       └── 2 parameters → 2D PDP (PdpChart2DState)
            ├── fast / linear              → Ridge regression
            ├── non-linear / discontinuous → Random Forest (LightGBM RF)
            ├── smooth / default           → GP-FITC (trains on all N points)
            ├── smooth / overfitting       → GP-VFE (conservative fit)
            └── discontinuous, multi-region→ GP-MOE
```
