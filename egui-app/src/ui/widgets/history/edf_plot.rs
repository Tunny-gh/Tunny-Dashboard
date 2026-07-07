use crate::state::types::StudyView;
use crate::theme::chart_colors::COLOR_OPT_TRIAL;
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};
use crate::ui::widgets::trial_detail_modal::{hit_test_nearest, HIT_THRESHOLD};

/// 比較 Study 1 件分の EDF 系列（選択中の目的に対応する値列 + 色 + 凡例名）。
pub struct EdfComparison {
    pub name: String,
    pub color: egui::Color32,
    /// 選択中の目的に対応する目的値列（COMPLETE trial の行順）。
    pub values: Vec<f64>,
}

/// EDF（経験分布関数）チャートウィジェット
///
/// Optuna の `plot_edf` に相当し、選択した目的関数値の経験分布（値 x 以下の
/// トライアル割合）をステップ関数で描画する。曲線が急峻なら値が集中しており、
/// 右にシフトしているほど（最小化なら）悪い結果が多いことを示す。
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct EdfPlotChart {
    pub obj_idx: usize,
    /// X 軸対数スケール切替。有効時は 0 以下の値を持つ点を曲線から除外する。
    pub log_x: bool,
    /// `build_edf_points`（ソート + 2 回のアロケーション）の結果キャッシュ。
    #[serde(skip)]
    cache: Option<EdfCache>,
}

/// EDF ステップ点列の再計算（O(n log n) ソート）を避けるためのキャッシュ。
///
/// base 系列は `view.df`（`Arc<DataFrame>`）の恒等性でキーイングできるが、
/// comparison 系列（`EdfComparison::values`）は呼び出し側（render_chart.rs）が
/// 毎フレーム新しい `Vec<f64>` として値を構築するため、安定したポインタ恒等性を
/// 持たない。そのため両者とも値列の内容フィンガープリント（FNV-1a 風の畳み込み。
/// O(n) だがヒープ確保なし）でキーイングする — ソート＋2 回のアロケーションより
/// 十分に軽い。
struct EdfCache {
    key: (usize, usize, bool, u64),
    base_points: Vec<[f64; 2]>,
    comparison_points: Vec<(String, egui::Color32, Vec<[f64; 2]>)>,
}

/// 値列の内容を安価な u64 フィンガープリントに畳み込む（FNV-1a 風。ヒープ確保なし）。
fn fingerprint_values(values: &[f64]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325 ^ (values.len() as u64);
    for &v in values {
        h = (h ^ v.to_bits()).wrapping_mul(0x100000001b3);
    }
    h
}

/// 比較 Study 一覧の内容フィンガープリントを base のフィンガープリントに畳み込む。
/// 系列名も含めるため、値が同じでも Study が入れ替わった場合は別キーになる。
fn fold_comparisons_fingerprint(mut h: u64, comparisons: &[EdfComparison]) -> u64 {
    for c in comparisons {
        h = h.wrapping_mul(0x100000001b3) ^ fingerprint_values(&c.values);
        for &b in c.name.as_bytes() {
            h = (h ^ (b as u64)).wrapping_mul(0x100000001b3);
        }
    }
    h
}

