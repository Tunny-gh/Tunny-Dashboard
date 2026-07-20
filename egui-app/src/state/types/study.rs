use std::collections::HashMap;
use std::sync::Arc;

use tunny_core::dataframe::DataFrame;

use crate::ui::widgets::range_math::value_range;

// ============================================================
// Basic type definitions
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Direction {
    Minimize,
    Maximize,
}

#[derive(Debug, Clone)]
pub struct StudyMeta {
    pub study_id: u32,
    pub name: String,
    pub directions: Vec<Direction>,
    pub completed_trials: usize,
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    /// Declared range (low, high) per parameter (display units, numeric parameters only).
    /// Search-space range derived from the log. Used as the search box for surrogate optimization. Empty = range unknown.
    pub param_bounds: HashMap<String, (f64, f64)>,
}

/// Edit state for the CSV import confirmation dialog.
///
/// A flat CSV doesn't include the optimization directions (maximize/minimize) or the variables'
/// declared ranges, so this dialog lets the user check and correct them right after loading.
/// The dialog is shown while `AppState::csv_import_settings` is `Some`.
#[derive(Debug, Clone)]
pub struct CsvImportSettings {
    /// The target Study (CSV always yields a single Study, so this is always exactly one).
    pub study_id: u32,
    pub study_name: String,
    /// Objective names (same order and length as `maximize`).
    pub objective_names: Vec<String>,
    /// Per-objective maximize flag (true=Maximize, false=Minimize).
    pub maximize: Vec<bool>,
    /// Range edits for numeric parameters (sorted by parameter name).
    pub param_bounds: Vec<ParamBoundEdit>,
}

/// A single range-edit row for one numeric parameter.
#[derive(Debug, Clone)]
pub struct ParamBoundEdit {
    pub name: String,
    pub low: f64,
    pub high: f64,
}

impl CsvImportSettings {
    /// Builds the edit state from a freshly parsed `StudyMeta` (directions default to Minimize,
    /// ranges default to the observed min/max).
    pub fn from_meta(meta: &StudyMeta) -> Self {
        let maximize: Vec<bool> = meta
            .directions
            .iter()
            .map(|d| matches!(d, Direction::Maximize))
            .collect();
        let mut param_bounds: Vec<ParamBoundEdit> = meta
            .param_bounds
            .iter()
            .map(|(name, &(low, high))| ParamBoundEdit {
                name: name.clone(),
                low,
                high,
            })
            .collect();
        param_bounds.sort_by(|a, b| a.name.cmp(&b.name));
        Self {
            study_id: meta.study_id,
            study_name: meta.name.clone(),
            objective_names: meta.objective_names.clone(),
            maximize,
            param_bounds,
        }
    }

    /// Applies the edited values to `meta` (overwrites `directions` and `param_bounds`).
    pub fn apply_to(&self, meta: &mut StudyMeta) {
        meta.directions = self
            .maximize
            .iter()
            .map(|&m| {
                if m {
                    Direction::Maximize
                } else {
                    Direction::Minimize
                }
            })
            .collect();
        for pb in &self.param_bounds {
            meta.param_bounds.insert(pb.name.clone(), (pb.low, pb.high));
        }
    }

    /// Whether all ranges are valid (min < max and finite). Loading is blocked if invalid.
    pub fn bounds_valid(&self) -> bool {
        self.param_bounds
            .iter()
            .all(|p| p.low.is_finite() && p.high.is_finite() && p.low < p.high)
    }
}

/// Row-oriented test fixture (legacy representation, already replaced by the column-oriented
/// `StudyView` in MEM-001).
#[cfg(test)]
#[derive(Debug, Clone, Default)]
pub struct TrialRow {
    pub trial_id: u32,
    /// 0-based sequence number within the Study (for display)
    pub trial_number: u32,
    pub params: HashMap<String, f64>,
    pub objectives: Vec<f64>,
    pub pareto_rank: u32,
    pub cluster_id: Option<i32>,
    pub user_attrs: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct StudyContext {
    pub meta: StudyMeta,
    /// Lightweight view over the column-oriented data (replaces the legacy `trial_rows: Vec<TrialRow>`, MEM-001).
    pub view: StudyView,
    pub pareto_indices: Vec<u32>,
}

impl StudyContext {
    /// Trial count (equivalent to the legacy `trial_rows.len()`). Read from the column view without copying.
    pub fn trial_count(&self) -> usize {
        self.view.row_count()
    }

