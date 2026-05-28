# Memory Efficiency Requirements

## 1. Background

Tunny Dashboard currently retains the same study data in multiple representations across `rust_core`, `egui-app`, and several UI widget caches. This increases both steady-state memory and peak memory during loading and analysis.

This document defines the requirements for reducing memory consumption while preserving current functionality.

## 2. Goal

- Reduce steady-state memory usage for large studies.
- Reduce peak memory usage during journal loading and analysis execution.
- Eliminate avoidable duplication of trial data across state and widgets.
- Preserve current UI behavior and analysis results.

## 3. Scope

In scope:

- `rust_core` dataframe storage and journal parsing flow
- `egui-app` study state, comparison state, and widget caches
- Analysis input construction for PDP, Surface Plot, and Sensitivity

Out of scope:

- Algorithmic changes whose main purpose is numerical accuracy
- GPU-side optimization unrelated to host memory

## 4. Current Issues

| ID | Issue | Main files | Expected impact |
|---|---|---|---:|
| MEM-001 | Study data is retained both as `DataFrame` and as `Vec<TrialRow>` with per-row `HashMap` payloads. | `rust_core\src\data\dataframe\state.rs`, `egui-app\src\io\study.rs`, `egui-app\src\state\types.rs` | **-40% to -70%** steady memory |
| MEM-002 | Pareto 2D / 3D widgets clone `TrialRow` into display caches. | `egui-app\src\ui\widgets\pareto_2d.rs`, `egui-app\src\ui\widgets\pareto_3d.rs` | **-80% to -95%** of those widget caches |
| MEM-003 | Parallel Coordinates and Scatter Matrix each build and retain independent full column caches. | `egui-app\src\ui\widgets\parallel_coords.rs`, `egui-app\src\ui\widgets\scatter_matrix.rs` | **~17.6 MB per widget** at 100k trials x 22 columns |
| MEM-004 | PDP / Surface Plot / Sensitivity reconstruct large temporary matrices repeatedly. | `egui-app\src\ui\poll_chart.rs`, `rust_core\src\pdp\utils.rs`, `rust_core\src\sensitivity\analysis\full.rs` | **15 MB to 50+ MB** peak reduction per run |
| MEM-005 | Comparison mode stores full `StudyContext` per compared study. | `egui-app\src\state\app_state.rs`, `egui-app\src\state\message_handler.rs`, `egui-app\src\io\study_worker.rs` | **-70% to -95% per additional comparison study** |
| MEM-006 | Journal parsing keeps intermediate row builders and then builds `DataFrame`, causing load-time duplication. | `rust_core\src\io\journal\parser\state.rs`, `rust_core\src\io\journal\parser\finalize.rs` | **-20% to -40%** load-time peak memory |
| MEM-007 | `StudyContext.gpu_data` appears to be retained in app state without meaningful read-side usage. | `egui-app\src\state\types.rs`, `egui-app\src\io\study.rs`, `egui-app\src\state\message_handler.rs` | **~4 MB per 100k trials** |

## 5. Requirements

### MEM-001: Eliminate full row-oriented duplication in app state

- The application must not materialize and retain a full `Vec<TrialRow>` for the selected study when equivalent columnar data already exists in `rust_core`.
- App state must use a shared, column-oriented study representation or a lightweight view over `DataFrame`.
- Per-row `HashMap<String, f64>` and `HashMap<String, String>` structures must not be the default persistent representation for the active study.

**Acceptance criteria**

- Selecting a study no longer creates a full persistent `Vec<TrialRow>` copy for normal operation.
- Single-study steady-state memory for large journals is reduced by at least 50% versus the current implementation baseline.

### MEM-002: Remove `TrialRow` clone caches from Pareto widgets

- Pareto 2D and Pareto 3D widgets must not cache `Vec<TrialRow>`.
- These widgets must cache only the minimum render data required, such as filtered indices, point coordinates, or rank slices.

**Acceptance criteria**

- `display_rows_cache: Option<Vec<TrialRow>>` is removed from Pareto 2D and Pareto 3D widget state.
- Rendering behavior remains unchanged for downsampled and non-downsampled modes.

### MEM-003: Share or externalize derived column caches

- Parallel Coordinates and Scatter Matrix must not each own separate full-column caches for the same study data.
- Derived numeric columns needed for visualization must be built once and shared, or computed on demand and released promptly.

**Acceptance criteria**

- Full-column cache ownership is centralized or deduplicated.
- Switching between these widgets does not increase memory linearly with the number of open visualizations.

### MEM-004: Reduce temporary matrix allocation in analysis pipelines

- PDP, Surface Plot, and Sensitivity must avoid rebuilding large `Vec<Vec<f64>>` matrices from row data for every execution when reusable columnar inputs already exist.
- Analysis pipelines must prefer borrowed slices, flat buffers, or shared prepared matrices over nested vector reconstruction.

**Acceptance criteria**

- Re-running the same analysis on the same study does not recreate equivalent large temporary matrices unless inputs changed.
- Peak memory during analysis execution is measurably lower than the current baseline.

### MEM-005: Make comparison studies lightweight

- Comparison mode must not retain full heavy `StudyContext` objects for each compared study unless explicitly required.
- Compared studies should use lightweight metadata plus lazily prepared render data, or a shared columnar backing store.

**Acceptance criteria**

- Adding comparison studies does not increase memory roughly proportional to full study duplication.
- Removing a comparison study releases the associated memory promptly.

### MEM-006: Reduce journal parse peak memory

- Journal parsing must minimize simultaneous retention of intermediate `TrialBuilder` state, per-study row vectors, and finalized `DataFrame` storage.
- The parser/finalizer flow should stream or compact intermediate data wherever possible.

**Acceptance criteria**

- Peak memory during journal load is lower than the current implementation for the same log file.
- No correctness regressions occur in parsed studies, objectives, user attributes, or constraints.

### MEM-007: Remove or defer unused GPU-side host buffers

- Host-side `gpu_data` must not remain in `StudyContext` if it is not actively read by the UI.
- If GPU-related arrays are needed only during specific workflows, they must be built lazily and released when no longer needed.

**Acceptance criteria**

- Unused persistent GPU-related host buffers are removed from steady-state app state.
- Live update continues to behave correctly after this change.

## 6. Implementation Priority

1. MEM-001
2. MEM-002
3. MEM-003
4. MEM-005
5. MEM-004
6. MEM-006
7. MEM-007

## 7. Non-Functional Requirements

- Existing chart outputs and analysis results must remain functionally equivalent.
- Memory optimizations must not introduce silent data loss.
- Memory release behavior must be explicit when studies are switched, comparison studies are removed, or caches become stale.
- New shared caches must have clear ownership and invalidation rules.

## 8. Success Criteria

The work is considered complete when the application demonstrates all of the following:

- Large-study steady-state memory is substantially lower than the current implementation.
- Widget switching no longer multiplies memory usage through duplicated study caches.
- Analysis execution peak memory is reduced.
- Comparison mode no longer scales memory almost linearly with duplicated full study state.
