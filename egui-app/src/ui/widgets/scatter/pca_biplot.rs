//! PCA バイプロット — 標準化 PCA の第1・第2主成分にトライアルを射影し、
//! 元の変数の寄与（loadings）を矢印で重ね描きするウィジェット。
//!
//! PCA 本体は `tunny_core::clustering::run_pca_standardized` が担い、現在アクティブな
//! Study の DataFrame（`with_active_df` 経由）を直接読む。同期ウィジェットとして描画パス内で
//! 都度呼び出し、結果は (Study 名, 行数, 対象空間) をキーに 1 件だけキャッシュする。
//! 詳細は `theory/{en,ja}/clustering/pca-biplot.md` を参照。

use std::collections::BTreeMap;

use crate::state::types::StudyView;
use crate::theme::chart_colors::{COLOR_EMPTY_STATE, COLOR_SCATTER_DOT};
use crate::theme::colormap::ColorMap;
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};
use tunny_core::clustering::PcaResult;

/// PCA の対象空間。`tunny_core::clustering::PcaSpace` は serde を実装していないため、
/// UI 状態の永続化用にこのミラー enum を用意し `to_core` で変換する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum PcaSpaceOption {
    #[default]
    Param,
    Objective,
    All,
}

impl PcaSpaceOption {
    pub fn label(self) -> &'static str {
        match self {
            PcaSpaceOption::Param => "Parameters",
            PcaSpaceOption::Objective => "Objectives",
            PcaSpaceOption::All => "All",
        }
    }

    pub fn to_core(self) -> tunny_core::clustering::PcaSpace {
        match self {
            PcaSpaceOption::Param => tunny_core::clustering::PcaSpace::Param,
            PcaSpaceOption::Objective => tunny_core::clustering::PcaSpace::Objective,
            PcaSpaceOption::All => tunny_core::clustering::PcaSpace::All,
        }
    }

    /// キャッシュキー用の判別子。
    fn disc(self) -> u8 {
        match self {
            PcaSpaceOption::Param => 0,
            PcaSpaceOption::Objective => 1,
            PcaSpaceOption::All => 2,
        }
    }
}

/// PCA バイプロットウィジェットの UI 状態。
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PcaBiplotChart {
    pub space: PcaSpaceOption,
    pub show_loadings: bool,
    /// 連続着色に使う目的関数名。`None` なら単色。
    pub color_objective: Option<String>,
    /// (Study 名, 行数, 対象空間判別子) をキーにした計算結果キャッシュ。
    #[serde(skip)]
    cache: Option<((String, usize, u8), PcaResult)>,
}

impl Default for PcaBiplotChart {
    fn default() -> Self {
        Self {
            space: PcaSpaceOption::default(),
            show_loadings: true,
            color_objective: None,
            cache: None,
        }
    }
}

