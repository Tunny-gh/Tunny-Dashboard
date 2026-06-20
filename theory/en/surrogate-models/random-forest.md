# Random Forest Surrogate

## Overview

The Random Forest surrogate uses **LightGBM** (`boosting_type=rf`, Rust FFI via `lgbm.rs`) to build an ensemble of regression trees and predict the response surface. It handles nonlinear, discontinuous, and noisy objectives that Ridge cannot model.

## LightGBM Random Forest Mode

LightGBM's RF mode (`boosting_type=rf`) trains an ensemble of gradient-boosted decision trees, each on an independent bootstrap sample of the data. Predictions are averaged across all trees.

**Default configuration (`LgbmRfConfig`):**

| Parameter           | Value | Notes                                    |
| ------------------- | ----- | ---------------------------------------- |
| `num_iterations`    | 100   | Number of trees in the ensemble          |
| `max_depth`         | 10    | Maximum tree depth                       |
| `min_data_in_leaf`  | 2     | Minimum samples per leaf node            |
| `bagging_fraction`  | 0.8   | Fraction of data sampled per tree        |
| `feature_fraction`  | 0.8   | Fraction of features used per tree       |
| `seed`              | 42    | Fixed seed for reproducibility           |

## PDP Computation

For 1D and 2D partial dependence plots the surrogate **marginalises over all non-target dimensions**: for each grid point the target column(s) are fixed to the grid value in every training row, the model predicts for all those rows, and the average is the PDP value. This is a proper PDP, not a 2D projection.

$$
\hat{y}_\text{PDP}(v) = \frac{1}{N} \sum_{i=1}^{N} \hat{f}(x_i \text{ with target column} = v)
$$

**2D grid predictions:** 50×50 grid × N training rows batched into a single prediction call.

## R² Interpretation

R² is computed on training data using the MSE from predictions on the original training set. It may be optimistic due to overfitting.

| R²    | Action                                           |
| ----- | ------------------------------------------------ |
| > 0.7 | Surface trend is broadly trustworthy             |
| < 0.5 | Consider GP-FITC for smoother fit                |

## Performance

| N (trials) | Training  | Grid prediction (50×50) |
| ---------- | --------- | ----------------------- |
| 50–200     | < 100 ms  | < 50 ms                 |
| 1,000      | < 500 ms  | < 200 ms                |
| 5,000      | < 2,000 ms| < 500 ms                |

## Strengths and Limitations

**Strengths**
- Handles nonlinear and discontinuous objectives
- Robust to outliers (ensemble averaging dilutes their effect)
- No feature scaling required (tree splits are threshold comparisons)
- LightGBM's histogram-based splits are fast on large N

**Limitations**
- Poor extrapolation — constant prediction outside training range
- Stepped / staircase surface compared to Gaussian Process's smooth surface
- Higher cost than Ridge

## When to Use

```
Nonlinear, discontinuous, or noisy objective?  → Random Forest (LightGBM RF backend)
Linear objective?                               → Ridge (faster)
Smooth nonlinear?                               → GP-FITC (higher quality)
Discontinuous / regime-switching?               → GP-MOE
```
