# ADR-0019: Responsibility refactoring goal and scope

Status: Accepted
Date: 2026-10-16

## Context

The responsibility refactoring is intended to support stable ongoing development
by limiting the impact of feature additions and fixes. Combining structural
changes with execution-path unification would mix and expand the change and
verification scope.

The existing data representations originated in a proof of concept. Most of the
necessary information is already present, but how it is grouped and held needs
review alongside responsibility boundaries rather than being fixed in advance.

## Decision

- Prioritize keeping feature additions and fixes within the relevant areas.
  Reducing crate count or build time is not itself a success criterion.
- Preserve existing user-facing behavior, data semantics, and execution
  approaches while clarifying responsibilities, dependency directions,
  ownership, and public APIs.
- Exclude unification of the generic runner and Grasshopper execution paths from
  this refactoring; that requires a separate decision.
- Clarify the responsibility boundary between data I/O and analysis computation
  in this refactoring. Analysis computation takes already-loaded data as input;
  storage connections and format-specific processing belong to data I/O.
  The design includes the data and APIs crossing this boundary, dependency
  directions, and verification of the boundary.
- Target separate library crates for four main responsibilities, with Cargo
  enforcing dependency constraints:
  - Study model and management: the logical trial source of truth, explicit
    state ownership, and consistent analysis input provision.
  - Storage and data I/O: format interpretation, load results and update contents,
    and writing.
  - Analysis and model computation: explicit-input computation, surrogate models,
    numerical optimization, and reports.
  - Optimization execution: real-evaluation loops, external integrations, and
    recording.

  Together with the GUI and MCP entry crates, this targets approximately six
  crates, not an exact final count. Module-only reorganization is not the
  completed state. This is four libraries plus two entries, not the earlier
  unapproved dashboard / analysis / execution / mcp four-crate proposal.
- Use product-scoped package names: Tunny is a product family, and this product
  is Tunny Dashboard. Use `tdashboard` as the short form of Tunny Dashboard for
  the supporting packages. The six final package-name choices are:

  | Package | Responsibility |
  | --- | --- |
  | `tunny-dashboard` | GUI entry |
  | `tdashboard-study` | Study model and management |
  | `tdashboard-io` | Storage and data I/O |
  | `tdashboard-analysis` | Analysis and model computation |
  | `tdashboard-optimization-runner` | Real-evaluation optimization execution |
  | `tdashboard-mcp` | MCP entry |

  Do not use names such as `tunny-engine` that imply a product-family-wide
  foundation or anticipate extraction of one. These are the current six targets,
  not a permanent exact crate-count constraint; do not preemptively add shared
  crates for convenience. Preserve the GUI executable name `TunnyDashboard`.
  MCP executable naming and directory renaming are not decided by this package
  naming choice.

  Use `optimization-runner` rather than `run` to distinguish control of
  optimization with real evaluations from application startup. This
  responsibility accommodates both the generic and Grasshopper execution paths;
  the name does not imply their unification.
- Storage and analysis must not depend on each other. Both use Study model and
  management, and callers compose their interaction. Centralize shared domain
  meanings and classification rules in the Study model rather than reinterpret
  them in each consumer. The logical source of truth is interpreted domain data,
  not raw file contents. External-file resolution belongs to I/O;
  analysis-specific derivations and results belong to analysis. Do not introduce
  a generic post-load processing layer or an additional crate for that purpose.
- Completion requires removing analysis computation's implicit dependencies on
  the process-wide shared store and active study. Callers obtain the target's
  already-loaded data and pass it explicitly to analysis computation. A staged
  internal migration is allowed, but retaining these implicit dependencies is
  not the completed state.
- Move loaded-Study management from implicit process-wide shared state to an
  explicitly created management unit owned by the caller. Data I/O returns load
  results or update contents; the management unit applies them and provides
  consistent analysis inputs. The GUI's active-study selection is screen-side
  state, not implicit analysis state. The unit belongs to Study model and
  management; its type name, concrete placement within that responsibility,
  whether it holds multiple Studies, and synchronization remain undecided. This
  defines ownership responsibilities without prescribing memory synchronization
  or approving a redesign of overall state management.
- Include data-model grouping, ownership, updates, and handoff to analysis in
  the overall design. Do not require preservation of the existing internal
  representations; preserve the meaning of the data instead.
