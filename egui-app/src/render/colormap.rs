/// カラーマップ補間ユーティリティ
#[derive(Clone)]
pub struct ColorMap {
    /// (t, color) の停止点リスト。t は [0.0, 1.0] の範囲。
    pub stops: Vec<(f32, egui::Color32)>,
}

impl ColorMap {
    /// Viridis カラーマップ（5停止点近似）
    pub fn viridis() -> Self {
        Self {
            stops: vec![
                (0.0, egui::Color32::from_rgb(68, 1, 84)),
                (0.25, egui::Color32::from_rgb(58, 82, 139)),
                (0.5, egui::Color32::from_rgb(32, 144, 140)),
                (0.75, egui::Color32::from_rgb(94, 201, 98)),
                (1.0, egui::Color32::from_rgb(253, 231, 37)),
            ],
        }
    }

    /// Plasma カラーマップ（5停止点近似）— 不確実性（標準偏差）の可視化に使用
    pub fn plasma() -> Self {
        Self {
            stops: vec![
                (0.0, egui::Color32::from_rgb(13, 8, 135)),
                (0.25, egui::Color32::from_rgb(126, 3, 168)),
                (0.5, egui::Color32::from_rgb(204, 71, 120)),
                (0.75, egui::Color32::from_rgb(248, 149, 64)),
                (1.0, egui::Color32::from_rgb(240, 249, 33)),
            ],
        }
    }

    /// Blue-to-Yellow カラーマップ（Pareto ランク用）
    pub fn blue_yellow() -> Self {
        Self {
            stops: vec![
                (0.0, egui::Color32::from_rgb(255, 220, 0)),
                (1.0, egui::Color32::from_rgb(0, 80, 200)),
            ],
        }
    }

    /// Jet カラーマップ（7停止点近似）
    pub fn jet() -> Self {
        Self {
            stops: vec![
                (0.0, egui::Color32::from_rgb(0, 0, 143)),
                (0.17, egui::Color32::from_rgb(0, 0, 255)),
                (0.33, egui::Color32::from_rgb(0, 200, 255)),
                (0.5, egui::Color32::from_rgb(100, 255, 0)),
                (0.67, egui::Color32::from_rgb(255, 255, 0)),
                (0.83, egui::Color32::from_rgb(255, 100, 0)),
                (1.0, egui::Color32::from_rgb(128, 0, 0)),
            ],
        }
    }

    /// Turbo カラーマップ（7停止点近似）
    pub fn turbo() -> Self {
        Self {
            stops: vec![
                (0.0, egui::Color32::from_rgb(48, 18, 59)),
                (0.17, egui::Color32::from_rgb(70, 108, 228)),
                (0.33, egui::Color32::from_rgb(30, 195, 149)),
                (0.5, egui::Color32::from_rgb(163, 222, 30)),
                (0.67, egui::Color32::from_rgb(249, 160, 27)),
                (0.83, egui::Color32::from_rgb(220, 50, 32)),
                (1.0, egui::Color32::from_rgb(122, 4, 3)),
            ],
        }
    }

    /// Inferno カラーマップ（5停止点近似）
    pub fn inferno() -> Self {
        Self {
            stops: vec![
                (0.0, egui::Color32::from_rgb(0, 0, 4)),
                (0.25, egui::Color32::from_rgb(87, 16, 110)),
                (0.5, egui::Color32::from_rgb(188, 55, 84)),
                (0.75, egui::Color32::from_rgb(249, 142, 9)),
                (1.0, egui::Color32::from_rgb(252, 255, 164)),
            ],
        }
    }

    /// Coolwarm カラーマップ（5停止点近似、発散型）
    pub fn coolwarm() -> Self {
        Self {
            stops: vec![
                (0.0, egui::Color32::from_rgb(59, 76, 192)),
                (0.25, egui::Color32::from_rgb(141, 176, 254)),
                (0.5, egui::Color32::from_rgb(237, 237, 237)),
                (0.75, egui::Color32::from_rgb(252, 146, 114)),
                (1.0, egui::Color32::from_rgb(180, 4, 38)),
            ],
        }
    }

    /// Spectral カラーマップ（7停止点近似）
    pub fn spectral() -> Self {
        Self {
            stops: vec![
                (0.0, egui::Color32::from_rgb(158, 1, 66)),
                (0.17, egui::Color32::from_rgb(213, 62, 79)),
                (0.33, egui::Color32::from_rgb(244, 109, 67)),
                (0.5, egui::Color32::from_rgb(253, 200, 128)),
                (0.67, egui::Color32::from_rgb(171, 222, 164)),
                (0.83, egui::Color32::from_rgb(53, 151, 143)),
                (1.0, egui::Color32::from_rgb(94, 79, 162)),
            ],
        }
    }

    /// Cividis カラーマップ（5停止点近似、色覚多様性対応）
    pub fn cividis() -> Self {
        Self {
            stops: vec![
                (0.0, egui::Color32::from_rgb(0, 32, 76)),
                (0.25, egui::Color32::from_rgb(57, 89, 129)),
                (0.5, egui::Color32::from_rgb(126, 160, 150)),
                (0.75, egui::Color32::from_rgb(204, 213, 122)),
                (1.0, egui::Color32::from_rgb(253, 252, 47)),
            ],
        }
    }

