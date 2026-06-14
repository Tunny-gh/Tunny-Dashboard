# ARD Parameter Importance (GP length scales)

## Overview

When a Gaussian Process (GP) surrogate is fitted with an **ARD** (Automatic Relevance Determination) kernel, it learns one length-scale hyperparameter per input dimension. The relative magnitude of these hyperparameters is a cheap, global measure of how sensitive the response surface is to each parameter.

In the dashboard this is exposed as the **ARD** metric in the Importance chart: selecting it fits a GP-FITC surrogate to the chosen objective and reports the per-parameter relevance (the validation R² of that GP is shown alongside, so you can judge how much to trust the scores).

This complements PDP (which shows *how* the response changes with a parameter) by giving a single scalar *smoothness-based sensitivity* per parameter.

- Backed by a **GP-FITC** fit. Mixture-of-experts (GP-MOE) has per-expert length scales whose aggregation is ambiguous, and Ridge / LightGBM have no length scales, so those models cannot produce ARD importance.

## ARD length scales and the θ convention

This dashboard uses egobox-gp with a Matérn 5/2 ARD kernel. The kernel exposes the ARD correlation parameters $\theta = (\theta_1, \dots, \theta_d)$, one per input dimension, fitted on the **normalized inputs** (each parameter min-max scaled to $[0,1]$). Normalizing the inputs is what makes the $\theta_d$ comparable across parameters with different physical units.

In the egobox / SMT parameterization, $\theta_d$ is **inversely** related to the length scale $\ell_d$:

$$
\theta_d \propto \frac{1}{\ell_d^2}
$$

So a **larger $\theta_d$ means a shorter length scale** in dimension $d$ — the surrogate varies more quickly along that axis and is therefore **more sensitive** to parameter $d$.

## Importance formula

Importance is the normalized $\theta$ so the scores sum to 1:

$$
\operatorname{importance}_d = \frac{\theta_d}{\sum_{k=1}^{d} \theta_k}
$$

If the sum is non-positive or any $\theta_d$ is non-finite, no importance is reported. Because $\theta$ is indexed by the GP's input columns, which are in the same order as the parameter names, $\operatorname{importance}_d$ aligns directly with parameter $d$.

## Interpretation and caveats

- A high value means the response surface is steep / wiggly along that parameter (high relevance); a low value means the surface is nearly flat along it.
- This is a **global, smoothness-based** sensitivity derived from the fitted kernel — not a variance decomposition (Sobol) and not a local effect (SHAP). It measures relevance to the GP's *fit*, so it is only as trustworthy as the GP itself (check the validation $R^2$).
- It is computed on normalized $[0,1]$ inputs, so it reflects sensitivity per *unit of normalized range*, not per physical unit.
- Use it as a quick screen and confirm the shape with PDP or a variance-based method (Sobol) when interactions matter.

See also: [PDP](./pdp.md), [Sobol](./sobol.md), and the [Surrogate Optimizer widget](../widgets/surrogate-optimizer.md) (which fits the same GP family for response-surface optimization).
