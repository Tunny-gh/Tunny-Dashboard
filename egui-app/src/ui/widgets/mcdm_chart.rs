use crate::state::app_state::TrialRow;
use crate::state::results::{McdmMethod, McdmResult};

/// MCDM compute request payload
pub struct McdmComputeRequest {
    pub method: McdmMethod,
    pub weights: Vec<f64>,
    pub v: f64,
}

/// 上位N件表示切替
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McdmTopN {
    Top5,
    Top10,
    Top20,
}

impl McdmTopN {
    pub fn value(&self) -> usize {
        match self {
            McdmTopN::Top5 => 5,
            McdmTopN::Top10 => 10,
            McdmTopN::Top20 => 20,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            McdmTopN::Top5 => "Top 5",
            McdmTopN::Top10 => "Top 10",
            McdmTopN::Top20 => "Top 20",
        }
    }

    fn show_combo(&mut self, ui: &mut egui::Ui, id: &str) {
        egui::ComboBox::from_id_salt(id)
            .selected_text(self.label())
            .show_ui(ui, |ui| {
                ui.selectable_value(self, McdmTopN::Top5, McdmTopN::Top5.label());
                ui.selectable_value(self, McdmTopN::Top10, McdmTopN::Top10.label());
                ui.selectable_value(self, McdmTopN::Top20, McdmTopN::Top20.label());
            });
    }
}

/// MCDMランキングバーチャートのUI状態
pub struct McdmRankChart {
    pub method: McdmMethod,
    pub weights: Vec<f64>,
    pub v_param: f64,
    pub computing: bool,
    pub pending_compute: Option<McdmComputeRequest>,
    pub top_n: McdmTopN,
}

impl Default for McdmRankChart {
    fn default() -> Self {
        Self {
            method: McdmMethod::Topsis,
            weights: Vec::new(),
            v_param: 0.5,
            computing: false,
            pending_compute: None,
            top_n: McdmTopN::Top10,
        }
    }
}

/// MCDMランキングテーブルのUI状態
pub struct McdmTable {
    pub top_n: McdmTopN,
}

impl Default for McdmTable {
    fn default() -> Self {
        Self {
            top_n: McdmTopN::Top10,
        }
    }
}

pub fn normalize_weights(weights: &[f64]) -> Vec<f64> {
    if weights.is_empty() {
        return vec![];
    }
    let sum: f64 = weights.iter().sum();
    if sum == 0.0 {
        let n = weights.len() as f64;
        vec![1.0 / n; weights.len()]
    } else {
        weights.iter().map(|&w| w / sum).collect()
    }
}