impl EdfPlotChart {
    /// EDF チャートを描画する。比較 Study の EDF 曲線を同一グラフに重ねる。
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        obj_names: &[String],
        base_name: &str,
        comparisons: &[EdfComparison],
    ) {
        if obj_names.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No objectives.").weak());
            });
            return;
        }

        self.obj_idx = self.obj_idx.min(obj_names.len() - 1);

        ui.horizontal(|ui| {
            if obj_names.len() > 1 {
                ui.label("Objective:");
                egui::ComboBox::from_id_salt("edf_plot_obj_combo")
                    .selected_text(&obj_names[self.obj_idx])
                    .show_ui(ui, |ui| {
                        for (i, name) in obj_names.iter().enumerate() {
                            ui.selectable_value(&mut self.obj_idx, i, name);
                        }
                    });
                ui.separator();
            }
            if ui.selectable_label(self.log_x, "Log Scale").clicked() {
                self.log_x = !self.log_x;
            }
        });

        let obj_name = &obj_names[self.obj_idx];
        let log_x = self.log_x;
        // numeric_column は DataFrame 内の列をゼロコピーで借用するだけなので、
        // フィンガープリント計算・ソートのどちらにも Vec 化は不要。
        let base_values: &[f64] = view.numeric_column(obj_name).unwrap_or(&[]);

        // キャッシュキー: view（DataFrame）の恒等性 + 選択中の目的 + log スケール +
        // base/comparison 値列の内容フィンガープリント。
        let df_ptr = std::sync::Arc::as_ptr(&view.df) as usize;
        let fp = fold_comparisons_fingerprint(fingerprint_values(base_values), comparisons);
        let key = (df_ptr, self.obj_idx, log_x, fp);

        let cache_valid = self.cache.as_ref().is_some_and(|c| c.key == key);
        if !cache_valid {
            let base_points = build_edf_points(base_values, log_x);
            let comparison_points: Vec<(String, egui::Color32, Vec<[f64; 2]>)> = comparisons
                .iter()
                .map(|c| (c.name.clone(), c.color, build_edf_points(&c.values, log_x)))
                .collect();
            self.cache = Some(EdfCache {
                key,
                base_points,
                comparison_points,
            });
        }
        let cache = self.cache.as_ref().expect("cache just populated above");
        let base_points = &cache.base_points;
        let comparison_points = &cache.comparison_points;

        if base_points.is_empty() && comparison_points.iter().all(|(_, _, pts)| pts.is_empty()) {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No finite objective values to plot.").weak());
            });
            return;
        }

        // X 軸対数スケール時は値を log10 変換して描画する
        // （build_edf_points は非正値をすでに除外済みのため log10 は常に有限）。
        let apply_log = |[x, y]: [f64; 2]| -> [f64; 2] { [if log_x { x.log10() } else { x }, y] };

        let base_label = if comparisons.is_empty() || base_name.is_empty() {
            "EDF"
        } else {
            base_name
        };

        let mut plot = egui_plot::Plot::new("edf_plot")
            .unified_nav()
            .legend(egui_plot::Legend::default())
            .x_axis_label(obj_name)
            .y_axis_label("Cumulative Probability")
            .include_y(0.0)
            .include_y(1.0);
        if log_x {
            plot = crate::ui::widgets::common::log_scale::apply_log_x_axis(plot);
        }

        // ホバー中の最寄り点（凡例名, 元の値, 累積割合）。
        let mut hovered: Option<(String, f64, f64)> = None;

        plot.show(ui, |plot_ui| {
            apply_wheel_zoom(plot_ui);

            if let Some(pos) = plot_ui.response().hover_pos() {
                // 他の history ウィジェットと同じ共通ヒットテストを再利用する。
                // 候補は描画座標系（apply_log 適用後）で並べ、`usize` フィールドに
                // ツールチップ用データ（凡例名・元の値・累積割合）の lookup index を埋め込む。
                let mut hit_points: Vec<(u32, usize, [f64; 2])> = Vec::new();
                let mut lookup: Vec<(String, f64, f64)> = Vec::new();
                let mut push = |name: &str, pts: &[[f64; 2]]| {
                    for &p in pts {
                        let plot_pt = apply_log(p);
                        hit_points.push((0, lookup.len(), plot_pt));
                        lookup.push((name.to_string(), p[0], p[1]));
                    }
                };
                push(base_label, base_points);
                for (name, _, pts) in comparison_points {
                    push(name, pts);
                }
                hovered = hit_test_nearest(plot_ui, &hit_points, pos, HIT_THRESHOLD)
                    .and_then(|(_, i)| lookup.get(i).cloned());
            }

            if !base_points.is_empty() {
                let pts: egui_plot::PlotPoints =
                    base_points.iter().copied().map(apply_log).collect();
                plot_ui.line(
                    egui_plot::Line::new(base_label, pts)
                        .color(COLOR_OPT_TRIAL())
                        .width(1.5),
                );
            }
            for (name, color, pts) in comparison_points {
                if pts.is_empty() {
                    continue;
                }
                let plot_pts: egui_plot::PlotPoints = pts.iter().copied().map(apply_log).collect();
                plot_ui.line(
                    egui_plot::Line::new(name.as_str(), plot_pts)
                        .color(*color)
                        .width(1.5),
                );
            }
        });

        // EDF の点は個別 trial ではなく曲線上の値なので、共有の
        // `show_hover_tooltip`（見出しが "Trial N" 固定）は使わず、
        // 系列名を見出しにした専用ツールチップを描く。
        if let Some((name, value, frac)) = hovered {
            egui::Tooltip::always_open(
                ui.ctx().clone(),
                ui.layer_id(),
                egui::Id::new("edf_plot_hover_tooltip"),
                egui::PopupAnchor::Pointer,
            )
            .show(|ui| {
                ui.strong(name);
                egui::Grid::new("edf_plot_hover_grid")
                    .num_columns(2)
                    .spacing([12.0, 2.0])
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(obj_name.as_str())
                                .color(crate::theme::TEXT_SECONDARY()),
                        );
                        ui.label(format!("{value:.6}"));
                        ui.end_row();
                        ui.label(
                            egui::RichText::new("Cumulative Fraction")
                                .color(crate::theme::TEXT_SECONDARY()),
                        );
                        ui.label(format!("{frac:.4}"));
                        ui.end_row();
                    });
            });
        }
    }
}

