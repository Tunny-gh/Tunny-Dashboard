# Pearson Product-Moment Correlation

## Overview

The Pearson product-moment correlation coefficient $r$ is the most basic correlation measure, expressing how strongly two quantities move together in a **straight-line (linear)** fashion, on a scale from $-1$ to $+1$.

- It approaches $+1$ when one increases proportionally as the other increases.
- It approaches $-1$ when one decreases proportionally as the other increases.
- It approaches $0$ when they do not move together (no correlation).

A larger absolute value $|r|$ means a stronger linear co-movement.

---

## Formula

For two series $x = (x_1, \dots, x_n)$ and $y = (y_1, \dots, y_n)$ with means $\bar{x}, \bar{y}$:

$$
r = \operatorname{Corr}(x, y)
= \frac{\sum_i (x_i - \bar{x})(y_i - \bar{y})}{\sqrt{\sum_i (x_i - \bar{x})^2 \, \sum_i (y_i - \bar{y})^2}}
$$

It is computed as "the **covariance** of the two quantities divided by the product of their **standard deviations**": the numerator is the covariance and the denominator is the product of the standard deviations. Subtracting the means $\bar{x}, \bar{y}$ from each value measures whether the deviations from the mean move in the same direction. Dividing by the product of the standard deviations cancels out scale, normalizing the value to $[-1, +1]$.

---

## Characteristics

**Strengths:**

- Scale-independent (normalized to $[-1, +1]$).
- Simple and fast to compute.

**Limitations:**

- Captures only **linear** co-movement. A non-linear (curved) relationship can yield a small $r$ even when the relationship is strong.
- Sensitive to outliers (uses raw values, so it is pulled by extreme points).

To robustly capture non-linear but monotonic relationships, apply Pearson correlation to the rank series rather than the raw values — see [Spearman rank correlation](../sensitivity-analysis/spearman.md).

---

## Where It Is Used in the App

- **Scatter Matrix**: the correlation coefficient of each variable pair (upper-triangle cells) is computed as the Pearson correlation of the raw values → [Scatter Matrix](../widgets/scatter-matrix.md)
- **Spearman rank correlation**: Spearman is simply Pearson correlation applied to the **rank series** → [Spearman rank correlation](../sensitivity-analysis/spearman.md)