    /// Returns the parameter's data range [min, max] (returns [0.0, 1.0] if there's no data)
    pub fn param_range(&self, param_name: &str) -> (f64, f64) {
        let Some(values) = self.view.numeric_column(param_name) else {
            return (0.0, 1.0);
        };
        if values.is_empty() {
            return (0.0, 1.0);
        }
        // `values` is guaranteed non-empty by the emptiness check just above.
        let (min, max) = value_range(values.iter().cloned()).unwrap();
        if (max - min).abs() < f64::EPSILON {
            (min - 0.5, max + 0.5)
        } else {
            (min, max)
        }
    }

    /// Test-only: replaces the row data of an existing StudyContext (rebuilds view/pareto_indices).
    #[cfg(test)]
    pub(crate) fn set_rows_for_test(&mut self, rows: Vec<TrialRow>) {
        let rebuilt = StudyContext::from_rows_for_test(self.meta.clone(), rows);
        self.view = rebuilt.view;
        self.pareto_indices = rebuilt.pareto_indices;
    }

    /// Test-only: builds a StudyContext from a Vec of egui `TrialRow`.
    /// Column names are derived from the row data, and a DataFrame -> StudyView is assembled.
    /// pareto_rank / cluster_id are carried over from the rows into parallel arrays.
    #[cfg(test)]
    pub(crate) fn from_rows_for_test(meta: StudyMeta, rows: Vec<TrialRow>) -> Self {
        use tunny_core::dataframe::TrialRow as CoreRow;

        let mut param_set = std::collections::BTreeSet::new();
        for r in &rows {
            for k in r.params.keys() {
                param_set.insert(k.clone());
            }
        }
        let param_names: Vec<String> = param_set.into_iter().collect();
        // Prefer meta.objective_names so it matches the DataFrame column names.
        // If meta is empty, auto-generate names from the row data (backward compatibility).
        let n_obj = rows.iter().map(|r| r.objectives.len()).max().unwrap_or(0);
        let obj_names: Vec<String> = if !meta.objective_names.is_empty() {
            meta.objective_names.clone()
        } else {
            (0..n_obj).map(|i| format!("obj{i}")).collect()
        };

        let core_rows: Vec<CoreRow> = rows
            .iter()
            .map(|r| CoreRow {
                trial_id: r.trial_id,
                trial_number: r.trial_number,
                param_display: r.params.clone(),
                param_category_label: HashMap::new(),
                objective_values: r.objectives.clone(),
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let df = DataFrame::from_trials(&core_rows, &param_names, &obj_names, &[], &[], 0);
        let pareto_rank: Vec<u32> = rows.iter().map(|r| r.pareto_rank).collect();
        let mut view = StudyView::new(Arc::new(df), pareto_rank);
        for (i, r) in rows.iter().enumerate() {
            view.cluster_id[i] = r.cluster_id;
        }
        StudyContext {
            meta,
            view,
            pareto_indices: vec![],
        }
    }
}

// ============================================================
// TASK-2331: StudyView — a lightweight view over a column-oriented DataFrame snapshot
//
// Wraps `Arc<DataFrame>` and holds app-layer-derived values not present in the DataFrame
// (pareto_rank / cluster_id / trial_ids) as parallel arrays. Doesn't persistently hold a
// row-oriented `Vec<TrialRow>` or per-row HashMap, so no column data is duplicated (MEM-001).
// For the staged migration, temporary compatibility helpers `row_at` / `to_trial_rows` are
// provided (planned for removal in TASK-2342).
// ============================================================

#[derive(Clone, Debug)]
pub struct StudyView {
    /// Immutable snapshot of the column data fetched from the shared store.
    pub df: Arc<DataFrame>,
    /// Row index -> trial_id.
    pub trial_ids: Vec<u32>,
    /// Pareto rank (in row-index order, computed at the app layer).
    pub pareto_rank: Vec<u32>,
    /// Cluster ID (in row-index order, None if unassigned).
    pub cluster_id: Vec<Option<i32>>,
}

impl StudyView {
    /// Builds a StudyView from an `Arc<DataFrame>` and Pareto ranks.
    /// If the length of pareto_rank doesn't match row_count, pads with 0.
    pub fn new(df: Arc<DataFrame>, pareto_rank: Vec<u32>) -> Self {
        let n = df.row_count();
        let trial_ids: Vec<u32> = (0..n)
            .map(|r| df.get_trial_id(r).unwrap_or(r as u32))
            .collect();
        let pareto_rank = if pareto_rank.len() == n {
            pareto_rank
        } else {
            vec![0; n]
        };
        StudyView {
            df,
            trial_ids,
            pareto_rank,
            cluster_id: vec![None; n],
        }
    }

    /// Row count (= DataFrame.row_count).
    pub fn row_count(&self) -> usize {
        self.df.row_count()
    }

    /// Borrowed slice of a numeric column (doesn't build a per-row HashMap).
    pub fn numeric_column(&self, name: &str) -> Option<&[f64]> {
        self.df.get_numeric_column(name)
    }

    /// Feasibility view. Centralizes whether the `is_feasible` column exists, its threshold, and
    /// the "no column = all rows feasible" fallback logic.
    pub fn feasibility(&self) -> tunny_core::dataframe::Feasibility<'_> {
        self.df.feasibility()
    }

    /// Resolves multiple column names to borrowed slices at once (None for missing columns).
    pub fn numeric_columns(&self, names: &[String]) -> Vec<Option<&[f64]>> {
        names.iter().map(|name| self.numeric_column(name)).collect()
    }

    /// Parameter column names.
    pub fn param_names(&self) -> &[String] {
        self.df.param_col_names()
    }

    /// Objective column names.
    pub fn objective_names(&self) -> &[String] {
        self.df.objective_col_names()
    }

    /// Compatibility shim: temporarily assembles a `TrialRow` from columns + parallel arrays (test-only).
    #[cfg(test)]
    pub(crate) fn row_at(&self, index: usize) -> TrialRow {
        let mut params = HashMap::with_capacity(self.df.param_col_names().len());
        for name in self.df.param_col_names() {
            if let Some(col) = self.df.get_numeric_column(name) {
                if let Some(v) = col.get(index) {
                    params.insert(name.clone(), *v);
                }
            }
        }
        let objectives: Vec<f64> = self
            .df
            .objective_col_names()
            .iter()
            .map(|name| {
                self.df
                    .get_numeric_column(name)
                    .and_then(|c| c.get(index).copied())
                    .unwrap_or(0.0)
            })
            .collect();
        TrialRow {
            trial_id: self.trial_ids.get(index).copied().unwrap_or(index as u32),
            trial_number: index as u32,
            params,
            objectives,
            pareto_rank: self.pareto_rank.get(index).copied().unwrap_or(0),
            cluster_id: self.cluster_id.get(index).copied().flatten(),
            user_attrs: HashMap::new(),
        }
    }

    /// Assembles all rows as `TrialRow` (test-only).
    #[cfg(test)]
    pub(crate) fn to_trial_rows(&self) -> Vec<TrialRow> {
        (0..self.row_count()).map(|i| self.row_at(i)).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ColormapName {
    Viridis,
    Plasma,
    Jet,
    Turbo,
    Inferno,
    Coolwarm,
    Spectral,
    Cividis,
    BlueYellow,
}

impl ColormapName {
    pub fn label(&self) -> &str {
        match self {
            Self::Viridis => "Viridis",
            Self::Plasma => "Plasma",
            Self::Jet => "Jet",
            Self::Turbo => "Turbo",
            Self::Inferno => "Inferno",
            Self::Coolwarm => "Coolwarm",
            Self::Spectral => "Spectral",
            Self::Cividis => "Cividis",
            Self::BlueYellow => "Blue-Yellow",
        }
    }

    pub fn all() -> &'static [ColormapName] {
        &[
            Self::Viridis,
            Self::Plasma,
            Self::Jet,
            Self::Turbo,
            Self::Inferno,
            Self::Coolwarm,
            Self::Spectral,
            Self::Cividis,
            Self::BlueYellow,
        ]
    }
}
