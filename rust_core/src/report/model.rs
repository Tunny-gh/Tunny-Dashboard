//! The `StudyReport` struct tree (language-independent structured facts).
//!
//! Every type defined here derives `serde::Serialize` and can be output
//! directly as JSON. Prose generation (en / ja) is the responsibility of
//! the renderer (`markdown` / `html`) templates; the model itself is
//! language-independent. Numbers are kept as f64, and rounding/formatting
//! is handled by the renderer's shared formatter
//! ([`crate::report::format_number`]).
//!
//! For determinism, dictionary-like collections are kept as
//! [`std::collections::BTreeMap`] or sorted `Vec`s, and no output depends
//! on `HashMap` iteration order.

use std::collections::BTreeMap;

/// Schema version. Bumped on every breaking change.
pub const SCHEMA_VERSION: u32 = 1;

/// Optimization direction of an objective (language-independent
/// representation for `serde` output).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum Direction {
    /// Minimize.
    Minimize,
    /// Maximize.
    Maximize,
}

impl Direction {
    /// Whether this is the minimize direction.
    pub fn is_minimize(self) -> bool {
        matches!(self, Direction::Minimize)
    }
}

/// Root struct of the report.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StudyReport {
    /// Schema version ([`SCHEMA_VERSION`]).
    pub schema_version: u32,
    /// Source info (storage display name, generation time).
    pub source: ReportSourceInfo,
    /// Study overview.
    pub overview: Overview,
    /// Key Findings (summary). Generated automatically and
    /// deterministically.
    pub key_findings: Vec<KeyFinding>,
    /// Optimization outcome (single-objective / multi-objective).
    pub outcome: Outcome,
    /// Convergence section (best-so-far / HV progression).
    pub convergence: ConvergenceSection,
    /// Parameter importance (`None` if it could not be computed).
    pub importance: Option<ImportanceSection>,
    /// Distribution statistics of objective values (per objective).
    pub objective_stats: Vec<ObjectiveStats>,
    /// Parameter x objective correlations (`None` if it could not be
    /// computed).
    pub correlations: Option<CorrelationSection>,
    /// Multi-objective decision analysis (MCDM). `None` for
    /// single-objective.
    pub mcdm: Option<McdmSection>,
    /// Execution info (only present when extras exist).
    pub execution: Option<ExecutionSection>,
    /// Reproduction info.
    pub reproduction: Reproduction,
}

/// Source info (a snapshot of `ReportSource`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReportSourceInfo {
    /// Storage display name (masked for RDB URLs).
    pub storage_display: String,
    /// Generation time (unix seconds). If `None`, the date/time field is
    /// omitted.
    pub generated_at_unix: Option<i64>,
}

/// Study overview.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Overview {
    /// Study name.
    pub name: String,
    /// Optimization direction per objective.
    pub directions: Vec<Direction>,
    /// Objective names.
    pub objective_names: Vec<String>,
    /// Parameter names.
    pub param_names: Vec<String>,
    /// user_attr names.
    pub user_attr_names: Vec<String>,
    /// Trial count per state label (BTreeMap for determinism).
    pub state_counts: BTreeMap<String, usize>,
    /// Number of COMPLETE trials (rows subject to analysis).
    pub complete_trials: usize,
    /// Total trial count (from meta).
    pub total_trials: usize,
    /// Measured wall-clock duration (seconds). Computed from extras'
    /// timestamps. `None` if unavailable.
    pub wall_clock_seconds: Option<f64>,
    /// Declared parameter ranges `(name, low, high)`, sorted by name.
    pub param_bounds: Vec<(String, f64, f64)>,
    /// Whether constraints are defined.
    pub has_constraints: bool,
}

/// One Key Finding (summary item).
///
/// `kind` is a fixed enum, and the renderer exhaustively `match`es it to
/// produce prose. `metrics` / `labels` are the numbers/strings to embed in
/// the template. Uses BTreeMap for determinism.
#[derive(Debug, Clone, serde::Serialize)]
pub struct KeyFinding {
    /// Kind.
    pub kind: FindingKind,
    /// Numeric values to embed in the template.
    pub metrics: BTreeMap<String, f64>,
    /// String values to embed in the template (param names, etc.).
    pub labels: BTreeMap<String, String>,
}

/// Kind of a Key Finding (fixed enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum FindingKind {
    /// Single-objective best value, trial number, and when it was found.
    BestSingle,
    /// Pareto front size and each objective's extremes.
    ParetoSummary,
    /// Convergence status (Converged / StillImproving / Insufficient).
    ConvergenceStatus,
    /// Top parameters (with method name).
    TopImportance,
    /// Trade-off between objectives (most negative Spearman pair).
    TradeOff,
    /// Constraint satisfaction rate and best feasible trial.
    Feasibility,
    /// Pruning efficiency (prune rate and median step).
    PruningEfficiency,
    /// Data quality (alert for FAIL / NaN objective values).
    DataQuality,
}

