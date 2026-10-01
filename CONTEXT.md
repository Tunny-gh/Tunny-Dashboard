# Tunny Dashboard Domain Terminology

Canonical domain terms for interpreting Optuna optimization results in Tunny
Dashboard.

## Language

**Feasible**: A trial whose complete, finite constraint evaluation confirms
that every constraint satisfies the Optuna convention `c <= 0`.

**Infeasible**: A trial with a confirmed constraint violation (`c > 0` for a
finite evaluated constraint), even if other constraint evaluations are missing,
incomplete, or non-finite.

**Feasibility unverified**: A trial with no confirmed constraint violation whose
feasibility has not been established because constraint evaluations are missing,
incomplete, or non-finite. This is not a synonym for a confirmed constraint
violation.
_Avoid_: Infeasible as a label for incomplete evaluation alone.

**Constrained study**: A study with a `constraints` attribute present or observed
constraint values, even if its constraint evaluations are empty or incomplete.

**Genuinely unconstrained study**: A study with neither a `constraints`
attribute nor observed constraint values. Its existing feasibility behavior
remains unchanged.

**Expected constraint count**: The maximum observed constraint-array length in
a study across all trial states, counting invalid element positions rather than
only valid values. DataFrame rows remain COMPLETE-only.
Wholly unobserved trailing constraints cannot be inferred from this count.

For the classification decision and its rationale, see
[ADR-0007](docs/adr/0007-distinguish-three-constraint-feasibility-states.md).

**Nominal categorical parameter**: A parameter whose category identity implies
no numerical order or distance. Optuna categorical choices remain nominal even
when their labels look numeric.
