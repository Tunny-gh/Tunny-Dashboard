# ARD Parameter Importance (GP length scales)

## Overview

When a Gaussian Process (GP) surrogate is fitted with an **ARD** (Automatic Relevance Determination) kernel, it learns one length-scale hyperparameter per input dimension. The relative magnitude of these hyperparameters is a cheap, global measure of how sensitive the response surface is to each parameter.

In the dashboard this is exposed as the **ARD** metric in the Importance chart: selecting it fits a GP-FITC surrogate to the chosen objective and reports the per-parameter relevance (the validation R² of that GP is shown alongside, so you can judge how much to trust the scores).

This complements PDP (which shows *how* the response changes with a parameter) by giving a single scalar *smoothness-based sensitivity* per parameter.

- Backed by a **GP-FITC** fit. Mixture-of-experts (GP-MOE) has per-expert length scales whose aggregation is ambiguous, and Ridge / LightGBM have no length scales, so those models cannot produce ARD importance.

## How it is computed

ARD importance is a by-product of the standard GP fit — there is no separate sensitivity pass:

1. **Assemble the data.** The numeric (and label-encoded categorical) parameter columns form the input matrix $X$; the selected objective is the target $y$. Inputs are min–max scaled to $[0,1]^d$ so the per-dimension hyperparameters are comparable across parameters with different physical units.
2. **Fit a GP-FITC surrogate** with the ARD Matérn 5/2 kernel:

   $$
   k(x_1, x_2) = \sigma_f^2 \left(1 + \sqrt{5}\,r + \frac{5r^2}{3}\right) \exp(-\sqrt{5}\,r), \qquad r^2 = \sum_{d} \left(\frac{x_{1,d} - x_{2,d}}{\ell_d}\right)^2
   $$

   The kernel has **one length scale $\ell_d$ per input dimension** — this per-dimension freedom is exactly what "ARD" means. The length scales $\{\ell_d\}$, together with the signal variance $\sigma_f^2$ and the noise variance $\sigma_n^2$, are fitted by **maximizing the FITC marginal-likelihood bound** (egobox-gp, gradient-free COBYLA with multistart). This is the same training step the Surrogate Optimizer runs — see [Gaussian Process](../surrogate-models/gaussian-process.md) for the inducing points, the FITC bound and the optimizer. The key point is that the length scales are *not* chosen by hand: they are whatever values let the surrogate explain $y$ best. That is why they double as a relevance measure — a parameter the objective barely responds to is driven to a long (nearly flat) length scale, while an influential one is driven to a short length scale.
3. **Read the ARD parameters.** After training, egobox exposes the per-dimension correlation parameters $\theta = (\theta_1, \dots, \theta_d)$ straight off the fitted kernel, at no extra computation. The θ-to-length-scale convention is detailed in the next section.
4. **Normalize to importances** so the scores sum to 1 (see [Importance formula](#importance-formula) below).

The dashboard also reports the fitted GP's cross-validated $R^2$ next to the scores: ARD relevance is only meaningful when the surrogate actually fits the data, so a low $R^2$ means the ranking should be treated with caution.

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

## References

- Rasmussen, C. E., & Williams, C. K. I. (2006). _Gaussian Processes for Machine Learning_. MIT Press.
