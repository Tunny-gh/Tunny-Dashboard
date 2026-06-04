use std::collections::BTreeMap;

use crate::state::app_state::AppState;
use crate::state::results::ClusterResult;
use crate::theme::chart_colors::COLOR_LINK;
use crate::theme::colormap::ColorMap;
use crate::theme::ERROR_COLOR;

/// クラスタ割当テーブルウィジェット。
/// クラスタリング結果（各トライアルがどのクラスタに属するか）を一覧表示する。
/// 行クリックでハイライト、📌 でピン留めが可能（TrialTable と同じ操作感）。
#[derive(Default)]
pub struct ClusterTable {
    /// クラスタリング対象外（パレートフロント以外）の解も表示するか
    pub show_unclustered: bool,
}

impl ClusterTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// テーブルを描画する
    pub fn show(&mut self, ui: &mut egui::Ui, app_state: &mut AppState, colormap: &ColorMap) {
        let Some(study_ctx) = app_state.current_study.as_ref() else {
            ui.centered_and_justified(|ui| {
                ui.label("Open a journal file");
            });
            return;
        };

        let view = &study_ctx.view;
        let n = view.row_count();

        let Some(cr) = app_state.cluster_result.as_ref() else {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("Clustering has not been run yet.")
                        .weak(),
                );
            });
            return;
        };

        if cr.labels.len() != n {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("Cluster result is inconsistent. Please run clustering again.")
                        .color(ERROR_COLOR),
                );
            });
            return;
        }

        // クラスタ別件数を集計（label < 0 は未クラスタ）
        let counts = cluster_counts(&cr.labels);

        self.show_header(ui, cr, &counts);

        // 表示対象の行インデックスを決定（クラスタ順 → trial 順）
        let visible = visible_indices(&cr.labels, self.show_unclustered);
        if visible.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No clustered trials to display.").weak());
            });
            return;
        }

        let param_names = study_ctx.meta.param_names.clone();
        let obj_names = study_ctx.meta.objective_names.clone();
        let param_cols = view.numeric_columns(&param_names);
        let obj_cols = view.numeric_columns(&obj_names);
        let trial_ids = &view.trial_ids;
        let pareto_rank = &view.pareto_rank;

        let pinned = app_state.pinned_trials.clone();
        let highlighted = app_state.highlighted_trial;

        let n_clusters = cr.n_clusters.max(1);
        let cluster_color = |label: i32| -> egui::Color32 {
            if label < 0 {
                return crate::theme::TEXT_SECONDARY;
            }
            let t = if n_clusters == 1 {
                0.5
            } else {
                label as f32 / (n_clusters - 1) as f32
            };
            colormap.interpolate(t)
        };

        use egui_extras::{Column, TableBuilder};

        let mut clicked_trial: Option<u32> = None;
        let mut pin_toggled: Option<u32> = None;

        // パラメータ・目的を 1 列ずつに展開し、横スクロール可能にする。
        // egui_extras の Table は横スクロールを内蔵しないため、固定幅カラムを
        // 水平 ScrollArea でラップして全列を 1 セルにまとめず個別表示する。
        egui::ScrollArea::horizontal().show(ui, |ui| {
            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .column(Column::exact(30.0)) // Pin column
                .column(Column::initial(70.0).at_least(50.0)) // Cluster
                .column(Column::initial(70.0).at_least(50.0)) // Trial ID
                .columns(Column::initial(90.0).at_least(50.0), param_names.len()) // 各変数
                .columns(Column::initial(90.0).at_least(50.0), obj_names.len()) // 各目的
                .column(Column::initial(90.0).at_least(50.0)) // Pareto Rank
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("📌");
                    });
                    header.col(|ui| {
                        ui.strong("Cluster");
                    });
                    header.col(|ui| {
                        ui.strong("Trial ID");
                    });
                    for name in &param_names {
                        header.col(|ui| {
                            ui.strong(name);
                        });
                    }
                    for name in &obj_names {
                        header.col(|ui| {
                            ui.strong(name);
                        });
                    }
                    header.col(|ui| {
                        ui.strong("Pareto Rank");
                    });
                })
                .body(|body| {
                    body.rows(18.0, visible.len(), |mut row| {
                        let idx = visible[row.index()];
                        let trial_id = trial_ids.get(idx).copied().unwrap_or(idx as u32);
                        let label = cr.labels.get(idx).copied().unwrap_or(-1);
                        let rank = pareto_rank.get(idx).copied().unwrap_or(0);
                        let is_highlighted = highlighted == Some(trial_id);
                        let is_pinned = pinned.contains(&trial_id);

                        row.col(|ui| {
                            let pin_label = if is_pinned { "📌" } else { "·" };
                            if ui.small_button(pin_label).clicked() {
                                pin_toggled = Some(trial_id);
                            }
                        });
                        row.col(|ui| {
                            let text = if label < 0 {
                                "—".to_string()
                            } else {
                                label.to_string()
                            };
                            let color = cluster_color(label);
                            ui.horizontal(|ui| {
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(10.0, 10.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(rect, 2.0, color);
                                ui.label(text);
                            });
                        });
                        row.col(|ui| {
                            let res = ui.selectable_label(is_highlighted, trial_id.to_string());
                            if res.clicked() {
                                clicked_trial = Some(trial_id);
                            }
                            if is_highlighted {
                                ui.painter().rect_filled(res.rect, 0.0, COLOR_LINK);
                            }
                        });
                        for col in &param_cols {
                            row.col(|ui| {
                                let v = col.and_then(|c| c.get(idx)).copied().unwrap_or(0.0);
                                ui.label(format!("{:.3}", v));
                            });
                        }
                        for col in &obj_cols {
                            row.col(|ui| {
                                let v = col.and_then(|c| c.get(idx)).copied().unwrap_or(0.0);
                                ui.label(format!("{:.4}", v));
                            });
                        }
                        row.col(|ui| {
                            ui.label(rank.to_string());
                        });
                    });
                });
        });

        if let Some(trial_id) = clicked_trial {
            app_state.set_highlight(trial_id);
        }
        if let Some(trial_id) = pin_toggled {
            let _ = app_state.toggle_pinned_trial(trial_id);
        }
    }

    fn show_header(
        &mut self,
        ui: &mut egui::Ui,
        cr: &ClusterResult,
        counts: &BTreeMap<i32, usize>,
    ) {
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new(format!("k = {}", cr.n_clusters)).strong());
            ui.separator();
            for (&label, &count) in counts {
                if label < 0 {
                    continue;
                }
                ui.label(format!("Cluster {label}: {count}"));
            }
            if let Some(&unclustered) = counts.get(&-1) {
                ui.separator();
                ui.label(
                    egui::RichText::new(format!("Unclustered: {unclustered}"))
                        .color(crate::theme::TEXT_SECONDARY),
                );
            }
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_unclustered, "Show Unclustered");
        });
    }
}