    /// t を [0.0, 1.0] にクランプして停止点間を線形補間する
    pub fn interpolate(&self, t: f32) -> egui::Color32 {
        if self.stops.is_empty() {
            return egui::Color32::WHITE;
        }
        let t = t.clamp(0.0, 1.0);

        // t 以下の最後の停止点と t 以上の最初の停止点を見つける
        let n = self.stops.len();
        if t <= self.stops[0].0 {
            return self.stops[0].1;
        }
        if t >= self.stops[n - 1].0 {
            return self.stops[n - 1].1;
        }

        for i in 0..n - 1 {
            let (t0, c0) = self.stops[i];
            let (t1, c1) = self.stops[i + 1];
            if t0 <= t && t <= t1 {
                let frac = if (t1 - t0).abs() < f32::EPSILON {
                    0.0
                } else {
                    (t - t0) / (t1 - t0)
                };
                let lerp = |a: u8, b: u8| -> u8 {
                    (a as f32 + frac * (b as f32 - a as f32)).round() as u8
                };
                return egui::Color32::from_rgb(
                    lerp(c0.r(), c1.r()),
                    lerp(c0.g(), c1.g()),
                    lerp(c0.b(), c1.b()),
                );
            }
        }
        self.stops[n - 1].1
    }
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

/// Tableau10 相当の離散カラーパレット（クラスタ表示用）
pub fn tab10_palette() -> Vec<egui::Color32> {
    vec![
        egui::Color32::from_rgb(31, 119, 180),  // Blue
        egui::Color32::from_rgb(255, 127, 14),  // Orange
        egui::Color32::from_rgb(44, 160, 44),   // Green
        egui::Color32::from_rgb(214, 39, 40),   // Red
        egui::Color32::from_rgb(148, 103, 189), // Purple
        egui::Color32::from_rgb(140, 86, 75),   // Brown
        egui::Color32::from_rgb(227, 119, 194), // Pink
        egui::Color32::from_rgb(127, 127, 127), // Gray
        egui::Color32::from_rgb(188, 189, 34),  // Olive
        egui::Color32::from_rgb(23, 190, 207),  // Cyan
    ]
}

use crate::state::app_state::{ColorMode, ColormapName, TrialRow};

/// ColorMode に基づいて TrialRow の値を [0.0, 1.0] に正規化する。
/// ClusterId の場合は呼び出し側で tab10_palette を使用する（この関数は 0.5 を返す）。
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
    let palette = tab10_palette();
    // trial_number は Study 内連番のため最大値で正規化する
    let (max_rank, max_trial_number) = trial_rows.iter().fold((0u32, 0u32), |(mr, mid), r| {
        (mr.max(r.pareto_rank), mid.max(r.trial_number))
    });

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
                    palette[(id.unsigned_abs() as usize) % palette.len()]
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
    fn interpolate_at_zero_returns_first_stop() {
        let cmap = ColorMap::viridis();
        let color = cmap.interpolate(0.0);
        assert_eq!(color, egui::Color32::from_rgb(68, 1, 84));
    }

    #[test]
    fn interpolate_at_one_returns_last_stop() {
        let cmap = ColorMap::viridis();
        let color = cmap.interpolate(1.0);
        assert_eq!(color, egui::Color32::from_rgb(253, 231, 37));
    }

    #[test]
    fn interpolate_at_half_returns_midpoint() {
        let cmap = ColorMap::blue_yellow();
        let color = cmap.interpolate(0.5);
        // lerp: r = 255 + 0.5*(0-255) = 127, g = 220 + 0.5*(80-220) = 150, b = 0 + 0.5*(200-0) = 100
        assert_eq!(color.r(), 128);
        assert_eq!(color.g(), 150);
        assert_eq!(color.b(), 100);
    }

    #[test]
    fn interpolate_clamped_negative() {
        let cmap = ColorMap::viridis();
        let color_neg = cmap.interpolate(-0.1);
        let color_zero = cmap.interpolate(0.0);
        assert_eq!(color_neg, color_zero);
    }

    #[test]
    fn interpolate_clamped_above_one() {
        let cmap = ColorMap::viridis();
        let color_over = cmap.interpolate(1.1);
        let color_one = cmap.interpolate(1.0);
        assert_eq!(color_over, color_one);
    }

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
    fn tab10_palette_has_ten_colors() {
        let palette = tab10_palette();
        assert_eq!(palette.len(), 10);
        // 各色が異なる値
        for i in 0..palette.len() {
            for j in (i + 1)..palette.len() {
                assert_ne!(palette[i], palette[j], "colors at {} and {} are same", i, j);
            }
        }
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
    fn compute_chart_colors_cluster_id_uses_palette() {
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
        let palette = tab10_palette();
        assert_eq!(colors[0], palette[0]);
        assert_eq!(colors[1], palette[1]);
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
        // score 0.0 → first stop, score 1.0 → last stop
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
