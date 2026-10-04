# ADR-0013: Preserve parameter distributions for grouping

Status: Accepted
Date: 2026-10-04

## Context

[Issue #227](https://github.com/Tunny-gh/Tunny-Dashboard/issues/227) gives Box
Plot and Violin Plot shared `Value` and optional `Group by` semantics. Numeric
grouping must distinguish declared integer or stepped parameters from continuous
parameters. Observations alone cannot reliably make that distinction: a declared
continuous parameter can produce integer-looking or repeated values.

## Decision

Preserve explicit Optuna parameter distribution metadata from ingestion through
chart inputs. Both charts must use this metadata to determine numeric `Group by`
candidates: only explicitly declared integer or stepped numeric parameters are
eligible. Never infer eligibility from integer-looking or repeated observations.
Categorical parameters remain supported for grouping.

CSV inputs without distribution metadata permit numeric columns as `Value`, but
not as numeric `Group by` candidates. Categorical grouping remains supported.

## Alternatives

- **Infer numeric grouping eligibility from observed values**, using
  integer-looking values plus repetition, or arbitrary repeated values. Rejected
  because neither rule reliably distinguishes declared continuous parameters
  from declared integer or stepped parameters.

## Consequences

The ingestion-to-chart data path must retain distribution metadata rather than
requiring charts to reconstruct parameter declarations from samples. This adds
metadata handling in exchange for grouping eligibility grounded in explicit
declarations.

Numeric grouping is unavailable when distribution metadata is absent, even if
observations look discrete or repeat. Numeric `Value` selection and categorical
grouping do not require numeric distribution metadata. This decision does not
settle sparse-group rendering or normalization policies.
