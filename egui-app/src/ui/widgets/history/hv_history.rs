use crate::state::app_state::HvHistory;
use crate::theme::chart_colors::COLOR_HV_LINE;

/// 1 本の HV 推移系列（凡例名 + 色 + データ）。
pub struct HvSeries {
    pub name: String,
    pub color: egui::Color32,
    pub history: HvHistory,
}

/// 参照点指定の変更要求。`render_chart` が app_state へ反映する。
#[derive(Debug, Clone, PartialEq)]
pub enum RefPointChange {
    /// 自動算出（nadir + 10% マージン）に戻す。
    Auto,
    /// 元の目的値の単位・目的ごとの参照点を指定する。
    Manual(Vec<f64>),
}

/// Hypervolume 推移チャートウィジェット
#[derive(Default)]
pub struct HvHistoryChart {
    pub hv_history: Option<HvHistory>,
    pub computing: bool,
    /// 基準 Study の凡例名（比較系列と区別するために表示する）。
    pub base_name: String,
    /// 目的名（参照点ラベルの目的ごとの見出しに使う）。
    pub objective_names: Vec<String>,
    /// 同一グラフに重ね描きする比較 Study の系列。
    pub comparisons: Vec<HvSeries>,
    /// 現在の参照点指定（元の目的値の単位）。`None` で自動算出。
    /// app_state からミラーされ、UI 操作の起点になる。
    pub ref_point_override: Option<Vec<f64>>,
    /// 参照点指定の変更要求（render_chart が `.take()` して app_state へ反映する）。
    pub pending_ref_point: Option<RefPointChange>,
    /// 目的ごとの入力バッファ（Manual 編集中の値を確定まで保持）。
    ref_point_buf: Vec<f64>,
}

impl HvHistoryChart {
    /// グローバル widget（処理済みの正状態）から実行フラグのみを取り込む。
    /// HV データは `app_state.hv_history` に集約され描画時に毎フレーム反映されるため、
    /// キャンバスの各アイテム（独立した WidgetStates）には computing のみ同期すればよい。
    /// これを行わないと計算完了後もアイテム側の computing が下りず spinner が回り続ける。
    pub fn adopt_compute_state(&mut self, src: &Self) {
        self.computing = src.computing;
    }

    /// `history` のサンプリングステップを使って (x=連番×step, y=hv) の点列を作る。
    /// X 軸はサンプリング順の連番 × ステップ (0, step, 2*step, …)。
    /// trial_id は途中試行から始まる場合があり 0 スタートにならないため使わない。
    fn to_points(history: &HvHistory) -> Vec<[f64; 2]> {
        let step = history.sample_step.max(1);
        history
            .hv_values
            .iter()
            .enumerate()
            .map(|(i, &hv)| [(i * step) as f64, hv])
            .collect()
    }

    /// 参照点コントロールを描画する（多目的のときのみ）。
    /// Auto チェックで自動算出に戻し、外すと目的ごとの数値フィールドで入力できる。
    /// 値の確定（フォーカスアウト / ドラッグ終了）時のみ `pending_ref_point` を立てて
    /// 再計算をトリガーし、入力途中の連続再計算を防ぐ。
    fn show_ref_point_controls(&mut self, ui: &mut egui::Ui) {
        let n_obj = self.objective_names.len();
        // HV は多目的のみ意味を持つ。単目的/未読み込みは表示しない。
        if n_obj < 2 {
            return;
        }

        let is_auto = self.ref_point_override.is_none();

        // 編集の起点。Manual なら override、Auto なら直近計算に使った参照点
        // （なければ 0.0）を初期値にする。
        let seed: Vec<f64> = if let Some(r) = &self.ref_point_override {
            let mut v = r.clone();
            v.resize(n_obj, 0.0);
            v
        } else {
            match &self.hv_history {
                Some(h) if h.ref_point.len() == n_obj => h.ref_point.clone(),
                _ => vec![0.0; n_obj],
            }
        };
        if self.ref_point_buf.len() != n_obj {
            self.ref_point_buf = seed.clone();
        }

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Reference point:")
                    .small()
                    .color(crate::theme::TEXT_SECONDARY),
            );

            // Auto トグル
            let mut auto = is_auto;
            if ui.checkbox(&mut auto, "Auto").changed() {
                if auto {
                    self.pending_ref_point = Some(RefPointChange::Auto);
                } else {
                    // Manual へ切替: 現在のシード値を初期指定として確定する。
                    self.ref_point_buf = seed.clone();
                    self.pending_ref_point = Some(RefPointChange::Manual(seed.clone()));
                }
            }

