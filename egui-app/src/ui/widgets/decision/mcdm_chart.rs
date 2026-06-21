use crate::state::results::{EntropyResult, McdmMethod, McdmResult, WeightMode};
use crate::state::types::StudyView;
use crate::theme::chart_colors::{
    COLOR_BAR_ACCENT, COLOR_BAR_NEGATIVE, COLOR_BAR_PRIMARY, COLOR_EMPTY_STATE,
};

/// MCDM compute request payload
pub struct McdmComputeRequest {
    pub method: McdmMethod,
    pub weights: Vec<f64>,
    pub v: f64,
}

/// MCDM 結果のキャッシュキー。
/// 同じ設定（手法・重みモード・重み・v 値）で計算した結果を共有・再利用するため、
/// 各チャート（Ranking / Scatter2D / Scatter3D / Table）はこのキーで
/// `app_state.mcdm_cache` を参照する。
///
/// 重みと v は連続値のため、量子化（小数 6 桁）して Hash/Eq 可能にする。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct McdmCacheKey {
    pub method: McdmMethod,
    pub weight_mode: WeightMode,
    pub weights_q: Vec<i64>,
    pub v_q: i64,
}

impl McdmCacheKey {
    fn quantize(x: f64) -> i64 {
        (x * 1_000_000.0).round() as i64
    }

    /// 正規化済みの重みからキーを構築する。
    fn from_normalized(
        method: McdmMethod,
        weight_mode: WeightMode,
        weights: &[f64],
        v: f64,
    ) -> Self {
        let weights_q = weights.iter().map(|&w| Self::quantize(w)).collect();
        // v 値は VIKOR でのみ意味を持つため、それ以外は 0 に正規化する。
        let v_q = if method == McdmMethod::Vikor {
            Self::quantize(v)
        } else {
            0
        };
        Self {
            method,
            weight_mode,
            weights_q,
            v_q,
        }
    }

    /// 現在の設定（未正規化の重み）からキーを構築する。
    pub fn from_settings(
        method: McdmMethod,
        weight_mode: WeightMode,
        weights: &[f64],
        v: f64,
    ) -> Self {
        Self::from_normalized(method, weight_mode, &normalize_weights(weights), v)
    }

    /// 計算リクエスト（重みは正規化済み）からキーを構築する。
    pub fn from_request(req: &McdmComputeRequest, weight_mode: WeightMode) -> Self {
        Self::from_normalized(req.method, weight_mode, &req.weights, req.v)
    }
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

/// MCDM チャート共通の設定・実行状態。
/// 手法 / 重みモード / 重み / v 値 / Top N と、計算の実行状態（computing / pending）を持つ。
/// Ranking / Scatter2D / Scatter3D / Table の各チャートがそれぞれ 1 つ保持し、
/// `cache_key()` で `app_state.mcdm_cache` を参照することで独立した結果を表示する。
pub struct McdmControls {
    pub method: McdmMethod,
    pub weight_mode: WeightMode,
    pub weights: Vec<f64>,
    pub v_param: f64,
    pub top_n: McdmTopN,
    pub computing: bool,
    pub pending_compute: Option<McdmComputeRequest>,
    pub pending_entropy: bool,
    pub entropy_result: Option<EntropyResult>,
}

impl Default for McdmControls {
    fn default() -> Self {
        Self {
            method: McdmMethod::Topsis,
            weight_mode: WeightMode::Manual,
            weights: Vec::new(),
            v_param: 0.5,
            top_n: McdmTopN::Top10,
            computing: false,
            pending_compute: None,
            pending_entropy: false,
            entropy_result: None,
        }
    }
}

/// MCDMランキングバーチャートのUI状態
#[derive(Default)]
pub struct McdmRankChart {
    pub controls: McdmControls,
}

/// MCDMランキングテーブルのUI状態
#[derive(Default)]
pub struct McdmTable {
    pub controls: McdmControls,
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

impl McdmControls {
    /// グローバル widget の計算実行状態と共有出力を取り込む。
    /// 計算結果は `app_state.mcdm_cache` に集約されるため、computing フラグ・
    /// エントロピー重み・エントロピー詳細などの実行状態のみをキャンバスの各アイテムに反映する。
    /// 手法・WeightMode・Top N・v 値などの UI 設定はアイテム固有なので維持する。
    pub fn adopt_compute_state(&mut self, src: &Self) {
        self.computing = src.computing;
        self.pending_entropy = src.pending_entropy;
        self.weights = src.weights.clone();
        self.entropy_result = src.entropy_result.clone();
    }

