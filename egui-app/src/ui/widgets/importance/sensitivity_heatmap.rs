use crate::state::app_state::HeatmapMatrix;
use crate::theme::chart_colors::{COLOR_CHART_TEXT, COLOR_GRID_STROKE};
use crate::theme::color_compute::{diverging_colormap, sequential_colormap};
use crate::ui::widgets::importance_chart::ImportanceMetric;

/// 感度ヒートマップウィジェット。手法は ImportanceChart と同じ `ImportanceMetric` を共有する。
/// 計算結果は `AppState::sensitivity_heatmap_cache` に集約されるため、ここでは
/// アイテム固有の UI 状態（選択手法・計算実行フラグ・計算要求）のみを持つ。
#[derive(Default)]
pub struct SensitivityHeatmap {
    pub metric: ImportanceMetric,
    pub computing: bool,
    /// 実行可能解のみでモデルをフィットするか（制約付きスタディのみ UI 表示）
    pub feasible_only: bool,
    /// poll_chart が消費する計算要求（対象手法, feasible_only）。
    pub pending_compute: Option<(ImportanceMetric, bool)>,
}

impl SensitivityHeatmap {
    pub fn new() -> Self {
        Self::default()
    }

    /// グローバル widget の計算実行状態を取り込む。
    /// 結果は `AppState::sensitivity_heatmap_cache` に集約されるため、キャンバスの各
    /// アイテム（独立した WidgetStates）には実行フラグのみ反映すればよい。
    /// 手法選択はアイテム固有なので維持する。
    pub fn adopt_compute_state(&mut self, src: &Self) {
        self.computing = src.computing;
    }

    /// 感度ヒートマップを描画する。`current` は選択中の手法の計算済み行列（あれば）。
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        current: Option<&HeatmapMatrix>,
        has_constraints: bool,
    ) {
        // コントロール行: Run ボタン + 手法選択 + spinner
        ui.horizontal(|ui| {
            if ui.button("Run").clicked() {
                self.pending_compute = Some((self.metric, self.feasible_only));
                self.computing = true;
            }

            egui::ComboBox::from_id_salt("sensitivity_heatmap_metric")
                .selected_text(self.metric.label())
                .show_ui(ui, |ui| {
                    // ImportanceChart と同じ系統別グループ分け。手法の性格が分かるようにする。
                    ui.label(group_header("── Correlation / Linear ──"));
                    for m in [ImportanceMetric::Spearman, ImportanceMetric::Ridge] {
                        ui.selectable_value(&mut self.metric, m, m.label());
                    }

                    ui.separator();
                    ui.label(group_header("── Tree-based ──"));
                    for m in [
                        ImportanceMetric::RfAnova,
                        ImportanceMetric::Mdi,
                        ImportanceMetric::Shap,
                        ImportanceMetric::Permutation,
                    ] {
                        ui.selectable_value(&mut self.metric, m, m.label());
                    }

                    ui.separator();
                    ui.label(group_header("── Global Sensitivity ──"));
                    for m in [ImportanceMetric::SobolFirst, ImportanceMetric::SobolTotal] {
                        ui.selectable_value(&mut self.metric, m, m.label());
                    }
                });

            // 実行可能解フィルタ（制約付きスタディのみ）
            if has_constraints {
                ui.toggle_value(&mut self.feasible_only, "Feasible only")
                    .on_hover_text("Fit the model using feasible trials only");
            }

            if self.computing {
                ui.spinner();
                ui.label("Computing...");
            }
        });

        // 低コストな手法（Spearman / Ridge）は未計算なら自動で計算要求を出す。
        // 高コストな手法は Run ボタン必須（意図しない重い計算を避ける）。
        if current.is_none()
            && !self.computing
            && self.pending_compute.is_none()
            && !self.metric.is_expensive()
        {
            self.pending_compute = Some((self.metric, self.feasible_only));
            self.computing = true;
        }

        let Some(matrix) = current else {
            ui.centered_and_justified(|ui| {
                if self.computing {
                    ui.add(egui::Spinner::new());
                } else if self.metric.is_expensive() {
                    ui.label(egui::RichText::new("Press Run to compute this metric.").weak());
                } else {
                    ui.label(egui::RichText::new("No sensitivity data.").weak());
                }
            });
            return;
        };

        if !matrix.is_well_formed() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No data.").weak());
            });
            return;
        }

        draw_matrix(ui, matrix);
    }
}

