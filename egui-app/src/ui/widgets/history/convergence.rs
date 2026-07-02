use std::collections::HashMap;

use crate::io::artifacts::ArtifactEntry;
use crate::state::app_state::ConvergenceHistory;
use crate::state::types::StudyView;
use crate::theme::chart_colors::COLOR_CONVERGENCE_LINE;
use crate::ui::widgets::trial_detail_modal::{
    hit_test_nearest, TrialDetailModal, TrialDetailTarget, HIT_THRESHOLD,
};
use tunny_core::indicators::MoIndicator;

/// 1 本の指標推移系列（凡例名 + 色 + データ）。
pub struct ConvergenceSeries {
    pub name: String,
    pub color: egui::Color32,
    pub history: ConvergenceHistory,
}

/// 参照点指定の変更要求。`render_chart` が app_state へ反映する。
#[derive(Debug, Clone, PartialEq)]
pub enum RefPointChange {
    /// 自動算出（nadir + 10% マージン）に戻す。
    Auto,
    /// 元の目的値の単位・目的ごとの参照点を指定する。
    Manual(Vec<f64>),
}

/// 多目的収束指標チャートウィジェット（HV / IGD+ / ε-indicator / R2）
pub struct ConvergenceChart {
    pub history: Option<ConvergenceHistory>,
    pub computing: bool,
    /// 基準 Study の凡例名（比較系列と区別するために表示する）。
    pub base_name: String,
    /// 目的名（参照点ラベルの目的ごとの見出しに使う）。
    pub objective_names: Vec<String>,
    /// 同一グラフに重ね描きする比較 Study の系列。
    pub comparisons: Vec<ConvergenceSeries>,
    /// 現在の参照点指定（元の目的値の単位）。`None` で自動算出。
    /// app_state からミラーされ、UI 操作の起点になる。
    pub ref_point_override: Option<Vec<f64>>,
    /// 参照点指定の変更要求（render_chart が `.take()` して app_state へ反映する）。
    pub pending_ref_point: Option<RefPointChange>,
    /// 現在表示中の収束指標（render_chart が毎フレーム app_state からセットする）。
    pub indicator: MoIndicator,
    /// 指標変更要求（render_chart が `.take()` して app_state へ反映する）。
    pub pending_indicator: Option<MoIndicator>,
    /// 目的ごとの入力バッファ（Manual 編集中の値を確定まで保持）。
    ref_point_buf: Vec<f64>,
    /// 点クリックで開くトライアル詳細モーダル（散布図と共有）。
    detail_modal: TrialDetailModal,
}

impl Default for ConvergenceChart {
    fn default() -> Self {
        Self {
            history: None,
            computing: false,
            base_name: String::new(),
            objective_names: Vec::new(),
            comparisons: Vec::new(),
            ref_point_override: None,
            pending_ref_point: None,
            indicator: MoIndicator::Hypervolume,
            pending_indicator: None,
            ref_point_buf: Vec::new(),
            detail_modal: TrialDetailModal::new(),
        }
    }
}

impl ConvergenceChart {
    /// グローバル widget（処理済みの正状態）から実行フラグのみを取り込む。
    /// 指標データは `app_state.convergence_history` に集約され描画時に毎フレーム反映されるため、
    /// キャンバスの各アイテム（独立した WidgetStates）には computing のみ同期すればよい。
    /// これを行わないと計算完了後もアイテム側の computing が下りず spinner が回り続ける。
    pub fn adopt_compute_state(&mut self, src: &Self) {
        self.computing = src.computing;
    }

    /// `history` のサンプリングステップを使って (x=連番×step, y=値) の点列を作る。
    /// X 軸はサンプリング順の連番 × ステップ (0, step, 2*step, …)。
    /// trial_id は途中試行から始まる場合があり 0 スタートにならないため使わない。
    fn to_points(history: &ConvergenceHistory) -> Vec<[f64; 2]> {
        let step = history.sample_step.max(1);
        history
            .values
            .iter()
            .enumerate()
            .map(|(i, &v)| [(i * step) as f64, v])
            .collect()
    }

