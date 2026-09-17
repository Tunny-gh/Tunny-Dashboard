# ADR-0002: Phase 2 scope — extend into the execution loop

Status: Accepted
Date: 2026-09-17

## Context

Phase 1 set out to build the decision-support layer for the Optuna ecosystem,
and that goal has been achieved. In 2026-07 the project made a policy decision
to extend the product beyond analysis, into the "analyze → suggest → execute →
re-analyze" loop, to occupy a position no commercial PIDO tool holds: an Optuna
dashboard that can itself execute optimizations. The question was how far into
execution to expand.

Commercial PIDO process-integration and execution capability breaks into three
layers: execution management (runner, workers, retries, monitoring), generic
process integration (a solver-agnostic file/command interface), and
vendor-specific integration (per-solver adapters and a workflow graph editor).

## Decision

Expand into Layer 1 (execution management: runner, parallel workers, retries,
monitoring) and Layer 2 (generic process integration: a Dakota-style
template-substitution → execution → output-extraction interface). Layer 3
(vendor-specific solver adapters and a workflow graph editor) is out of scope.
The sole Layer-3 exception is Grasshopper (Tunny), treated as a first-class
integration because it connects directly to the existing user base and
commercial tools are weak there.

The runtime optimization loop runs entirely in the Dashboard, using Rust
samplers and the application's own Optuna-compatible journal writer. No Python
or Optuna is required at runtime; the journal is only a file format, so Optuna
can read the results later but is never needed to produce them.

## Alternatives

- **Full Layer-3 vendor-specific adapters.** Rejected: tracking each solver
  version upgrade requires ongoing maintenance, and verification testing
  requires licenses the project cannot sustain as a small team. Users who need
  a workflow graph editor are already served by commercial tools, and price
  would be their only reason to switch.
- **Candidate write-back to Optuna via `enqueue_trial`.** Dropped once the
  Phase 2B runner could execute suggested candidates directly, which closes the
  loop without an enqueue hand-off. The existing export remains for users who
  drive Optuna themselves.

## Consequences

- Work proceeds in the order 2A (storage write layer) → 2B (runner) → 2C
  (automation and agentification). The storage write layer underpins the 2B
  runner's journal output.
- The generic Layer-2 interface keeps the project solver-agnostic, at the cost
  of no out-of-the-box integration for specific commercial solvers.
- Enterprise features (user management, PLM integration) are deferred until
  demand becomes visible.
