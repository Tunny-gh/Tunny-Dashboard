//! PCA バイプロット — 標準化 PCA の第1・第2主成分にトライアルを射影し、
//! 元の変数の寄与（loadings）を矢印で重ね描きするウィジェット。
//!
//! PCA 本体は `tunny_core::clustering::run_pca_standardized` が担い、現在アクティブな
//! Study の DataFrame（`with_active_df` 経由）を直接読む。同期ウィジェットとして描画パス内で
//! 都度呼び出し、結果は (df の Arc 恒等性, Study 名, 行数, 対象空間) をキーに 1 件だけ
//! キャッシュする。着色点群・loadings 矢印などの描画物はさらにカラーマップ・着色目的に
//! 依存するため、別キーで併せてキャッシュする。
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
    /// PCA 本体と描画物のキャッシュ。
    #[serde(skip)]
    cache: Option<PcaBiplotCache>,
}

/// PCA 本体（`PcaResult`）と描画物（色グループ・loadings 矢印）をまとめてキャッシュする。
///
/// PCA 本体は df の恒等性・Study・行数・対象空間が変わったときのみ再計算する。
/// 描画物はさらにカラーマップ・着色目的にも依存するため、それらが変わったときは
/// PCA 本体を保ったまま描画物だけ再構築する（M-17）。
struct PcaBiplotCache {
    /// PCA 本体のキー: (df 恒等性, Study 名, 行数, 対象空間判別子)。
    pca_key: (usize, String, usize, u8),
    result: PcaResult,
    /// 描画物のキー: (カラーマップのフィンガープリント, 着色目的)。
    draw_key: (u64, Option<String>),
    draw: PcaDraw,
}

/// PCA バイプロットの描画物（毎フレーム再構築を避けるためキャッシュする・M-17）。
struct PcaDraw {
    /// 着色済みの点群（色 → 射影座標一覧）。
    color_groups: BTreeMap<[u8; 4], Vec<[f64; 2]>>,
    /// loadings 矢印の始点（常に原点）と終点。
    loading_origins: Vec<[f64; 2]>,
    loading_tips: Vec<[f64; 2]>,
    /// 寄与率つきの軸ラベル。
    x_label: String,
    y_label: String,
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
        self.cache.as_ref().map(|c| &c.result)
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

        // PCA 本体のキー: df の Arc 恒等性 + Study 名 + 行数 + 対象空間（low 指摘）。
        // 恒等性を含めることで、同一 (Study 名, 行数, 空間) の別データへ切り替えても取り違えない。
        let df_ptr = std::sync::Arc::as_ptr(&view.df) as usize;
        let pca_key = (
            df_ptr,
            study_name.to_string(),
            view.row_count(),
            self.space.disc(),
        );
        // 描画物のキー: カラーマップ + 着色目的（M-17）。
        let draw_key = (
            super::rank_plot::cmap_fingerprint(cmap),
            self.color_objective.clone(),
        );

        // PCA 本体は pca_key が変わったときのみ再計算する。描画物は draw_key が
        // 変わったときだけ、PCA 本体を保ったまま再構築する。
        let pca_valid = self.cache.as_ref().is_some_and(|c| c.pca_key == pca_key);
        if !pca_valid {
            self.cache = tunny_core::clustering::run_pca_standardized(2, self.space.to_core()).map(
                |result| {
                    let draw = compute_pca_draw(&result, &self.color_objective, view, cmap);
                    PcaBiplotCache {
                        pca_key,
                        result,
                        draw_key,
                        draw,
                    }
                },
            );
        } else if self.cache.as_ref().is_some_and(|c| c.draw_key != draw_key) {
            if let Some(c) = self.cache.as_mut() {
                c.draw = compute_pca_draw(&c.result, &self.color_objective, view, cmap);
                c.draw_key = draw_key;
            }
        }

        let Some(cache) = self.cache.as_ref() else {
            ui.vertical_centered(|ui| {
                ui.colored_label(
                    COLOR_EMPTY_STATE(),
                    "PCA needs at least 2 numeric columns and 2 trials.",
                );
            });
            return;
        };
        let result = &cache.result;
        if result.projections.is_empty() || result.loadings.len() < 2 {
            ui.vertical_centered(|ui| {
                ui.colored_label(
                    COLOR_EMPTY_STATE(),
                    "PCA needs at least 2 numeric columns and 2 trials.",
                );
            });
            return;
        }

        let draw = &cache.draw;
        let show_loadings = self.show_loadings;
        egui_plot::Plot::new("pca_biplot")
            .unified_nav()
            .x_axis_label(&draw.x_label)
            .y_axis_label(&draw.y_label)
            .data_aspect(1.0)
            .legend(egui_plot::Legend::default())
            .show(ui, |plot_ui| {
                apply_wheel_zoom(plot_ui);
                for ([r, g, b, a], pts) in &draw.color_groups {
                    let color = egui::Color32::from_rgba_unmultiplied(*r, *g, *b, *a);
                    plot_ui.points(
                        egui_plot::Points::new("Trials", pts.clone())
                            .color(color)
                            .radius(3.0),
                    );
                }

                if show_loadings {
                    plot_ui.arrows(
                        egui_plot::Arrows::new(
                            "Loadings",
                            draw.loading_origins.clone(),
                            draw.loading_tips.clone(),
                        )
                        .color(crate::theme::chart_colors::COLOR_CONTOUR()),
                    );
                    for (j, name) in result.feature_names.iter().enumerate() {
                        if let Some(&[tx, ty]) = draw.loading_tips.get(j) {
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

/// PCA 本体（`result`）から描画物（着色点群・loadings 矢印・軸ラベル）を構築する（M-17）。
/// カラーマップ・着色目的にのみ依存する部分を分離し、これらが変わったときだけ再構築する。
fn compute_pca_draw(
    result: &PcaResult,
    color_objective: &Option<String>,
    view: &StudyView,
    cmap: &ColorMap,
) -> PcaDraw {
    let color_col = color_objective
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
                    COLOR_SCATTER_DOT()
                }
            }
            None => COLOR_SCATTER_DOT(),
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

    // loadings が 2 主成分に満たない退化ケースでは矢印を描かない（添字 panic を防ぐ）。
    let (loading_origins, loading_tips) = if result.loadings.len() >= 2 {
        let origins = vec![[0.0, 0.0]; result.feature_names.len()];
        let tips: Vec<[f64; 2]> = (0..result.feature_names.len())
            .map(|j| {
                let lx = result.loadings[0].get(j).copied().unwrap_or(0.0);
                let ly = result.loadings[1].get(j).copied().unwrap_or(0.0);
                [lx * loading_scale, ly * loading_scale]
            })
            .collect();
        (origins, tips)
    } else {
        (Vec::new(), Vec::new())
    };

    let x_label = format!(
        "PC1 ({:.1}%)",
        result.explained_ratio.first().unwrap_or(&0.0) * 100.0
    );
    let y_label = format!(
        "PC2 ({:.1}%)",
        result.explained_ratio.get(1).unwrap_or(&0.0) * 100.0
    );

    PcaDraw {
        color_groups,
        loading_origins,
        loading_tips,
        x_label,
        y_label,
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
