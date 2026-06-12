# Surrogate Models — Overview

Surrogate models fit a function to the trial data, then predict the objective across a dense grid to visualize the response surface (PDP Chart 2D).

## Model Comparison

| Model                  | Speed       | Nonlinear | Best for                        |
| ---------------------- | ----------- | --------- | ------------------------------- |
| Ridge                  | < 100 ms    | No        | Linear responses, any N         |
| Random Forest          | < 2,000 ms  | Yes       | Nonlinear / discontinuous       |
| Gaussian Process       | < 10,000 ms | Yes       | Smooth, any N (trains on all)   |
| Sparse Gaussian Process| < 5,000 ms  | Yes       | Smooth, large N (lower M)       |

## How to Choose

```
Response shape?
  ├─ Linear                    → Ridge (fastest)
  ├─ Nonlinear / noisy         → Random Forest
  └─ Smooth nonlinear
       ├─ Best quality         → Gaussian Process (M = min(N, 100))
       └─ Faster / large N     → Sparse Gaussian Process (M = 20 or 50)
```

## R² Interpretation

All models report R² against training data. Higher is better, but training-set R² can be inflated by overfitting.

| R²    | Action                                          |
| ----- | ----------------------------------------------- |
| ≥ 0.8 | Model fits well; surface is reliable            |
| < 0.5 | Switch to a more expressive model               |
