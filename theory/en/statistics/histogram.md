# Histogram

## Overview

A histogram summarizes the **distribution of a single variable** by dividing its range into consecutive bins and counting how many observations fall into each one. It is the most direct way to see the shape of a distribution — whether it is skewed, has multiple peaks (multimodal), or contains outliers — that a single summary statistic such as the mean cannot reveal.

---

## Binning Rules

The number (or width) of bins strongly affects how the distribution appears, so several rules exist to choose a reasonable default automatically.

**Sturges' rule** picks the bin count $k$ from the sample size alone:

$$
k = \lceil \log_2 n \rceil + 1
$$

It is derived assuming the data are approximately **normally distributed**, and tends to under-bin (produce too few, too-wide bins) as $n$ grows large, since $k$ only grows logarithmically.

**Scott's rule** instead derives the bin width $h$ from the data's spread, minimizing the integrated mean squared error under a normality assumption:

$$
h = 3.49 \, \sigma \, n^{-1/3}
$$

where $\sigma$ is the sample standard deviation, computed with the unbiased ($n-1$-corrected) estimator. Because it uses $\sigma$, it is sensitive to outliers, which inflate $\sigma$ and widen the bins.

**Freedman–Diaconis' rule** replaces $\sigma$ with the interquartile range (IQR, see [Box Plot](box-plot.md)):

$$
h = 2 \cdot \mathrm{IQR} \cdot n^{-1/3}
$$

Since the IQR is a robust spread measure unaffected by extreme values, this rule stays reliable even when the data contain outliers or are far from normal — making it a safer general-purpose default than Scott's rule.

If the computed width $h$ is non-positive or non-finite — e.g., the IQR collapses to $0$ under Freedman–Diaconis when many values repeat, or $\sigma = 0$ under Scott's rule for near-constant data — the implementation falls back to Sturges' rule instead of producing a degenerate bin count.

---

## Characteristics

- The apparent shape of a histogram (number of peaks, skewness) can change noticeably depending on the chosen bin count/width — a feature that looks like a real peak with one binning may vanish with another. Always be aware that a histogram is one particular *view* of the data, not the data itself.
- A histogram is a hard-binned, discontinuous estimate of the underlying distribution. Kernel density estimation (KDE) smooths this into a continuous curve, trading a bin-boundary artifact for a bandwidth-choice artifact — neither is strictly "more correct," but a smooth KDE overlay is often easier to read alongside the histogram bars.
- Regardless of the rule used, the resulting bin count is always clamped to $[1, 200]$, so pathological inputs (or very large samples) never produce an unbounded or unusably fine set of bins.
- Bins are half-open, $[\text{edge}_i, \text{edge}_{i+1})$, except the last one, which is closed on both ends, $[\text{edge}_{k-1}, \text{edge}_k]$ — the same convention as `numpy.histogram` — so the maximum value in the data is always counted in the last bin rather than dropped.

---

## Where It Is Used in the App

- **Histogram widget**: shows the distribution of a selected parameter or objective's values, helping identify skewness, multimodality, and where the optimizer's search has concentrated its exploration. The **Fit** selector overlays a maximum-likelihood parametric density — see [Distribution fitting](distribution-fitting.md).

---

## References

- Sturges, H. A. (1926). The Choice of a Class Interval. _Journal of the American Statistical Association_, 21(153), 65–66.
- Scott, D. W. (1979). On optimal and data-based histograms. _Biometrika_, 66(3), 605–610.
- Freedman, D., & Diaconis, P. (1981). On the histogram as a density estimator: L2 theory. _Zeitschrift für Wahrscheinlichkeitstheorie und verwandte Gebiete_, 57(4), 453–476.
