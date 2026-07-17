//! Label, choice, and quality-verdict helpers for the surrogate optimization widget.
//!
//! Contains only display-only pure functions and constants (no side effects).

use tunny_core::surrogate_opt::{AcquisitionKind, OptimizerKind, SurrogateModelKind};

/// Label for the "Auto" entry in the model combo.
pub(super) const AUTO_MODEL_LABEL: &str = "Auto (cross-validated)";

/// Optimizer choices (in combo display order).
pub(super) const OPTIMIZER_CHOICES: [OptimizerKind; 4] = [
    OptimizerKind::MultiStartLbfgs,
    OptimizerKind::Nsga2,
    OptimizerKind::CmaEs,
    OptimizerKind::RandomSearch,
];

pub(crate) fn model_label(kind: SurrogateModelKind) -> &'static str {
    match kind {
        SurrogateModelKind::Ridge => "Ridge",
        SurrogateModelKind::GpFitc => "GP-FITC",
        SurrogateModelKind::GpVfe => "GP-VFE",
        SurrogateModelKind::GpMoe => "GP-MOE",
        SurrogateModelKind::Lgbm => "LightGBM",
    }
}

pub(crate) fn acq_label(kind: AcquisitionKind) -> &'static str {
    match kind {
        AcquisitionKind::ExpectedImprovement => "EI (Expected Improvement)",
        AcquisitionKind::LowerConfidenceBound => "LCB (Lower Confidence Bound)",
    }
}

pub(crate) fn optimizer_label(kind: OptimizerKind) -> &'static str {
    match kind {
        OptimizerKind::MultiStartLbfgs => "Multi-start L-BFGS",
        OptimizerKind::Nsga2 => "NSGA-II",
        OptimizerKind::CmaEs => "CMA-ES",
        OptimizerKind::RandomSearch => "Random Search",
    }
}

/// Pure function that returns a quality-verdict string and color from the mean CV R².
pub(crate) fn verdict(cv_r2_mean: f64) -> (&'static str, egui::Color32) {
    if cv_r2_mean >= 0.9 {
        (
            "Good — surrogate is reliable",
            egui::Color32::from_rgb(22, 163, 74), // green-600
        )
    } else if cv_r2_mean >= 0.7 {
        (
            "Fair — use with caution",
            egui::Color32::from_rgb(202, 138, 4), // amber-600
        )
    } else {
        (
            "Poor — consider more trials or a different model",
            egui::Color32::RED,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::widgets::surrogate::MODEL_CHOICES;

    #[test]
    fn verdict_returns_correct_category() {
        let (text, color) = verdict(0.95);
        assert!(text.contains("Good"));
        assert_eq!(color, egui::Color32::from_rgb(22, 163, 74));

        let (text, _) = verdict(0.75);
        assert!(text.contains("Fair"));

        let (text, color) = verdict(0.5);
        assert!(text.contains("Poor"));
        assert_eq!(color, egui::Color32::RED);
    }

    #[test]
    fn labels_cover_all_choices() {
        for kind in MODEL_CHOICES {
            assert!(!model_label(kind).is_empty());
        }
        for kind in OPTIMIZER_CHOICES {
            assert!(!optimizer_label(kind).is_empty());
        }
    }
}
