# ADR-0022: Retain recorded Journal attributes for postprocessing

Status: Accepted
Date: 2026-10-18

## Context

Tunny Dashboard visualizes recorded optimization results rather than executing
the producer's storage transactions. Investigation of #258 reproduced different
final user attributes and constraint evidence between bulk and streaming Journal
readers. Official reference implementations diverge: Optuna core and Rustuna
reject finished-trial updates, while Optuna Dashboard's browser importer applies
them with its own limitations. These are references, not this product's requirements.

The user chose information-preserving postprocessing with early incremental
display, rather than reproducing producer acceptance/rejection or waiting for
the entire acquisition before showing COMPLETE trials.

## Decision

- For an existing known Journal trial, interpret recorded op8 user attributes in
  record order, merging keys with later values winning, including after COMPLETE,
  PRUNED and FAIL. Preserve original JSON types, explicit null and empty values.
- Interpret op9 `constraints` observations in record order as replacement of that
  trial's prior observation, also after terminal state. Preserve presence and
  invalid/nonfinite positions. Infer constraint evidence from the final current
  records across all states, using ADR-0007's existing classification rules;
  replacing a longer observation does not make its obsolete width permanent.
- Do not infer producer success from a recorded operation or replay the producer's
  rejection policy. This decision does not permit post-terminal parameter,
  state/objective/time or intermediate-value mutation (op5/op6/op7), arbitrary
  system-attribute retention, or new trial transitions/formats.
- Continue publishing early COMPLETE prefixes at existing batch boundaries.
  Coalesce dirty trial updates until a batch boundary or EOF, and publish the
  final dirty portion before marking the acquisition Complete. Do not rebuild a
  table for every attribute record or append a second row for an updated trial.
- Preserve acquired immutable inputs, stable trial identities, per-path row and
  column discovery order, and fallible delivery/backpressure. Analysis tables
  remain COMPLETE-only; non-COMPLETE records still explain constraint evidence.
- Preserve the independent storage-wide Artifact scanner/map, session root and
  GUI resources. Ordinary trial replay must not replace that scanner or narrow
  its existing late-association coverage.

## Alternatives

- **Match Optuna core's finished-trial rejection.** Not chosen: recorded
  information is useful for postprocessing regardless of producer acceptance.
  Upstream conformity is not the success criterion.
- **Wait for EOF before any display.** Not chosen: it avoids corrections but loses
  early visualization of large acquisitions.
- **Publish and project each attribute operation immediately.** Not chosen:
  repeated corrections to one trial would cause unnecessary publications and
  projections. Accept visibility at batch/EOF boundaries instead.

## Consequences

A displayed prefix can change when later recorded attributes arrive. Final bulk,
streaming and owned-input results must agree by trial identity, without unifying
their established row order. Consumers of incremental results must support
replacement of earlier derived rows rather than assume every batch is append-only.
Retained old inputs still describe their own revision. Replay builders remain
transient acquisition state, not a second writable Study authority. No fastest,
constant-time or memory-bound guarantee is established by this decision.