impl McdmRankChart {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        obj_names: &[String],
        result: &Option<McdmResult>,
        trial_rows: &[TrialRow],
    ) {
        let obj_count = obj_names.len();
        if obj_count == 0 {
            ui.vertical_centered(|ui| {
                ui.colored_label(egui::Color32::GRAY, "Select a study first");
            });
            return;
        }

        if self.weights.len() != obj_count {
            self.weights = vec![1.0; obj_count];
        }

        // 手法セレクタ + Top N + Runボタン + spinner
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("mcdm_method_combo")
                .selected_text(self.method.label())
                .show_ui(ui, |ui| {
                    for m in McdmMethod::all() {
                        ui.selectable_value(&mut self.method, *m, m.label());
                    }
                });

            self.top_n.show_combo(ui, "mcdm_top_n_combo");

            if ui
                .add_enabled(!self.computing, egui::Button::new("Run"))
                .clicked()
            {
                let normalized = normalize_weights(&self.weights);
                self.pending_compute = Some(McdmComputeRequest {
                    method: self.method,
                    weights: normalized,
                    v: self.v_param,
                });
                self.computing = true;
            }

            if self.computing {
                ui.spinner();
                ui.label("Computing...");
            }
        });

        ui.separator();

        // 重みスライダー
        ui.collapsing("Weights", |ui| {
            let normalized = normalize_weights(&self.weights);
            for (i, obj_name) in obj_names.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(obj_name.as_str()).strong());
                    let mut w = self.weights[i];
                    if ui
                        .add(egui::Slider::new(&mut w, 0.0..=1.0).text("weight"))
                        .changed()
                    {
                        self.weights[i] = w;
                    }
                    ui.label(format!("(norm: {:.2})", normalized[i]));
                });
            }
            let norm_sum: f64 = normalized.iter().sum();
            ui.label(format!("Sum: {:.2}", norm_sum));

            if self.method == McdmMethod::Vikor {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Strategy weight v").strong());
                    ui.add(egui::Slider::new(&mut self.v_param, 0.0..=1.0).text("v"));
                    ui.label("(0=min-regret, 1=max-consensus)");
                });
            }
        });

        ui.separator();

        if self.computing {
            return;
        }

        let Some(result) = result else {
            ui.vertical_centered(|ui| {
                ui.colored_label(egui::Color32::GRAY, "Press Run to compute MCDM ranking");
            });
            return;
        };

        ui.label(format!(
            "Computed in {:.1}ms ({})",
            result.duration_ms(),
            result.method_label()
        ));

        // ランキングバーチャート（ImportanceChartと同じカスタム描画）
        let entries = enumerate_ranked(result, trial_rows, self.top_n.value());
        if entries.is_empty() {
            ui.label("No data");
            return;
        }

        let max_score = entries.iter().map(|e| e.score).fold(0.0_f64, f64::max);
        let label_width = 100.0_f32;
        let bar_height = 20.0_f32;
        let bar_gap = 4.0_f32;
        let value_text_width = 60.0_f32;
        let bar_color = egui::Color32::from_rgb(0x0c, 0x6a, 0xc0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            let available_width = ui.available_width() - label_width - value_text_width - 8.0;
            let bar_max_width = available_width.max(50.0);

            for entry in &entries {
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [label_width, bar_height],
                        egui::Label::new(
                            egui::RichText::new(format!("Trial {}", entry.trial_id))
                                .text_style(egui::TextStyle::Body),
                        )
                        .truncate(),
                    );

                    let bar_width = if max_score > 0.0 {
                        (entry.score / max_score * bar_max_width as f64) as f32
                    } else {
                        0.0
                    };

                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(bar_max_width, bar_height - bar_gap),
                        egui::Sense::hover(),
                    );
                    if ui.is_rect_visible(rect) {
                        let bar_rect = egui::Rect::from_min_size(
                            rect.min,
                            egui::vec2(bar_width, rect.height()),
                        );
                        ui.painter().rect_filled(bar_rect, 2.0, bar_color);
                    }

                    ui.label(format!("{:.4}", entry.score));
                });
            }
        });
    }
}

impl McdmTable {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        result: &Option<McdmResult>,
        trial_rows: &[TrialRow],
        obj_names: &[String],
    ) {
        // Top N セレクタ
        ui.horizontal(|ui| {
            ui.label("Show:");
            self.top_n.show_combo(ui, "mcdm_table_top_n_combo");
        });

        ui.separator();

        let Some(result) = result else {
            ui.vertical_centered(|ui| {
                ui.colored_label(
                    egui::Color32::GRAY,
                    "No MCDM result — run computation from MCDM Ranking first",
                );
            });
            return;
        };

        use egui_extras::{Column, TableBuilder};

        let rows = build_ranking_rows(result, trial_rows, self.top_n.value());
        if rows.is_empty() {
            ui.colored_label(egui::Color32::GRAY, "No results to display");
            return;
        }

        TableBuilder::new(ui)
            .striped(true)
            .column(Column::exact(50.0))
            .column(Column::exact(80.0))
            .column(Column::exact(80.0))
            .columns(Column::remainder(), obj_names.len())
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("Rank");
                });
                header.col(|ui| {
                    ui.strong("Trial");
                });
                header.col(|ui| {
                    ui.strong("Score");
                });
                for name in obj_names {
                    header.col(|ui| {
                        ui.strong(name);
                    });
                }
            })
            .body(|mut body| {
                for row_data in &rows {
                    body.row(18.0, |mut row| {
                        row.col(|ui| {
                            ui.label(format!("{}", row_data.rank));
                        });
                        row.col(|ui| {
                            ui.label(format!("{}", row_data.trial_id));
                        });
                        row.col(|ui| {
                            ui.label(format!("{:.4}", row_data.score));
                        });
                        for &val in &row_data.objectives {
                            row.col(|ui| {
                                ui.label(format!("{:.4}", val));
                            });
                        }
                    });
                }
            });
    }
}

