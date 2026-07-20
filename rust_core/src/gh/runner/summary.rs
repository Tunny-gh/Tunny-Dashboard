//! Result and diagnostic types produced by a run.

/// Per-iteration diagnostics for the adaptive sampler.
#[derive(Debug, Clone, PartialEq)]
pub struct GhIterationDiagnostic {
    /// 1-based iteration index (0 = the random bootstrap phase).
    pub iteration: usize,
    /// Cumulative number of successful evaluations after this iteration.
    pub trials_completed: usize,
    /// Convergence metric after this iteration: the feasible Pareto front's
    /// hypervolume against a fixed reference point (multi-objective), or the
    /// shifted best value `ref - best` (single-objective). Monotonically
    /// non-decreasing; larger is better.
    pub metric: f64,
    /// Relative improvement in `metric` versus the previous recorded iteration
    /// (`(metric - prev) / max(|prev|, eps)`). `f64::INFINITY` on the first
    /// non-zero metric.
    pub relative_improvement: f64,
}

/// Why the adaptive loop stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GhStopReason {
    /// Not an adaptive run, or the loop ran its full iteration budget.
    Completed,
    /// The convergence criterion (patience × min-improvement) was met.
    Converged,
    /// A whole batch of suggestions duplicated already-evaluated points.
    NoNewCandidates,
    /// The user cancelled.
    Cancelled,
}

/// Summary of a run's results.
#[derive(Debug, Clone)]
pub struct GhRunSummary {
    pub study_id: u32,
    /// Number of trials recorded as COMPLETE
    pub completed: usize,
    /// Number of trials that failed evaluation (recorded as FAIL)
    pub failed: usize,
    /// Whether the run was cut short by cancellation
    pub cancelled: bool,
    /// Per-iteration diagnostics for the adaptive sampler (empty for the other
    /// samplers).
    pub adaptive_diagnostics: Vec<GhIterationDiagnostic>,
    /// Why the run ended (always `Completed` for the non-adaptive samplers).
    pub stop_reason: GhStopReason,
}
