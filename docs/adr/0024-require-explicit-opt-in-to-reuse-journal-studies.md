# ADR-0024: Require explicit opt-in to reuse Journal Studies

Status: Accepted
Date: 2026-10-22

## Context

[Issue #276](https://github.com/Tunny-gh/Tunny-Dashboard/issues/276) exposes a
Journal writer/reader mismatch: the writer allocates an additional Study ID for
a repeated name, while the reader skips the duplicate creation record. A returned
ID can therefore be absent on readback or refer to a different Study later.

The user chose Optuna-like explicit reuse for both generic-process and
Grasshopper optimization, available through both the API and GUI. Optuna's
[official create_study API](https://optuna.readthedocs.io/en/stable/reference/generated/optuna.create_study.html)
defaults `load_if_exists` to `False`: a name conflict raises an error unless the
caller explicitly requests the existing Study. The pinned
[Optuna v5.0.0 create_study implementation](https://github.com/optuna/optuna/blob/v5.0.0/optuna/study/study.py)
handles `DuplicatedStudyError` as reuse only when explicitly enabled.

Optuna v5.0.0 `create_study` does not compare requested directions with the
existing Study on reuse; `Study.__init__` reads the stored directions. The user
chose a deliberately stricter, earlier compatibility check for this product:
reject a different objective count or ordered direction list before evaluation,
rather than silently adopt stored directions. Retaining existing objective names
also avoids relabelling historical results when recording further trials.

The pinned [Optuna v5.0.0 Journal implementation](https://github.com/optuna/optuna/blob/v5.0.0/optuna/storages/journal/_storage.py)
writes creation attempts before replay validates them. Its
`JournalStorageReplayResult._apply_create_study` rejects a duplicate name without
incrementing the next Study ID. Legitimate Journals can consequently contain
rejected duplicate CREATE_STUDY attempts; those records are not additional
Studies.

The user chose recording-only reuse rather than history-aware optimization
continuation. The latter is a separate future concern discussed in
[Issue #277](https://github.com/Tunny-gh/Tunny-Dashboard/issues/277), not a
requirement of the duplicate-name correction.

## Decision

- Default to rejecting a Study-name conflict within the same Journal with an
  error, preventing the new optimization execution unless reuse is explicitly
  enabled.
- Provide an explicit `load_if_exists` opt-in through the API and a corresponding
  GUI option for both generic-process and Grasshopper optimization. The API
  defaults to false and the GUI option defaults to OFF.
- With the opt-in enabled, a matching name selects the existing Study and its
  existing identity rather than allocating another Study, subject to the
  compatibility check below. In the absence of a matching name, create a new
  Study.
- For explicit reuse on both execution paths, compare the requested objective
  count and ordered Minimize/Maximize directions with the stored Study. If either
  differs, return an error before evaluating any new trial; the caller must
  correct the request. Do not silently adopt the stored directions.
- Retain the existing Study's objective names on reuse. Do not overwrite them
  with requested names or relabel historical results. Differing requested labels
  do not add a name-equality restriction to recording reuse.
- Do not implicitly reuse a Study or automatically rename a conflicting name.
- Preserve Optuna-compatible reading of legitimate rejected duplicate
  CREATE_STUDY attempts: they do not create another Study or consume a Study ID.
- At append preparation, reject detectable invalid references and unsupported
  Study deletion/recreation records rather than append under uncertain
  identities. Do not automatically repair or renumber an existing ambiguous or
  broken Journal, or guess its intended Study membership. Legitimate rejected
  duplicate CREATE_STUDY attempts in otherwise supported Optuna Journals remain
  accepted and do not consume Study IDs; their presence alone is not corruption.
- Reuse selects the existing Study as a recording destination only. Each
  invocation starts its optimizer fresh; existing trial history is not used for
  bootstrap, model fitting, populations, or candidate deduplication. Do not
  restore sampler or optimizer state.
- Requested trial budgets, progress, and run-summary counts describe the new
  invocation, not historical Study totals. Retain existing trial records and
  continue global Journal trial IDs and Study-local trial numbers correctly;
  neither is reset by reuse.
- The GUI description must distinguish adding new results to an existing Study
  from resuming optimization using its history. This applies to both execution
  paths, with the reuse option still defaulting to OFF.

## Alternatives

- **Always reject duplicate names, even with explicit consent.** Not chosen:
  the user wants Optuna-like reuse when explicitly requested, not a prohibition
  on recording further trials in an existing Study.
- **Implicitly reuse or append on a name conflict.** Rejected: combining trials
  under an existing persisted identity must require explicit user intent.
- **Automatically generate a different name on conflict.** Rejected: silently
  changing the requested Study identity does not express the user's choice
  between a new Study and reuse.
- **Treat every CREATE_STUDY record as a new Study.** Rejected: legitimate Optuna
  Journals retain rejected attempts, so record count is not accepted Study count.
- **Silently adopt stored directions on explicit reuse, as Optuna create_study
  does.** Rejected: a changed objective count or direction changes how evaluations
  are interpreted. Require the caller to correct the request before evaluation,
  rather than execute under a different objective definition.
- **Overwrite existing objective names on reuse.** Rejected: recording another
  run must not relabel the existing Study's historical objective results.
- **Use existing trials to resume optimization when reuse is enabled.** Not
  chosen for this correction: the user explicitly separated recording reuse from
  history-aware continuation. Keep the current optimizers fresh per invocation
  rather than introduce historical training, populations, or deduplication.
- **Repair or renumber ambiguous existing Journals while preparing an append.**
  Rejected: the records may not establish the producer's intended Study
  membership. Reject detectable unsafe references or unsupported Study
  deletion/recreation instead of silently assigning a different identity.

## Consequences

API and GUI callers must express reuse explicitly on both execution paths. The
default protects against unintended combination of trials, at the cost of an
error when a name is already present. Explicit reuse retains the existing Study
identity and must not be presented as sampler-state resume.
Existing results remain in the Study, but do not guide the new invocation's
search. Repeated candidate evaluations across invocations are therefore possible,
and a requested budget is additional work rather than a target historical total.
History-aware continuation and adoption of Rustuna are outside this correction's
scope; recording reuse does not select either a continuation design or a new
optimization dependency.
Objective-count or ordered-direction mismatches prevent new evaluation even with
the opt-in enabled. This is stricter and earlier than Optuna `create_study` reuse,
and does not imply complete parameter or problem-schema compatibility.

The append-preparation policy does not guarantee detection of all corruption:
syntactically valid references to the wrong Study may be indistinguishable from
intended membership. Rejecting detectable unsafe input does not authorize
parser-wide loading-policy changes, a recovery UI, or a generic validation or
migration framework. Existing ambiguous data is not repaired by this correction.

The correction does not add full problem-schema compatibility restrictions for
parameters, constraints or attributes. Existing dynamic observation semantics
remain unchanged. Append preparation validates the acquired prefix's identity
references and rejects malformed or incomplete records and unsupported operations;
it does not change parser-wide loading or trial semantics, or add deletion or
recovery support. Serialize writers to a Journal, including preparation: no
file-wide locking, concurrent-writer, snapshot or additional durability guarantee
is introduced. Validation errors do not append Journal records, but this does
not promise rollback of other filesystem side effects; GH preparation may write
its definition sidecar before Journal validation.
