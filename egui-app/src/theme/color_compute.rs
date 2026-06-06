use crate::state::app_state::{ColorMode, ColormapName, TrialRow};
use crate::state::types::StudyView;

/// RGBA バイト配列（非プリマルチプライドアルファ、順序 [R, G, B, A]）を
/// egui の Color32 へ変換する。
/// state 層は egui 依存を持たないため `[u8; 4]` で色を保持しており、
/// UI 描画時にこの関数を使って Color32 へ変換する。
pub fn rgba_to_color32(rgba: [u8; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3])
}

/// 比較 Study に割り当てる代表色のパレット（基準 Study の緑系とは別の色相）。
/// 各要素は `[R, G, B, A]` の非プリマルチプライドアルファ。
const COMPARISON_PALETTE: [[u8; 4]; 6] = [
    [66, 133, 244, 255], // 青
    [234, 67, 53, 255],  // 赤
    [251, 188, 4, 255],  // 黄
    [171, 71, 188, 255], // 紫
    [255, 112, 67, 255], // オレンジ
    [0, 172, 193, 255],  // シアン
];

/// `idx` 番目の比較 Study に割り当てる色を返す（パレットを循環）。
pub fn comparison_color_at(idx: usize) -> [u8; 4] {
    COMPARISON_PALETTE[idx % COMPARISON_PALETTE.len()]
}

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
    let cmap = crate::theme::colormap_name::colormap_from_name(colormap_name);
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

/// `StudyView` ベースの色計算（`compute_chart_colors` の view 版）。
pub fn compute_chart_colors_view(
    color_mode: &ColorMode,
    colormap_name: &ColormapName,
    view: &StudyView,
    objective_names: &[String],
    mcdm_scores: Option<&[f64]>,
) -> Vec<egui::Color32> {
    let cmap = crate::theme::colormap_name::colormap_from_name(colormap_name);
    let n = view.row_count();

    let max_rank = view.pareto_rank.iter().copied().max().unwrap_or(0);
    let max_cluster_id = view
        .cluster_id
        .iter()
        .filter_map(|&c| c)
        .map(|id| id.unsigned_abs())
        .max()
        .unwrap_or(0);

    let obj_idx = match color_mode {
        ColorMode::ObjectiveValue(name) => objective_names.iter().position(|n| n == name),
        _ => None,
    };
    let obj_col: Option<&[f64]> = obj_idx.and_then(|idx| {
        objective_names
            .get(idx)
            .and_then(|name| view.numeric_column(name))
    });
    let obj_min_max: Option<(f64, f64)> = obj_col.map(|col| {
        col.iter()
            .copied()
            .filter(|v| v.is_finite())
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(mn, mx), v| {
                (mn.min(v), mx.max(v))
            })
    });

    (0..n)
        .map(|i| match color_mode {
            ColorMode::ClusterId => {
                if let Some(id) = view.cluster_id.get(i).copied().flatten() {
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
            ColorMode::ParetoRank => {
                let rank = view.pareto_rank.get(i).copied().unwrap_or(0);
                let mr = max_rank.max(1) as f32 + 1.0;
                let t = 1.0 - rank as f32 / mr;
                cmap.interpolate(t)
            }
            ColorMode::TrialNumber => {
                let t = i as f32 / n.max(1) as f32;
                cmap.interpolate(t)
            }
            ColorMode::ObjectiveValue(_) => {
                let val = obj_col.and_then(|c| c.get(i)).copied().unwrap_or(f64::NAN);
                if val.is_finite() {
                    let (min, max) = obj_min_max.unwrap_or((0.0, 1.0));
                    let range = max - min;
                    let t = if range.abs() < f64::EPSILON {
                        0.5
                    } else {
                        ((val - min) / range) as f32
                    };
                    cmap.interpolate(t)
                } else {
                    egui::Color32::LIGHT_GRAY
                }
            }
        })
        .collect()
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + t * (b as f32 - a as f32)) as u8
}

fn lerp_color(c0: egui::Color32, c1: egui::Color32, t: f32) -> egui::Color32 {
    egui::Color32::from_rgb(
        lerp_u8(c0.r(), c1.r(), t),
        lerp_u8(c0.g(), c1.g(), t),
        lerp_u8(c0.b(), c1.b(), t),
    )
}

/// [-1, +1] の値を low(-1) → white(0) → high(+1) の3点グラデーションに変換する。
fn signed_to_diverging_color(v: f64, low: egui::Color32, high: egui::Color32) -> egui::Color32 {
    let t = ((v + 1.0) / 2.0).clamp(0.0, 1.0) as f32;
    if t < 0.5 {
        lerp_color(low, egui::Color32::WHITE, t * 2.0)
    } else {
        lerp_color(egui::Color32::WHITE, high, (t - 0.5) * 2.0)
    }
}

/// 発散型カラーマップ: score=-1.0 → 青, 0.0 → 白, +1.0 → 赤
pub fn diverging_colormap(score: f64) -> egui::Color32 {
    signed_to_diverging_color(score, egui::Color32::BLUE, egui::Color32::RED)
}

/// 相関係数を Color32 に変換する（赤=負相関, 白=無相関, 青=正相関）
pub fn correlation_color(corr: f64) -> egui::Color32 {
    signed_to_diverging_color(corr, egui::Color32::RED, egui::Color32::BLUE)
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
        let cmap = crate::theme::colormap_name::colormap_from_name(&ColormapName::Viridis);
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
        use crate::theme::colormap::ColorMap;
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

    #[test]
    fn diverging_colormap_negative_one_is_blue() {
        let color = diverging_colormap(-1.0);
        assert!(color.b() > color.r(), "score=-1 should be blue-dominant");
    }

    #[test]
    fn diverging_colormap_zero_is_white() {
        let color = diverging_colormap(0.0);
        assert_eq!(color.r(), 255);
        assert_eq!(color.g(), 255);
        assert_eq!(color.b(), 255);
    }

    #[test]
    fn diverging_colormap_positive_one_is_red() {
        let color = diverging_colormap(1.0);
        assert!(color.r() > color.b(), "score=+1 should be red-dominant");
    }

    #[test]
    fn diverging_colormap_intermediate_values_bounded() {
        for i in -10..=10 {
            let score = i as f64 / 10.0;
            let _ = diverging_colormap(score);
        }
    }

    #[test]
    fn correlation_color_negative_is_reddish() {
        let color = correlation_color(-1.0);
        assert!(color.r() > color.b());
    }

    #[test]
    fn correlation_color_positive_is_bluish() {
        let color = correlation_color(1.0);
        assert!(color.b() > color.r());
    }
}