    /// 現在の設定に対応するキャッシュキーを返す。
    pub fn cache_key(&self) -> McdmCacheKey {
        McdmCacheKey::from_settings(self.method, self.weight_mode, &self.weights, self.v_param)
    }

    /// 設定 UI（手法 / 重みモード / Top N / Run / 重み / エントロピー詳細）を描画する。
    /// 目的が無い場合は false を返し、呼び出し側は以降の描画をスキップする。
    /// `id_prefix` で egui の ID 名前空間を分け、同一画面に複数の MCDM チャートを
    /// 置いてもコントロールの ID が衝突しないようにする。
    pub fn show_controls(
        &mut self,
        ui: &mut egui::Ui,
        obj_names: &[String],
        id_prefix: &str,
    ) -> bool {
        let obj_count = obj_names.len();
        if obj_count == 0 {
            ui.vertical_centered(|ui| {
                ui.colored_label(COLOR_EMPTY_STATE, "Select a study first");
            });
            return false;
        }

        if self.weights.len() != obj_count {
            self.weights = vec![1.0; obj_count];
        }

        ui.push_id(id_prefix, |ui| {
            // 手法セレクタ + WeightMode + Top N + Runボタン + spinner
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("mcdm_method_combo")
                    .selected_text(self.method.label())
                    .show_ui(ui, |ui| {
                        for m in McdmMethod::all() {
                            ui.selectable_value(&mut self.method, *m, m.label());
                        }
                    });

                // WeightMode セレクタ（手法セレクタの横）
                let prev_weight_mode = self.weight_mode;
                egui::ComboBox::from_id_salt("mcdm_weight_mode_combo")
                    .selected_text(format!("Weight: {}", self.weight_mode.label()))
                    .show_ui(ui, |ui| {
                        for wm in WeightMode::all() {
                            ui.selectable_value(&mut self.weight_mode, *wm, wm.label());
                        }
                    });

                // WeightMode 切替ロジック
                if prev_weight_mode != self.weight_mode {
                    self.pending_entropy = self.weight_mode == WeightMode::Entropy;
                }

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
                let is_entropy = self.weight_mode == WeightMode::Entropy;
                for (i, obj_name) in obj_names.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(obj_name.as_str()).strong());
                        if is_entropy {
                            // Entropy mode: 読み取り専用スライダー
                            let mut w = self.weights[i];
                            ui.add_enabled(
                                false,
                                egui::Slider::new(&mut w, 0.0..=1.0).text("weight"),
                            );
                        } else {
                            let mut w = self.weights[i];
                            if ui
                                .add(egui::Slider::new(&mut w, 0.0..=1.0).text("weight"))
                                .changed()
                            {
                                self.weights[i] = w;
                            }
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

            // エントロピー結果テーブル
            if self.weight_mode == WeightMode::Entropy {
                if let Some(ref entropy) = self.entropy_result {
                    ui.collapsing("Entropy Details", |ui| {
                        ui.label(format!("Computed in {:.1}ms", entropy.duration_ms));

                        use egui_extras::{Column, TableBuilder};
                        let n_obj = entropy.weights.len();
                        if n_obj == 0 {
                            ui.colored_label(COLOR_EMPTY_STATE, "No data");
                            return;
                        }

                        TableBuilder::new(ui)
                            .striped(true)
                            .column(Column::exact(120.0))
                            .columns(Column::remainder(), n_obj)
                            .header(20.0, |mut header| {
                                header.col(|ui| {
                                    ui.strong("Metric");
                                });
                                for name in obj_names.iter().take(n_obj) {
                                    header.col(|ui| {
                                        ui.strong(name);
                                    });
                                }
                            })
                            .body(|mut body| {
                                // Entropy row
                                body.row(18.0, |mut row| {
                                    row.col(|ui| {
                                        ui.label("Entropy");
                                    });
                                    for &e in &entropy.entropies {
                                        row.col(|ui| {
                                            ui.label(format!("{:.4}", e));
                                        });
                                    }
                                });
                                // Diversity row
                                body.row(18.0, |mut row| {
                                    row.col(|ui| {
                                        ui.label("Diversity");
                                    });
                                    for &d in &entropy.diversities {
                                        row.col(|ui| {
                                            ui.label(format!("{:.4}", d));
                                        });
                                    }
                                });
                                // Weight row
                                body.row(18.0, |mut row| {
                                    row.col(|ui| {
                                        ui.strong("Weight");
                                    });
                                    for &w in &entropy.weights {
                                        row.col(|ui| {
                                            ui.strong(format!("{:.4}", w));
                                        });
                                    }
                                });
                            });
                    });
                }
            }

            ui.separator();
        });

        true
    }
}

impl McdmRankChart {
    pub fn adopt_compute_state(&mut self, src: &Self) {
        self.controls.adopt_compute_state(&src.controls);
    }

