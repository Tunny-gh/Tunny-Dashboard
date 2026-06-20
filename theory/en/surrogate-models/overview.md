# Surrogate Models — Overview

Surrogate models fit a function to the trial data, then predict the objective across a dense grid to visualize the response surface (PDP Chart 2D).

## Model Comparison

| Model       | Speed        | Nonlinear | Best for                                        |
| ----------- | ------------ | --------- | ----------------------------------------------- |
| Ridge       | < 100 ms     | No        | Linear responses, any N                         |
| Random Forest | < 2,000 ms | Yes       | Nonlinear / discontinuous / noisy               |
| GP-FITC     | < 10,000 ms  | Yes       | Smooth, any N — default GP                      |
| GP-VFE      | < 10,000 ms  | Yes       | Smooth, any N — conservative/smoother fit       |
| GP-MOE      | < 30,000 ms  | Yes       | Discontinuous / regime-switching / multi-modal  |

All GP variants (GP-FITC, GP-VFE, GP-MOE) are backed by egobox-gp / egobox-moe (Apache-2.0) and use M = min(N, 100) inducing points. When N ≤ 100 this is mathematically equivalent to an exact GP with noise estimation.

## How to Choose

```
Response shape?
  ├─ Linear                              → Ridge (fastest)
  ├─ Nonlinear / noisy / tabular         → Random Forest (LightGBM RF backend)
  └─ Smooth nonlinear
       ├─ Default                        → GP-FITC
       ├─ Surface looks overfit/spiky    → GP-VFE (smoother, more conservative)
       └─ Discontinuous / multi-regime   → GP-MOE
```

## R² Interpretation

All models report R² against training data. Higher is better, but training-set R² can be inflated by overfitting.

| R²    | Action                                          |
| ----- | ----------------------------------------------- |
| ≥ 0.8 | Model fits well; surface is reliable            |
| < 0.5 | Switch to a more expressive model               |
