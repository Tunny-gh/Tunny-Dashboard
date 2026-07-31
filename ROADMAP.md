# Tunny Dashboard Differentiation Roadmap

A feature-positioning and implementation roadmap for establishing this tool's edge, based
on a feature comparison against commercial PIDO tools (modeFRONTIER / Simcenter HEEDS /
Isight / Optimus) conducted in 2026-07.

## Strategic Axis

**Aim to be the "decision-support layer for the Optuna ecosystem," not a "scaled-down PIDO."**

Don't compete on the commercial tools' core value proposition — process integration
(solver connectivity, workflow definition, job execution). That territory belongs to
Optuna itself. Conversely, since commercial tools cannot read Optuna's results, the most
defensible position is to become "the only serious analysis and decision-support tool for
every user who optimizes with Optuna."

> **2026-07 update**: The goal above (the analysis/decision-support layer) has been
> achieved. For the next strategic axis, see
> "[Phase 2: Extending into the Execution Loop](#phase-2-extending-into-the-execution-loop-2026-07-policy-decision)".
> We are expanding into execution, but the scope of that expansion is deliberately limited.

## Assets Where We Currently Lead Commercial Tools

| Asset | Comparison with commercial tools |
|---|---|
| Formal MCDM methods (TOPSIS / VIKOR / PROMETHEE I & II / entropy weighting) | modeFRONTIER's MCDM tools come closest, but this tool covers a broader range of methods |
| History of multi-objective convergence metrics (Hypervolume / IGD+ / ε / R2) | None of the four commercial tools offer this as a public feature |
| Breadth of sensitivity analysis methods (Spearman / Ridge / RF-ANOVA / MDI / SHAP / Permutation / Sobol / ARD) | SHAP is absent from commercial tools (no legal barrier — it's a product-culture issue, so we can maintain our lead) |
| Algorithm transparency (bilingual online documentation + Python cross-validation) | An area where commercial tools are often criticized as black boxes |
| Native performance from Rust + wgpu | A basis for outperforming commercial Java/Qt-based UIs on large-scale trials |

These are assets to defend and grow. Messaging in the README, etc. should keep pace with
implementation.

## Short Term (closing strength gaps, low cost / high impact)

### 1. Complete Optuna Compatibility

Reach a state where we can confidently claim to be "a complete superset of
optuna-dashboard."

- [x] Ingestion of intermediate values (journal `SET_TRIAL_INTERMEDIATE_VALUE`
      / SQLite `trial_intermediate_values`)
- [x] Learning curve / pruning analysis plots (visualization of intermediate values
      including PRUNED trials = the Intermediate Values widget)
- [x] EDF plot
- [x] Timeline plot (all trial states × datetime_start/complete)
- [x] Rank plot
- [x] Live updates for SQLite storage (fingerprint polling + single-study
      reload approach)

### 2. Self-Contained HTML Report Export ✅

Export snapshots of all widgets, statistics, and the MCDM ranking into a single HTML
file — a lightweight, server-free counter to VOLTA / HEEDS Connect, offering
"shareable by email" as the value proposition.

- [x] Structured report model + builder + Key Findings (renderer-agnostic)
- [x] Markdown / HTML / JSON renderers (self-contained HTML, light/dark, Japanese/English
      support)
- [x] SVG chart primitives (scatter, line, bar, histogram, heatmap)
- [x] egui-app export UI (modal for selecting format, language, and Top-N)
- [x] Quality review against real data (SQLite / PostgreSQL; multi-objective,
      constrained, pruned, and incomplete studies)

- [x] Constraint handling for the Pareto front (the front is computed from feasible
      trials only; falls back to non-dominated solutions in objective space, with an
      explicit note, only when there are zero feasible solutions; constraint-violating
      trials are also flagged in the extreme-value table)
- [x] Marking of trials on the front that share identical objective values
      ("(= #N)" annotation + legend)

### 3. UI Polish

- [x] Dark mode (toolbar toggle + theme-aware color functions + session persistence)
- Japanese localization of the main UI (currently only the online documentation is
  bilingual). An area where commercial tools are weak with the domestic CAE user
  base — on hold (pending user decision)
- [x] Updated README feature list (reflecting all 33 widgets, storage support, report
      export, etc.)

## Mid Term (catching up and overtaking on analysis features)

### 4. Candidate Suggestions → Write-Back to Optuna (dropped)

**Dropped (2026-07)**: enqueue-based write-back to an external Optuna process is no
longer planned. The Phase 2B runner (item 15) lets the dashboard execute suggested
candidates directly via Rhino.Compute, which closes the "analyze → suggest next
experiment → run" loop without an enqueue hand-off; the "Copy enqueue JSON" export in
the suggestion tables remains for users who drive Optuna themselves.

### 5. Upgrading Robustness Analysis ✅

Add quantitative display of σ-level / success probability, plus distribution selection
(Weibull, etc.), to the existing Monte Carlo noise propagation. Even without going as
far as FORM/SORM, being able to claim "capable of a 6σ-equivalent judgment" is enough
to enter consideration as an Isight alternative.

- [x] Input noise distribution selection (Normal / Uniform / Weibull(k), standardized
      to mean 0 / variance 1 to unify the 1σ scale)
- [x] Quantitative display of success probability, σ-level (Φ⁻¹ with
      empirical-probability clamping), and Cpk relative to specification limits
      (LSL/USL), with color-coded judgment (σ≥4 / ≥2 / below)
- [x] Limit-line rendering on histograms and widget UI (distribution combo box, limit
      input fields)

### 6. Surrogate Comparison View ✅

Overlay comparison of CV accuracy and prediction surfaces across multiple models
(equivalent to HEEDS 2504's Compare Surrogates). The backend (Auto selection / CV-R²)
already exists, so this is mainly a UI addition.

- [x] New "Compare Surrogates" widget: batch fitting of all models + a CV R² /
      Holdout / Train metrics table (with best-value marking) + per-model overlay of a
      1D prediction slice through the best trial (GP-MoE optional)

### 7. Expanded Storage Support ✅

Support for Optuna's PostgreSQL / MySQL RDB backends. Since RDB backends are the
standard choice for team-based distributed optimization, this has high practical
priority.

- [x] Backend abstraction for query logic (the `OptunaBackend` trait, shared with
      SQLite)
- [x] PostgreSQL / MySQL readers (connection URL specification; SQLAlchemy-style URLs
      also accepted)
- [x] UI: toolbar "Open URL…" dialog, password masking in the title bar
- [x] Live updates (fingerprint polling, sharing the same loop as SQLite)
- Not yet supported: TLS connections, reading artifacts from the RDB

## Long Term (differentiation aligned with industry trends)

### 8. LLM / MCP Integration ✅

Optimus productizing LLM post-processing in 2024 signals the industry direction.
Leverage the structural advantage of `tunny-core` being headless by exposing it as an
MCP server, becoming "an optimization analysis tool that agents can query in natural
language."

- [x] `tunny-mcp` binary (stdio JSON-RPC, MCP tools implementation, with `serde` as
      its only dependency)
- [x] Tools: `list_studies` / `study_summary` / `study_report` (LLM-oriented Markdown
      / structured JSON, Japanese/English) / `trials` (paginated raw data)
- [x] Support for all storage backends (journal / SQLite / PostgreSQL / MySQL, with
      password masking)
- [x] Unit tests for the protocol and tools + integration tests for binary startup,
      plus end-to-end verification against real databases
- Candidates for future expansion: turning importance / MCDM into standalone tools,
  `resources` support (serving report HTML)

### 9. Publishing Performance Benchmarks

Publish load and rendering time comparisons against optuna-dashboard and others for
studies of 100K to 1M trials, making a quantitative case for being "the only practical
tool for large-scale studies."

### 10. Branding Transparency

Foreground the online documentation and Python cross-validation as proof that "every
algorithm is verifiable and citable." This is a clear differentiator against
commercial black boxes for regulated industries (aerospace, medical devices) and
academic use.

## Priorities (Phase 1)

- Top priority for closing feature gaps: **Short Term 1 (complete Optuna
  compatibility) → Mid Term 7 (PostgreSQL) → Short Term 2 (HTML report)**
- Top priority for differentiation: **Long Term 8 (MCP/LLM integration), and
  positioning as a "decision-support dashboard" built around MCDM + convergence
  metrics**
- Investment decisions on reliability analysis (FORM/SORM, etc.) and multi-fidelity
  surrogates will be made after the above is complete

---

# Phase 2: Extending into the Execution Loop (2026-07 Policy Decision)

The Phase 1 goal (the analysis/decision-support layer) has been achieved. In the next
phase, we close the "analyze → suggest candidates → execute → re-analyze" loop and
claim a position no commercial PIDO holds: **"an Optuna dashboard that can execute."**

## Scoping Principles

Commercial PIDO's "process integration / execution" capability is made up of three
layers with distinct characteristics; we will expand into only Layers 1 and 2.

| Layer | Content | Policy |
|---|---|---|
| 1. Execution management layer | Runner, parallel workers, retries, monitoring | **In scope** (the primary target) |
| 2. Generic process integration layer | Dakota-style file interface | **In scope** (to guarantee generality) |
| 3. Vendor-specific integration | Official per-solver adapters, workflow graph editor | **Out of scope** (except as noted below) |

- Reason for avoiding Layer 3: it requires ongoing maintenance to track each solver
  version upgrade, plus licenses for verification testing — unsustainable for a small
  team. Users who need a graph editor are already on commercial tools, and price would
  be the only reason for them to switch
- **The sole exception is Grasshopper (Tunny)**. Commercial PIDO is weak here, and it
  connects directly to our existing user base, so we treat it as a first-class solver
  integration
- For the generic side, we follow the "template substitution → execution → output
  extraction" approach that Dakota (Sandia) has proven out over more than 20 years. It
  allows integration with any solver without requiring vendor-specific knowledge

## Core Work (Phase 2A: Storage Write Layer)

Item 11 (candidate write-back to Optuna via `enqueue_trial`, formerly Mid Term 4) was
dropped in 2026-07: the Phase 2B runner executes candidates directly, making the
enqueue hand-off unnecessary (see Mid Term 4).

### 12. Storage Write Layer and Safety Design

Foundational work to avoid undermining our credibility as an analysis tool.

- [x] Write API for creating studies and adding trials (journal support first) —
      `io::journal::writer`. Format compatibility is guaranteed via round-trip tests
      against the existing parser
- [ ] Explicit read-only mode, a confirmation UI before writes, and an Optuna version
      compatibility check

## Core Work (Phase 2B: Runner — the tool drives the loop)

### 13. Lightweight Runner

The framing: "Optuna is the workflow engine; Dashboard is the cockpit." The
optimization loop runs entirely in the Dashboard (Rust samplers + the
Dashboard's own journal writer); evaluation is delegated to Grasshopper
(Rhino.Compute) or a user-specified tool. **No Python / Optuna is required at
runtime** — the journal is only a file format, so Optuna can read the results
later but is never needed to produce them.

- [ ] UI for study creation + sampler configuration
- [x] Register the objective function as "a command + a parameter-passing convention
      (environment variables / JSON / CLI arguments)" and evaluate it in a subprocess:
      the `process` module's `ProcessEvaluator` runs one trial via an external command,
      delivering parameters through any of the four conventions (input-file template,
      env vars, JSON stdin, CLI args)
- [x] Failure retries and per-command timeouts (subprocess killed on timeout)
- [x] Self-contained optimization loop (`runner` module): drives the Random / NSGA-II
      samplers over any `Evaluator` (the process command, Rhino.Compute, or a mock) and
      writes every trial to an Optuna-compatible journal with the Dashboard's own Rust
      writer — so a process optimization runs with **only the Dashboard and the target
      tool**, no Python/Optuna at runtime. `ProcessEvaluator` implements
      `runner::Evaluator`. Migrating the Grasshopper runner onto the shared `runner`
      module, the adaptive sampler for the generic path, and parallel worker management
      are follow-ups
- [ ] Monitoring UI during execution (beyond the progress overlay and the
      on-demand Reload)

### 14. Dakota-Style Generic Process Integration

- [x] A pipeline of input template substitution (parameter embedding) → solver
      execution → output extraction (regex / JSON / CSV): `process::ProcessEvaluator`
      substitutes `{param}` placeholders into an input-file template (or builds
      args/env/JSON stdin), runs the command, and extracts each objective/constraint
      from stdout or an output file via regex capture group, dotted JSON path, or CSV
      cell (`process::Extractor`)
- [x] Persist the integration definition as JSON, editable via the GUI: the whole
      `ProcessDefinition` is `serde`-serializable (`to_json` / `from_json`), and the
      GUI editor (toolbar **New Tool…** → builder modal: author / import / edit every
      field and save back to JSON) is implemented; a TOML encoding is a drop-in
      follow-up
- [x] A sequential hook chain of pre-command → evaluation → post-command
      (no graph editor will be built; revisit once demand is proven)

### 15. First-Class Grasshopper (Tunny) Integration

**Ideal form (the core goal of Phase 2B)**: dragging and dropping a .gh file that runs
an optimization via Tunny onto Dashboard lets Dashboard run that optimization directly
using Rhino.Compute.

**Approach (decided 2026-07)**: the MVP is built by **parsing .ghx (XML) directly**.
Since .ghx is the XML serialization of GH_Archive, Dashboard can extract variable
sliders and Gene Pools (name, range, precision), wire connections (the Variables /
Objectives inputs of the Tunny component), and Tunny's settings entirely on its own,
**with no dependency on a Tunny release**. Injection of RH_IN / RH_OUT groups is likewise
performed by Dashboard directly against the ghx (XML). The original proposal —
"embedding a problem-definition manifest into the .gh file" (a feature addition on the
Tunny side) — is demoted to a later stage, kept in reserve as a fallback for
supporting D&D of plain .gh files and for future changes to the GH_Archive format.

- [x] ghx parser + problem-definition extraction (Dashboard side): GH_Archive XML →
      an intermediate representation `GhProblem` of variables, objectives, and
      connections. Tunny component detection (`gh::problem`)
- [x] Generation of Compute-ready definitions (Dashboard side): attaching RH_IN
      groups to variable sliders and injecting RH_OUT parameters into objective
      wires, performed directly on the ghx XML (`gh::compute_def`). Gene Pools are
      supported as variable sources too: each pool becomes one variable per gene
      (sharing the pool's range/decimals) and is injected as a single RH_IN list
      input carrying all of the pool's values
- [x] Rhino.Compute client: an HTTP client that feeds variable values to a local
      rhino.compute instance (no extra cost with a Rhino license), solves, and
      extracts objective values. Parallel workers = concurrent requests (concurrency
      is capped with a semaphore, `gh::compute`)
- [x] D&D UI: drop a .ghx file → review the problem definition and configure the
      sampler → create a study (journal, using the Item 11/12 write layer) → launch
      the runner → follow the progress overlay, and press Reload to pull in the
      trials completed so far. Because trials land in Optuna-compatible storage,
      every analysis feature works unmodified
- [x] Samplers: launch by repurposing the existing Rust implementations (Random /
      NSGA-II) for real objective-function evaluation (`gh::runner`). Adding CMA-ES /
      TPE, etc. will be decided based on demand
- [x] Constraints: sources wired to the Constraint input of the attribute component
      (Construct Fish Attribute) are extracted from the ghx, evaluated through
      injected RH_OUT relays, and recorded per trial as the Optuna-compatible
      `constraints` system attribute (feasible when every value <= 0, soft
      constraints). Feasibility is read back into the existing constraint-aware
      analysis, and steers NSGA-II via a constrained-domination
      penalty (feasible solutions always dominate; infeasible ones rank by total
      violation)
- [x] Attributes: sources wired to the attribute component's Attribute input are
      recorded per trial as Optuna user attributes (numeric values become numeric
      user-attr columns, anything else text) and are read back into the
      user-attribute-aware widgets. The Geometry input is not captured (no journal
      representation; deferred to a future artifact store)
- [ ] Later stage: the problem-definition manifest (.gh support and format fallback,
      on the Tunny side), and end-to-end verification against a real Rhino.Compute
      instance (confirming type GUIDs and other environment specifics)
- [x] Launching and port management for the rhino.compute process: resolved by
      supporting both modes. Given a URL, it connects to an already-running server;
      given an EXE path, Dashboard launches it with `--port`, waits for the health
      check, and stops it when the run finishes (`gh::compute_server`)
- [x] Compute connection and sampler settings (EXE path, port, URL, parallelism,
      sampler parameters) persist across app sessions (eframe storage), so the
      setup dialog opens pre-filled with the last-used values
- [ ] Open question: reconciling Compute's Windows-only assumption with Dashboard's
      cross-platform nature

## Core Work (Phase 2C: Automation and Agentification)

### 16. Automating the Adaptive Loop

- [x] An automatic loop of surrogate suggestion → evaluation via the runner (item 15)
      → refitting (equivalent to commercial adaptive sampling): the .ghx run dialog's
      "Adaptive (surrogate)" sampler (`gh::adaptive`) — random bootstrap, then per
      iteration fit (Auto model selection) → suggest (EI single-objective / EHVI
      multi-objective, feasibility-aware for single-objective) → evaluate via
      Rhino.Compute → refit. Extending the loop to the generic runners follows
      items 13/14
- [x] Convergence-based stopping and loop diagnostics (improvement history per
      iteration) beyond the fixed iteration budget: the adaptive loop records a
      per-iteration convergence metric (the feasible Pareto front's hypervolume
      against a fixed reference, reducing to the shifted best value for a single
      objective) and stops early once the relative improvement stays below a
      threshold for a configurable number of iterations (patience). The setup
      dialog exposes "Stop early on convergence" + patience / min-improvement,
      and the completion message reports the stop reason and final metric

### 17. Write-Capable Tools for tunny-mcp

Enable LLM agents to drive the optimization loop itself — a differentiator none of the
four commercial tools have, extending the direction set out in Long Term 8.

- [ ] Write tools such as `create_study` and starting runner-based optimizations
- [ ] Opt-in design for write permissions (read-only by default)

## Prerequisites (to be in place by Phase 2B)

- [ ] TLS connections for the RDB (required, since the execution system assumes
      team-based operation)
- [ ] Reading artifacts from the RDB

## Priorities (Phase 2)

- Proceed in the order **2A (Item 12) → 2B (Items 13-15) → 2C (Items 16-17)**.
  The storage write layer underpins the 2B runner's journal output (item 11 was
  dropped; see Phase 2A)
- Investment decisions on FORM/SORM and multi-fidelity surrogates will be made once
  the execution loop is closed (maintaining the Phase 1 policy)
- Enterprise features (user management, PLM integration) will be decided once demand
  becomes visible
