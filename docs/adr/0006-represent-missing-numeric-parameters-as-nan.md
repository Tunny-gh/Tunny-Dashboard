# ADR-0006: Represent missing numeric parameters as NaN

Status: Accepted
Date: 2026-09-30

## Context

[Issue #192](https://github.com/Tunny-gh/Tunny-Dashboard/issues/192) identifies
that initial DataFrame construction and incremental append fill absent numeric
parameters with `0.0`, including historical rows when a parameter first appears
later. This turns absence into a fabricated measurement indistinguishable from
an observed zero, which numerical analyses can consume silently.

The DataFrame already stores numeric columns as dense `Vec<f64>` values and
uses NaN for missing objective values and numeric user attributes. Numeric
parameter missingness needs a representation consistent across both ingestion
paths without requiring a distinction between absence and explicitly supplied
NaN.

## Decision

Represent absent numeric parameter values as `f64::NAN` in the existing dense
`Vec<f64>` columns. Apply the same semantics to initial construction,
incremental append, and historical backfill for newly appearing parameters.
Preserve actual `0.0` as a valid observed value. No distinction is required
between an absent numeric parameter and an explicitly supplied NaN in these
columns.

Numerical-analysis boundaries must deliberately handle non-finite values,
including NaN and infinities, through exclusion or rejection rather than
silently treating them as valid measurements or imputing zero.

PCA uses complete-case rows: retain only rows whose values are finite across
all selected numeric parameter features. Filter these rows before centering or
standardization, and preserve their correspondence to source trial rows for
projections and coloring. If too few rows remain to perform PCA, the result is
unavailable; do not impute missing values to make PCA available. Other analysis
methods retain their existing method-appropriate finite-pair or complete-row
filtering, or reject non-finite inputs at numerical fitting boundaries. Unsafe
boundaries must be protected without introducing silent zero imputation.

This decision concerns numeric parameter missingness only. It does not change
categorical feature encoding or missing-constraint semantics.

## Alternatives

- **Keep zero as the missing-value default.** Rejected because it conflates
  absence with a valid observed zero and fabricates measurements for analysis.
- **Use a separate validity mask alongside numeric values.** A mask could
  distinguish absence from explicitly supplied NaN, but that distinction is not
  required. It would add separate validity storage and propagation obligations
  to numeric consumers instead of reusing the existing dense columns and
  missing objective/attribute convention. NaN is the simpler choice, accepting
  the need for deliberate non-finite handling at analysis boundaries.
- **Reject PCA for the whole study when a selected feature has non-finite
  values.** Rejected because complete-case filtering can retain usable observed
  rows without discarding the entire study's PCA result.
- **Impute non-finite values for PCA.** Rejected because it introduces assumed
  measurements that can change centering, scaling, and the resulting components.
  Complete-case filtering uses only observed finite feature values; an
  unavailable result is preferable when too few such rows remain.

## Consequences

Numeric parameter columns retain their existing dense representation, and
absence no longer consumes a valid numeric value as a sentinel. Both ingestion
paths must preserve observed zeros and use the same missing-value semantics.

The numeric columns cannot recover whether a NaN originated from absence or
an explicitly supplied NaN; this information loss is accepted. NaN storage
alone does not make downstream calculations safe: analysis boundaries must
deliberately handle non-finite inputs without reintroducing silent zero
imputation.

PCA excludes incomplete or otherwise non-finite rows for the selected numeric
parameter features, reducing the analyzed sample and potentially making the
result unavailable. Projections and coloring must remain aligned with the
retained source trial rows rather than the original unfiltered row positions.
No global preprocessing redesign is introduced. Broader commonization of
preprocessing and extraction remains separate future work.
