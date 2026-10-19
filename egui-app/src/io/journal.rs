use crate::state::app_state::{Direction, StudyMeta};

use tdashboard_io::journal::parser::{self, OptimizationDirection};

/// Converts a rust_core StudyMeta into an egui-app StudyMeta.
pub fn convert_study_meta(meta: parser::StudyMeta) -> StudyMeta {
    StudyMeta {
        study_id: meta.study_id,
        name: meta.name,
        directions: meta
            .directions
            .into_iter()
            .map(|d| match d {
                OptimizationDirection::Minimize => Direction::Minimize,
                OptimizationDirection::Maximize => Direction::Maximize,
            })
            .collect(),
        completed_trials: meta.completed_trials as usize,
        param_names: meta.param_names,
        objective_names: meta.objective_names,
        param_bounds: meta.param_bounds,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_minimize_direction() {
        let core_meta = parser::StudyMeta {
            study_id: 0,
            name: "test".to_string(),
            directions: vec![
                OptimizationDirection::Minimize,
                OptimizationDirection::Maximize,
            ],
            completed_trials: 5,
            total_trials: 10,
            param_names: vec!["x".to_string()],
            objective_names: vec!["y".to_string()],
            user_attr_names: vec![],
            has_constraints: false,
            param_bounds: Default::default(),
        };
        let app_meta = convert_study_meta(core_meta);
        assert_eq!(app_meta.directions[0], Direction::Minimize);
        assert_eq!(app_meta.directions[1], Direction::Maximize);
        assert_eq!(app_meta.completed_trials, 5usize);
    }
}
