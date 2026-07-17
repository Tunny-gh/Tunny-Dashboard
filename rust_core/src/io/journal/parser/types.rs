/// The optimization direction (minimize / maximize).
#[derive(Debug, Clone, PartialEq)]
pub enum OptimizationDirection {
    Minimize,
    Maximize,
}

/// Study metadata (parameter/objective/attribute name lists, trial counts, etc.).
#[derive(Debug, Clone)]
pub struct StudyMeta {
    pub study_id: u32,
    pub name: String,
    pub directions: Vec<OptimizationDirection>,
    pub completed_trials: u32,
    pub total_trials: u32,
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub user_attr_names: Vec<String>,
    pub has_constraints: bool,
    /// Declared range (low, high) per parameter (display units, numeric parameters only).
    /// The search space bounds recorded in the log. Used as the search box for surrogate optimization.
    pub param_bounds: std::collections::HashMap<String, (f64, f64)>,
}

/// Parse result of the journal file (metadata for all studies and elapsed time).
#[derive(Debug)]
pub struct ParseResult {
    pub studies: Vec<StudyMeta>,
    pub duration_ms: f64,
}
