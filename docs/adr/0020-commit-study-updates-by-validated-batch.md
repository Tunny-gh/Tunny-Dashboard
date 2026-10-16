# ADR-0020: Commit Study updates by validated batch

Status: Accepted
Date: 2026-10-16

## Context

[ADR-0019](0019-responsibility-refactoring-goal-and-scope.md) requires explicit
Study management and consistent acquired analysis inputs. The design gate in
[Issue #249](https://github.com/Tunny-gh/Tunny-Dashboard/issues/249) must define
the logical update boundary without prematurely choosing synchronization or UI
behavior. Existing Journal loading presents batches incrementally, and constraint
metadata can change historical feasibility without adding trial rows.

Source inspection identified possible ingestion inconsistencies: Journal
streaming discards terminal trial builders before later attribute records, and
RDB fingerprints do not include attribute or constraint contents. These are
unreproduced candidates, not established runtime failures or intended semantics.
Their reproduction work is tracked separately in
[Issue #258](https://github.com/Tunny-gh/Tunny-Dashboard/issues/258) and
[Issue #259](https://github.com/Tunny-gh/Tunny-Dashboard/issues/259).

## Decision

- Commit loads and updates one validated batch at a time. Related trial data and
  metadata in an accepted batch become a consistent state together; a failed
  batch has no effect, and previously accepted batches remain committed.
- Track loading progress, completion, and failure separately from the data
  revision. A constraint-information update that changes the data without
  changing the row count also produces a new revision.
- Preserve existing user-facing behavior as the governing principle.
- Analysis retains read-only input from acquisition time for its duration;
  updates produce separate new revisions rather than change that input. Do not
  stop updates for the entire analysis runtime. Accept the memory cost of old
  data remaining until consumers finish, use sharing or incremental structures
  to reduce copying, and release unneeded revisions. This does not require a
  full copy for every invocation, lock-free operation, a fixed time limit, or
  unlimited cache retention.
- Use a management unit per Study, independently managing its logical source of
  truth, updates, and revisions. Callers such as the GUI explicitly own a
  collection of these units and coordinate comparison and reload. Share
  storage-scoped Artifacts and other resources under their existing separate
  lifetimes. Updating Study A alone must not uniformly invalidate an unrelated
  Study B's revision or results; this does not prevent necessary invalidation
  caused by changed shared domain inputs or source replacement.
- Distinguish data consistency from analysis readiness. Make each analysis's
  existing execution requirements, such as load completion and required data,
  explicit and evaluate them in the common analysis API rather than duplicate
  them in widgets. Preserve existing differences between analyses instead of
  uniformly allowing or prohibiting analysis during partial loading.
- Results from partial inputs correspond to their input revision and load state.
  Inventory current execution conditions first; ask the user about ambiguous
  cases separately rather than deciding further method-specific conditions here.
  This does not introduce new UI or automatic recomputation behavior.
- A computation that takes a Pareto front as input requires a finalized front
  for the same Study, acquisition generation, and input revision. Provisional
  ranks during loading are not valid front input. Preserve the existing Pareto
  finalization timing; do not add correct partial-input front computation as a
  new feature in this refactoring. This condition applies to front-dependent
  paths, not a blanket prohibition on all clustering or MCDM computations.
- Treat reload as a new acquisition generation even for the same file and Study.
  Do not adopt asynchronous analysis results from an older generation into the
  new one. Within a generation, distinguish accepted batch updates by data
  revision. Do not reuse analysis caches across generations based on identical
  contents in this refactoring.
- During reload, retain the previous generation's display until the first
  validated batch of the new generation is accepted, then switch to the new
  generation's committed portion. A failure before that first batch leaves the
  old display; a later failure retains the new generation's accepted portion
  without automatically reverting to the old generation. Retained old display
  data and results must not be treated as belonging to the new generation.
- Acquisition generation distinguishes acquired analysis data; it does not
  uniformly reset Artifact roots, maps, or other storage/session-scoped resources.
  Preserve their existing lifetimes under
  [ADR-0012](0012-scope-artifacts-to-storage.md).
- Investigate these ingestion candidates in separate, reproduction-first Issues,
  not as part of crate extraction. Preserve the expected meaning of supported
  attribute records and all-state constraint evidence across ingestion paths;
  do not freeze a confirmed bug as a compatibility requirement. A change-detection
  fingerprint is not proof of semantic data equivalence. Record any dependency
  needed by affected migration paths before implementing those paths.
- Target a common external database snapshot for the related queries in one
  Study acquisition, using a backend-appropriate read transaction or equivalent
  read session. Validate concurrent-writer behavior, locking and resource costs
  before implementation; any necessary correction belongs in a separate
  Issue/PR, not crate extraction. Consistent in-memory batch application alone
  cannot prevent mixed external reads.
- This database acquisition target does not make a writer's multi-transaction
  business operation atomic, guarantee no waiting or writer interference, or
  promise snapshot support for every database configuration. Journal acquisition
  instead replays the acquired byte range in order; no external locking rule
  for all formats is imposed. Do not hold a database transaction for the analysis
  runtime.

Accepted status covers only the per-Study management, logical batch-commit,
per-analysis readiness, acquired-input retention, acquisition-generation,
reload display/retention, and external-read design policies above. Concrete
management and collection types, identifier namespaces, readiness APIs, sharing,
and synchronization are not fixed by these policy agreements; their adopted
realization is specified in
[ADR-0021](0021-study-analysis-and-io-boundary-contracts.md). Individual conditions
beyond the stated requirements, error wording and operation availability must
preserve the inventoried behavior rather than introduce new UI. No automatic
cancellation policy or numerical memory limit is selected.
This narrows the previously unresolved synchronization choices only by excluding
update suspension for the full analysis runtime; it does not prohibit locks for
other parts of input acquisition or update application.
The external-read policy is a design target requiring validation, not an
implemented or unconditional snapshot guarantee. Backend transaction APIs,
isolation levels, read-session/public APIs, failure/retry policies, and supported
configurations require the separate
[Issue #260](https://github.com/Tunny-gh/Tunny-Dashboard/issues/260) validation;
ADR-0021 specifies the I/O-private candidate mechanisms and no-automatic-retry
boundary. These design choices are adopted, but backend behavior remains to be
validated. Consistency of state application and consistency
of external reads are separate design concerns. This does not complete the
overall design gate.

## Alternatives

- **Rely only on model validation after independent database queries.** Not chosen
  as the acquisition design target: structurally valid old/new mixtures can
  evade model checks. Prefer a shared external read snapshot, accepting
  backend-specific transaction design and measured locking/resource trade-offs
  before choosing concrete mechanisms.
- **Fix suspected ingestion bugs during crate extraction, or preserve them as
  intended behavior.** Not chosen: separate reproduction and narrowly scoped
  correction from structural migration, accepting explicit coordination of any
  necessary dependencies. Unreproduced suspicions do not establish new behavior.
- **Suspend updates throughout each analysis to keep its input stable.** Not
  chosen: retain read-only input while updates advance to new revisions,
  accepting overlapping data lifetimes and memory use instead of coupling
  update progress to the duration of analysis. Concrete retention and copying
  mechanisms remain implementation design questions.
- **Commit only after the entire load or update operation succeeds.** Not chosen
  as the general commit boundary: it would require separating incremental display
  state from committed Study state. Validated batches fit incremental loading,
  accepting that failure of a later batch does not roll back earlier batches.
- **Uniformly allow or prohibit all analysis during partial loading, or let each
  widget independently decide readiness.** Not chosen: preserve the existing
  analysis-specific conditions while centralizing their evaluation, accepting
  the work of inventorying current conditions and defining a common API.
- **Reuse analysis results or caches across reloads when contents appear
  identical.** Not chosen for this scope: generation separation avoids requiring
  content-equivalence validation and prevents old work from being adopted into a
  new acquisition, accepting recomputation even when reloaded contents match.
- **Use provisional rank-zero rows as a finalized Pareto front for downstream
  computation.** Not chosen: require matching finalized front provenance,
  accepting that front-dependent computations wait for existing finalization
  rather than interpreting temporary ranks as analysis results.
- **Manage multiple Studies as one update/revision unit.** Not chosen: per-Study
  management localizes independent changes, accepting explicit caller-side
  coordination of the collection for comparison and reload rather than making
  the collection a single Study revision or tying Artifact lifetimes to it.

- **Discard the old display as soon as reload starts, or automatically restore it
  after a later failure.** Not chosen: preserve the existing display handoff at
  the first new batch and the accepted-batch retention contract. This accepts
  keeping old display data until that handoff and potentially retaining a
  partial new generation after failure, rather than implying an all-or-nothing
  reload or treating the old display as new input.

## Consequences

Batch validation and application must preserve trial/metadata consistency,
including metadata-only changes and historical constraint reclassification.
Consumers cannot use row count as a revision identity or infer load completion
from a committed revision. Analysis readiness checks must use the relevant load
state and data requirements, and consumers must retain result correspondence to
the input revision and load state. ADR-0021 specifies retained-input mechanisms
and per-analysis readiness contracts as adopted design, without claiming
implementation.
Retained inputs must remain read-only and valid until their consumers finish;
obsolete data must not be kept indefinitely merely because it once backed a
computation. Concrete release, sharing, and cache policies are in that adopted
boundary design rather than new requirements inferred from memory accounting.

Result adoption must distinguish acquisition generation from within-generation
revision as well as target Study and analysis conditions. Neither matching paths,
Study IDs, nor row counts establish that a result belongs to the current input.
Resource invalidation must continue to follow each resource's own lifetime, not
automatically inherit the analysis generation boundary.

Collection ownership does not make every Study update invalidate the collection.
Callers coordinate affected Studies and shared resources while preserving each
Study's independent state and result validity. Concrete unit/collection APIs and
synchronization are addressed in ADR-0021. The reload retention policy above does not
select error wording, operation availability, or individual readiness conditions
beyond the front-dependent requirement above.

Reproduction evidence, correction work, and dependency status belong in Issues.
If a candidate is not reproduced, retain the evidence there rather than claiming
a fix. Separating those candidates does not add a general polling redesign or
itself establish the external-read guarantee, and authorizes no ingestion code
fix in #249. External-read validation and any correction remain separate work;
the database read session ends with acquisition rather than retained analysis
input lifetime.
