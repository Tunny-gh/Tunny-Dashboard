# Random Forest Surrogate

## Overview

Random Forest is an ensemble of CART regression trees trained on bootstrap samples. Each tree learns a different subset of the data; their average prediction reduces variance and handles nonlinear, discontinuous objectives that Ridge cannot model.

## CART Decision Tree

Each tree uses Mean Squared Error (MSE) splitting:

$$
\text{Gain}(j, t) = \text{MSE}(y) - \left[\frac{n_L}{n} \cdot \text{MSE}(y_L) + \frac{n_R}{n} \cdot \text{MSE}(y_R)\right]
$$

Pick the split (j, t) that maximizes Gain. Leaf nodes return the mean of their samples.

**Stopping conditions:**

| Condition               | Meaning                          |
| ----------------------- | -------------------------------- |
| depth ≥ max_depth (10)  | Reached maximum tree depth       |
| n ≤ min_samples_leaf (2)| Too few samples to split further |
| No valid split          | All thresholds violate min-leaf  |

## Bagging

$$
\hat{y}(x) = \frac{1}{B} \sum_{b=1}^{B} T_b(x) \quad (B = 100 \text{ trees})
$$

Bootstrap sampling uses LCG pseudo-random numbers (Knuth's constants — no external crate). Each tree's independent variance contributes 1/100 of a single tree's variance in the ensemble.

## 2D Projection for PDP

Projects all trials onto 2 selected parameters, fits a 2D Random Forest, then predicts on a 50×50 grid:

$$
\text{values}[i][j] = \text{RF.predict}([g_1[i], g_2[j]])
$$

Total grid predictions: 2,500 × 100 trees × depth 10 ≈ 2.5M operations.

## R² Interpretation

R² is computed on training data (may be inflated due to overfitting). Use it directionally: if R² < 0.7, the surface trend may not be reliable.

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

**Limitations**
- Poor extrapolation — constant prediction outside training range
- Stepped / staircase surface compared to Gaussian Process's smooth surface
- Higher cost than Ridge

## When to Use

```
Nonlinear, discontinuous, or noisy objective?  → Random Forest (or LightGBM)
Linear objective?                               → Ridge (faster)
Smooth nonlinear?                               → GP-FITC (higher quality)
Discontinuous / regime-switching?               → GP-MOE
```
