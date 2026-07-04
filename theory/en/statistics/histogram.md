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

where $\sigma$ is the sample standard deviation. Because it uses $\sigma$, it is sensitive to outliers, which inflate $\sigma$ and widen the bins.

**Freedman–Diaconis' rule** replaces $\sigma$ with the interquartile range (IQR, see [Box Plot](box-plot.md)):

$$
h = 2 \cdot \mathrm{IQR} \cdot n^{-1/3}
$$

Since the IQR is a robust spread measure unaffected by extreme values, this rule stays reliable even when the data contain outliers or are far from normal — making it a safer general-purpose default than Scott's rule.

---

## Characteristics

- The apparent shape of a histogram (number of peaks, skewness) can change noticeably depending on the chosen bin count/width — a feature that looks like a real peak with one binning may vanish with another. Always be aware that a histogram is one particular *view* of the data, not the data itself.
- A histogram is a hard-binned, discontinuous estimate of the underlying distribution. Kernel density estimation (KDE) smooths this into a continuous curve, trading a bin-boundary artifact for a bandwidth-choice artifact — neither is strictly "more correct," but a smooth KDE overlay is often easier to read alongside the histogram bars.

---

## Where It Is Used in the App

- **Histogram widget**: shows the distribution of a selected parameter or objective's values, helping identify skewness, multimodality, and where the optimizer's search has concentrated its exploration. The **Fit** selector overlays a maximum-likelihood parametric density — see [Distribution fitting](distribution-fitting.md).