            // 目的ごとの数値フィールド（Manual 時のみ編集可能）
            ui.add_enabled_ui(!is_auto, |ui| {
                let mut commit = false;
                for (j, name) in self.objective_names.iter().enumerate() {
                    ui.label(
                        egui::RichText::new(name)
                            .small()
                            .color(crate::theme::TEXT_SECONDARY),
                    );
                    let resp = ui.add(
                        egui::DragValue::new(&mut self.ref_point_buf[j])
                            .speed(0.1)
                            .max_decimals(6),
                    );
                    if resp.lost_focus() || resp.drag_stopped() {
                        commit = true;
                    }
                }
                if commit && !is_auto {
                    self.pending_ref_point =
                        Some(RefPointChange::Manual(self.ref_point_buf.clone()));
                }
            });
        });
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        // 参照点コントロールは spinner / データ無しでも常に操作できるよう先に描画する。
        self.show_ref_point_controls(ui);

        if self.computing {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Computing hypervolume...");
            });
            return;
        }

        let Some(history) = &self.hv_history else {
            ui.label("No hypervolume data");
            return;
        };

        let step = history.sample_step;
        let base_points = Self::to_points(history);
        let base_label = if self.base_name.is_empty() {
            "Hypervolume".to_string()
        } else {
            self.base_name.clone()
        };

        // 比較系列の点列を事前計算（空履歴はスキップ）。
        let comparison_series: Vec<(&str, egui::Color32, Vec<[f64; 2]>)> = self
            .comparisons
            .iter()
            .filter(|s| !s.history.hv_values.is_empty())
            .map(|s| (s.name.as_str(), s.color, Self::to_points(&s.history)))
            .collect();

        let sampling_label = if step <= 1 {
            "Sampling: Every trial".to_string()
        } else {
            format!("Sampling: Every {} trials", step)
        };
        ui.label(
            egui::RichText::new(sampling_label)
                .small()
                .color(crate::theme::TEXT_SECONDARY),
        );

        egui_plot::Plot::new("hv_history_plot")
            .legend(egui_plot::Legend::default())
            .x_axis_label("Trial")
            .y_axis_label("Hypervolume")
            .include_x(0.0)
            .show(ui, |plot_ui| {
                // 基準 Study
                if !base_points.is_empty() {
                    let color = COLOR_HV_LINE;
                    let line_pts: egui_plot::PlotPoints = base_points.iter().copied().collect();
                    plot_ui.line(
                        egui_plot::Line::new(line_pts)
                            .name(&base_label)
                            .color(color),
                    );
                    let scatter: egui_plot::PlotPoints = base_points.iter().copied().collect();
                    plot_ui.points(
                        egui_plot::Points::new(scatter)
                            .name(&base_label)
                            .color(color)
                            .radius(3.0),
                    );
                }

                // 比較 Study を色分けして重ね描きする
                for (name, color, points) in &comparison_series {
                    let line_pts: egui_plot::PlotPoints = points.iter().copied().collect();
                    plot_ui.line(egui_plot::Line::new(line_pts).name(*name).color(*color));
                    let scatter: egui_plot::PlotPoints = points.iter().copied().collect();
                    plot_ui.points(
                        egui_plot::Points::new(scatter)
                            .name(*name)
                            .color(*color)
                            .radius(3.0),
                    );
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::HvHistory;

    #[test]
    fn hv_history_chart_default() {
        let chart = HvHistoryChart::default();
        assert!(chart.hv_history.is_none());
        assert!(!chart.computing);
        // 既定は Auto（override なし）・変更要求なし。
        assert!(chart.ref_point_override.is_none());
        assert!(chart.pending_ref_point.is_none());
    }

    #[test]
    fn pending_ref_point_encodes_auto_and_manual() {
        // 変更要求は Auto / Manual(値) の 2 値で表す。
        let to_auto = Some(RefPointChange::Auto);
        let to_manual = Some(RefPointChange::Manual(vec![1.0, 2.0]));
        assert!(matches!(to_auto, Some(RefPointChange::Auto)));
        assert!(matches!(to_manual, Some(RefPointChange::Manual(ref v)) if v == &[1.0, 2.0]));
    }

    #[test]
    fn adopt_compute_state_clears_stuck_computing() {
        // 計算完了後にグローバル側の computing=false を取り込むと、
        // spinner で固まっていたアイテム側の computing が下りる。
        let mut item = HvHistoryChart {
            computing: true,
            ..Default::default()
        };
        let global = HvHistoryChart::default(); // computing=false
        item.adopt_compute_state(&global);
        assert!(!item.computing);
    }

    #[test]
    fn hv_history_show_uses_index_times_step() {
        let history = HvHistory {
            trial_ids: vec![10000, 10050, 10100],
            hv_values: vec![0.1, 0.5, 0.8],
            sample_step: 50,
            ref_point: vec![],
        };
        // x values should be 0, 50, 100 — not 10000, 10050, 10100
        let step = history.sample_step;
        let points: Vec<[f64; 2]> = history
            .hv_values
            .iter()
            .enumerate()
            .map(|(i, &hv)| [(i * step) as f64, hv])
            .collect();
        assert_eq!(points[0][0], 0.0);
        assert_eq!(points[1][0], 50.0);
        assert_eq!(points[2][0], 100.0);
    }
}
