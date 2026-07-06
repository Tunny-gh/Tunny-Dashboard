//! ピン留めした trial を重ね描きするレーダー比較ウィジェット。
//!
//! 軸は目的関数（常時）+ 数値パラメータ（トグルで追加）。各軸は Study 全体の
//! 数値列で min-max 正規化し、"Outward = better" が有効なら最小化目的の軸だけ
//! 反転する（外側 = 良い、で統一するため）。描画そのものはトライアル詳細モーダルの
//! レーダーチャートと共通の [`crate::ui::widgets::common::radar_chart::draw_radar`] に委譲する。
//! 詳細は `theory/{en,ja}/widgets/radar-comparison.md` を参照。

use crate::state::types::{Direction, StudyView};
use crate::theme::chart_colors::COLOR_EMPTY_STATE;
use crate::theme::colormap::ColorMap;
use crate::ui::widgets::common::radar_chart::{draw_radar, swatch, RadarSeries};

/// レーダー比較ウィジェットの UI 状態。計算キャッシュは持たない
/// （数個の多角形を毎フレーム再計算しても軽量なため）。
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct RadarComparisonChart {
    /// 数値パラメータも軸に含めるか（既定は目的関数のみ）。
    pub include_params: bool,
    /// 最小化目的の軸を反転し、外側ほど良いに統一するか。
    pub outward_better: bool,
}

impl Default for RadarComparisonChart {
    fn default() -> Self {
        Self {
            include_params: false,
            outward_better: true,
        }
    }
}

/// レーダー 1 軸ぶんの情報（列の借用込み）。`show` と CSV エクスポートで共有する。
pub struct AxisInfo<'a> {
    pub name: &'a str,
    pub col: &'a [f64],
    pub is_objective: bool,
    /// 目的関数軸の場合、`directions` 内のインデックス（反転判定に使う）。
    pub obj_idx: Option<usize>,
}

/// 軸リストを構築する（目的関数 → 数値パラメータの順）。数値列を持たない
/// 目的・パラメータはスキップする。`include_params` が false ならパラメータ軸は追加しない。
pub fn build_axes<'a>(
    view: &'a StudyView,
    param_names: &'a [String],
    obj_names: &'a [String],
    include_params: bool,
) -> Vec<AxisInfo<'a>> {
    let mut axes = Vec::with_capacity(obj_names.len() + param_names.len());
    for (i, name) in obj_names.iter().enumerate() {
        if let Some(col) = view.numeric_column(name) {
            axes.push(AxisInfo {
                name,
                col,
                is_objective: true,
                obj_idx: Some(i),
            });
        }
    }
    if include_params {
        for name in param_names {
            if let Some(col) = view.numeric_column(name) {
                axes.push(AxisInfo {
                    name,
                    col,
                    is_objective: false,
                    obj_idx: None,
                });
            }
        }
    }
    axes
}

/// 軸の値域（非有限値を除いた min/max）。範囲が空なら `(0.0, 0.0)`（degenerate 扱い）。
fn axis_range(col: &[f64]) -> (f64, f64) {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for &v in col {
        if v.is_finite() {
            lo = lo.min(v);
            hi = hi.max(v);
        }
    }
    if lo.is_finite() && hi.is_finite() {
        (lo, hi)
    } else {
        (0.0, 0.0)
    }
}

/// 値を軸上の半径割合 [0,1] に正規化する。`min == max`（degenerate）なら 0.5。
/// `flip` が true なら `u -> 1 - u`（"outward = better" 用）。
pub fn normalize(v: f64, min: f64, max: f64, flip: bool) -> f64 {
    let span = max - min;
    let u = if span.abs() <= f64::EPSILON {
        0.5
    } else {
        ((v - min) / span).clamp(0.0, 1.0)
    };
    if flip {
        1.0 - u
    } else {
        u
    }
}

