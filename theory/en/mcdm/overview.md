# MCDM — Multi-Criteria Decision Making

MCDM methods rank optimization trials by aggregating multiple objective functions into a single score. Tunny Dashboard provides three ranking methods and an objective-weight calculator.

## Method Comparison

| Method | Approach | Score range | Cost |
| --- | --- | --- | --- |
| TOPSIS | Distance from ideal / anti-ideal solution | [0, 1] | O(m × n) |
| VIKOR | Compromise between utility and regret | [0, 1] | O(m × n) |
| PROMETHEE | Pairwise preference comparison | Φnet ∈ [−1, 1] | O(m² × n) |
| Entropy Weight | Auto-compute weights from data variance | — | O(m × n) |

## Method Summaries

### TOPSIS
Scores each trial by how close it is to the ideal solution (best possible) and how far it is from the anti-ideal solution (worst possible). Score 1 = ideal, Score 0 = worst. Fast and intuitive.

### VIKOR
Measures the "gap from ideal" using both L1 (Manhattan) and L∞ (Chebyshev) distances. Parameter v (0–1) balances overall utility vs. worst-case regret. Lower Q score = better compromise solution.

### PROMETHEE I / II
Evaluates every trial pair (a, b) — "how much is a preferred over b?" — and aggregates into positive flow (Φ+), negative flow (Φ−), and net flow (Φnet). PROMETHEE I gives a partial ranking; PROMETHEE II gives a complete ranking.

### Entropy Weight Method
Automatically computes objective weights from the variance (information content) of each objective across trials. Higher variance = higher weight. Use when you want data-driven, objective weights without manual input.

## How to Choose

```
How should weights be set?
├─ Objectively, from data      → Entropy Weight Method
└─ Manually                    → Weight sliders

Which ranking method?
├─ Fast, intuitive [0,1] score → TOPSIS
├─ Balance utility & regret    → VIKOR (adjust v parameter)
└─ Pairwise preference detail  → PROMETHEE I / II
```

## Recommended Combinations

| Scenario | Recommendation |
| --- | --- |
| Quick start | TOPSIS + equal weights |
| Large scale differences between objectives | TOPSIS/VIKOR + Entropy weights |
| Minimize worst-case objective | VIKOR (v < 0.5) |
| Detailed pairwise comparison | PROMETHEE I + II |
| Objective weights from data | Entropy Weight + any method |
