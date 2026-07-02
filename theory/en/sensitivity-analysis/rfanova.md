# RF-ANOVA (fANOVA: Functional ANOVA)

## Overview

RF-ANOVA (Random Forest ANOVA) computes the **functional ANOVA (fANOVA)** method of Hutter et al. (2014) on a random forest. It quantifies parameter importance by **exactly marginalizing the variance of the objective function into per-parameter main effects, using the leaf-node intervals (boxes) of trained regression trees.**

> **Relationship to Optuna:** this implementation is the same algorithmic family as Optuna's `FanovaImportanceEvaluator` (Hutter et al. 2014), but tree-training details (split search, stopping criteria, number of trees, etc.) differ, so **the numeric values do not match exactly**.

---

## Theoretical Background

### Basic Principle

Each tree $T$ in the random forest can be viewed as a piecewise-constant function that partitions the parameter space into axis-aligned intervals (leaf boxes). Let leaf $\ell$'s box be $B_\ell = \prod_d B_{\ell d}$ and its prediction $y_\ell$ (the mean of in-leaf samples).

Assuming a uniform prior over the observed range of each parameter $d$, $\mathrm{range}_d = [\min_d, \max_d]$, the weight of leaf $\ell$ is

$$
w_\ell = \prod_d \frac{\mathrm{len}(B_{\ell d} \cap \mathrm{range}_d)}{\mathrm{len}(\mathrm{range}_d)}
$$

(a dimension with a degenerate range contributes a ratio of 1).

The tree's expectation and total variance are

$$
f_0 = \sum_\ell w_\ell y_\ell, \qquad V = \sum_\ell w_\ell y_\ell^2 - f_0^2
$$

The **main-effect variance** $V_j$ of parameter $j$ is defined as the variance of the one-dimensional function obtained by marginalizing over every other dimension. For dimension $j$, over each elementary interval $I$ delimited by the endpoints of all leaf boxes,

$$
f_j(I) = \sum_{\ell:\, I \subseteq B_{\ell j}} \left( \prod_{d \neq j} \frac{\mathrm{len}(B_{\ell d} \cap \mathrm{range}_d)}{\mathrm{len}(\mathrm{range}_d)} \right) y_\ell
$$

and

$$
V_j = \sum_I \frac{\mathrm{len}(I)}{\mathrm{len}(\mathrm{range}_j)} \left( f_j(I) - f_0 \right)^2
$$

This is not an approximation — it is an **exact** variance decomposition over the tree's leaf partition (equivalent to Algorithm 2 in Hutter et al. 2014).

The per-tree importance is $V_j / V$ (trees with $V < 10^{-12}$, i.e. essentially constant output, are skipped). The forest importance is the mean of $V_j/V$ over the non-skipped trees, normalized to sum to 1.

---

## Tunny Dashboard's Implementation

### CART Regression Forest

Because LightGBM does not expose leaf-node boxes cleanly, this method uses a dedicated pure-Rust CART regression tree implementation.

- **Bootstrap sampling**: n-of-n sampling with replacement per tree (deterministic seed: `RF_ANOVA_SEED + tree_index`)
- **Split search**: considers all features; candidate thresholds are midpoints between consecutive distinct sorted values within the node, and the split minimizing weighted child variance is chosen (no feature subsampling — exact main-effect decomposition depends on the tree's axis-aligned partition exactly covering the parameter space, so using all features keeps the decomposition exact rather than approximate)
- **Stopping criteria**: a node becomes a leaf when `max_depth` is reached, the node has fewer than `2 * min_samples_leaf` samples, or no valid split exists (constant y or all features constant)
- **Leaf boxes**: initialized from the full training data's per-dimension observed range (before bootstrapping) and progressively shrunk by ancestor split constraints

### Pipeline

```
Actual trial data
    ↓
Step 1: Preprocessing
  ├── Drop NaN/Inf rows
  └── Downsample to 2000 rows if N > 2000

Step 2: 80/20 holdout split
  ├── Shuffle, then 80% train / 20% eval
  └── If N < 4, use all data for both train and eval

Step 3: Train the CART regression forest (64 trees, on train split)
  └── Each tree is trained on an independent bootstrap sample

Step 4: fANOVA-decompose each tree
  ├── Compute leaf weights w_ℓ from leaf boxes and training data ranges
  ├── Compute f_0, V (skip trees with V < 1e-12)
  └── Compute V_j per dimension via piecewise quadrature

Step 5: Forest-level importance
  ├── Average V_j/V over non-skipped trees
  └── Normalize to sum to 1

Step 6: Compute R²
  ├── Predict eval data with the forest (mean of per-tree leaf values)
  └── R² = 1 - MSE_eval × N_eval / SS_total (clipped to 0 if negative)

Output: (importances[P], r_squared)
```

### Hyperparameters

| Parameter | Value | Notes |
| --- | --- | --- |
| Trees | 64 | Matches Optuna's fanova default `n_trees=64` |
| Max depth | 10 | Optuna uses `max_depth=64` (effectively unbounded), but fANOVA decomposition cost grows with leaf count, so depth is deliberately capped at 10 as a compute/accuracy trade-off |
| Min leaf samples | 2 | Same as MDI/SHAP |
| Random seed | 42 | Per-tree seed is `42 + tree_index` |
| Max rows | 2,000 | MDI uses 1,000 |

---

## R² Interpretation

RF-ANOVA's R² is the **coefficient of determination of the forest's predictions on holdout data**:

$$
R^2 = 1 - \frac{\mathrm{MSE}_{\mathrm{eval}} \times N_{\mathrm{eval}}}{\sum_i (y_i - \bar{y})^2}
$$

| R² | Meaning |
| --- | --- |
| ≥ 0.8 (green) | Good fit. Scores are reliable. |
| 0.5–0.8 (yellow) | Moderate. Use as reference. |
| < 0.5 (red) | Poor fit. Scores are unreliable. |

---

## Comparison with MDI

| Aspect | RF-ANOVA (fANOVA) | MDI |
| --- | --- | --- |
| Measured | **Post-training** variance decomposition over leaf boxes | **During training** (recorded while building trees) |
| Cost | Tree training + per-tree variance decomposition (scales with leaf count) | O(N·P·D) per tree |
| Bias | Does not attribute interaction effects to a single parameter's main effect | Tends to overestimate high-cardinality features |
| Interpretation | "Fraction of the objective's variance explained by this parameter's main effect" | "How useful this parameter was during training" |

---

## Reference

F. Hutter, H. Hoos, K. Leyton-Brown, "An Efficient Approach for Assessing Hyperparameter Importance", ICML 2014.
