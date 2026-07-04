# Box Plot

## Overview

A box plot (box-and-whisker plot) summarizes a variable's distribution with a **five-number summary** — minimum (within a bound), first quartile, median, third quartile, and maximum (within a bound) — rendered as a compact box-and-whisker glyph. Because many box plots can be placed side by side, it is especially effective for comparing the distribution of several variables, or of one variable across several clusters/groups.

---

## Formula

Quartiles are computed by **linear interpolation between order statistics** — the same convention as NumPy's default `"linear"` method (also known as type 7). For a sorted sample $x_{(1)}, \dots, x_{(n)}$ and quantile $q \in [0, 1]$:

$$
h = (n - 1) q, \qquad
Q(q) = x_{(\lfloor h \rfloor + 1)} + (h - \lfloor h \rfloor) \left( x_{(\lfloor h \rfloor + 2)} - x_{(\lfloor h \rfloor + 1)} \right)
$$

$Q_1 = Q(0.25)$, $Q_2 = Q(0.5)$ (median), $Q_3 = Q(0.75)$.

The **interquartile range** is $\mathrm{IQR} = Q_3 - Q_1$, and the **Tukey fences** are:

$$
\text{lower fence} = Q_1 - 1.5 \cdot \mathrm{IQR}, \qquad
\text{upper fence} = Q_3 + 1.5 \cdot \mathrm{IQR}
$$

The whiskers do **not** simply extend to the fence values: each whisker extends to the most extreme *actual data point* that still lies within its fence. Any data point beyond a fence is treated as an outlier and plotted individually rather than absorbed into the whisker.

---

## Characteristics

- The factor $1.5$ is a conventional threshold, not derived from a formal hypothesis test: for data drawn from a normal distribution, it flags roughly the most extreme $0.7\%$ of points as outliers. It is a widely adopted rule of thumb (introduced by John Tukey) rather than a universal statistical law.
- A box plot cannot reveal **multimodality** — a bimodal distribution and a unimodal one can produce an identical-looking box if their five-number summaries coincide. When the underlying shape matters, pair the box plot with a [Histogram](histogram.md) or a density plot.

---

## Where It Is Used in the App

- **Box Plot widget**: compares the distributions of the objective columns or the parameter columns side by side. Columns on different scales can be compared by enabling min-max normalization.