impl PcaBiplotChart {
    /// CSV エクスポート用にキャッシュ済みの PCA 結果を返す。
    pub fn cached_result(&self) -> Option<&PcaResult> {
        self.cache.as_ref().map(|(_, r)| r)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        obj_names: &[String],
        cmap: &ColorMap,
        study_name: &str,
    ) {
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("pca_biplot_space")
                .selected_text(self.space.label())
                .show_ui(ui, |ui| {
                    for space in [
                        PcaSpaceOption::Param,
                        PcaSpaceOption::Objective,
                        PcaSpaceOption::All,
                    ] {
                        ui.selectable_value(&mut self.space, space, space.label());
                    }
                });
            ui.checkbox(&mut self.show_loadings, "Show loadings");

            ui.label("Color by:");
            let color_label = self.color_objective.as_deref().unwrap_or("None");
            egui::ComboBox::from_id_salt("pca_biplot_color")
                .selected_text(color_label)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.color_objective, None, "None");
                    for name in obj_names {
                        ui.selectable_value(
                            &mut self.color_objective,
                            Some(name.clone()),
                            name.as_str(),
                        );
                    }
                });
        });

        let key = (study_name.to_string(), view.row_count(), self.space.disc());
        if self.cache.as_ref().map(|(k, _)| k) != Some(&key) {
            let result = tunny_core::clustering::run_pca_standardized(2, self.space.to_core());
            self.cache = result.map(|r| (key, r));
        }

        let Some(result) = self.cached_result() else {
            ui.vertical_centered(|ui| {
                ui.colored_label(
                    COLOR_EMPTY_STATE,
                    "PCA needs at least 2 numeric columns and 2 trials.",
                );
            });
            return;
        };
        if result.projections.is_empty() || result.loadings.len() < 2 {
            ui.vertical_centered(|ui| {
                ui.colored_label(
                    COLOR_EMPTY_STATE,
                    "PCA needs at least 2 numeric columns and 2 trials.",
                );
            });
            return;
        }

        let color_col = self
            .color_objective
            .as_deref()
            .and_then(|name| view.numeric_column(name));
        let (color_min, color_max) = color_range(color_col);

        let mut color_groups: BTreeMap<[u8; 4], Vec<[f64; 2]>> = BTreeMap::new();
        for (i, row) in result.projections.iter().enumerate() {
            let x = row.first().copied().unwrap_or(0.0);
            let y = row.get(1).copied().unwrap_or(0.0);
            let color = match color_col {
                Some(col) => {
                    let v = col.get(i).copied().unwrap_or(f64::NAN);
                    if v.is_finite() && color_max > color_min {
                        let t = ((v - color_min) / (color_max - color_min)) as f32;
                        cmap.interpolate(t)
                    } else {
                        COLOR_SCATTER_DOT
                    }
                }
                None => COLOR_SCATTER_DOT,
            };
            let key = [color.r(), color.g(), color.b(), color.a()];
            color_groups.entry(key).or_default().push([x, y]);
        }

        let max_abs_score = result
            .projections
            .iter()
            .flat_map(|row| row.iter())
            .fold(0.0f64, |acc, &v| acc.max(v.abs()))
            .max(1e-9);
        let loading_scale = 0.8 * max_abs_score;

        let x_label = format!(
            "PC1 ({:.1}%)",
            result.explained_ratio.first().unwrap_or(&0.0) * 100.0
        );
        let y_label = format!(
            "PC2 ({:.1}%)",
            result.explained_ratio.get(1).unwrap_or(&0.0) * 100.0
        );

        egui_plot::Plot::new("pca_biplot")
            .unified_nav()
            .x_axis_label(x_label)
            .y_axis_label(y_label)
            .data_aspect(1.0)
            .legend(egui_plot::Legend::default())
            .show(ui, |plot_ui| {
                apply_wheel_zoom(plot_ui);
                for ([r, g, b, a], pts) in &color_groups {
                    let color = egui::Color32::from_rgba_unmultiplied(*r, *g, *b, *a);
                    plot_ui.points(
                        egui_plot::Points::new("Trials", pts.clone())
                            .color(color)
                            .radius(3.0),
                    );
                }

                if self.show_loadings {
                    let origins: Vec<[f64; 2]> = vec![[0.0, 0.0]; result.feature_names.len()];
                    let tips: Vec<[f64; 2]> = (0..result.feature_names.len())
                        .map(|j| {
                            let lx = result.loadings[0].get(j).copied().unwrap_or(0.0);
                            let ly = result.loadings[1].get(j).copied().unwrap_or(0.0);
                            [lx * loading_scale, ly * loading_scale]
                        })
                        .collect();
                    plot_ui.arrows(
                        egui_plot::Arrows::new("Loadings", origins, tips.clone())
                            .color(crate::theme::chart_colors::COLOR_CONTOUR),
                    );
                    for (j, name) in result.feature_names.iter().enumerate() {
                        if let Some(&[tx, ty]) = tips.get(j) {
                            plot_ui.text(egui_plot::Text::new(
                                "",
                                egui_plot::PlotPoint::new(tx, ty),
                                name.as_str(),
                            ));
                        }
                    }
                }
            });
    }
}

/// 連続着色列の有限値のみを対象にした (min, max) を返す。列が無ければ (0.0, 0.0)。
fn color_range(col: Option<&[f64]>) -> (f64, f64) {
    let Some(col) = col else {
        return (0.0, 0.0);
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_param_space_with_loadings_and_no_color() {
        let chart = PcaBiplotChart::default();
        assert_eq!(chart.space, PcaSpaceOption::Param);
        assert!(chart.show_loadings);
        assert!(chart.color_objective.is_none());
        assert!(chart.cache.is_none());
    }

    #[test]
    fn space_disc_is_stable_and_distinct() {
        assert_eq!(PcaSpaceOption::Param.disc(), 0);
        assert_eq!(PcaSpaceOption::Objective.disc(), 1);
        assert_eq!(PcaSpaceOption::All.disc(), 2);
    }

    #[test]
    fn to_core_maps_each_variant() {
        assert_eq!(
            PcaSpaceOption::Param.to_core(),
            tunny_core::clustering::PcaSpace::Param
        );
        assert_eq!(
            PcaSpaceOption::Objective.to_core(),
            tunny_core::clustering::PcaSpace::Objective
        );
        assert_eq!(
            PcaSpaceOption::All.to_core(),
            tunny_core::clustering::PcaSpace::All
        );
    }

    #[test]
    fn color_range_ignores_non_finite() {
        let col = [1.0, f64::NAN, 5.0, f64::INFINITY, -2.0];
        assert_eq!(color_range(Some(&col)), (-2.0, 5.0));
    }

    #[test]
    fn color_range_none_column_is_zero_zero() {
        assert_eq!(color_range(None), (0.0, 0.0));
    }

    #[test]
    fn color_range_all_non_finite_is_zero_zero() {
        let col = [f64::NAN, f64::INFINITY];
        assert_eq!(color_range(Some(&col)), (0.0, 0.0));
    }
}
