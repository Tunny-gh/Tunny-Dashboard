# Tunny Dashboard Domain Terminology

Canonical domain terms for interpreting Optuna optimization results in Tunny
Dashboard.

## Language

**Journal trial ID**: The zero-based global ordinal of a trial's `CREATE_TRIAL`
(`op_code=4`) record in a journal, counting trials across all studies and states,
including trials with no artifacts. Artifact metadata is associated with trials
by this ID.

**Study trial number**: A trial's zero-based creation-order ordinal within its
Study, counting all trial states, not its Journal trial ID.
_Avoid_: Journal trial ID as a synonym for Study trial number.

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

**Criterion weight**: The nonnegative relative importance of an objective in
multi-criteria decision-making (MCDM). Objective direction specifies whether to
minimize or maximize, not the weight's sign.

**Artifact root directory**: A directory used as the root for resolving trial
artifacts, specified independently of the optimization result source. It is not
an individual artifact file.

**Artifact #**: The zero-based index of an artifact entry within a trial, not
the artifact count or the count of unique MIME types.

**DesignExplorer-format CSV**: The CSV output format from the DesignExplorer
application targeted by Tunny Dashboard's parser: one trial per row, with
`in:<name>` parameter columns, `out:<name>` objective columns, and an optional
`img` column containing relative artifact paths.
_Avoid_: Generic CSV or Flat CSV as names for this specific format.
