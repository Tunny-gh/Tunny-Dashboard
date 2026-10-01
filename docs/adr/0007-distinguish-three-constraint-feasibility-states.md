# ADR-0007: Distinguish three constraint feasibility states

Status: Accepted
Date: 2026-09-30

## Context

[Issue #194](https://github.com/Tunny-gh/Tunny-Dashboard/issues/194) identifies
that missing constraint entries are filled with zero and feasibility is derived
from the available values alone. An empty or incomplete evaluation in a
constrained study can therefore appear feasible without evidence that all
constraints were satisfied.

The user chose an explicit third state rather than conservatively treating an
incomplete evaluation as infeasible. [ADR-0006](0006-represent-missing-numeric-parameters-as-nan.md)
concerns numeric parameter missingness and explicitly leaves constraint
semantics separate; it does not settle this decision.

## Decision

Use three distinct constraint feasibility states, with the agreed English
labels **Feasible**, **Infeasible**, and **Feasibility unverified**. Their
canonical domain definitions are recorded in [CONTEXT.md](../../CONTEXT.md).

- **Feasible** requires a complete, finite constraint evaluation confirming
  that every constraint satisfies `c <= 0`.
- **Infeasible** denotes a confirmed constraint violation, not merely absent
  evidence of feasibility. A finite evaluated value `c > 0` is a violation.
- **Feasibility unverified** covers missing, incomplete, or non-finite
  constraint evaluations when no finite evaluated value is positive, and is
  distinct from confirmed constraint violation.
  Use exactly `Feasibility unverified` as its UI label.

In a constrained study, apply classification in this order:

1. Any finite constraint value `c > 0` establishes **Infeasible**, even if other
   entries are missing, incomplete, or non-finite.
2. Otherwise, missing, incomplete, or non-finite evaluations establish
   **Feasibility unverified**.
3. A complete, finite evaluation with every value `c <= 0` establishes
   **Feasible**.

A confirmed violation is sufficient evidence of infeasibility; uncertainty in
other entries cannot undo it. In contrast, verified feasibility requires all
constraint evaluations to be complete and finite.

### Study detection and expected constraint count

The presence of a `constraints` attribute or observed constraint values
establishes a constrained study. A genuinely unconstrained study has neither;
its existing behavior remains unchanged.

Infer the expected constraint count as the maximum observed constraint-array
length in the study across all trial states, including PRUNED, FAIL, and RUNNING.
Study constraint evidence is independent of the COMPLETE-only DataFrame rows;
metadata-only streaming batches can establish constraints or grow the schema.
Preserve invalid element positions when determining array
length and evaluation completeness rather than dropping them. An invalid entry
must not shorten the inferred count or shift subsequent constraint positions.

If a `constraints` attribute is present but all arrays are empty, the study is
constrained and its trials are **Feasibility unverified**, not vacuously
**Feasible** with an expected count of zero. Otherwise, completeness is assessed
against the inferred expected count, with the classification precedence above.

When append observes a longer constraint array, increase the expected count
and reclassify historical trials against it. Append must also account for newly
established constrained-study status. For the same final trial data, initial
load and incremental append must agree on study detection, expected count, and
feasibility classification.

Missing evaluations must not silently become observed zero values or establish
verified feasibility. Genuinely unconstrained studies retain their existing
behavior; absence of constraints is not the same as missing evaluations in a
constrained study.

### Pareto ordering and presentation

Only verified **Feasible** trials may enter the constrained Pareto front.
Rank trial groups in this order: **Feasible**, then **Infeasible**, then
**Feasibility unverified**. Keep the existing within-group algorithms for the
verified states (**Feasible** and **Infeasible**); unverified trials retain their
input row order without constraint-based comparison.

This group ordering is a presentation policy, not an assertion that uncertainty
is worse than confirmed constraint violation. Scatter plots display unverified
trials as a distinct third category labeled exactly `Feasibility unverified`.

### Report fallback with zero verified feasible trials

When a constrained study has zero verified **Feasible** trials, preserve the
report's objective-only nondominated fallback candidates. Clearly label them as
**objective-only candidates**, not as a verified feasible Pareto front, and show
each candidate trial's three-state feasibility label.

Fallback candidates are excluded from verified feasible trial counts and
verified feasible Pareto-front counts. Both counts remain zero in this case;
objective-only nondominance does not establish constraint feasibility or
membership in the constrained Pareto front.

This decision establishes terminology, the three-state distinction,
mixed-evaluation classification precedence, study detection, expected-count
inference, Pareto ordering and presentation, and the zero-feasible report
fallback policy.

## Alternatives

- **Conservatively classify incomplete evaluations as Infeasible.** Rejected
  because failure to verify feasibility is not evidence of constraint
  violation. A binary label would hide that distinction from users.
- **Treat missing evaluations as zero or classify only available values.**
  Rejected because this can report verified feasibility without a complete
  evaluation and conflates missing results with satisfied constraints.
- **Let missing or non-finite entries override a confirmed violation.** Rejected
  because a finite positive value already proves that a constraint is violated;
  missing evidence about other constraints does not invalidate that proof.

## Consequences

The domain and eventual presentation must distinguish evaluation uncertainty
from confirmed violation. The extra state adds classification and presentation
work compared with a binary model, but preserves the meaning of the evidence.

Unverified trials remain visible as a separate scatter category but cannot enter
the constrained Pareto front. Their stable input order avoids constraint-based
comparisons on unverified evaluations without changing the existing within-group
algorithms for verified states.

Reports retain useful objective-only candidates when no verified feasible trials
exist, while labeling each candidate's feasibility and keeping verified counts
separate. This avoids presenting fallback candidates as a feasible front without
removing the existing objective-only fallback.

Expected-count inference is limited to observed array positions: wholly
unobserved trailing constraints cannot be inferred. A complete evaluation under
this policy is complete relative to the inferred count, not proof that no
additional constraints exist outside the observed data. This limitation is
accepted rather than inventing a count without evidence.

A longer array arriving during append can change historical classifications;
for example, a previously complete non-positive evaluation becomes unverified
when it lacks newly observed trailing positions. Confirmed finite positive
violations remain **Infeasible** under the agreed precedence.

The implementation retains the dense numeric DataFrame: missing or invalid
constraint positions are NaN, and the derived `is_feasible` column stores 1 for
Feasible, 0 for Infeasible, and NaN for Feasibility unverified. Consumers use the
explicit `FeasibilityState` view rather than interpreting not-feasible as a
violation. Attribute-only studies retain constrained status even with zero
constraint columns. Constraint schema growth reclassifies historical rows from
their observed columns. This reuses existing column storage without a parallel
validity mask or invented zero measurements.

Ingestion accepts Python JSON's bare `NaN`, `Infinity`, and `-Infinity` tokens
without dropping their containing arrays or journal records. A quote-aware regex
from the existing regex dependency normalizes only these bare literals to JSON
null before strict serde_json parsing; constraint extraction retains their
positions as NaN. Quoted strings and finite values remain unchanged. This keeps
the agreed non-finite feasibility semantics without broadening accepted JSON
syntax or adding another parser dependency.
