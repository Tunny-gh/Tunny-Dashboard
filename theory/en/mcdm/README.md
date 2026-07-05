# MCDM (Multi-Criteria Decision Making)

A set of ranking methods for selecting "overall superior trials" from multi-objective optimization results. Tunny Dashboard provides the following 3 methods plus the Entropy Weight Method for objectively determining weights.

## Method List

| Method                              | Characteristic                                       | Score range    | Cost      |
| ----------------------------------- | ---------------------------------------------------- | -------------- | --------- |
| [TOPSIS](topsis.md)                 | Ranking by distance ratio from ideal solution        | [0, 1]         | O(m × n)  |
| [VIKOR](vikor.md)                   | Compromise solution balancing utility and regret     | [0, 1]         | O(m × n)  |
| [PROMETHEE](promethee.md)           | Ranking by pairwise preference comparison            | Φnet ∈ [-1, 1] | O(m² × n) |
| [Entropy Weight](entropy-weight.md) | Objectively compute weights from data variance       | —              | O(m × n)  |

---

## Method Summaries

### TOPSIS

Scores each trial by how close it is to the "ideal solution (positive ideal solution)". Computes: normalization → weighting → Euclidean distance from ideal/anti-ideal solutions → relative closeness. A score closer to 1 means closer to the ideal solution. Intuitive and fast to compute.

### VIKOR

Measures the "gap from ideal" for each trial using two distances — L1 (Manhattan) and L∞ (Chebyshev) — and adjusts the balance with parameter v to find a compromise solution. v > 0.5 emphasizes overall utility; v < 0.5 minimizes worst-case regret.

### PROMETHEE I / II

Evaluates "how much is a preferred over b?" for every pair of trials using a preference function, and aggregates the results into flows (Φ+/Φ−/Φnet). PROMETHEE I is conceptually a partial ranking (allowing incomparable pairs); in this implementation it is returned as a single total order (Φ+ descending, tiebreak Φ- ascending) without explicitly flagging incomparable pairs. PROMETHEE II provides a complete ranking. Intuitive due to pairwise comparison, but note the O(m²) computational cost.

### Entropy Weight Method

Automatically computes the weight of each objective function from the variance in the data itself. Objectives with greater spread are considered to have higher "discriminatory power" and are assigned larger weights. Can be used as weight input for the 3 methods above.

---

## How to Choose

```
How should weights be set?
├─ Objectively, from data      → Entropy Weight Method (auto-compute weights)
└─ Manually                    → Set weights with manual sliders

Which ranking method?
├─ Fast, intuitive score
│   └─ → TOPSIS (distance from ideal solution, [0,1] score)
│
├─ Consider both overall balance and worst case
│   └─ → VIKOR (adjust utility/regret balance with v parameter)
│
└─ Want detailed pairwise preference relationships
    ├─ Small number of trials (<10,000) → PROMETHEE I/II
    └─ Large number of trials (>10,000) → Note computation time
```

### Recommended Combinations

| Scenario                                         | Recommendation                         |
| ------------------------------------------------ | -------------------------------------- |
| Try first                                        | TOPSIS + equal weights                 |
| Large scale differences between objectives       | TOPSIS/VIKOR + Entropy weights         |
| Minimize worst-case for a specific objective     | VIKOR (v < 0.5)                        |
| Detailed pairwise comparison between trials      | PROMETHEE I + PROMETHEE II             |
| Want objective basis for weights                 | Entropy Weight Method + any method     |