    pub fn show(&mut self, ui: &mut egui::Ui, obj_names: &[String], result: Option<&McdmResult>) {
        if !self.controls.show_controls(ui, obj_names, "mcdm_rank") {
            return;
        }

        if self.controls.computing {
            return;
        }

        let Some(result) = result else {
            ui.vertical_centered(|ui| {
                ui.colored_label(COLOR_EMPTY_STATE, "Press Run to compute MCDM ranking");
            });
            return;
        };

        ui.label(format!(
            "Computed in {:.1}ms ({})",
            result.duration_ms(),
            result.method_label()
        ));

        let label_width = 100.0_f32;
        let bar_height = 20.0_f32;
        let bar_gap = 4.0_f32;
        let value_text_width = 60.0_f32;

        if let McdmResult::PrometheeI(r) = result {
            let top_n = self.controls.top_n.value().min(r.ranked_indices_i.len());
            if top_n == 0 {
                ui.label("No data");
                return;
            }
            let max_val = r
                .phi_plus
                .iter()
                .chain(r.phi_minus.iter())
                .fold(0.0_f64, |a, &b| a.max(b));
            egui::ScrollArea::vertical().show(ui, |ui| {
                let available_width = ui.available_width() - label_width - value_text_width - 8.0;
                let bar_max_width = (available_width / 2.0).max(25.0);
                for rank in 0..top_n {
                    let idx = r.ranked_indices_i[rank] as usize;
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [label_width, bar_height],
                            egui::Label::new(
                                egui::RichText::new(format!("Trial {idx}"))
                                    .text_style(egui::TextStyle::Body),
                            )
                            .truncate(),
                        );
                        let phi_plus_w = if max_val > 0.0 {
                            (r.phi_plus[idx] / max_val * bar_max_width as f64) as f32
                        } else {
                            0.0
                        };
                        let phi_minus_w = if max_val > 0.0 {
                            (r.phi_minus[idx] / max_val * bar_max_width as f64) as f32
                        } else {
                            0.0
                        };
                        let (rect_plus, _) = ui.allocate_exact_size(
                            egui::vec2(bar_max_width, bar_height - bar_gap),
                            egui::Sense::hover(),
                        );
                        if ui.is_rect_visible(rect_plus) {
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(
                                    rect_plus.min,
                                    egui::vec2(phi_plus_w, rect_plus.height()),
                                ),
                                2.0,
                                COLOR_BAR_PRIMARY,
                            );
                        }
                        let (rect_minus, _) = ui.allocate_exact_size(
                            egui::vec2(bar_max_width, bar_height - bar_gap),
                            egui::Sense::hover(),
                        );
                        if ui.is_rect_visible(rect_minus) {
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(
                                    rect_minus.min,
                                    egui::vec2(phi_minus_w, rect_minus.height()),
                                ),
                                2.0,
                                COLOR_BAR_NEGATIVE,
                            );
                        }
                        ui.label(format!(
                            "Φ+{:.3} Φ-{:.3}",
                            r.phi_plus[idx], r.phi_minus[idx]
                        ));
                    });
                }
            });
            return;
        }

        if let McdmResult::PrometheeII(r) = result {
            let top_n = self.controls.top_n.value().min(r.ranked_indices_ii.len());
            if top_n == 0 {
                ui.label("No data");
                return;
            }
            let max_abs = r.phi_net.iter().fold(0.0_f64, |a, &b| a.max(b.abs()));
            egui::ScrollArea::vertical().show(ui, |ui| {
                let available_width = ui.available_width() - label_width - value_text_width - 8.0;
                let bar_max_width = available_width.max(50.0);
                for rank in 0..top_n {
                    let idx = r.ranked_indices_ii[rank] as usize;
                    let phi_net = r.phi_net[idx];
                    let bar_w = if max_abs > 0.0 {
                        (phi_net.abs() / max_abs * bar_max_width as f64) as f32
                    } else {
                        0.0
                    };
                    let color = if phi_net >= 0.0 {
                        COLOR_BAR_PRIMARY
                    } else {
                        COLOR_BAR_ACCENT
                    };
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [label_width, bar_height],
                            egui::Label::new(
                                egui::RichText::new(format!("Trial {idx}"))
                                    .text_style(egui::TextStyle::Body),
                            )
                            .truncate(),
                        );
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(bar_max_width, bar_height - bar_gap),
                            egui::Sense::hover(),
                        );
                        if ui.is_rect_visible(rect) {
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(
                                    rect.min,
                                    egui::vec2(bar_w, rect.height()),
                                ),
                                2.0,
                                color,
                            );
                        }
                        ui.label(format!("{phi_net:.4}"));
                    });
                }
            });
            return;
        }

        let entries = enumerate_ranked(result, self.controls.top_n.value());
        if entries.is_empty() {
            ui.label("No data");
            return;
        }

        let max_score = entries.iter().map(|e| e.score).fold(0.0_f64, f64::max);
        let bar_color = COLOR_BAR_PRIMARY;

        egui::ScrollArea::vertical().show(ui, |ui| {
            let available_width = ui.available_width() - label_width - value_text_width - 8.0;
            let bar_max_width = available_width.max(50.0);

            for entry in &entries {
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [label_width, bar_height],
                        egui::Label::new(
                            egui::RichText::new(format!("Trial {}", entry.trial_idx))
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
    pub fn adopt_compute_state(&mut self, src: &Self) {
        self.controls.adopt_compute_state(&src.controls);
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        result: Option<&McdmResult>,
        view: &StudyView,
        param_names: &[String],
        obj_names: &[String],
    ) {
        if !self.controls.show_controls(ui, obj_names, "mcdm_table") {
            return;
        }

        if self.controls.computing {
            return;
        }

        let Some(result) = result else {
            ui.vertical_centered(|ui| {
                ui.colored_label(COLOR_EMPTY_STATE, "Press Run to compute the MCDM ranking");
            });
            return;
        };

        use egui_extras::{Column, TableBuilder};

        let rows = build_ranking_rows(
            result,
            view,
            param_names,
            obj_names,
            self.controls.top_n.value(),
        );
        if rows.is_empty() {
            ui.colored_label(COLOR_EMPTY_STATE, "No results to display");
            return;
        }

        // 各変数・目的を 1 列ずつに展開し、横スクロール可能にする
        // （Cluster Table と同形式）。
        egui::ScrollArea::horizontal().show(ui, |ui| {
            // ストライプの色を強調して偶数/奇数行を見分けやすくする。
            ui.visuals_mut().faint_bg_color = crate::theme::TABLE_STRIPE_BG;
            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .column(Column::initial(50.0).at_least(40.0)) // Rank
                .column(Column::initial(70.0).at_least(50.0)) // Trial
                .column(Column::initial(80.0).at_least(50.0)) // Score
                .columns(Column::initial(90.0).at_least(50.0), obj_names.len()) // 各目的
                .columns(Column::initial(90.0).at_least(50.0), param_names.len()) // 各変数
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
                    for name in param_names {
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
                                ui.label(format!("{}", row_data.trial_number));
                            });
                            row.col(|ui| {
                                ui.label(format!("{:.4}", row_data.score));
                            });
                            for &val in &row_data.objectives {
                                row.col(|ui| {
                                    ui.label(format!("{:.4}", val));
                                });
                            }
                            for &val in &row_data.parameters {
                                row.col(|ui| {
                                    ui.label(format!("{:.3}", val));
                                });
                            }
                        });
                    }
                });
        });
    }
}

