# ADR-0021: Study, analysis, and I/O boundary contracts

Status: Accepted
Date: 2026-10-16

## Context

[ADR-0019](0019-responsibility-refactoring-goal-and-scope.md) fixes the product
responsibilities and names. [ADR-0020](0020-commit-study-updates-by-validated-batch.md)
records the accepted update, readiness, generation, retention, and external-read
policies. [Issue #249](https://github.com/Tunny-gh/Tunny-Dashboard/issues/249)
requires concrete contracts before implementation, not another working global
store or a wrapper around the COMPLETE-only DataFrame and all-state extras.

This concrete design was independently reviewed and adopted before implementation.
Acceptance applies to the design contracts, not completed implementation or
database validation. The sketches are semantic Rust API sketches, not compiled
code. The
[validation report](../reports/2026-10-16_study-boundary-design-validation.md)
separates current evidence, readiness contracts, fixtures, and migration handoff.
Neither document claims tests passed or closes an implementation Issue.

## Decision

### Dependency graph and physical boundaries

The arrows below mean direct production dependency; no other workspace edge is
allowed in the final layout. Test fixtures that need both sides live above them,
not as reverse production dependencies.

```text
tunny-dashboard -> tdashboard-study, tdashboard-io,
                   tdashboard-analysis, tdashboard-optimization-runner
tdashboard-mcp  -> tdashboard-study, tdashboard-io, tdashboard-analysis
tdashboard-optimization-runner -> tdashboard-study, tdashboard-io, tdashboard-analysis
tdashboard-io       -> tdashboard-study
tdashboard-analysis -> tdashboard-study
tdashboard-study    -> no other workspace crate
```

The GUI and MCP compose I/O, Study management, and analysis; I/O and analysis do
not call each other. Keep `egui-app/` and `mcp-server/` directory names and explicit
binary names `TunnyDashboard` and `tunny-mcp`. Change package names, not user-facing
executable names. New libraries use `crates/study/`, `crates/io/`,
`crates/analysis/`, and `crates/optimization-runner/`. Keep `rust_core/` as the
temporary legacy package during extraction, then retire it; do not rename all
existing directories in anticipation of a different product layout.

- **Study** owns neutral IDs, trial/metadata records, shared feasibility and
  distribution semantics, validated updates, immutable snapshots, and per-Study
  management. Dependencies are standard facilities and already used serde/JSON
  types; no SQL, numerical model, image, GUI, or process dependencies.
- **I/O** owns format parsing/serialization, storage connection/read sessions,
  journal writer, CSV record encoding, safe external-file resolution and writes.
  It returns domain records/updates, never installs them in a global store.
- **Analysis** owns numerical math/RNG, optimizers (including NSGA-II and CMA-ES),
  model fitting/acquisition, `FitProgress`, projections/tables, result sessions,
  report generation and pure Markdown/HTML/JSON/SVG rendering. Expose the existing
  optimizer configs/minimize functions and progress update/check/subtask methods
  through narrowly documented public modules. Preserve seeds, cancellation checks,
  counters, and penalty algorithms; do not duplicate optimizers in execution.
- **Execution** owns the distinct generic and Grasshopper loops, evaluator/process
  integration, GH parsing/Compute lifecycle, recording orchestration and run
  progress. It calls analysis algorithms and the I/O journal writer. Numerical
  optimization on a fitted model is still analysis. No loop unification.
- **Entry adapters** choose report inputs/options, invoke pure analysis rendering,
  and hand bytes to I/O. CSV export selects rows/derived values in analysis or the
  caller, passes an explicit column/row view to I/O encoding, then writes via I/O.
  No I/O dependency on report or DataFrame implementations is needed.

Move `lgbm`, `lgbm_sys` and numerical dependencies with analysis. Move native link
and Windows DLL-copy responsibility from legacy core to analysis's build script;
resolve the existing workspace `libs/` directory from the new manifest location.
Retain necessary final-binary link/rpath handling on macOS/Windows/Linux, without
assuming an rlib's link arguments automatically reach every executable. Verify
analysis tests, GUI, MCP and execution test binaries on existing CI platforms;
do not add a second native library provider or change the distribution format.
Update GUI license-metadata root lookup from `tunny-desktop` to `tunny-dashboard`
when renaming the package. Keep current dependency versions unless extraction
actually requires a separate reviewed change.

### Logical all-state model

Use immutable per-trial records, not DataFrame rows as authority. Absence is not
zero, an empty category, JSON null, or an invalid evaluated number. Preserve
nonfinite numeric observations; method-specific filtering remains in analysis.

| Model element | Stored information and interpretation |
| --- | --- |
| `StudyMetadata` | Stored Study ID/name, ordered objective descriptors (explicit index, name, direction), observed parameter names, declared numeric bounds, user-attribute names, constrained-study evidence and expected constraint width. Names do not identify columns across roles; objective identity is its index. |
| `TrialRecord` | Source-scoped trial key, immutable Study membership and Study trial number, all five trial states, parameter observations/distributions, indexed objective observations, original JSON user attributes, constraint observation, timestamps, intermediate values, Artifact associations and acquisition-order evidence. Read all available fields for non-COMPLETE trials too. |
| `ParameterValue` | `Numeric(f64)` or `Category(serde_json::Value)`; missing parameters are absent map entries. A declared category with an unresolvable choice is present but unavailable, represented separately from absent. Category identity retains JSON type and presence; display labels are derived. |
| `ParameterObservation` | Explicit value availability plus known distribution kind/bounds/step/log/discrete eligibility/choices (optional when unrecorded). Keep values in external units; category index decoding is I/O interpretation. Do not invent Optuna distributions for CSV. Preserve source declarations without changing current bounds/grouping/rounding; do not retain every uninterpreted distribution/system-attribute blob. |
| `ObjectiveObservation` | Map keyed by objective index; absent entry means unobserved, present `f64` can be NaN or infinity. Do not compact sparse indices into different objectives. Analysis builds ordered columns using descriptors and current missing-value rules. |
| `ConstraintObservation` | Attribute presence independent of values; ordered slots with invalid/nonfinite positions preserved. Width is not reduced by invalid values. The maximum observed array width over all current trial states is schema evidence. Attribute-only constrained Studies remain distinct from unconstrained ones. |
| User attributes | Map of original present JSON values (including explicit null, arrays, objects); absence is no entry. Numeric/text display maps are projections, not parallel authorities. |
| Time/intermediate values | Optional naive Unix seconds as currently interpreted; sorted step/value observations and all-state status. No timezone or pruning-policy change. |
| `ArtifactAssociation` | Trial key, source Artifact ID or CSV relative reference, original name, MIME and entry order. No resolved path or texture. Do not capture geometry or all system attrs. |

Use enum/newtype boundaries even where both representations use `u32` or `f64`.
An unavailable parameter value has a reason (missing/invalid encoded value), not
a fabricated category. Distributions retain numeric kind and discrete eligibility
without declaring numeric-looking categories ordinal. A trial may have no
objective result in any state. Keep JSON numeric/category equality separate from
floating-point feature encoding. The existing adapter's rounding/display fallback
and CSV type inference remain at their current compatibility boundary; do not
silently apply one analysis method's encoding to all consumers.

Study maintains constraint evidence over all records; analysis invokes Study's
pure three-state classifier with that evidence, then constructs `is_feasible`
and `constraint_sum` columns where needed. Schema growth invalidates all affected
derived classifications, even with no new COMPLETE rows. Authoritative full
replacement recomputes evidence from the replacement; incremental evidence must
agree with the same final records. Model validation checks identity, membership,
sequence and shape, but does not reject incomplete evaluations just for being
incomplete or convert them to observed zeros.
Keep declared metadata and observed all-state fields distinct from the
COMPLETE-only parameter/objective/attribute column discovery currently used by
analysis. Retaining a non-COMPLETE parameter must not by itself add new visible
columns or change a method's feature set. I/O supplies objective indices; source
metric names and their existing fallback order remain preserved.
Keep an optional `completed_order` acquisition ordinal for the order in which
the decoder exposes COMPLETE observations. Streaming Journal assigns it on first
completion emission; full Journal/RDB readers assign their existing returned-row
order. Attribute-only upserts retain it; authoritative replacements supply their
own order. It is neither trial number nor a new persistent trial identifier.
The sorted handle directory is only a lookup layout: the default COMPLETE
projection orders by this evidence, not by hash-map traversal or a newly imposed
ID sort. Reject duplicate ordinals in one COMPLETE projection. Compare semantic
ingestion equivalence by TrialKey, while separately testing each path's existing
display/tie-breaking order. No all-path row-order unification is implied.

### Namespaces and update protocol

These are process-local keys, not persisted IDs or new Optuna numbering:

```rust
struct SourceInstanceId { namespace: NamespaceId, serial: u64 }
struct StudyKey { source: SourceInstanceId, stored_id: u32 }
struct TrialKey { source: SourceInstanceId, stored_id: u32 }
struct InputStamp { study: StudyKey, generation: u64, revision: u64 }
struct AcquiredStamp { input: InputStamp, load_epoch: u64 }
struct LoadTicket { study: StudyKey, generation: u64, operation: u64 }
struct BatchHeader { ticket: LoadTicket, sequence: u64, base_revision: u64 }
```

Caller-owned identity allocation creates a namespace per owner and never reuses
source serials during that namespace's lifetime. Namespace allocation may use a
checked process counter, but stores no Study data or active selection. No counter
wrap is allowed. Independent collections sharing a result cache must share that
allocator or have distinct namespaces; caches/keys are never exchanged across
processes. Trial IDs are the existing global Journal ordinal/RDB ID/CSV row ID;
Study membership is a separate record field. `StudyTrialNumber(u32)` is not an
index or TrialKey. Source instance changes on Open/New/replacement, even if the
path is identical. Explicit Reload preserves source instance and stored Study
identity but increments that Study's acquisition generation. Reloading comparison
Studies is caller composition, not one atomic cross-Study transaction.

Select an explicit single-writer `StudyManager` per Study:

```rust
impl StudyManager {
    fn new(key: StudyKey) -> Self;
    fn begin(&mut self, kind: InitialOrReloadOrRefresh) -> Result<LoadTicket, BeginError>;
    fn apply(&mut self, header: BatchHeader, batch: StudyBatch)
        -> Result<ApplyReceipt, ApplyError>;
    fn finish(&mut self, ticket: LoadTicket, next_sequence: u64)
        -> Result<AcquiredStamp, ApplyError>;
    fn fail(&mut self, ticket: LoadTicket, error: LoadFailure);
    fn supersede(&mut self, ticket: LoadTicket);
    fn acquire(&self) -> Option<StudyInput>;
    fn load_state(&self) -> LoadState;
}
```

`Initial` is generation 1. `Reload` starts the next generation and invalidates
the prior loader's ticket, but retains the old visible snapshot until the first
accepted new batch. A refresh of already acquired DB data (if called by an existing
consumer) stays in the same generation and uses a new operation ID and captured
base revision; it is not an explicit Reload or a new polling feature. Each
operation starts at sequence 0 and has at most one outstanding apply operation.
Initial/reload has a private empty candidate base at revision 0, even while an
older visible root is retained. Its first batch builds the new prefix from that
empty base, never appends it to old-generation trials or extras. Refresh instead
uses the current generation/revision as its base. A second begin supersedes the
first operation; late failure/finish messages are ticket-checked too.

The entry's delivery adapter adds BatchHeader using the last ApplyReceipt and
acknowledges each batch before submitting the next. I/O emits owned batches to a
caller-supplied fallible sink (or iterator); it does not predict revisions or
call a StudyManager. The sink can marshal to the owner's thread and await the
receipt. This preserves streaming/backpressure without making I/O depend on
GUI message types, and supports metadata-only/no-op batches whose sequence and
revision do not advance together.

`StudyBatch` is either a delta of full trial upserts/removals plus explicit
metadata edits, or an authoritative full replacement. New records supply their
identity/state/number; omission inside an upsert means absent, not 'unchanged'.
An unchanged trial is not sent. Metadata edits explicitly distinguish unchanged,
set and remove; parsers assemble repeated low-level records before producing
coherent trial upserts. This permits late attribute correction without a second
authoritative row store. The Journal adapter tracks replay builders only as
acquisition state, including non-COMPLETE records; #258 decides supported late
records before that affected path migrates.
A streaming flush includes observed non-COMPLETE trial changes needed to explain
its schema evidence, not only newly COMPLETE rows. Acquisition builders may
retain or share references to prior decoded records until replay ends to assemble
upserts; they are released at acquisition end and do not accept independent
post-load writes. Snapshot/key fields are private in implementation: consumers
cannot relabel an old root by constructing a new-generation header themselves.

Validation builds a candidate off the published root. Reject a different source,
Study, ticket/generation, unexpected sequence, or stale base revision without
modifying data or consuming the sequence. Duplicate/replayed batches are rejected,
not silently appended or assumed equivalent. I/O retries do not replay accepted
updates. On valid application increment expected sequence; advance data revision
only if domain contents change (first publication in a generation is revision 1,
including empty input). Compare numeric observations using stable representation
equality for change detection, including NaNs; do not make NaN force endless
revisions. This equality is not categorical feature ordering or cross-generation
cache equivalence. Checked counters fail rather than wrap.

- A metadata-only change can change the revision and historical feasibility.
- A no-data-change batch consumes its sequence but keeps data revision.
- `finish` validates ticket and next sequence, marks Complete, and advances
  `load_epoch`, not data revision if contents are unchanged. An empty acquisition
  must first apply an empty authoritative batch, then finish; never accidentally
  expose an old generation as a successful empty load.
- `fail` or `supersede` changes operation/load state and load epoch, not committed
  contents. A terminal ticket cannot subsequently apply/finish. Failed validation
  is reported to the caller, which terminates that attempt via `fail`; there is
  no hidden model retry. An explicit user reload creates a new generation.
- DB refresh returns an authoritative replacement, removing omitted records and
  retaining a revision only when domain contents are unchanged. A stale base is
  discarded; a future existing poll/explicit request may reacquire. No automatic
  retry loop is added.

`LoadState` records requested ticket, phase (Loading/Complete/Failed/Superseded),
and last accepted sequence. An input has the state captured with its root, not
a mutable reference to this status. Every published input header whose captured
load state changes gets a new checked manager-local load epoch, including
accepted-sequence progress with unchanged contents. Failure/completion of the
current visible generation publishes a new header sharing data. A request for a
different generation does not retag the retained old display's header.
During reload before first acceptance,
`acquire` still returns the old generation's explicitly stamped visible input;
the new request's load state is separate. It is never relabeled as new data.
Result consumers match their designated input slot, not a path or newest request
counter; old-display results cannot populate a new-generation slot. Fail before
first acceptance retains old display; fail later retains accepted new data.

There is no new GUI Cancel Load operation. Replaced-source/ticket rejection is
mandatory; stopping wasted loader work is cooperative and cannot publish after
supersession. Existing fit/run cancellation keeps its own progress handle and
does not roll back committed Study batches. Error presentation and enabled UI
controls preserve existing adapters; these APIs do not invent new warnings.

### Immutable retention without a persistent collection framework

Use `Arc<StudyData>` as the immutable root with `Arc<StudyMetadata>` and
`Arc<Vec<Arc<TrialRecord>>>`. The trial directory is sorted by stored trial ID;
binary search provides lookup without a second snapshot-owned ID index. Analysis
chooses its own row order and carries TrialKeys. Copy-on-write clones the vector
of handles when shared, not every trial's parameter/JSON payload. Each record's
large maps/arrays are Arc-owned; changed maps share unchanged JSON values through
`Arc<Value>` entries. Mutating a candidate uses `Arc::make_mut` only on private
candidate paths, and never mutates a published shared record.

Upsert replaces one record handle; insertion/removal builds the sorted candidate
directory; a metadata-only update shares the trial directory. Validate the entire
batch (including duplicates/membership/schema effects) before publishing the new
root. Full replacement may reuse equal existing record handles but must not keep
omitted records. Revision comparison includes metadata, not only trial counts.
This intentionally accepts O(N) handle-directory copying on a shared-root update,
not O(total trial payload) copying. Measure before introducing pages or persistent
collections; no throughput/RSS bound is promised. This avoids a custom page/index
protocol and arbitrary page-size API in the first slice.

The caller owns `StudyManager` with `&mut` access on its orchestration thread;
workers receive cloned immutable `StudyInput`s and send results/updates back.
No global store, full-analysis lock or ArcSwap requirement. If a caller uses a
Mutex, hold it only for acquisition/application, never while running analysis.
`StudyInput` holds an immutable header (AcquiredStamp and captured load state)
plus the root Arc. Status-only completion publishes a new header sharing data.
The manager keeps one visible root, not a history or parent-root chain. Old
display, prepared inputs and running jobs retain exactly the roots they need;
dropping these handles frees obsolete roots. A running DB reader never survives
inside a StudyInput. Failure records contain masked diagnostic text, not raw
credential-bearing storage URLs.

### Analysis readiness, provenance and result ownership

Use a caller-owned `AnalysisSession`, not caches in Study or a static singleton:

```rust
fn prepare(inputs: Vec<StudyInput>, request: AnalysisRequest,
           dependencies: &[ResultHandle]) -> Result<PreparedAnalysis, Readiness>;
fn execute(work: AnalysisWork, progress: Option<&FitProgress>)
    -> Result<ResultHandle, AnalysisFailure>;
fn can_adopt(result: &ResultHandle, target: &ResultKey) -> bool;
impl AnalysisSession {
    fn lookup(&self, key: &AnalysisKey) -> Option<ResultHandle>;
    fn start(&mut self, prepared: PreparedAnalysis, mode: ReuseOrRerun) -> WorkOrShared;
    fn complete(&mut self, completion: AnalysisCompletion) -> Adoption;
    fn retain_interests(&mut self, keys: &[AnalysisKey]);
}
```

Preparation owns the acquired inputs, selected columns/objective indices,
feasibility policy, finite-row selections, per-input `Vec<TrialKey>` row maps and
typed settings. Single-Study requests use a one-element input vector; comparison
requests retain every participating input. Each row map identifies its input
slot, so source rows are never flattened across Studies accidentally.
Numerical implementations may borrow slices or make their existing
matrices; they need not use one universal internal representation. Readiness
distinguishes Waiting (missing completion/dependency), Unavailable (insufficient
or unsupported data), and Ready. Runtime fitting failures and existing NaN,
empty-result/zero fallbacks are not all converted into readiness rejection.
The [method table](../reports/2026-10-16_study-boundary-design-validation.md#readiness-contracts)
defines the concrete per-path boundary; widgets display those decisions and
keep only local busy/pending/presentation state.
Single-metric and heatmap preparation are distinct request kinds: a heatmap can
remain a ready displayable result with existing zero entries for unavailable
individual metrics, while the single-metric path returns its existing error.
Do not turn per-metric minimums into a new blanket heatmap rejection.

`AnalysisKey` contains operation kind, ordered AcquiredStamps for all Study
inputs, canonical conditions and dependency ResultKeys. Conditions include ordered
feature/objective identities, subset/feasible choice, model/acquisition/seed/grid
and method parameters actually used. Preserve existing MCDM normalized/quantized
weight keys and method-specific v normalization. Do not quantize unrelated
methods merely for one key format; validate floats then use canonical bit keys.
Presentation-only Top N, colormap, camera, pin/highlight/filter display ordering
are excluded unless they actually select calculation inputs.
`ResultKey` adds a session-local checked run serial to an AnalysisKey. An explicit
rerun supersedes the pending serial for that key; its late predecessor cannot
overwrite it. Dependency keys retain ResultKeys, not unstamped payload pointers.
`start` assigns that serial to an AnalysisWork or returns the existing shared
job/result handle. `complete` validates it against the selected current key;
failures and cancellation completions carry the same key as successes, so stale
errors cannot overwrite a newer request's UI. Existing diagnostic adapters remain.
Reuse versus rerun is an orchestration choice mapped from each existing action's
cache behavior, not a new universal rule that every Run button forces a refit.
Report requests include their language/Top N/options and source presentation
metadata; generated timestamps are request-local and reports are not cached
across invocations by default.

Prepared dependencies are read-only handles whose keys identify their inputs and
conditions. A finalized Pareto front dependency must match Study, generation and
data revision and have a Complete load-state stamp. Loading's zero-padded ranks
are never a front result. Cluster/MCDM ranking require it; entropy does not.
Report's existing internal snapshot-local candidate calculation is not a new
shared partial Pareto front or a substitute for finalized dependency results.
Status-only Complete can therefore enable front finalization without inventing
a data change. Load epoch remains in result provenance, so a partial/failed
result is not silently labeled completed even when data revision is unchanged.

Results contain their key, typed payload and explicit row/subset mapping. Core
row indices are translated via the preparation map, never treated as global IDs
or Study trial numbers. Compare/convergence keys contain each input's stamp and
shared reference-point conditions in caller order; validating only the base
Study is insufficient. A change in B invalidates results depending on B, not all
independent A results. Retained trained models carry their fit key; optimize/
suggest keys include that dependency, and cannot silently act on a new input.

Shared settings select the same session result for table/plots/Gallery. Different
settings select different keys. In-flight jobs with the same key share their
completion handle rather than duplicate work just for a second display; explicit
rerun can replace a result for that key without changing other settings. Admission
and busy flags remain caller orchestration. The GUI keeps a handle/key for
most-recent MCDM coloring, separate from each widget's selected MCDM key; analysis
does not choose a global coloring policy.

Cache interests are the keys selected by live consumers, explicit most-recent
presentation handles, and running requests. On revision/generation/load-epoch
change detach obsolete cache lookup entries; old jobs may finish but cannot be
inserted as current results. Uninterested completed entries are released, not
accumulated for every historical settings combination. Computed payloads normally
retain provenance/row keys, not the source root once computation ends; prepared
tables may retain input while explicitly in use. External result handles can
outlive lookup entries but remain stamped. MCP uses request-local sessions; no
cross-process cache or numerical memory cap is introduced.

### Artifact resources

I/O extracts associations and resolves files with the existing root containment,
existence/MIME/CSV rules. A caller-owned `ArtifactResources` holds source-scoped
association/resolution maps and a scan revision for the open storage. It is not
owned by one StudyManager. The GUI session owns explicit root choice, preserving
CLI/GUI precedence; the GUI owns URI/image/texture and Animation caches.
Resolution tickets include source instance, root-choice revision and scan ID;
accept only the matching request. Study selection filters associations through
TrialKeys without erasing the storage map. Reload rescans/increments resource
revision as today without resetting session root or conflating it with analysis
generation. Source replacement invalidates old scans even if stored IDs repeat.
Existing RDB Artifact support is not expanded, and Animation's resource budget
is not an analysis cache budget. Preserve ADR-0011/0012/0018 behavior.

### Database acquisition sessions

Keep backend mechanics private to I/O. The common domain handoff is owned
`StudyBatch` data, not a DB connection, transaction lifetime, SQL row or isolation
enum. Internally use a scoped callback/read-session interface so every query
needed for a Study (including metadata/extras/constraints) uses the same session:

```rust
fn read_study(&mut self, stored_study: u32) -> Result<OwnedStudyLoad, ReadError>;
// Backend-private implementation boundary, not a Study/analysis trait:
fn with_study_read<T>(&mut self, read: impl FnOnce(&mut ReadSession) -> Result<T, ReadError>)
    -> Result<T, ReadError>;
```

Use concrete backend implementations or enum dispatch for this scoped generic
boundary; do not add a generic method to a `dyn OptunaBackend` interface and
assume it remains object-safe. The borrowed query facade is confined to I/O.

For the SQLite correction reviewed on 2026-10-19, retain both public file-path
APIs but use one acquisition path: `sqlite::parse_single_study_rows` owns the
read-only connection and deferred transaction; `sqlite::parse_single_study`
reuses its owned result and the shared rows-to-DataFrame conversion only after
the session and connection end. The generic RDB parsers remain query decoders
with caller-owned session responsibility, not automatic snapshot APIs. This
keeps the correction local without adding server transaction hooks or duplicate
conversion logic. The user accepted acquisition-limited DELETE writer contention
and WAL checkpoint retention; the
[SQLite validation report](../reports/2026-10-19_sqlite-study-snapshot-investigation.md)
records the tested configuration and limits. PostgreSQL/MySQL/MariaDB are
explicitly excluded from this correction and remain unverified.

Selected mechanisms to validate in [#260](https://github.com/Tunny-gh/Tunny-Dashboard/issues/260):
SQLite explicit deferred read transaction on the existing read-only connection
(snapshot established by first read); PostgreSQL read-only REPEATABLE READ;
MySQL/MariaDB read-only REPEATABLE READ with a consistent snapshot on supported
transactional tables. Related queries and type conversion finish before session
end. Success ends the read transaction before returning owned data; query,
callback, cancellation or conversion failure rolls back/drops the session and
returns no invalid batch. Do not borrow transaction-backed cursors into a batch.

Do not automatically retry a failed read in this refactor: surface the existing
error and preserve accepted manager state; an existing poll or explicit reload
can initiate another request. No fallback to mixed reads while claiming a
snapshot. SQLite WAL versus rollback-journal waits/checkpoint retention, server
versions/engines/permissions and transaction cleanup are #260's configuration
gate, not assumptions. If a required configuration cannot meet the selected
mechanism, review that concrete finding before its I/O path ships; it does not
change #250's domain API. The selected mechanisms do not imply an implemented
all-backend guarantee, writer-wide business transaction, simultaneous multi-Study
snapshot, or no-wait promise.
Journal uses ordered replay of acquired bytes; no all-format external lock.

## Alternatives

- **Wrap DataFrame plus extras as the new authority.** Rejected: non-COMPLETE
  parameter/result/attribute coverage is absent, identities and schema evidence
  remain split, and analysis projections would still dictate the domain model.
- **Clone all trial payloads, keep a parent-root chain, or add persistent pages
  immediately.** Rejected in favor of Arc record/field sharing and a sorted handle
  directory: simpler standard facilities, no unlimited history, accepting handle
  copying until measured cost justifies a separate change.
- **Hold a lock or database transaction throughout analysis.** Rejected under
  ADR-0020: acquisition and update coordination end before numerical work, at the
  cost of overlapping retained inputs and explicit provenance checks.
- **Place optimizers/progress in execution or an extra common engine crate.**
  Rejected: surrogate analysis already uses them; execution can depend on analysis
  without a cycle or duplicating algorithms. Accept a small public numerical API.
- **I/O calls analysis to finish loading/reporting.** Rejected: entry composition
  preserves independent libraries while sharing neutral Study data and explicit
  rendered bytes/CSV values.

## Consequences

The first implementable slice has specified identity, model coverage, batch
validation, retention and API boundaries; no backend configuration choice is
needed to implement it. Signatures still require normal Rust implementation and
tests; this ADR does not assert compilation, memory measurements, or DB validation.
The [report](../reports/2026-10-16_study-boundary-design-validation.md) assigns
fixture expectations and bridge removal to existing Issues, not permanent TODOs
in this record. #258/#259/#260 remain independent investigation gates for their
affected I/O paths, not hidden fixes in #249 or unconditional blockers on all
work. A result that requires a correction cannot be shipped by merely moving the
bug and calling it a compatibility guarantee.

Independent review confirmed these contracts and handoff boundaries. Issue #249
remains open until its design PR is merged; design acceptance does not claim
implemented code, compiled APIs, or completed DB verification. No code extraction,
Issue completion, PR creation or merge is performed by adopting this ADR.
