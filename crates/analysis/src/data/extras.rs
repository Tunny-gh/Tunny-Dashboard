//! Derived all-state presentation data from one acquired input.
pub use tdashboard_study::TrialState;

#[derive(Debug, Clone)]
pub struct TrialExtra {
    pub trial_id: u32,
    pub trial_number: u32,
    pub state: TrialState,
    pub datetime_start: Option<f64>,
    pub datetime_complete: Option<f64>,
    pub intermediate_values: Vec<(u64, f64)>,
}

#[derive(Debug, Clone, Default)]
pub struct StudyExtras {
    pub trials: Vec<TrialExtra>,
}

impl StudyExtras {
    pub fn has_intermediate(&self) -> bool {
        self.trials
            .iter()
            .any(|t| !t.intermediate_values.is_empty())
    }

    pub fn has_datetimes(&self) -> bool {
        self.trials.iter().any(|t| t.datetime_start.is_some())
    }
}
