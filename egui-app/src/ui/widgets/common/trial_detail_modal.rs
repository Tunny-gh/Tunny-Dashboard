//! 散布図の点クリックで開く「トライアル詳細」モーダル。
//!
//! Pareto 2D / Cluster 2D / MCDM 散布図が共有する。各散布図はクリックされた点の
//! trial を検出して [`TrialDetailModal::open`] で対象を渡し、毎フレーム
//! [`TrialDetailModal::show`] を呼んで描画する。モーダルは散布図固有の情報
//! （Pareto ランク / クラスタ番号 / MCDM ランクなど）に加えて、目的関数値・変数値・
//! アーティファクト（サムネイル＋ファイル名）を表示する。

use std::collections::HashMap;

use crate::io::artifacts::{ArtifactEntry, ArtifactFileType};
use crate::state::types::StudyView;

use super::radar_chart;

/// サムネイル一辺のサイズ（px）。
const THUMB_SIZE: f32 = 220.0;

/// 点クリック判定のしきい値（クリック位置から点までのスクリーン距離・px）。
pub const HIT_THRESHOLD: f32 = 12.0;

/// モーダルが表示する対象 trial と、散布図固有の付加情報。
#[derive(Debug, Clone, PartialEq)]
pub struct TrialDetailTarget {
    /// 対象トライアルのグローバル ID（アーティファクト参照に使う。表示はしない）。
    pub trial_id: u32,
    /// `StudyView` 上の行 index。目的関数値・変数値の参照に加え、
    /// Study 内 0 始まり番号としてヘッダー表示にも使う。
    pub row_index: usize,
    /// 散布図固有の情報（例: `[("Pareto Rank", "0")]`）。表示は配列順。
    pub context: Vec<(String, String)>,
}

/// 散布図共有のトライアル詳細モーダル。
#[derive(Default)]
pub struct TrialDetailModal {
    /// 表示中の対象。`None` のとき閉じている。
    open: Option<TrialDetailTarget>,
}

impl TrialDetailModal {
    pub fn new() -> Self {
        Self::default()
    }

    /// 対象 trial を設定してモーダルを開く。
    pub fn open(&mut self, target: TrialDetailTarget) {
        self.open = Some(target);
    }

    /// モーダルを閉じる。
    pub fn close(&mut self) {
        self.open = None;
    }

