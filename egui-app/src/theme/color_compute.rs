use crate::state::app_state::{ColorMode, ColormapName, TrialRow};
use crate::theme::colormap::ColorMap;

/// trial_id が selected_indices に含まれるかでアルファ値を計算する。
/// selected_indices が空の場合は全点が不透明（255）を返す。
pub fn compute_point_alpha(trial_id: u32, selected_indices: &[u32]) -> u8 {
    if selected_indices.is_empty() || selected_indices.contains(&trial_id) {
        255
    } else {
        50
    }
}

/// ColorMode に基づいて TrialRow の値を [0.0, 1.0] に正規化する。
pub fn normalize_trial(
    trial: &TrialRow,
    color_mode: &ColorMode,
    max_rank: u32,
    obj_idx: Option<usize>,
    obj_min_max: Option<(f64, f64)>,
    max_trial_number: u32,
) -> f32 {
    match color_mode {
        ColorMode::ParetoRank => {
            let mr = max_rank.max(1) as f32 + 1.0;
            1.0 - trial.pareto_rank as f32 / mr
        }
        ColorMode::ObjectiveValue(_) => {
            if let Some(idx) = obj_idx {
                if let Some(val) = trial.objectives.get(idx).copied() {
                    let (min, max) = obj_min_max.unwrap_or((0.0, 1.0));
                    let range = max - min;
                    if range.abs() < f64::EPSILON {
                        0.5
                    } else {
                        ((val - min) / range) as f32
                    }
                } else {
                    0.5
                }
            } else {
                0.5
            }
        }
        ColorMode::TrialNumber => trial.trial_number as f32 / max_trial_number.max(1) as f32,
        ColorMode::ClusterId | ColorMode::McdmScore => 0.5,
    }
}

