# Box-Muller Transform

## Overview

The Box-Muller transform converts a pair of independent uniform random numbers into a pair of independent **standard normal** samples. It is the sampler behind every Gaussian draw in this app — most prominently the input-noise perturbations of the [robustness analysis](../optimization/robustness-analysis.md) — chosen because it is exact (not an approximation), branch-free, and easy to make reproducible on top of the app's deterministic ChaCha8-based RNG.

---

## Formula

Given two independent uniform variates $U_1, U_2 \sim \mathcal{U}(0, 1)$:

$$
Z_0 = \sqrt{-2 \ln U_1} \, \cos(2\pi U_2), \qquad
Z_1 = \sqrt{-2 \ln U_1} \, \sin(2\pi U_2)
$$

Then $Z_0, Z_1 \sim \mathcal{N}(0, 1)$ and they are independent.

The construction is a change of variables to polar coordinates: $R^2 = -2\ln U_1$ gives the squared radius the chi-squared distribution with 2 degrees of freedom (the distribution of $Z_0^2 + Z_1^2$ for a standard bivariate normal), while $\Theta = 2\pi U_2$ picks the angle uniformly. Mapping the pair $(R, \Theta)$ back to Cartesian coordinates yields two independent standard normals.

A general Gaussian sample is obtained by scaling and shifting: $X = \mu + \sigma Z$.

---

## Characteristics

- **Exact**: the output follows the normal distribution exactly (up to floating-point error), unlike approximations such as summing twelve uniforms.
- **Domain care**: $\ln U_1$ requires $U_1 > 0$. Since the underlying generator returns values in $[0, 1)$, the implementation feeds $1 - U$ (which lies in $(0, 1]$) to the logarithm, so a raw draw of exactly $0$ cannot produce $\ln 0 = -\infty$.
- This app uses only the cosine branch ($Z_0$) and discards $Z_1$, trading a factor-of-two efficiency for a simpler, stateless call. The cost is irrelevant at the sample counts involved (thousands per analysis).
- **Reproducibility**: driven by the seeded ChaCha8 RNG, the same seed always yields the same Gaussian sequence — which is what makes robustness-analysis results deterministic for fixed settings.
- Tail behavior is limited by the resolution of $U_1$ near $1$: with 53-bit doubles the largest attainable $|Z|$ is about $8.5\sigma$, far beyond anything relevant at the sample sizes used here.

---

## Where It Is Used in the App

- **Robustness widget**: generates the Gaussian input-noise perturbations around the candidate design, and (when "Model uncertainty" is enabled) the draws from the GP posterior.

---

## References

- Box, G. E. P., & Muller, M. E. (1958). A Note on the Generation of Random Normal Deviates. _Annals of Mathematical Statistics_, 29(2), 610–611.
