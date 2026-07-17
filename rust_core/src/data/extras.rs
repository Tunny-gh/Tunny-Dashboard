//! Auxiliary information for all trials (all states) that runs alongside the
//! COMPLETE-only `DataFrame`.
//!
//! `DataFrame` holds only COMPLETE trials column-wise and underpins all analysis,
//! but intermediate values, trial state, and start/complete datetimes are needed
//! for all trials (learning curves, timelines, progress display, etc.). These are
//! kept in a separate per-study structure, `StudyExtras`, distinct from
//! `DataFrame`, and stored per study_id in the shared store.

/// Optuna trial state (converted from both the journal's numeric representation
/// and SQLite's string representation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrialState {
    Running,
    Complete,
    Pruned,
    Fail,
    Waiting,
}

impl TrialState {
    /// Converts from the journal storage's numeric state.
    /// 0=Running, 1=Complete, 2=Pruned, 3=Fail, 4=Waiting. Unknown values are treated as Running.
    pub fn from_journal(value: u8) -> Self {
        match value {
            1 => TrialState::Complete,
            2 => TrialState::Pruned,
            3 => TrialState::Fail,
            4 => TrialState::Waiting,
            _ => TrialState::Running,
        }
    }

    /// Converts from SQLite (RDBStorage)'s string state.
    /// Unknown values are treated as Running.
    pub fn from_rdb_str(value: &str) -> Self {
        match value {
            "COMPLETE" => TrialState::Complete,
            "PRUNED" => TrialState::Pruned,
            "FAIL" => TrialState::Fail,
            "WAITING" => TrialState::Waiting,
            _ => TrialState::Running,
        }
    }

    /// Returns Optuna's uppercase state name.
    pub fn label(&self) -> &'static str {
        match self {
            TrialState::Running => "RUNNING",
            TrialState::Complete => "COMPLETE",
            TrialState::Pruned => "PRUNED",
            TrialState::Fail => "FAIL",
            TrialState::Waiting => "WAITING",
        }
    }
}

/// Auxiliary information for a single trial (state / datetimes / intermediate values).
#[derive(Debug, Clone)]
pub struct TrialExtra {
    /// Global trial_id across storages (order of op_code=4 occurrence).
    pub trial_id: u32,
    /// 0-based trial.number within the study (creation order).
    pub trial_number: u32,
    pub state: TrialState,
    /// Start datetime. Unix seconds (naive, no timezone conversion).
    pub datetime_start: Option<f64>,
    /// Complete datetime. Unix seconds (naive, no timezone conversion).
    pub datetime_complete: Option<f64>,
    /// Intermediate values `(step, value)`. Sorted by step ascending.
    pub intermediate_values: Vec<(u64, f64)>,
}

/// Auxiliary information for all trials (all states) within a study. Sorted by trial_id ascending.
#[derive(Debug, Clone, Default)]
pub struct StudyExtras {
    pub trials: Vec<TrialExtra>,
}

impl StudyExtras {
    /// Whether any trial has intermediate values.
    pub fn has_intermediate(&self) -> bool {
        self.trials
            .iter()
            .any(|t| !t.intermediate_values.is_empty())
    }

    /// Whether any trial has a start/complete datetime.
    pub fn has_datetimes(&self) -> bool {
        self.trials
            .iter()
            .any(|t| t.datetime_start.is_some() || t.datetime_complete.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trial_state_from_journal_maps_known_and_unknown() {
        assert_eq!(TrialState::from_journal(0), TrialState::Running);
        assert_eq!(TrialState::from_journal(1), TrialState::Complete);
        assert_eq!(TrialState::from_journal(2), TrialState::Pruned);
        assert_eq!(TrialState::from_journal(3), TrialState::Fail);
        assert_eq!(TrialState::from_journal(4), TrialState::Waiting);
        assert_eq!(TrialState::from_journal(99), TrialState::Running);
    }

    #[test]
    fn trial_state_from_rdb_str_maps_known_and_unknown() {
        assert_eq!(TrialState::from_rdb_str("RUNNING"), TrialState::Running);
        assert_eq!(TrialState::from_rdb_str("COMPLETE"), TrialState::Complete);
        assert_eq!(TrialState::from_rdb_str("PRUNED"), TrialState::Pruned);
        assert_eq!(TrialState::from_rdb_str("FAIL"), TrialState::Fail);
        assert_eq!(TrialState::from_rdb_str("WAITING"), TrialState::Waiting);
        assert_eq!(TrialState::from_rdb_str("???"), TrialState::Running);
    }

    #[test]
    fn trial_state_label_is_uppercase_optuna_name() {
        assert_eq!(TrialState::Running.label(), "RUNNING");
        assert_eq!(TrialState::Complete.label(), "COMPLETE");
        assert_eq!(TrialState::Pruned.label(), "PRUNED");
        assert_eq!(TrialState::Fail.label(), "FAIL");
        assert_eq!(TrialState::Waiting.label(), "WAITING");
    }

    #[test]
    fn study_extras_helpers_detect_presence() {
        let mut extras = StudyExtras::default();
        assert!(!extras.has_intermediate());
        assert!(!extras.has_datetimes());

        extras.trials.push(TrialExtra {
            trial_id: 0,
            trial_number: 0,
            state: TrialState::Complete,
            datetime_start: Some(1.0),
            datetime_complete: None,
            intermediate_values: vec![],
        });
        assert!(!extras.has_intermediate());
        assert!(extras.has_datetimes());

        extras.trials.push(TrialExtra {
            trial_id: 1,
            trial_number: 1,
            state: TrialState::Running,
            datetime_start: None,
            datetime_complete: None,
            intermediate_values: vec![(0, 0.5)],
        });
        assert!(extras.has_intermediate());
    }
}