/// ランキング上位N件の共通抽出データ
struct RankingEntry {
    rank: usize,
    trial_idx: usize,
    score: f64,
}

/// McdmResultから上位N件のランキングエントリを生成する
fn enumerate_ranked(result: &McdmResult, top_n: usize) -> Vec<RankingEntry> {
    let scores = result.primary_scores();
    let ranked = result.ranked_indices();
    let count = top_n.min(ranked.len());

    (0..count)
        .map(|rank| {
            let trial_idx = ranked[rank] as usize;
            let score = scores.get(trial_idx).copied().unwrap_or(0.0);
            RankingEntry {
                rank: rank + 1,
                trial_idx,
                score,
            }
        })
        .collect()
}

/// テーブル行データ
pub struct RankingRow {
    pub rank: usize,
    pub trial_number: u32,
    pub score: f64,
    pub parameters: Vec<f64>,
    pub objectives: Vec<f64>,
}

/// McdmResultから上位N件のテーブル行データを生成する
pub fn build_ranking_rows(
    result: &McdmResult,
    view: &StudyView,
    param_names: &[String],
    obj_names: &[String],
    top_n: usize,
) -> Vec<RankingRow> {
    let param_cols = view.numeric_columns(param_names);
    let obj_cols = view.numeric_columns(obj_names);
    enumerate_ranked(result, top_n)
        .into_iter()
        .map(|e| {
            let parameters: Vec<f64> = param_cols
                .iter()
                .map(|col| col.and_then(|c| c.get(e.trial_idx)).copied().unwrap_or(0.0))
                .collect();
            let objectives: Vec<f64> = obj_cols
                .iter()
                .map(|col| col.and_then(|c| c.get(e.trial_idx)).copied().unwrap_or(0.0))
                .collect();
            RankingRow {
                rank: e.rank,
                trial_number: e.trial_idx as u32,
                score: e.score,
                parameters,
                objectives,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::results::TopsisResult;
    use crate::state::types::TrialRow;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tunny_core::dataframe::{DataFrame, TrialRow as CoreRow};

    #[test]
    fn adopt_compute_state_syncs_runtime_and_preserves_ui_settings() {
        let mut item = McdmRankChart {
            controls: McdmControls {
                computing: true,
                method: McdmMethod::Vikor,
                top_n: McdmTopN::Top20,
                v_param: 0.7,
                ..Default::default()
            },
        };
        let global = McdmRankChart {
            controls: McdmControls {
                computing: false,
                weights: vec![0.25, 0.75],
                ..Default::default()
            },
        };

        item.adopt_compute_state(&global);

        // 実行状態・共有出力は取り込まれる。
        assert!(!item.controls.computing);
        assert_eq!(item.controls.weights, vec![0.25, 0.75]);
        // UI 設定はアイテム固有で維持される。
        assert_eq!(item.controls.method, McdmMethod::Vikor);
        assert_eq!(item.controls.top_n, McdmTopN::Top20);
        assert_eq!(item.controls.v_param, 0.7);
    }

    fn make_simple_view(n: usize) -> StudyView {
        if n == 0 {
            let df = DataFrame::from_trials(&[], &[], &[], &[], &[], 0);
            return StudyView::new(Arc::new(df), vec![]);
        }
        let core_rows: Vec<CoreRow> = (0..n)
            .map(|i| CoreRow {
                trial_id: i as u32,
                trial_number: i as u32,
                param_display: HashMap::new(),
                param_category_label: HashMap::new(),
                objective_values: vec![],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let df = DataFrame::from_trials(&core_rows, &[], &[], &[], &[], 0);
        StudyView::new(Arc::new(df), vec![0; n])
    }

    fn make_view_with_objectives(objective_rows: &[Vec<f64>]) -> (StudyView, Vec<String>) {
        let n = objective_rows.len();
        if n == 0 {
            return (make_simple_view(0), vec![]);
        }
        let n_obj = objective_rows[0].len();
        let obj_names: Vec<String> = (0..n_obj).map(|i| format!("obj{i}")).collect();
        let core_rows: Vec<CoreRow> = (0..n)
            .map(|i| CoreRow {
                trial_id: i as u32,
                trial_number: i as u32,
                param_display: HashMap::new(),
                param_category_label: HashMap::new(),
                objective_values: objective_rows[i].clone(),
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let df = DataFrame::from_trials(&core_rows, &[], &obj_names, &[], &[], 0);
        (StudyView::new(Arc::new(df), vec![0; n]), obj_names)
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
        let c = &chart.controls;
        assert_eq!(c.method, McdmMethod::Topsis);
        assert_eq!(c.weight_mode, WeightMode::Manual);
        assert!(!c.computing);
        assert!(c.pending_compute.is_none());
        assert!(!c.pending_entropy);
        assert!(c.entropy_result.is_none());
        assert_eq!(c.top_n, McdmTopN::Top10);
        assert!(c.weights.is_empty());
        assert!((c.v_param - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn mcdm_table_default() {
        let table = McdmTable::default();
        assert_eq!(table.controls.top_n, McdmTopN::Top10);
    }

    #[test]
    fn enumerate_ranked_top5_with_5_results() {
        let result = make_topsis_result(vec![0.9, 0.7, 0.5, 0.3, 0.1], vec![0, 1, 2, 3, 4]);
        let view = make_simple_view(5);
        let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
        assert_eq!(ranking.len(), 5);
        assert!((ranking[0].score - 0.9).abs() < 1e-9);
        assert!((ranking[4].score - 0.1).abs() < 1e-9);
    }

    #[test]
    fn enumerate_ranked_top10_with_20_results() {
        let scores: Vec<f64> = (0..20).map(|i| 1.0 - i as f64 / 20.0).collect();
        let ranked: Vec<u32> = (0..20).collect();
        let result = make_topsis_result(scores, ranked);
        let view = make_simple_view(20);
        let ranking = build_ranking_rows(&result, &view, &[], &[], 10);
        assert_eq!(ranking.len(), 10);
    }

    #[test]
    fn enumerate_ranked_top5_with_3_results_min_applied() {
        let result = make_topsis_result(vec![0.9, 0.5, 0.1], vec![0, 1, 2]);
        let view = make_simple_view(3);
        let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
        assert_eq!(ranking.len(), 3);
    }

    #[test]
    fn enumerate_ranked_scores_match_ranked_order() {
        let result = make_topsis_result(vec![0.1, 0.9, 0.5], vec![1, 2, 0]);
        let view = make_simple_view(3);
        let ranking = build_ranking_rows(&result, &view, &[], &[], 10);
        assert_eq!(ranking.len(), 3);
        assert!((ranking[0].score - 0.9).abs() < 1e-9);
        assert!((ranking[1].score - 0.5).abs() < 1e-9);
        assert!((ranking[2].score - 0.1).abs() < 1e-9);
    }

    #[test]
    fn enumerate_ranked_empty_result() {
        let result = make_topsis_result(vec![], vec![]);
        let view = make_simple_view(0);
        let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
        assert!(ranking.is_empty());
    }

    #[test]
    fn top_n_toggle_cycle() {
        let mut chart = McdmRankChart::default();
        assert_eq!(chart.controls.top_n, McdmTopN::Top10);
        chart.controls.top_n = McdmTopN::Top5;
        assert_eq!(chart.controls.top_n.value(), 5);
        chart.controls.top_n = McdmTopN::Top20;
        assert_eq!(chart.controls.top_n.value(), 20);
    }

    #[test]
    fn build_ranking_rows_basic() {
        let result = make_topsis_result(vec![0.9, 0.5, 0.1], vec![0, 1, 2]);
        let view = make_simple_view(3);
        let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
        assert_eq!(ranking.len(), 3);
        assert_eq!(ranking[0].rank, 1);
        assert_eq!(ranking[0].trial_number, 0);
        assert!((ranking[0].score - 0.9).abs() < 1e-9);
    }

    #[test]
    fn build_ranking_rows_top_n_limit() {
        let scores: Vec<f64> = (0..20).map(|i| 1.0 - i as f64 / 20.0).collect();
        let ranked: Vec<u32> = (0..20).collect();
        let result = make_topsis_result(scores, ranked);
        let view = make_simple_view(20);
        let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
        assert_eq!(ranking.len(), 5);
    }

    #[test]
    fn build_ranking_rows_rank_starts_at_1() {
        let result = make_topsis_result(vec![0.8], vec![0]);
        let view = make_simple_view(1);
        let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
        assert_eq!(ranking[0].rank, 1);
    }

    #[test]
    fn build_ranking_rows_empty() {
        let result = make_topsis_result(vec![], vec![]);
        let view = make_simple_view(0);
        let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
        assert!(ranking.is_empty());
    }

    #[test]
    fn build_ranking_rows_objectives_included() {
        let result = make_topsis_result(vec![0.9, 0.5], vec![0, 1]);
        let (view, obj_names) = make_view_with_objectives(&[vec![1.0, 2.0], vec![3.0, 4.0]]);
        let ranking = build_ranking_rows(&result, &view, &[], &obj_names, 10);
        assert_eq!(ranking[0].objectives, vec![1.0, 2.0]);
        assert_eq!(ranking[1].objectives, vec![3.0, 4.0]);
    }

    // ── E2E / integration tests ──

    fn multi_obj_data() -> Vec<Vec<f64>> {
        vec![
            vec![0.1, 0.9],
            vec![0.5, 0.5],
            vec![0.9, 0.1],
            vec![0.3, 0.7],
            vec![0.7, 0.3],
        ]
    }

    fn multi_obj_rows_for_topsis() -> Vec<TrialRow> {
        multi_obj_data()
            .into_iter()
            .enumerate()
            .map(|(i, objs)| TrialRow {
                trial_id: i as u32,
                objectives: objs,
                ..Default::default()
            })
            .collect()
    }

    #[test]
    fn topsis_full_pipeline_equal_weights() {
        let data = multi_obj_data();
        let objectives: Vec<f64> = data.iter().flat_map(|r| r.iter().copied()).collect();
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

        let (view, obj_names) = make_view_with_objectives(&data);
        let ranking = build_ranking_rows(&mcdm_result, &view, &[], &obj_names, 5);
        assert_eq!(ranking.len(), 5);
        assert_eq!(ranking[0].rank, 1);
        for i in 1..ranking.len() {
            assert!(ranking[i - 1].score >= ranking[i].score);
        }
    }

    #[test]
    fn topsis_weight_bias_changes_ranking() {
        let data = multi_obj_data();
        let objectives: Vec<f64> = data.iter().flat_map(|r| r.iter().copied()).collect();
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
        let objectives: Vec<f64> = (0..5).map(|i| i as f64 * 0.2).collect();
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
        assert!(chart.controls.pending_compute.is_none());
        assert!(!chart.controls.computing);

        let normalized = normalize_weights(&[1.0, 1.0]);
        chart.controls.pending_compute = Some(McdmComputeRequest {
            method: McdmMethod::Topsis,
            weights: normalized,
            v: 0.5,
        });
        chart.controls.computing = true;

        assert!(chart.controls.pending_compute.is_some());
        assert!(chart.controls.computing);

        let payload = chart.controls.pending_compute.take();
        assert!(payload.is_some());
        assert!(chart.controls.pending_compute.is_none());
        assert!(chart.controls.computing);
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
    fn top_n_toggle_updates_display() {
        let data = multi_obj_data();
        let objectives: Vec<f64> = data.iter().flat_map(|r| r.iter().copied()).collect();
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

        let (view, obj_names) = make_view_with_objectives(&data);

        let rows5 = build_ranking_rows(&mcdm, &view, &[], &obj_names, 5);
        assert_eq!(rows5.len(), 5);

        let rows3 = build_ranking_rows(&mcdm, &view, &[], &obj_names, 3);
        assert_eq!(rows3.len(), 3);

        let rows10 = build_ranking_rows(&mcdm, &view, &[], &obj_names, 10);
        assert_eq!(rows10.len(), 5);
    }
}