/// コンボボックスの系統見出し（ImportanceChart と同じ弱色・小サイズ）。
fn group_header(text: &str) -> egui::RichText {
    egui::RichText::new(text).weak().small()
}

fn draw_matrix(ui: &mut egui::Ui, matrix: &HeatmapMatrix) {
    let n_params = matrix.param_names.len();
    let n_objs = matrix.objective_names.len();

    // 非負系は目的（列）ごとに最大値で正規化する。列内のパラメータ相対比較を見やすくする。
    let col_max: Vec<f64> = (0..n_objs)
        .map(|j| {
            matrix
                .values
                .iter()
                .map(|row| row[j].abs())
                .fold(0.0_f64, f64::max)
        })
        .collect();

    let header_w = 80.0_f32;
    let header_h = 20.0_f32;
    let available = ui.available_rect_before_wrap();
    let cell_w = (available.width() - header_w) / n_objs as f32;
    let cell_h = (available.height() - header_h) / n_params as f32;

    let painter = ui.painter();
    let text_color = ui.visuals().text_color();

    // 列ヘッダ（目的関数名）
    for (j, obj_name) in matrix.objective_names.iter().enumerate() {
        let x = available.min.x + header_w + j as f32 * cell_w;
        let rect =
            egui::Rect::from_min_size(egui::pos2(x, available.min.y), egui::vec2(cell_w, header_h));
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            obj_name,
            egui::FontId::proportional(10.0),
            text_color,
        );
    }

    // 行ヘッダ（パラメータ名）+ セルグリッド
    for (i, param_name) in matrix.param_names.iter().enumerate() {
        let y = available.min.y + header_h + i as f32 * cell_h;

        let row_header_rect =
            egui::Rect::from_min_size(egui::pos2(available.min.x, y), egui::vec2(header_w, cell_h));
        painter.text(
            row_header_rect.center(),
            egui::Align2::CENTER_CENTER,
            param_name,
            egui::FontId::proportional(10.0),
            text_color,
        );

        for (j, &val) in matrix.values[i].iter().enumerate() {
            let x = available.min.x + header_w + j as f32 * cell_w;
            let cell_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(cell_w, cell_h));
            let color = if matrix.signed {
                // 符号付き: そのまま [-1,1] 想定で発散表示（範囲外はクランプ）。
                diverging_colormap(val)
            } else {
                // 非負: 列最大値で正規化して逐次表示。
                let denom = col_max[j];
                let t = if denom > 0.0 { val.abs() / denom } else { 0.0 };
                sequential_colormap(t)
            };
            painter.rect_filled(cell_rect, 0.0, color);
            painter.rect_stroke(
                cell_rect,
                0.0,
                egui::Stroke::new(0.5, COLOR_GRID_STROKE),
                egui::StrokeKind::Inside,
            );
            painter.text(
                cell_rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("{val:.2}"),
                egui::FontId::proportional(9.0),
                COLOR_CHART_TEXT,
            );
        }
    }

    ui.allocate_rect(available, egui::Sense::hover());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitivity_heatmap_default() {
        let hm = SensitivityHeatmap::default();
        assert!(!hm.computing);
        assert_eq!(hm.metric, ImportanceMetric::Spearman);
        assert!(!hm.feasible_only);
        assert!(hm.pending_compute.is_none());
    }

    #[test]
    fn adopt_compute_state_copies_computing_flag() {
        let mut global = SensitivityHeatmap::new();
        global.computing = true;
        let mut item = SensitivityHeatmap::new();
        item.metric = ImportanceMetric::Ridge; // アイテム固有の選択は維持される
        item.adopt_compute_state(&global);
        assert!(item.computing);
        assert_eq!(item.metric, ImportanceMetric::Ridge);
    }
}
