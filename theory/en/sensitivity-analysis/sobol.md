# Sobol Sensitivity Indices

## Overview

Sobol sensitivity indices quantify parameter importance through **variance decomposition** of the objective function. They work for linear and nonlinear functions alike and can measure **interaction effects** between parameters. Values are in [0, 1].

Tunny Dashboard provides two indices:

| Index | Symbol | Meaning |
| --- | --- | --- |
| First-order | $S_i$ | Fraction of variance explained by $x_i$ alone |
| Total-effect | $ST_i$ | $x_i$ alone plus all interactions involving $x_i$ |

## Theory

The objective's variance is decomposed as:

$$
\operatorname{Var}(Y) = \sum_i V_i + \sum_{i<j} V_{ij} + \cdots + V_{1..p}
$$

First-order index:

$$
S_i = \frac{V_i}{\operatorname{Var}(Y)}
$$

Total-effect index:

$$
ST_i = 1 - \frac{\operatorname{Var}(E[Y \mid X_{\sim i}])}{\operatorname{Var}(Y)}
$$

where $X_{\sim i}$ means "all parameters except $x_i$."

$ST_i \geq S_i$ always holds. A large gap $(ST_i - S_i)$ indicates strong interactions.

> **Note on finite-sample estimates:** the Saltelli first-order estimator and the Jansen total-effect estimator are computed independently, so raw estimates can violate $\hat{ST}_i \geq \hat{S}_i$ in finite samples. The implementation enforces the property after estimation by taking $\hat{ST}_i \leftarrow \max(\hat{ST}_i, \hat{S}_i)$ before clamping both values to $[0, 1]$.

## Implementation

Because Monte Carlo integration is impractical for real trial data, a **quadratic Ridge surrogate** is fitted on the trials, and Saltelli sampling is run on that surrogate using the Jansen estimator.

## Parameters

| Setting | Value |
| --- | --- |
| Saltelli sample count $N$ | 1,024 |
| Ridge regularization strength $\alpha$ | 1.0 |
| Random number generator | ChaCha8 (deterministic, seed `0xDEAD_BEEF_1234_5678`) |

## Interpreting Results

- $S_i$ high, $ST_i \approx S_i$ → parameter has a large independent effect, few interactions.
- $S_i$ small, $ST_i$ high → parameter mainly acts through interactions with others.
- Both near zero → parameter has little influence on the objective.

## Strengths and Limitations

**Strengths:**
- Handles linear, nonlinear, and interaction effects.
- Values in [0, 1] allow easy comparison across parameters.
- $ST - S$ quantifies interaction strength.

**Limitations:**
- Results depend on surrogate model quality (quadratic Ridge).
- Accuracy decreases when the true function is highly nonlinear.
- Categorical parameters are label-encoded (string labels → integer IDs 0.0, 1.0, …); Sobol indices for categoricals are approximate because integer IDs carry no ordinal/distance information.

## When to Use

- When interactions between parameters are important.
- When a global, model-agnostic sensitivity measure is needed.
- After Spearman / Ridge screening identifies top candidates.