    /// モーダルが開いているか。
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// モーダルを描画する。閉じている場合は何もしない。
    /// 背景クリック / Esc / Close ボタンで閉じる。
    pub fn show(
        &mut self,
        ui: &egui::Ui,
        view: &StudyView,
        param_names: &[String],
        obj_names: &[String],
        artifact_map: &HashMap<u32, Vec<ArtifactEntry>>,
    ) {
        let Some(target) = self.open.clone() else {
            return;
        };
        let egui_ctx = ui.ctx().clone();
        let screen = egui_ctx.screen_rect();
        // Artifact プレビューモーダルと同等に画面の大半を占めるサイズにする。
        let max_w = (screen.width() * 0.95).max(320.0);
        let max_h = (screen.height() * 0.95).max(240.0);
        // ヘッダー・区切り・余白を除いた本文スクロール領域の高さ。
        let body_max_h = (max_h - 80.0).max(160.0);
        // 3 段組: 左=テキスト情報 / 中央=レーダー / 右=アーティファクト。
        // 左・中央は固定幅、残りを右（アーティファクト）に充てる。
        let left_w = (max_w * 0.26).clamp(280.0, 460.0);
        let radar_w = (max_w * 0.3).clamp(300.0, 500.0);

        let mut close = false;
        let modal = egui::Modal::new(egui::Id::new("trial_detail_modal")).show(&egui_ctx, |ui| {
            ui.set_max_width(max_w);
            // 画像のアスペクト比に依らずモーダルを大きく確保する。
            ui.set_min_width(max_w);
            ui.set_min_height(max_h);
            ui.horizontal(|ui| {
                // ヘッダーは Optuna の `trial.number`（Study 内 0 始まりの作成順番号）を表示する。
                // `trial_id` はストレージ横断のグローバル ID で、他 study や
                // pruned/failed トライアルの分だけ番号がずれるため表示に使わない
                // （アーティファクト参照には引き続き `trial_id` を使う）。
                let trial_number = view
                    .df
                    .get_trial_number(target.row_index)
                    .unwrap_or(target.row_index as u32);
                ui.heading(format!("Trial {trial_number}"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("× Close").clicked() {
                        close = true;
                    }
                });
            });
            ui.separator();

            egui::ScrollArea::vertical()
                .max_height(body_max_h)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    // 3 段組: 左=テキスト情報 / 中央=レーダー / 右=アーティファクト。
                    ui.horizontal_top(|ui| {
                        // 左: テキスト情報（Chart Info / Objectives / Variables）。
                        ui.allocate_ui_with_layout(
                            egui::vec2(left_w, body_max_h),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                // 散布図固有の情報（ランク・クラスタ番号など）。
                                if !target.context.is_empty() {
                                    section_label(ui, "Chart Info");
                                    kv_grid(ui, "trial_detail_context", &target.context);
                                    ui.add_space(8.0);
                                }

                                // 目的関数値。
                                if !obj_names.is_empty() {
                                    section_label(ui, "Objectives");
                                    let rows = value_rows(view, obj_names, target.row_index, 4);
                                    kv_grid(ui, "trial_detail_objectives", &rows);
                                    ui.add_space(8.0);
                                }

                                // 変数値。
                                if !param_names.is_empty() {
                                    section_label(ui, "Variables");
                                    let rows = value_rows(view, param_names, target.row_index, 4);
                                    kv_grid(ui, "trial_detail_params", &rows);
                                    ui.add_space(8.0);
                                }
                            },
                        );

                        ui.separator();

                        // 中央: レーダーチャート（目的＋変数）。パレートフロント各個体を
                        // 薄い線で重ね、外周＝フロント最大（包絡）。選択トライアルを赤で重ねる。
                        ui.allocate_ui_with_layout(
                            egui::vec2(radar_w, body_max_h),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                let radar_data = radar_chart::build(
                                    view,
                                    obj_names,
                                    param_names,
                                    target.row_index,
                                );
                                if radar_data.axes.len() >= 3 {
                                    section_label(ui, "Comparison (Radar)");
                                    radar_chart::show(ui, &radar_data);
                                } else {
                                    ui.label(
                                        egui::RichText::new("Radar chart unavailable.").weak(),
                                    );
                                }
                            },
                        );

                        ui.separator();

                        // 右: アーティファクト（サムネイル＋ファイル名）。
                        ui.vertical(|ui| {
                            section_label(ui, "Artifacts");
                            match artifact_map.get(&target.trial_id) {
                                Some(entries) if !entries.is_empty() => {
                                    render_artifacts(ui, entries)
                                }
                                _ => {
                                    ui.label(
                                        egui::RichText::new("No artifacts for this trial.").weak(),
                                    );
                                }
                            }
                        });
                    });
                });
        });

        if close || modal.should_close() {
            self.open = None;
        }
    }
}

/// セクション見出し。
fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).strong());
}

/// 列名→値スライスから (名前, 整形済み値) のペア列を作る。
fn value_rows(
    view: &StudyView,
    names: &[String],
    row_index: usize,
    prec: usize,
) -> Vec<(String, String)> {
    let cols = view.numeric_columns(names);
    names
        .iter()
        .zip(cols.iter())
        .map(|(name, col)| {
            let v = col.and_then(|c| c.get(row_index)).copied();
            (name.clone(), fmt_opt(v, prec))
        })
        .collect()
}

/// 2 列のキー/値グリッドを描画する。
fn kv_grid(ui: &mut egui::Ui, id: &str, rows: &[(String, String)]) {
    egui::Grid::new(id)
        .num_columns(2)
        .spacing([16.0, 2.0])
        .show(ui, |ui| {
            for (k, v) in rows {
                ui.label(egui::RichText::new(k).color(crate::theme::TEXT_SECONDARY));
                ui.label(v);
                ui.end_row();
            }
        });
}

/// `Option<f64>` を固定小数で整形する（None は em dash）。
fn fmt_opt(v: Option<f64>, prec: usize) -> String {
    match v {
        Some(x) => format!("{x:.prec$}"),
        None => "—".to_string(),
    }
}