/// クラスタ別の件数を集計する（キー: ラベル、値: 件数。-1 は未クラスタ）。
fn cluster_counts(labels: &[i32]) -> BTreeMap<i32, usize> {
    let mut counts: BTreeMap<i32, usize> = BTreeMap::new();
    for &label in labels {
        *counts.entry(label).or_insert(0) += 1;
    }
    counts
}

/// 表示対象の行インデックスを「クラスタ順 → trial 順」で返す。
/// `show_unclustered` が false の場合、label < 0 の行は除外する。
fn visible_indices(labels: &[i32], show_unclustered: bool) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..labels.len())
        .filter(|&i| {
            let label = labels[i];
            show_unclustered || label >= 0
        })
        .collect();
    // 未クラスタ（-1）は末尾にまとめるため、ソートキーを (sort_label, index) とする。
    indices.sort_by_key(|&i| {
        let label = labels[i];
        let sort_label = if label < 0 { i32::MAX } else { label };
        (sort_label, i)
    });
    indices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cluster_table_default_hides_unclustered() {
        let table = ClusterTable::default();
        assert!(!table.show_unclustered);
    }

    #[test]
    fn cluster_counts_aggregates_per_label() {
        let labels = vec![0, 1, 0, 2, 1, 0, -1];
        let counts = cluster_counts(&labels);
        assert_eq!(counts.get(&0), Some(&3));
        assert_eq!(counts.get(&1), Some(&2));
        assert_eq!(counts.get(&2), Some(&1));
        assert_eq!(counts.get(&-1), Some(&1));
    }

    #[test]
    fn visible_indices_excludes_unclustered_by_default() {
        let labels = vec![0, -1, 1, -1, 0];
        let visible = visible_indices(&labels, false);
        // -1 のインデックス 1, 3 は除外される
        assert_eq!(visible, vec![0, 4, 2]);
    }

    #[test]
    fn visible_indices_includes_unclustered_when_requested() {
        let labels = vec![0, -1, 1, -1, 0];
        let visible = visible_indices(&labels, true);
        // クラスタ順 (0,0,1) のあとに未クラスタ (-1,-1) が続く
        assert_eq!(visible, vec![0, 4, 2, 1, 3]);
    }

    #[test]
    fn visible_indices_sorts_by_cluster_then_trial() {
        let labels = vec![2, 0, 1, 0, 2];
        let visible = visible_indices(&labels, false);
        assert_eq!(visible, vec![1, 3, 2, 0, 4]);
    }

    #[test]
    fn visible_indices_empty_when_all_unclustered_and_hidden() {
        let labels = vec![-1, -1, -1];
        let visible = visible_indices(&labels, false);
        assert!(visible.is_empty());
    }
}