/// Convergence status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum ConvergenceStatus {
    /// Converged (no best update in the latter 20%).
    Converged,
    /// Still improving (best updated in the latter 20%).
    StillImproving,
    /// Insufficient data (COMPLETE < 10).
    Insufficient,
}

/// Summary of a single trial.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TrialSummary {
    /// trial.number, 0-based within the study.
    pub trial_number: u32,
    /// Objective values (in objective order).
    pub objectives: Vec<f64>,
    /// Parameters `(name, value)` (in meta's parameter order).
    pub params: Vec<(String, ParamValue)>,
    /// Max constraint value (constrained studies only; the max over
    /// Optuna's raw constraint values for the row — `<= 0` means all
    /// constraints satisfied, positive means a constraint violation).
    pub max_constraint: Option<f64>,
    /// user_attr `(name, value)` (sorted by name).
    pub user_attrs: Vec<(String, String)>,
    /// The number of the first trial sharing the same objective value
    /// vector (only determined within the Pareto table). `Some(n)` means
    /// this trial is a duplicate solution whose objective values exactly
    /// match trial `n`.
    pub duplicate_of: Option<u32>,
}

impl TrialSummary {
    /// Whether this trial violates constraints (max constraint value is
    /// positive).
    ///
    /// Unconstrained studies (`max_constraint == None`) never count as a
    /// violation. Shares the violation-mark determination logic in one
    /// place for both the HTML and Markdown renderers.
    pub fn violates_constraints(&self) -> bool {
        self.max_constraint.is_some_and(|v| v > 0.0)
    }
}

/// Whether the table contains any duplicate solutions (trials with
/// `duplicate_of` set).
///
/// Shared by both the HTML and Markdown renderers to decide whether to
/// emit the "(= #N)" legend note.
pub fn has_duplicate_marks(trials: &[TrialSummary]) -> bool {
    trials.iter().any(|t| t.duplicate_of.is_some())
}

/// Parameter value (numeric / categorical).
#[derive(Debug, Clone, serde::Serialize)]
pub enum ParamValue {
    /// Numeric parameter.
    Num(f64),
    /// Categorical parameter (display label).
    Cat(String),
}

/// Optimization outcome.
#[derive(Debug, Clone, serde::Serialize)]
pub enum Outcome {
    /// Single-objective.
    SingleObj {
        /// Best trial (`None` if there are no COMPLETE trials).
        best_trial: Option<TrialSummary>,
        /// Top trials (best-first, `top_n` entries).
        top_n: Vec<TrialSummary>,
    },
    /// Multi-objective.
    MultiObj {
        /// Pareto front size.
        pareto_size: usize,
        /// COMPLETE count.
        complete_count: usize,
        /// Objective count.
        objective_count: usize,
        /// Extremes per objective.
        per_objective_extremes: Vec<ObjectiveExtreme>,
        /// Pareto front trial table (TOPSIS order, capped at `top_n*2`).
        pareto_table: Vec<TrialSummary>,
        /// Number of constraint-violating trials contained in the Pareto
        /// front (the full set, before capping). Only becomes positive
        /// when there are zero feasible solutions and the front falls
        /// back to non-dominated points in objective space. The
        /// renderer's fallback note uses this value (counting from the
        /// capped `pareto_table` could under-report).
        pareto_infeasible_count: usize,
        /// Scatter plot points (all COMPLETE trials + front
        /// determination, first two objective axes).
        scatter: Vec<ParetoPoint>,
        /// Objective indices `(x, y)` used as the scatter plot axes. The
        /// builder currently always passes the first two objectives
        /// `(0, 1)`, but since the renderer reads this index for axis
        /// labels and note text, it is kept in the model even as a fixed
        /// value (also a placeholder for a future axis-selection option).
        scatter_axes: (usize, usize),
    },
}

/// Extremes for one objective.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ObjectiveExtreme {
    /// Objective name.
    pub objective_name: String,
    /// Direction.
    pub direction: Direction,
    /// Best value.
    pub best_value: f64,
    /// trial.number that achieved the best value.
    pub best_trial_number: u32,
    /// Whether the best trial satisfies all constraints (always `true`
    /// for unconstrained studies). Because extremes are descriptive
    /// statistics over all COMPLETE trials, a constraint-violating trial
    /// can end up as the best. The renderer adds an explicit mark in that
    /// case.
    pub best_feasible: bool,
    /// Worst value.
    pub worst_value: f64,
}

/// One point of the scatter plot (with Pareto front determination).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ParetoPoint {
    /// trial.number.
    pub trial_number: u32,
    /// X-axis value (first objective).
    pub x: f64,
    /// Y-axis value (second objective).
    pub y: f64,
    /// Whether this point lies on the Pareto front.
    pub on_front: bool,
    /// Whether it satisfies all constraints (always `true` for
    /// unconstrained studies).
    pub feasible: bool,
}

