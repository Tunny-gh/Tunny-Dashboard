# ADR-0010: Reject negative MCDM weights

Status: Accepted
Date: 2026-10-01

## Context

[Issue #197](https://github.com/Tunny-gh/Tunny-Dashboard/issues/197) identifies
that MCDM weight normalization accepts negative entries when the sum is positive
and replaces them with uniform weights when the sum is nonpositive or nonfinite.
A [criterion weight](../../CONTEXT.md) expresses nonnegative relative importance;
minimize/maximize direction is specified separately. Silently replacing invalid
weights can change the intended preferences. Rejection versus uniform replacement
is a meaningful trade-off warranting this ADR under the repository's rule.

## Decision

Change public `normalize_weights` to return a `Result`. Reject the entire weight
vector if any entry is less than zero, before applying the sum fallback. All
three public rankings (TOPSIS, VIKOR, and PROMETHEE) consistently reject negative
weights, including `[2, -1]`, `[-1, -1]`, and negative infinity, even alongside
NaN.

For vectors without negative entries, retain the uniform fallback when the sum
is zero or nonfinite, including NaN, positive infinity, and sum overflow. Empty
normalization remains a successful empty result. Nonnegative weights with a
finite positive sum retain their existing normalization and ranking behavior.

This decision does not redesign weighting UX or ranking formulas and does not
introduce compatibility scaffolding.

## Alternatives

- **Replace negative weights with uniform weights.** Rejected because it hides
  invalid input and silently substitutes different preferences.
- **Keep accepting negative entries when their sum is positive.** Rejected
  because a weight's sign is not an objective direction and negative importance
  violates the criterion weight meaning.

## Consequences

Callers must handle normalization failure instead of receiving a silently
altered preference vector. This accepts an explicit error and a public API
change in exchange for preserving the user's intended preferences. Existing
negative-free fallback behavior remains unchanged.