/// 全 TrialRow の色を ColorMode + ColormapName に基づいて計算する。
pub fn compute_chart_colors(
    color_mode: &ColorMode,
    colormap_name: &ColormapName,
    trial_rows: &[TrialRow],
    objective_names: &[String],
    mcdm_scores: Option<&[f64]>,
) -> Vec<egui::Color32> {
    let cmap = colormap_name.to_colormap();
    let (max_rank, max_trial_number) = trial_rows.iter().fold((0u32, 0u32), |(mr, mid), r| {
        (mr.max(r.pareto_rank), mid.max(r.trial_number))
    });
    let max_cluster_id = trial_rows
        .iter()
        .filter_map(|r| r.cluster_id)
        .map(|id| id.unsigned_abs())
        .max()
        .unwrap_or(0);

    let obj_idx = match color_mode {
        ColorMode::ObjectiveValue(name) => objective_names.iter().position(|n| n == name),
        _ => None,
    };
    let obj_min_max: Option<(f64, f64)> = obj_idx.map(|idx| {
        trial_rows
            .iter()
            .filter_map(|r| r.objectives.get(idx).copied())
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(mn, mx), v| {
                (mn.min(v), mx.max(v))
            })
    });

    trial_rows
        .iter()
        .enumerate()
        .map(|(i, trial)| match color_mode {
            ColorMode::ClusterId => {
                if let Some(id) = trial.cluster_id {
                    let t = id.unsigned_abs() as f32 / max_cluster_id.max(1) as f32;
                    cmap.interpolate(t)
                } else {
                    egui::Color32::LIGHT_GRAY
                }
            }
            ColorMode::McdmScore => {
                if let Some(scores) = mcdm_scores {
                    let score = scores.get(i).copied().unwrap_or(0.0);
                    cmap.interpolate(score as f32)
                } else {
                    egui::Color32::LIGHT_GRAY
                }
            }
            _ => {
                let t = normalize_trial(
                    trial,
                    color_mode,
                    max_rank,
                    obj_idx,
                    obj_min_max,
                    max_trial_number,
                );
                cmap.interpolate(t)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_point_alpha_empty_selected_returns_opaque() {
        assert_eq!(compute_point_alpha(0, &[]), 255);
        assert_eq!(compute_point_alpha(99, &[]), 255);
    }

    #[test]
    fn compute_point_alpha_selected_returns_opaque() {
        assert_eq!(compute_point_alpha(5, &[1, 5, 10]), 255);
    }

    #[test]
    fn compute_point_alpha_not_selected_returns_transparent() {
        assert_eq!(compute_point_alpha(3, &[1, 5, 10]), 50);
    }

    #[test]
    fn normalize_pareto_rank_zero_is_highest() {
        use crate::state::app_state::TrialState;
        let trial = TrialRow {
            trial_id: 0,
            trial_number: 0,
            params: Default::default(),
            objectives: vec![],
            pareto_rank: 0,
            cluster_id: None,
            state: TrialState::Complete,
            user_attrs: Default::default(),
        };
        let t = normalize_trial(&trial, &ColorMode::ParetoRank, 5, None, None, 0);
        assert!(t > 0.8, "rank 0 should be high t, got {}", t);
    }

    #[test]
    fn normalize_trial_number_linear() {
        use crate::state::app_state::TrialState;
        let trial = TrialRow {
            trial_id: 9,
            trial_number: 9,
            params: Default::default(),
            objectives: vec![],
            pareto_rank: 0,
            cluster_id: None,
            state: TrialState::Complete,
            user_attrs: Default::default(),
        };
        let t = normalize_trial(&trial, &ColorMode::TrialNumber, 0, None, None, 9);
        assert!(
            (t - 1.0).abs() < 0.01,
            "trial_number=9/max_id=9 should be 1.0, got {}",
            t
        );
    }

    #[test]
    fn compute_chart_colors_length_matches_trials() {
        use crate::state::app_state::TrialState;
        use std::collections::HashMap;
        let rows = vec![
            TrialRow {
                trial_id: 0,
                trial_number: 0,
                params: HashMap::new(),
                objectives: vec![0.5],
                pareto_rank: 0,
                cluster_id: None,
                state: TrialState::Complete,
                user_attrs: HashMap::new(),
            },
            TrialRow {
                trial_id: 1,
                trial_number: 1,
                params: HashMap::new(),
                objectives: vec![1.0],
                pareto_rank: 1,
                cluster_id: None,
                state: TrialState::Complete,
                user_attrs: HashMap::new(),
            },
        ];
        let colors = compute_chart_colors(
            &ColorMode::ParetoRank,
            &ColormapName::Viridis,
            &rows,
            &[],
            None,
        );
        assert_eq!(colors.len(), 2);
    }

    #[test]
    fn compute_chart_colors_pareto_rank_different_colors() {
        use crate::state::app_state::TrialState;
        use std::collections::HashMap;
        let rows = vec![
            TrialRow {
                trial_id: 0,
                trial_number: 0,
                params: HashMap::new(),
                objectives: vec![],
                pareto_rank: 0,
                cluster_id: None,
                state: TrialState::Complete,
                user_attrs: HashMap::new(),
            },
            TrialRow {
                trial_id: 1,
                trial_number: 1,
                params: HashMap::new(),
                objectives: vec![],
                pareto_rank: 5,
                cluster_id: None,
                state: TrialState::Complete,
                user_attrs: HashMap::new(),
            },
        ];
        let colors = compute_chart_colors(
            &ColorMode::ParetoRank,
            &ColormapName::Viridis,
            &rows,
            &[],
            None,
        );
        assert_ne!(
            colors[0], colors[1],
            "different ranks should have different colors"
        );
    }

    #[test]
    fn compute_chart_colors_cluster_id_uses_colormap() {
        use crate::state::app_state::TrialState;
        use std::collections::HashMap;
        let rows = vec![
            TrialRow {
                trial_id: 0,
                trial_number: 0,
                params: HashMap::new(),
                objectives: vec![],
                pareto_rank: 0,
                cluster_id: Some(0),
                state: TrialState::Complete,
                user_attrs: HashMap::new(),
            },
            TrialRow {
                trial_id: 1,
                trial_number: 1,
                params: HashMap::new(),
                objectives: vec![],
                pareto_rank: 0,
                cluster_id: Some(1),
                state: TrialState::Complete,
                user_attrs: HashMap::new(),
            },
        ];
        let colors = compute_chart_colors(
            &ColorMode::ClusterId,
            &ColormapName::Viridis,
            &rows,
            &[],
            None,
        );
        let cmap = ColormapName::Viridis.to_colormap();
        assert_eq!(colors[0], cmap.interpolate(0.0));
        assert_eq!(colors[1], cmap.interpolate(1.0));
        assert_ne!(colors[0], colors[1]);
    }

    #[test]
    fn compute_chart_colors_cluster_none_is_gray() {
        use crate::state::app_state::TrialState;
        use std::collections::HashMap;
        let rows = vec![TrialRow {
            trial_id: 0,
            trial_number: 0,
            params: HashMap::new(),
            objectives: vec![],
            pareto_rank: 0,
            cluster_id: None,
            state: TrialState::Complete,
            user_attrs: HashMap::new(),
        }];
        let colors = compute_chart_colors(
            &ColorMode::ClusterId,
            &ColormapName::Viridis,
            &rows,
            &[],
            None,
        );
        assert_eq!(colors[0], egui::Color32::LIGHT_GRAY);
    }

    #[test]
    fn compute_chart_colors_mcdm_score_with_scores() {
        use crate::state::app_state::TrialState;
        use std::collections::HashMap;
        let rows = vec![
            TrialRow {
                trial_id: 0,
                trial_number: 0,
                params: HashMap::new(),
                objectives: vec![],
                pareto_rank: 0,
                cluster_id: None,
                state: TrialState::Complete,
                user_attrs: HashMap::new(),
            },
            TrialRow {
                trial_id: 1,
                trial_number: 1,
                params: HashMap::new(),
                objectives: vec![],
                pareto_rank: 0,
                cluster_id: None,
                state: TrialState::Complete,
                user_attrs: HashMap::new(),
            },
        ];
        let scores = vec![0.0, 1.0];
        let colors = compute_chart_colors(
            &ColorMode::McdmScore,
            &ColormapName::Viridis,
            &rows,
            &[],
            Some(&scores),
        );
        assert_eq!(colors.len(), 2);
        assert_eq!(colors[0], ColorMap::viridis().interpolate(0.0));
        assert_eq!(colors[1], ColorMap::viridis().interpolate(1.0));
    }

    #[test]
    fn compute_chart_colors_mcdm_score_none_is_gray() {
        use crate::state::app_state::TrialState;
        use std::collections::HashMap;
        let rows = vec![TrialRow {
            trial_id: 0,
            trial_number: 0,
            params: HashMap::new(),
            objectives: vec![],
            pareto_rank: 0,
            cluster_id: None,
            state: TrialState::Complete,
            user_attrs: HashMap::new(),
        }];
        let colors = compute_chart_colors(
            &ColorMode::McdmScore,
            &ColormapName::Viridis,
            &rows,
            &[],
            None,
        );
        assert_eq!(colors[0], egui::Color32::LIGHT_GRAY);
    }

    #[test]
    fn compute_chart_colors_mcdm_empty_scores() {
        let colors = compute_chart_colors(
            &ColorMode::McdmScore,
            &ColormapName::Viridis,
            &[],
            &[],
            Some(&[]),
        );
        assert!(colors.is_empty());
    }
}
