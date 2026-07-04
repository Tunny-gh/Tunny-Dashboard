# Distribution Fitting

## Overview

Distribution fitting overlays a parametric probability density on the [histogram](histogram.md), estimated from the same sample by **maximum likelihood**. It turns the qualitative impression of a histogram ("roughly bell-shaped", "skewed right") into a quantitative statement: which named family describes the data best, with which parameters — useful for reporting, for spotting log-normally distributed quantities (common for strictly positive engineering responses), and as groundwork for reliability statements.

Three families are supported: **Normal**, **Log-normal**, and **Weibull**. The latter two apply only to strictly positive samples.

---

## Formula

For a sample $x_1, \dots, x_n$ the maximum-likelihood estimates are:

**Normal** $\mathcal{N}(\mu, \sigma^2)$ — closed form:

$$
\hat\mu = \frac{1}{n}\sum_i x_i, \qquad
\hat\sigma^2 = \frac{1}{n}\sum_i (x_i - \hat\mu)^2
$$

**Log-normal** — the Normal MLE applied to $\ln x_i$ (parameters $\hat\mu_{\ln}, \hat\sigma_{\ln}$).

**Weibull** with shape $k$ and scale $\lambda$, density $f(x) = \frac{k}{\lambda}\left(\frac{x}{\lambda}\right)^{k-1} e^{-(x/\lambda)^k}$ — no closed form; the shape solves the MLE equation

$$
\frac{\sum_i x_i^k \ln x_i}{\sum_i x_i^k} - \frac{1}{k} - \frac{1}{n}\sum_i \ln x_i = 0
$$

found by bisection (the left side is monotone in $k$), after which $\hat\lambda = \left(\tfrac{1}{n}\sum_i x_i^{\hat k}\right)^{1/\hat k}$. The equation is scale-invariant, so the implementation normalizes by $\max_i x_i$ first to avoid overflow of $x^k$ at large $k$.

**Model comparison** uses the Akaike information criterion. All three families have two parameters, so

$$
\mathrm{AIC} = 2\cdot 2 - 2\ln \hat L
$$

and ranking by AIC is equivalent to ranking by likelihood here; AIC is reported for continuity with mixed-parameter-count comparisons.

To overlay the density on a count histogram, the PDF is scaled by $n \cdot w$ (sample size × bin width), so the curve and the bars share the same vertical axis.

---

## Characteristics

- **MLE is not a goodness-of-fit test.** The best AIC among the three families is only "the least bad of these three" — a sample from a bimodal or heavy-tailed distribution still gets a winner. Judge the overlay visually against the bars before quoting parameters.
- The Log-normal and Weibull fits silently become unavailable when the sample contains zero or negative values; only the Normal fit remains.
- The Weibull shape parameter is interpretable: $k < 1$ indicates a decreasing hazard (infant-mortality-like), $k = 1$ reduces to the exponential distribution, $k > 1$ an increasing hazard — the reason Weibull dominates reliability engineering.
- MLE variance estimates are biased low by the factor $(n-1)/n$ relative to the unbiased sample variance; at the sample sizes where fitting is meaningful this is negligible.

---

## Where It Is Used in the App

- **Histogram widget → Fit selector**: overlays the selected family (or the best-AIC family with *Auto*) on the histogram, and prints the fitted parameters and AIC for each applicable family.