    /// 参照点コントロールを描画する（多目的 + HV 選択時のみ）。
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
            match &self.history {
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

    /// 収束指標チャートを描画する。
    ///
    /// `view` / `param_names` / `artifact_map` は基準 Study の点をクリックしたときに
    /// 開くトライアル詳細モーダル用。比較 Study の点は基準 Study の `view` に対応行が
    /// ないためクリック対象にしない。
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        param_names: &[String],
        obj_names: &[String],
        artifact_map: &HashMap<u32, Vec<ArtifactEntry>>,
    ) {
        // 単目的（または目的数未確定）の場合は収束指標を描画しない。
        if self.objective_names.len() < 2 {
            ui.label("Convergence indicators are defined only for multi-objective studies (≥2 objectives).");
            return;
        }

        // 指標セレクタと補足情報（方向・サンプリング間隔）を 1 行に並べる。
        // コンボボックス右の余白を活用し、縦方向のスペースを節約する。
        let mut new_indicator = self.indicator;
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("convergence_indicator")
                .selected_text(self.indicator.label())
                .show_ui(ui, |ui| {
                    for ind in MoIndicator::all() {
                        ui.selectable_value(&mut new_indicator, ind, ind.label());
                    }
                });

            // 方向（大小どちらが良いか）
            let direction_text = if self.indicator.higher_is_better() {
                "Higher is better"
            } else {
                "Lower is better"
            };
            ui.label(
                egui::RichText::new(direction_text)
                    .small()
                    .color(crate::theme::TEXT_SECONDARY),
            );

            // サンプリング間隔（データがあるときのみ）
            if !self.computing {
                if let Some(history) = &self.history {
                    let step = history.sample_step;
                    let sampling_label = if step <= 1 {
                        "Sampling: Every trial".to_string()
                    } else {
                        format!("Sampling: Every {step} trials")
                    };
                    ui.separator();
                    ui.label(
                        egui::RichText::new(sampling_label)
                            .small()
                            .color(crate::theme::TEXT_SECONDARY),
                    );
                }
            }
        });
        if new_indicator != self.indicator {
            self.pending_indicator = Some(new_indicator);
        }

        // 参照点コントロールは HV 選択時のみ表示する。
        if self.indicator == MoIndicator::Hypervolume {
            self.show_ref_point_controls(ui);
        }

        if self.computing {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(format!("Computing {}...", self.indicator.label()));
            });
            return;
        }

        let Some(history) = &self.history else {
            ui.label(format!("No {} data", self.indicator.label()));
            return;
        };

        let base_points = Self::to_points(history);
        let base_label = if self.base_name.is_empty() {
            self.indicator.label().to_string()
        } else {
            self.base_name.clone()
        };

        // クリック判定用に基準 Study の点を (trial_id, 行 index, [x, y]) で構築する。
        // 描画点と座標を一致させるため `base_points` をそのまま流用し、trial_id から
        // `view` 上の行を解決する（解決できない点はクリック対象外）。
        let base_hit_points: Vec<(u32, usize, [f64; 2])> = base_points
            .iter()
            .enumerate()
            .filter_map(|(i, &pt)| {
                let tid = *history.trial_ids.get(i)?;
                let row = view.trial_ids.iter().position(|&t| t == tid)?;
                Some((tid, row, pt))
            })
            .collect();

        // 比較系列の点列を事前計算（空履歴はスキップ）。
        let comparison_series: Vec<(&str, egui::Color32, Vec<[f64; 2]>)> = self
            .comparisons
            .iter()
            .filter(|s| !s.history.values.is_empty())
            .map(|s| (s.name.as_str(), s.color, Self::to_points(&s.history)))
            .collect();

        // クリックされた基準 Study の点（trial_id, 行 index, 指標値）。
        let mut clicked_detail: Option<(u32, usize, f64)> = None;

        egui_plot::Plot::new("convergence_plot")
            .legend(egui_plot::Legend::default())
            .x_axis_label("Trial")
            .y_axis_label(self.indicator.label())
            .include_x(0.0)
            .show(ui, |plot_ui| {
                // 点クリックでトライアル詳細モーダルを開く（基準 Study の点のみ）。
                let resp = plot_ui.response();
                if resp.clicked_by(egui::PointerButton::Primary) {
                    if let Some(pos) = resp.interact_pointer_pos() {
                        if let Some((tid, row)) =
                            hit_test_nearest(plot_ui, &base_hit_points, pos, HIT_THRESHOLD)
                        {
                            let value = base_hit_points
                                .iter()
                                .find(|(t, _, _)| *t == tid)
                                .map(|(_, _, [_, y])| *y)
                                .unwrap_or(f64::NAN);
                            clicked_detail = Some((tid, row, value));
                        }
                    }
                }

                // 基準 Study
                if !base_points.is_empty() {
                    let color = COLOR_CONVERGENCE_LINE;
                    let line_pts: egui_plot::PlotPoints = base_points.iter().copied().collect();
                    plot_ui.line(egui_plot::Line::new(&base_label, line_pts).color(color));
                    let scatter: egui_plot::PlotPoints = base_points.iter().copied().collect();
                    plot_ui.points(
                        egui_plot::Points::new(&base_label, scatter)
                            .color(color)
                            .radius(3.0),
                    );
                }

                // 比較 Study を色分けして重ね描きする
                for (name, color, points) in &comparison_series {
                    let line_pts: egui_plot::PlotPoints = points.iter().copied().collect();
                    plot_ui.line(egui_plot::Line::new(*name, line_pts).color(*color));
                    let scatter: egui_plot::PlotPoints = points.iter().copied().collect();
                    plot_ui.points(
                        egui_plot::Points::new(*name, scatter)
                            .color(*color)
                            .radius(3.0),
                    );
                }
            });

        // クリックされた点があれば、指標名と値を付加情報としてモーダルを開く。
        if let Some((trial_id, row, value)) = clicked_detail {
            let context = vec![(self.indicator.label().to_string(), format!("{value:.6}"))];
            self.detail_modal.open(TrialDetailTarget {
                trial_id,
                row_index: row,
                context,
            });
        }

        // 詳細モーダルを描画する（散布図と同じ共有実装）。
        if self.detail_modal.is_open() {
            self.detail_modal
                .show(ui, view, param_names, obj_names, artifact_map);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::ConvergenceHistory;

    #[test]
    fn convergence_chart_default() {
        let chart = ConvergenceChart::default();
        assert!(chart.history.is_none());
        assert!(!chart.computing);
        // 既定は Auto（override なし）・変更要求なし。
        assert!(chart.ref_point_override.is_none());
        assert!(chart.pending_ref_point.is_none());
        // 既定の指標は Hypervolume。
        assert_eq!(chart.indicator, MoIndicator::Hypervolume);
        assert!(chart.pending_indicator.is_none());
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
        let mut item = ConvergenceChart {
            computing: true,
            ..Default::default()
        };
        let global = ConvergenceChart::default(); // computing=false
        item.adopt_compute_state(&global);
        assert!(!item.computing);
    }

    #[test]
    fn convergence_show_uses_index_times_step() {
        let history = ConvergenceHistory {
            trial_ids: vec![10000, 10050, 10100],
            values: vec![0.1, 0.5, 0.8],
            sample_step: 50,
            ref_point: vec![],
        };
        // x values should be 0, 50, 100 — not 10000, 10050, 10100
        let step = history.sample_step;
        let points: Vec<[f64; 2]> = history
            .values
            .iter()
            .enumerate()
            .map(|(i, &v)| [(i * step) as f64, v])
            .collect();
        assert_eq!(points[0][0], 0.0);
        assert_eq!(points[1][0], 50.0);
        assert_eq!(points[2][0], 100.0);
    }

    #[test]
    fn indicator_variants_accessible() {
        // 全 4 指標が列挙可能であることを確認する。
        let all = MoIndicator::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&MoIndicator::Hypervolume));
        assert!(all.contains(&MoIndicator::IgdPlus));
        assert!(all.contains(&MoIndicator::Epsilon));
        assert!(all.contains(&MoIndicator::R2));
    }

    #[test]
    fn indicator_higher_is_better_only_for_hv() {
        assert!(MoIndicator::Hypervolume.higher_is_better());
        assert!(!MoIndicator::IgdPlus.higher_is_better());
        assert!(!MoIndicator::Epsilon.higher_is_better());
        assert!(!MoIndicator::R2.higher_is_better());
    }
}