/// アーティファクトをサムネイル（画像）＋ファイル名で横並びに描画する。
fn render_artifacts(ui: &mut egui::Ui, entries: &[ArtifactEntry]) {
    ui.horizontal_wrapped(|ui| {
        for entry in entries {
            ui.allocate_ui(egui::vec2(THUMB_SIZE, THUMB_SIZE + 24.0), |ui| {
                ui.vertical(|ui| {
                    match entry.file_type() {
                        ArtifactFileType::Image => {
                            let uri = format!("file://{}", entry.path.to_string_lossy());
                            ui.add(
                                egui::Image::from_uri(uri)
                                    .fit_to_exact_size(egui::vec2(THUMB_SIZE, THUMB_SIZE)),
                            );
                        }
                        other => {
                            let icon = if matches!(other, ArtifactFileType::Csv) {
                                "📊"
                            } else {
                                "📦"
                            };
                            ui.vertical_centered(|ui| {
                                ui.add_space(THUMB_SIZE * 0.25);
                                ui.label(egui::RichText::new(icon).size(THUMB_SIZE * 0.4));
                                ui.add_space(THUMB_SIZE * 0.25);
                                if ui.small_button("Open").clicked() {
                                    let _ = open::that(&entry.path);
                                }
                            });
                        }
                    }
                    ui.add(
                        egui::Label::new(egui::RichText::new(&entry.filename).small()).truncate(),
                    );
                });
            });
        }
    });
}

/// クリック座標に最も近い候補点の index を返す（スクリーン座標・しきい値 px 以内）。
///
/// `egui_plot` のクロージャ内で点のスクリーン座標を計算してから呼ぶ。純粋関数として
/// テスト可能にするため、候補のスクリーン座標とクリック座標のみを受ける。
pub fn nearest_within(
    screen_points: &[egui::Pos2],
    click: egui::Pos2,
    threshold: f32,
) -> Option<usize> {
    let mut best: Option<(f32, usize)> = None;
    for (i, &p) in screen_points.iter().enumerate() {
        let d = p.distance(click);
        if d <= threshold && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, i));
        }
    }
    best.map(|(_, i)| i)
}

/// 候補点（trial_id, row_index, plot 座標）から、クリック位置（スクリーン座標）に
/// 最も近くかつ `threshold` px 以内の点を `(trial_id, row_index)` で返す。
pub fn hit_test_nearest(
    plot_ui: &egui_plot::PlotUi,
    candidates: &[(u32, usize, [f64; 2])],
    click: egui::Pos2,
    threshold: f32,
) -> Option<(u32, usize)> {
    let screen_points: Vec<egui::Pos2> = candidates
        .iter()
        .map(|&(_, _, [x, y])| plot_ui.screen_from_plot(egui_plot::PlotPoint::new(x, y)))
        .collect();
    nearest_within(&screen_points, click, threshold).map(|i| (candidates[i].0, candidates[i].1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_within_returns_closest_in_threshold() {
        let pts = vec![
            egui::pos2(0.0, 0.0),
            egui::pos2(10.0, 0.0),
            egui::pos2(100.0, 100.0),
        ];
        // クリック (11, 0) に最も近いのは index 1（距離 1）。
        assert_eq!(nearest_within(&pts, egui::pos2(11.0, 0.0), 12.0), Some(1));
    }

    #[test]
    fn nearest_within_none_outside_threshold() {
        let pts = vec![egui::pos2(0.0, 0.0)];
        assert_eq!(nearest_within(&pts, egui::pos2(50.0, 50.0), 12.0), None);
    }

    #[test]
    fn nearest_within_empty_is_none() {
        assert_eq!(nearest_within(&[], egui::pos2(0.0, 0.0), 12.0), None);
    }

    #[test]
    fn nearest_within_picks_strictly_closest() {
        let pts = vec![egui::pos2(5.0, 0.0), egui::pos2(3.0, 0.0)];
        // どちらもしきい値内だが、より近い index 1 を選ぶ。
        assert_eq!(nearest_within(&pts, egui::pos2(0.0, 0.0), 12.0), Some(1));
    }

    #[test]
    fn fmt_opt_formats_and_handles_none() {
        assert_eq!(fmt_opt(Some(1.23456), 4), "1.2346");
        assert_eq!(fmt_opt(None, 4), "—");
    }
}