impl RadarComparisonChart {
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        param_names: &[String],
        obj_names: &[String],
        directions: &[Direction],
        pinned_trials: &[u32],
    ) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.include_params, "Include parameters");
            ui.checkbox(&mut self.outward_better, "Outward = better")
                .on_hover_text("Flip minimized-objective axes so larger polygons are better");
        });

        let axes = build_axes(view, param_names, obj_names, self.include_params);
        if axes.len() < 3 {
            ui.vertical_centered(|ui| {
                ui.colored_label(
                    COLOR_EMPTY_STATE(),
                    "Radar needs at least 3 axes — enable parameters.",
                );
            });
            return;
        }

        if pinned_trials.is_empty() {
            ui.vertical_centered(|ui| {
                ui.colored_label(
                    COLOR_EMPTY_STATE(),
                    "Pin trials (📌) in the Trial Table to compare them here.",
                );
            });
            return;
        }

        let ranges: Vec<(f64, f64)> = axes.iter().map(|a| axis_range(a.col)).collect();
        let axis_labels: Vec<(String, bool)> = axes
            .iter()
            .map(|a| (a.name.to_string(), a.is_objective))
            .collect();
        let cmap = ColorMap::turbo();
        let n_pins = pinned_trials.len();

        let mut series: Vec<RadarSeries> = Vec::with_capacity(n_pins);
        let mut legend_entries: Vec<(egui::Color32, String)> = Vec::with_capacity(n_pins);
        for (pin_idx, &trial_id) in pinned_trials.iter().enumerate() {
            let Some(row) = view.trial_ids.iter().position(|&t| t == trial_id) else {
                continue;
            };
            let fractions: Vec<Option<f32>> = axes
                .iter()
                .enumerate()
                .map(|(k, axis)| {
                    let raw = axis.col.get(row).copied().unwrap_or(f64::NAN);
                    let (lo, hi) = ranges[k];
                    let flip = self.outward_better
                        && axis.is_objective
                        && axis
                            .obj_idx
                            .and_then(|oi| directions.get(oi))
                            .map(|d| matches!(d, Direction::Minimize))
                            .unwrap_or(false);
                    let u = if raw.is_finite() {
                        normalize(raw, lo, hi, flip)
                    } else {
                        0.5
                    };
                    Some(u as f32)
                })
                .collect();

            let number = view.df.get_trial_number(row).unwrap_or(trial_id);
            let label = format!("Trial #{number}");
            let t = if n_pins <= 1 {
                0.5
            } else {
                pin_idx as f32 / (n_pins - 1) as f32
            };
            let color = cmap.interpolate(t);
            legend_entries.push((color, label.clone()));
            series.push(RadarSeries {
                color,
                fractions,
                // 強調表示（扇形メッシュ塗り + ドット）はせず、太めの輪郭線のみで
                // 複数トライアルを重ね描きしても見分けやすくする。
                width: 2.0,
                emphasized: false,
            });
        }

        draw_radar(ui, &axis_labels, &series);

        // ── 凡例（ピン留めトライアルごとの色見本 + トライアル番号）──────
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            for (color, label) in &legend_entries {
                swatch(ui, *color);
                ui.label(egui::RichText::new(label).small());
                ui.add_space(10.0);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_objectives_only_and_outward_better() {
        let chart = RadarComparisonChart::default();
        assert!(!chart.include_params);
        assert!(chart.outward_better);
    }

    #[test]
    fn normalize_normal_case() {
        assert!((normalize(5.0, 0.0, 10.0, false) - 0.5).abs() < 1e-9);
        assert!((normalize(2.0, 0.0, 10.0, false) - 0.2).abs() < 1e-9);
    }

    #[test]
    fn normalize_flip_case() {
        assert!((normalize(5.0, 0.0, 10.0, true) - 0.5).abs() < 1e-9);
        assert!((normalize(2.0, 0.0, 10.0, true) - 0.8).abs() < 1e-9);
    }

    #[test]
    fn normalize_degenerate_range_is_mid_radius() {
        assert!((normalize(5.0, 3.0, 3.0, false) - 0.5).abs() < 1e-9);
        assert!((normalize(5.0, 3.0, 3.0, true) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn normalize_clamps_out_of_range_values() {
        assert!((normalize(-5.0, 0.0, 10.0, false) - 0.0).abs() < 1e-9);
        assert!((normalize(15.0, 0.0, 10.0, false) - 1.0).abs() < 1e-9);
    }
}