/// EDF（経験分布関数）のステップ点列を構築する。
///
/// - NaN / ±Inf の値は除外する。
/// - `log_x` が true のとき、0 以下の値も除外する（対数軸に表せないため）。
/// - 残った値を昇順ソートし、同値はまとめて 1 段のステップとして扱う
///   （右連続: 値 `v` にちょうど一致する点で累積割合が跳ね上がる）。
/// - y は 0..1 の累積割合（フィルタ後の件数で正規化）。
/// - フィルタ後に値が残らない場合は空を返す。
pub fn build_edf_points(values: &[f64], log_x: bool) -> Vec<[f64; 2]> {
    let mut filtered: Vec<f64> = values
        .iter()
        .copied()
        .filter(|v| v.is_finite() && (!log_x || *v > 0.0))
        .collect();
    if filtered.is_empty() {
        return Vec::new();
    }
    filtered.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = filtered.len();

    let mut points = Vec::with_capacity(n * 2);
    let mut prev_frac = 0.0;
    let mut cum = 0usize;
    let mut i = 0;
    while i < filtered.len() {
        let v = filtered[i];
        let mut j = i;
        while j < filtered.len() && filtered[j] == v {
            j += 1;
        }
        cum += j - i;
        let frac = cum as f64 / n as f64;
        points.push([v, prev_frac]);
        points.push([v, frac]);
        prev_frac = frac;
        i = j;
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_edf_points_basic_staircase() {
        let pts = build_edf_points(&[1.0, 2.0, 3.0], false);
        assert_eq!(
            pts,
            vec![
                [1.0, 0.0],
                [1.0, 1.0 / 3.0],
                [2.0, 1.0 / 3.0],
                [2.0, 2.0 / 3.0],
                [3.0, 2.0 / 3.0],
                [3.0, 1.0],
            ]
        );
    }

    #[test]
    fn build_edf_points_handles_ties() {
        let pts = build_edf_points(&[1.0, 1.0, 2.0], false);
        assert_eq!(
            pts,
            vec![[1.0, 0.0], [1.0, 2.0 / 3.0], [2.0, 2.0 / 3.0], [2.0, 1.0],]
        );
    }

    #[test]
    fn build_edf_points_skips_nan_and_inf() {
        let pts = build_edf_points(
            &[1.0, f64::NAN, 2.0, f64::INFINITY, f64::NEG_INFINITY],
            false,
        );
        assert_eq!(pts, vec![[1.0, 0.0], [1.0, 0.5], [2.0, 0.5], [2.0, 1.0],]);
    }

    #[test]
    fn build_edf_points_log_scale_drops_non_positive() {
        let pts = build_edf_points(&[-1.0, 0.0, 1.0, 2.0], true);
        assert_eq!(pts, vec![[1.0, 0.0], [1.0, 0.5], [2.0, 0.5], [2.0, 1.0],]);
    }

    #[test]
    fn build_edf_points_empty_input_returns_empty() {
        assert!(build_edf_points(&[], false).is_empty());
    }

    #[test]
    fn build_edf_points_all_non_finite_returns_empty() {
        assert!(build_edf_points(&[f64::NAN, f64::INFINITY], false).is_empty());
    }

    #[test]
    fn build_edf_points_all_dropped_by_log_filter_returns_empty() {
        assert!(build_edf_points(&[-1.0, 0.0], true).is_empty());
    }

    #[test]
    fn edf_plot_chart_default() {
        let chart = EdfPlotChart::default();
        assert_eq!(chart.obj_idx, 0);
        assert!(!chart.log_x);
    }
}
