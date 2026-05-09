# Surrogate Models — Overview

Surrogate models fit a function to the trial data, then predict the objective across a dense grid to visualize the response surface (PDP Chart 2D).

## Model Comparison

| Model         | Speed       | Nonlinear | Best for                   |
| ------------- | ----------- | --------- | -------------------------- |
| Ridge         | < 100 ms    | No        | Linear responses, any N    |
| Random Forest | < 2,000 ms  | Yes       | Nonlinear / discontinuous  |
| Kriging       | < 10,000 ms | Yes       | Smooth, N ≤ 500            |
| Sparse Kriging| < 5,000 ms  | Yes       | Smooth, N ≤ 5,000          |

## How to Choose

```
Response shape?
  ├─ Linear          → Ridge (fastest)
  ├─ Nonlinear / noisy / outliers → Random Forest
  └─ Smooth nonlinear
       ├─ N ≤ 500    → Kriging (highest quality)
       └─ N ≤ 5,000  → Sparse Kriging (fast + quality balance)
```

## R² Interpretation

All models report R² against training data. Higher is better, but training-set R² can be inflated by overfitting.

| R²    | Action                                          |
| ----- | ----------------------------------------------- |
| ≥ 0.8 | Model fits well; surface is reliable            |
| < 0.5 | Switch to a more expressive model               |