/// ランキング上位N件の共通抽出データ
struct RankingEntry {
    rank: usize,
    trial_idx: usize,
    trial_id: u32,
    score: f64,
}

/// McdmResultから上位N件のランキングエントリを生成する
fn enumerate_ranked(
    result: &McdmResult,
    trial_rows: &[TrialRow],
    top_n: usize,
) -> Vec<RankingEntry> {
    let scores = result.primary_scores();
    let ranked = result.ranked_indices();
    let count = top_n.min(ranked.len());

    (0..count)
        .map(|rank| {
            let trial_idx = ranked[rank] as usize;
            let trial_id = trial_rows.get(trial_idx).map(|r| r.trial_id).unwrap_or(0);
            let score = scores.get(trial_idx).copied().unwrap_or(0.0);
            RankingEntry {
                rank: rank + 1,
                trial_idx,
                trial_id,
                score,
            }
        })
        .collect()
}

/// テーブル行データ
pub struct RankingRow {
    pub rank: usize,
    pub trial_id: u32,
    pub score: f64,
    pub objectives: Vec<f64>,
}

/// McdmResultから上位N件のテーブル行データを生成する
pub fn build_ranking_rows(
    result: &McdmResult,
    trial_rows: &[TrialRow],
    top_n: usize,
) -> Vec<RankingRow> {
    enumerate_ranked(result, trial_rows, top_n)
        .into_iter()
        .map(|e| {
            let trial = &trial_rows[e.trial_idx];
            RankingRow {
                rank: e.rank,
                trial_id: e.trial_id,
                score: e.score,
                objectives: trial.objectives.clone(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::results::TopsisResult;

    fn make_trial_rows(n: usize) -> Vec<TrialRow> {
        (0..n)
            .map(|i| TrialRow {
                trial_id: i as u32,
                ..Default::default()
            })
            .collect()
    }

    fn make_topsis_result(scores: Vec<f64>, ranked_indices: Vec<u32>) -> McdmResult {
        let n = scores.len();
        McdmResult::Topsis(TopsisResult {
            scores,
            ranked_indices,
            positive_ideal: vec![1.0; n],
            negative_ideal: vec![0.0; n],
            duration_ms: 10.0,
        })
    }

    #[test]
    fn mcdm_top_n_values() {
        assert_eq!(McdmTopN::Top5.value(), 5);
        assert_eq!(McdmTopN::Top10.value(), 10);
        assert_eq!(McdmTopN::Top20.value(), 20);
    }

    #[test]
    fn normalize_weights_equal() {
        let result = normalize_weights(&[0.5, 0.5]);
        assert!((result[0] - 0.5).abs() < 1e-9);
        assert!((result[1] - 0.5).abs() < 1e-9);
    }

    #[test]
    fn normalize_weights_unequal() {
        let result = normalize_weights(&[1.0, 3.0]);
        assert!((result[0] - 0.25).abs() < 1e-9);
        assert!((result[1] - 0.75).abs() < 1e-9);
    }

    #[test]
    fn normalize_weights_three_equal() {
        let result = normalize_weights(&[2.0, 2.0, 2.0]);
        for w in &result {
            assert!((w - 1.0 / 3.0).abs() < 1e-9);
        }
    }

    #[test]
    fn normalize_weights_zero_fallback() {
        let result = normalize_weights(&[0.0, 0.0]);
        assert!((result[0] - 0.5).abs() < 1e-9);
        assert!((result[1] - 0.5).abs() < 1e-9);
    }

    #[test]
    fn normalize_weights_empty() {
        let result = normalize_weights(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn mcdm_rank_chart_default() {
        let chart = McdmRankChart::default();
        assert_eq!(chart.method, McdmMethod::Topsis);
        assert!(!chart.computing);
        assert!(chart.pending_compute.is_none());
        assert_eq!(chart.top_n, McdmTopN::Top10);
        assert!(chart.weights.is_empty());
        assert!((chart.v_param - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn mcdm_table_default() {
        let table = McdmTable::default();
        assert_eq!(table.top_n, McdmTopN::Top10);
    }

    #[test]
    fn enumerate_ranked_top5_with_5_results() {
        let result = make_topsis_result(vec![0.9, 0.7, 0.5, 0.3, 0.1], vec![0, 1, 2, 3, 4]);
        let rows = make_trial_rows(5);
        let ranking = build_ranking_rows(&result, &rows, 5);
        assert_eq!(ranking.len(), 5);
        assert!((ranking[0].score - 0.9).abs() < 1e-9);
        assert!((ranking[4].score - 0.1).abs() < 1e-9);
    }

    #[test]
    fn enumerate_ranked_top10_with_20_results() {
        let scores: Vec<f64> = (0..20).map(|i| 1.0 - i as f64 / 20.0).collect();
        let ranked: Vec<u32> = (0..20).collect();
        let result = make_topsis_result(scores, ranked);
        let rows = make_trial_rows(20);
        let ranking = build_ranking_rows(&result, &rows, 10);
        assert_eq!(ranking.len(), 10);
    }

    #[test]
    fn enumerate_ranked_top5_with_3_results_min_applied() {
        let result = make_topsis_result(vec![0.9, 0.5, 0.1], vec![0, 1, 2]);
        let rows = make_trial_rows(3);
        let ranking = build_ranking_rows(&result, &rows, 5);
        assert_eq!(ranking.len(), 3);
    }

    #[test]
    fn enumerate_ranked_scores_match_ranked_order() {
        let result = make_topsis_result(vec![0.1, 0.9, 0.5], vec![1, 2, 0]);
        let rows = make_trial_rows(3);
        let ranking = build_ranking_rows(&result, &rows, 10);
        assert_eq!(ranking.len(), 3);
        assert!((ranking[0].score - 0.9).abs() < 1e-9);
        assert!((ranking[1].score - 0.5).abs() < 1e-9);
        assert!((ranking[2].score - 0.1).abs() < 1e-9);
    }

    #[test]
    fn enumerate_ranked_empty_result() {
        let result = make_topsis_result(vec![], vec![]);
        let ranking = build_ranking_rows(&result, &[], 5);
        assert!(ranking.is_empty());
    }

    #[test]
    fn top_n_toggle_cycle() {
        let mut chart = McdmRankChart::default();
        assert_eq!(chart.top_n, McdmTopN::Top10);
        chart.top_n = McdmTopN::Top5;
        assert_eq!(chart.top_n.value(), 5);
        chart.top_n = McdmTopN::Top20;
        assert_eq!(chart.top_n.value(), 20);
    }

    #[test]
    fn build_ranking_rows_basic() {
        let result = make_topsis_result(vec![0.9, 0.5, 0.1], vec![0, 1, 2]);
        let rows = make_trial_rows(3);
        let ranking = build_ranking_rows(&result, &rows, 5);
        assert_eq!(ranking.len(), 3);
        assert_eq!(ranking[0].rank, 1);
        assert_eq!(ranking[0].trial_id, 0);
        assert!((ranking[0].score - 0.9).abs() < 1e-9);
    }

    #[test]
    fn build_ranking_rows_top_n_limit() {
        let scores: Vec<f64> = (0..20).map(|i| 1.0 - i as f64 / 20.0).collect();
        let ranked: Vec<u32> = (0..20).collect();
        let result = make_topsis_result(scores, ranked);
        let rows = make_trial_rows(20);
        let ranking = build_ranking_rows(&result, &rows, 5);
        assert_eq!(ranking.len(), 5);
    }

    #[test]
    fn build_ranking_rows_rank_starts_at_1() {
        let result = make_topsis_result(vec![0.8], vec![0]);
        let rows = make_trial_rows(1);
        let ranking = build_ranking_rows(&result, &rows, 5);
        assert_eq!(ranking[0].rank, 1);
    }

    #[test]
    fn build_ranking_rows_empty() {
        let result = make_topsis_result(vec![], vec![]);
        let ranking = build_ranking_rows(&result, &[], 5);
        assert!(ranking.is_empty());
    }

    #[test]
    fn build_ranking_rows_objectives_included() {
        let result = make_topsis_result(vec![0.9, 0.5], vec![0, 1]);
        let rows = vec![
            TrialRow {
                trial_id: 0,
                objectives: vec![1.0, 2.0],
                ..Default::default()
            },
            TrialRow {
                trial_id: 1,
                objectives: vec![3.0, 4.0],
                ..Default::default()
            },
        ];
        let ranking = build_ranking_rows(&result, &rows, 10);
        assert_eq!(ranking[0].objectives, vec![1.0, 2.0]);
        assert_eq!(ranking[1].objectives, vec![3.0, 4.0]);
    }

    // ── E2E / integration tests ──

    fn make_multi_obj_trial_rows() -> Vec<TrialRow> {
        vec![
            TrialRow {
                trial_id: 0,
                objectives: vec![0.1, 0.9],
                ..Default::default()
            },
            TrialRow {
                trial_id: 1,
                objectives: vec![0.5, 0.5],
                ..Default::default()
            },
            TrialRow {
                trial_id: 2,
                objectives: vec![0.9, 0.1],
                ..Default::default()
            },
            TrialRow {
                trial_id: 3,
                objectives: vec![0.3, 0.7],
                ..Default::default()
            },
            TrialRow {
                trial_id: 4,
                objectives: vec![0.7, 0.3],
                ..Default::default()
            },
        ]
    }

    #[test]
    fn topsis_full_pipeline_equal_weights() {
        let rows = make_multi_obj_trial_rows();
        let objectives: Vec<f64> = rows
            .iter()
            .flat_map(|r| r.objectives.iter().copied())
            .collect();
        let weights = normalize_weights(&[1.0, 1.0]);
        let is_minimize = vec![true, true];

        let core_result =
            tunny_core::topsis::compute_topsis(&objectives, 5, 2, &weights, &is_minimize).unwrap();

        let mcdm_result = McdmResult::Topsis(TopsisResult {
            scores: core_result.scores.clone(),
            ranked_indices: core_result.ranked_indices.clone(),
            positive_ideal: core_result.positive_ideal.clone(),
            negative_ideal: core_result.negative_ideal.clone(),
            duration_ms: core_result.duration_ms,
        });

        assert_eq!(mcdm_result.primary_scores().len(), 5);
        assert!(!mcdm_result.primary_scores().iter().any(|s| s.is_nan()));

        let ranking = build_ranking_rows(&mcdm_result, &rows, 5);
        assert_eq!(ranking.len(), 5);
        assert_eq!(ranking[0].rank, 1);
        for i in 1..ranking.len() {
            assert!(ranking[i - 1].score >= ranking[i].score);
        }
    }

    #[test]
    fn topsis_weight_bias_changes_ranking() {
        let rows = make_multi_obj_trial_rows();
        let objectives: Vec<f64> = rows
            .iter()
            .flat_map(|r| r.objectives.iter().copied())
            .collect();
        let is_minimize = vec![true, true];

        let weights_obj0 = normalize_weights(&[1.0, 0.0]);
        let r0 = tunny_core::topsis::compute_topsis(&objectives, 5, 2, &weights_obj0, &is_minimize)
            .unwrap();

        let weights_obj1 = normalize_weights(&[0.0, 1.0]);
        let r1 = tunny_core::topsis::compute_topsis(&objectives, 5, 2, &weights_obj1, &is_minimize)
            .unwrap();

        assert_ne!(
            r0.ranked_indices, r1.ranked_indices,
            "different weights should produce different rankings"
        );
    }

    #[test]
    fn topsis_single_objective_works() {
        let rows: Vec<TrialRow> = (0..5)
            .map(|i| TrialRow {
                trial_id: i,
                objectives: vec![i as f64 * 0.2],
                ..Default::default()
            })
            .collect();
        let objectives: Vec<f64> = rows
            .iter()
            .flat_map(|r| r.objectives.iter().copied())
            .collect();
        let weights = normalize_weights(&[1.0]);
        let is_minimize = vec![true];

        let result = tunny_core::topsis::compute_topsis(&objectives, 5, 1, &weights, &is_minimize);
        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.scores.len(), 5);
    }

    #[test]
    fn mcdm_chart_run_button_sets_pending_compute() {
        let mut chart = McdmRankChart::default();
        assert!(chart.pending_compute.is_none());
        assert!(!chart.computing);

        let normalized = normalize_weights(&[1.0, 1.0]);
        chart.pending_compute = Some(McdmComputeRequest {
            method: McdmMethod::Topsis,
            weights: normalized,
            v: 0.5,
        });
        chart.computing = true;

        assert!(chart.pending_compute.is_some());
        assert!(chart.computing);

        let payload = chart.pending_compute.take();
        assert!(payload.is_some());
        assert!(chart.pending_compute.is_none());
        assert!(chart.computing);
    }

    #[test]
    fn mcdm_compute_request_vikor_includes_v() {
        let req = McdmComputeRequest {
            method: McdmMethod::Vikor,
            weights: vec![0.5, 0.5],
            v: 0.3,
        };
        assert_eq!(req.method, McdmMethod::Vikor);
        assert!((req.v - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn mcdm_score_color_mode_integration() {
        let rows = make_multi_obj_trial_rows();
        let objectives: Vec<f64> = rows
            .iter()
            .flat_map(|r| r.objectives.iter().copied())
            .collect();
        let weights = normalize_weights(&[1.0, 1.0]);
        let is_minimize = vec![true, true];

        let core_result =
            tunny_core::topsis::compute_topsis(&objectives, 5, 2, &weights, &is_minimize).unwrap();

        let scores = core_result.scores.clone();
        let colors = crate::render::colormap::compute_chart_colors(
            &crate::state::types::ColorMode::McdmScore,
            &crate::state::app_state::ColormapName::Viridis,
            &rows,
            &[],
            Some(&scores),
        );

        assert_eq!(colors.len(), 5);
        let score_min = scores.iter().cloned().fold(f64::INFINITY, f64::min);
        let score_max = scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        if (score_max - score_min).abs() > f64::EPSILON {
            let max_score_idx = scores
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(i, _)| i)
                .unwrap();
            let min_score_idx = scores
                .iter()
                .enumerate()
                .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(i, _)| i)
                .unwrap();
            assert_ne!(colors[max_score_idx], colors[min_score_idx]);
        }
        assert!(colors.iter().any(|c| *c != egui::Color32::LIGHT_GRAY));
    }

    #[test]
    fn top_n_toggle_updates_display() {
        let rows = make_multi_obj_trial_rows();
        let objectives: Vec<f64> = rows
            .iter()
            .flat_map(|r| r.objectives.iter().copied())
            .collect();
        let weights = normalize_weights(&[1.0, 1.0]);
        let is_minimize = vec![true, true];

        let core_result =
            tunny_core::topsis::compute_topsis(&objectives, 5, 2, &weights, &is_minimize).unwrap();
        let mcdm = McdmResult::Topsis(TopsisResult {
            scores: core_result.scores,
            ranked_indices: core_result.ranked_indices,
            positive_ideal: core_result.positive_ideal,
            negative_ideal: core_result.negative_ideal,
            duration_ms: core_result.duration_ms,
        });

        let rows5 = build_ranking_rows(&mcdm, &rows, 5);
        assert_eq!(rows5.len(), 5);

        let rows3 = build_ranking_rows(&mcdm, &rows, 3);
        assert_eq!(rows3.len(), 3);

        let rows10 = build_ranking_rows(&mcdm, &rows, 10);
        assert_eq!(rows10.len(), 5);
    }
}