- Establish one logical source of truth for trial information across all states
  within a Study, with clear ownership of trial identity, state, parameters,
  results, and related information. Analysis tables, column-oriented data,
  arrays, and caches are derived from that source, not independent sources of
  truth. This does not prescribe one large struct, row-oriented storage, full
  recomputation, or concrete storage, APIs, types, owner implementations, or
  update and synchronization mechanisms. Existing trial ID and other data
  semantics remain unchanged.
- Study management provides consistent domain data, including trial states,
  parameters, results, and metadata. Analysis constructs its derived
  representations, such as completed-trial selections, column-oriented tables,
  and features. These representations and necessary caches belong to explicit
  analysis-side objects; Study management does not call analysis to build them.
  This does not require full reconstruction: incremental updates, reuse, and
  concrete cache lifetime policies remain undecided. Algorithm-internal types
  need not be unified, and the existing DataFrame is not designated as the source
  of truth.
- Each analysis completes using a consistent set of data as acquired for that
  invocation. Subsequent updates belong to a later analysis, not the running
  computation. Results correspond to the target Study and input data revision,
  allowing callers to decide whether to adopt them or recompute. This contract
  does not prescribe copying all data on each invocation, locking, shared
  references, atomic publication, revision types, or automatic recomputation
  policies.
- Shared data and results require explicit owners, target Study and input
  revision correspondence, and defined lifetimes; analysis results also require
  their analysis conditions. Crate separation does not prohibit sharing these
  objects. Preserve the storage-scoped Artifact map and session-scoped explicit
  Artifact root rather than forcing them into Study-scoped lifetimes. This does
  not require new MCP Artifact features or GUI/MCP cross-process cache sharing.
- Keep the design coherent, but stage implementation through a parent Issue
  describing the shared target state and child Issues that can each be verified
  and completed individually. This approves the decomposition approach, not a
  particular child-Issue sequence or implementation plan.
- Do not mix in new features or storage formats, changes to existing ID or
  constraint semantics or other data meanings, unification of all numerical
  algorithms' internal representations, or a general-purpose framework.

Accepted status applies to the goal, scope, responsibility policies, library
separation target, package names, and staged Issue approach above, not to a
complete concrete architecture. Beyond the dependency constraints above, the
remaining dependency graph, exact final crate count, public APIs, concrete data
types and module boundaries, owner implementations, memory synchronization and
atomic publication mechanisms, child-Issue sequence, and migration steps remain
undecided.

## Alternatives

- **Use a product-family-wide name such as `tunny-engine`.** Rejected because it
  suggests a shared foundation for all Tunny products rather than this Dashboard
  refactoring. Keep product scope explicit without anticipating shared-platform
  extraction.
- **Use the full `tunny-dashboard-` prefix for supporting packages.** Replaced by
  the shorter `tdashboard-` prefix to keep names concise while retaining their
  Tunny Dashboard scope. The GUI package remains `tunny-dashboard`.
- **Couple storage and analysis to perform shared post-load work, or duplicate
  domain rules in consumers.** Rejected because either spreads change impact or
  risks divergent meanings. Prefer shared Study-model rules and caller-composed
  handoff, accepting explicit ownership, revision, condition, and lifetime
  contracts for shared objects.
- **Use modules only, without separating the four library responsibilities.**
  Rejected as the target because it does not enforce these dependency boundaries
  through Cargo, despite requiring fewer cross-crate API changes.
- **Split only some of the four library responsibilities.** May be an intermediate
  stage, but not the target: it reduces initial migration work while leaving
  some intended dependency constraints unenforced at crate boundaries. Accept
  the API and dependency-design work needed for the four-library target instead.
- **Unify the generic runner and Grasshopper execution paths as part of this
  refactoring.** Rejected for this scope because it mixes execution-path changes
  with responsibility clarification and expands both the changes and the
  verification required. This does not reject future unification.
- **Retain implicit shared-store and active-study access in analysis computation.**
  Rejected in favor of explicit inputs to localize change impact and make
  analysis easier to test, accepting the need to update callers.
- **Retain implicit process-wide loaded-Study management while making analysis
  inputs explicit.** Rejected because load/update application and lifetime
  ownership would still depend on ambient state. Prefer caller-owned management,
  accepting changes to caller lifecycle handling and I/O result/update handoff.