/// Convergence section.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConvergenceSection {
    /// Metric the series represents.
    pub metric: ConvergenceMetric,
    /// Series (trial.number, value). Already thinned to <= 500 points.
    pub series: Vec<ConvergencePoint>,
    /// trial.number where the best was found (`None` if data is
    /// insufficient).
    pub found_at_trial_number: Option<u32>,
    /// Whether the best was updated in the most recent 20% of trials.
    pub improved_in_last_20pct: bool,
    /// Convergence status.
    pub status: ConvergenceStatus,
}

/// Kind of metric for the convergence series.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum ConvergenceMetric {
    /// Single-objective best-so-far.
    BestSoFar,
    /// Multi-objective Hypervolume progression.
    Hypervolume,
}

/// One point of the convergence series.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConvergencePoint {
    /// trial.number.
    pub trial_number: u32,
    /// Metric value.
    pub value: f64,
}

/// Parameter importance section.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ImportanceSection {
    /// Name of the importance-computation method (e.g. `"spearman_abs"`).
    pub method: String,
    /// Objective name that importance was evaluated against.
    pub objective_name: String,
    /// `(param, score)` pairs sorted descending (largest score first).
    pub scores: Vec<(String, f64)>,
}

/// Distribution statistics of objective values.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ObjectiveStats {
    /// Objective name.
    pub name: String,
    /// Direction.
    pub direction: Direction,
    /// Count of finite values.
    pub n: usize,
    /// Mean.
    pub mean: f64,
    /// Population standard deviation.
    pub std: f64,
    /// Min.
    pub min: f64,
    /// First quartile.
    pub q1: f64,
    /// Median.
    pub median: f64,
    /// Third quartile.
    pub q3: f64,
    /// Max.
    pub max: f64,
    /// Histogram (<= 20 bins). `None` if there are no finite values.
    pub histogram: Option<HistogramData>,
}

/// Histogram bin edges and counts.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HistogramData {
    /// Bin edges (ascending, `len() == counts.len() + 1`).
    pub bin_edges: Vec<f64>,
    /// Counts.
    pub counts: Vec<usize>,
}

/// Parameter x objective correlation section.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CorrelationSection {
    /// Correlation method name (`"spearman"`).
    pub method: String,
    /// Parameter names corresponding to each row (sorted descending by
    /// max |ρ|, capped at `max_heatmap_params`).
    pub params: Vec<String>,
    /// Objective names corresponding to each column.
    pub objectives: Vec<String>,
    /// `matrix[i][j]` = correlation between params[i] and objectives[j].
    /// NaN if it could not be computed.
    pub matrix: Vec<Vec<f64>>,
}

/// Multi-objective decision analysis (MCDM) section.
#[derive(Debug, Clone, serde::Serialize)]
pub struct McdmSection {
    /// Weighting scheme (`"equal"` = equal weights).
    pub weight_scheme: String,
    /// Weight for each objective.
    pub weights: Vec<f64>,
    /// Top trials by TOPSIS.
    pub topsis_top: Vec<McdmEntry>,
    /// Top trials by VIKOR.
    pub vikor_top: Vec<McdmEntry>,
    /// Top trials by PROMETHEE II.
    pub promethee_top: Vec<McdmEntry>,
    /// trial.numbers appearing in the top 10 of all three methods
    /// (ascending).
    pub consensus_trials: Vec<u32>,
}

/// One entry of an MCDM ranking.
#[derive(Debug, Clone, serde::Serialize)]
pub struct McdmEntry {
    /// Rank (1-based).
    pub rank: usize,
    /// trial.number.
    pub trial_number: u32,
    /// Objective values.
    pub objectives: Vec<f64>,
}

/// Execution info section.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExecutionSection {
    /// Trial count per state label.
    pub state_counts: BTreeMap<String, usize>,
    /// Pruning rate (PRUNED / all finished trials).
    pub pruned_rate: f64,
    /// Median pruning step (final intermediate-value step for PRUNED
    /// trials). `None` if unavailable.
    pub median_prune_step: Option<f64>,
    /// Mean duration per trial, in seconds. `None` if unavailable.
    pub mean_trial_seconds: Option<f64>,
    /// Population standard deviation of per-trial duration, in seconds.
    /// `None` if unavailable.
    pub std_trial_seconds: Option<f64>,
    /// Total duration (seconds). `None` if unavailable.
    pub total_seconds: Option<f64>,
}

/// Reproduction info.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Reproduction {
    /// Study ID.
    pub study_id: u32,
    /// Storage display name (masked).
    pub storage_display: String,
    /// Number of entries in the top table (echoed from options).
    pub top_n: usize,
    /// Max parameter count for the correlation heatmap (echoed from
    /// options).
    pub max_heatmap_params: usize,
    /// Schema version.
    pub schema_version: u32,
}