- **Keep completed-trial analysis data and all-trial auxiliary information as
  independent sources of truth.** Rejected because separate authorities leave
  ownership and reconciliation responsibilities split. Prefer one logical
  source with derived analysis representations, accepting the need to define
  derivation and update responsibilities without prescribing synchronization.
- **Generate and retain a common analysis table in Study management.** Rejected
  because analysis-specific representation changes would then affect Study
  management. Keep derivation and caches on the analysis side instead, accepting
  responsibility there for constructing and maintaining derived representations
  from consistent domain inputs.
- **Follow incoming updates during an analysis.** Rejected because a computation
  could mix data revisions and lose a clear correspondence between its inputs
  and result. Prefer a consistent acquired input, accepting that the result may
  no longer describe the latest data when it completes.
- **Require data updates to stop for the duration of an analysis.** Not adopted
  as a requirement: the agreement defines consistent inputs and result revision
  correspondence, not how synchronization achieves them. Locking and whether
  updates wait remain implementation decisions; no nonblocking-update guarantee
  is established.
- **Preserve the existing data representations as a design constraint.** Rejected
  because fixing the current proof-of-concept organization could force later
  boundary changes. Include data-model organization in this design instead,
  accepting that internal representations may need to change.
- **Implement the entire redesign as one change.** Rejected in favor of
  individually verifiable child Issues under one shared target state, accepting
  coordination between implementation stages rather than one large change and
  verification scope.

## Consequences

Architecture proposals must be evaluated against change locality while
preserving existing behavior and execution approaches. Existing execution paths
remain separate for this work, accepting that their duplication is not resolved
here. This decision does not establish a final implementation plan.

Analysis callers must acquire and pass the intended data rather than rely on
ambient study selection. This requires caller changes as well as analysis API
changes while preserving existing user-facing behavior and execution approaches.

Data-model and responsibility decisions must fit the same target state even
when implemented separately. Staging limits individual change and verification
scope without treating existing internal representations as permanent boundaries.

Analysis representations may remain specialized for their consumers, but their
derivation and updates must follow the logical trial source rather than become
independently authoritative. Logical authority does not require one physical
representation or a particular recomputation strategy.

Study management supplies domain inputs without constructing analysis tables.
Analysis-side objects own derived representations and caches, keeping their
evolution out of Study management without prescribing a rebuild or reuse policy.

Callers must be able to relate results to their target Study and input revision
before adoption or recomputation. This adds input/result correspondence and
validation responsibilities without choosing a retention mechanism for acquired
data or an automatic refresh policy.

Callers must explicitly create and own loaded-Study management. I/O no longer
implicitly installs loaded data or updates into process-wide Study state; it
returns them for the management unit to apply. This separates I/O from state
ownership without selecting a concrete manager API or synchronization strategy.

The library separation requires explicit cross-crate APIs and dependency design,
including decisions about currently shared computation. Beyond the stated
constraints, specific dependency edges and shared optimizer placement remain
undecided. The generic and Grasshopper execution paths are not unified.

Use these concrete design and verification scenarios to preserve common meaning
and sharing across the boundaries:

- Preserve result sharing between the trial table and Pareto, clustering, and
  MCDM widgets. For the same Study, input revision, and analysis conditions,
  consumers reuse the same computed result rather than recompute it for display
  or maintain independent authoritative results. Preserve row-to-trial
  correspondence through filtering, cluster ordering, and MCDM ranking using
  stable trial identity, not display position or a conflation of row index,
  global trial ID, and Study trial number. Verify both matching settings sharing
  results and different settings retaining their own results. This does not
  prescribe identifier types, automatic synchronization of all widget settings,
  or new UI behavior for trials outside an analysis result's target set.
- Constraint schema evidence comes from all trial states. Growth in expected
  constraint count reclassifies historical trials; initial and incremental loads
  must agree under [ADR-0007](0007-distinguish-three-constraint-feasibility-states.md).
  Shared rules do not require identical analysis-specific selection policies.
- Separate Artifact association metadata from external-file resolution and GUI
  image/texture resources. Preserve trial identity and the root/map lifetimes in
  [ADR-0011](0011-session-artifact-root-default.md) and
  [ADR-0012](0012-scope-artifacts-to-storage.md), including invalidation on source
  replacement and reload, without imposing a single lifetime on every resource.

Sharing requires explicit handoff and validity contracts, not a new global store
or a requirement to share runtime caches between separate processes. Concrete
APIs, cache policies, and synchronization remain to be designed.
