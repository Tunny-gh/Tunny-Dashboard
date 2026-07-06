use std::collections::HashMap;

use crate::io::artifacts::ArtifactEntry;
use crate::state::types::{Direction, StudyView};
use crate::theme::colormap::ColorMap;
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};
use crate::ui::widgets::trial_detail_modal::{
    axis_row, fmt_opt, resolve_click_hover, show_hover_tooltip, TrialDetailModal, TrialDetailTarget,
};

/// Rank Plot ウィジェット
///
/// Optuna の `plot_rank` に相当する。選択した 2 パラメータの散布図を、選択した
/// 目的関数値のランク（パーセンタイル。0=最良〜1=最悪）で色付けする。生の値では
/// なく順位で色付けするため外れ値に頑健で、パラメータの組み合わせが目的関数の
/// 良し悪しにどう影響するかを俯瞰しやすい。
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct RankPlotChart {
    pub x_param_idx: usize,
    pub y_param_idx: usize,
    pub obj_idx: usize,
    /// 点クリックで開くトライアル詳細モーダル（他の散布図と共有）。
    #[serde(skip)]
    detail_modal: TrialDetailModal,
    /// ランクのソート・点群構築・色グルーピングの結果キャッシュ。
    /// `RankPlotCache::key` が変わらない限り毎フレームの再計算を避ける
    /// （robustness.rs の Arc 恒等性パターンに倣う）。
    #[serde(skip)]
    cache: Option<RankPlotCache>,
}

impl Default for RankPlotChart {
    fn default() -> Self {
        Self {
            x_param_idx: 0,
            y_param_idx: 1,
            obj_idx: 0,
            detail_modal: TrialDetailModal::new(),
            cache: None,
        }
    }
}

/// フレーム毎の再計算（ランクの O(n log n) ソート、点群構築、色グルーピング用
/// HashMap 構築）を避けるためのキャッシュ。
///
/// キーは (DataFrame の Arc 恒等性, X パラメータ選択, Y パラメータ選択, 目的選択,
/// カラーマップのフィンガープリント)。いずれかが変わったときだけ再構築する。
/// カラーマップはユーザーがテーマ設定で切り替えられるため、色そのものが
/// キャッシュ結果（color_groups のキー）に影響することを反映してキーに含める。
struct RankPlotCache {
    key: (usize, usize, usize, usize, u64),
    ranks: Vec<f64>,
    color_groups: HashMap<[u8; 4], Vec<[f64; 2]>>,
    hit_candidates: Vec<(u32, usize, [f64; 2])>,
}

/// カラーマップの内容を安価な u64 フィンガープリントに畳み込む（FNV-1a 風）。
/// 停止点数は少数（数点）のため、毎フレーム計算してもヒープ確保なしで軽量。
/// 散布図行列・PCA バイプロットのキャッシュキーでも共有する。
pub(super) fn cmap_fingerprint(cmap: &ColorMap) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325 ^ (cmap.stops.len() as u64);
    for &(t, color) in &cmap.stops {
        let packed = u32::from(color.r())
            | (u32::from(color.g()) << 8)
            | (u32::from(color.b()) << 16)
            | (u32::from(color.a()) << 24);
        h = (h ^ t.to_bits() as u64).wrapping_mul(0x100000001b3);
        h = (h ^ packed as u64).wrapping_mul(0x100000001b3);
    }
    h
}

/// 描画対象 1 点分（トライアル ID・行 index・パラメータ座標・ランクパーセンタイル）。
struct RankPoint {
    trial_id: u32,
    row: usize,
    x: f64,
    y: f64,
    rank: f64,
}

impl RankPlotChart {
    /// Rank Plot を描画する。
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        param_names: &[String],
        obj_names: &[String],
        directions: &[Direction],
        cmap: &ColorMap,
        artifact_map: &HashMap<u32, Vec<ArtifactEntry>>,
    ) {
        if view.row_count() == 0 {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No trial data.").weak());
            });
            return;
        }
        if param_names.len() < 2 {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("Rank plot requires at least 2 parameters.").weak());
            });
            return;
        }
        if obj_names.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No objectives.").weak());
            });
            return;
        }

        // インデックスのクランプ（データ更新後の範囲外アクセスを防ぐ）。
        self.x_param_idx = self.x_param_idx.min(param_names.len() - 1);
        self.y_param_idx = self.y_param_idx.min(param_names.len() - 1);
        // X/Y が同一パラメータになった場合（クランプ後の衝突など）は、
        // 対角線上に潰れた無意味な散布図にならないよう Y を次のパラメータへ送る。
        if self.x_param_idx == self.y_param_idx {
            self.y_param_idx = (self.x_param_idx + 1) % param_names.len();
        }
        self.obj_idx = self.obj_idx.min(obj_names.len() - 1);

        ui.horizontal(|ui| {
            ui.label("X:");
            egui::ComboBox::from_id_salt("rank_plot_x_combo")
                .selected_text(&param_names[self.x_param_idx])
                .show_ui(ui, |ui| {
                    for (i, name) in param_names.iter().enumerate() {
                        ui.selectable_value(&mut self.x_param_idx, i, name);
                    }
                });
            ui.label("Y:");
            egui::ComboBox::from_id_salt("rank_plot_y_combo")
                .selected_text(&param_names[self.y_param_idx])
                .show_ui(ui, |ui| {
                    for (i, name) in param_names.iter().enumerate() {
                        ui.selectable_value(&mut self.y_param_idx, i, name);
                    }
                });
            if obj_names.len() > 1 {
                ui.label("Objective:");
                egui::ComboBox::from_id_salt("rank_plot_obj_combo")
                    .selected_text(&obj_names[self.obj_idx])
                    .show_ui(ui, |ui| {
                        for (i, name) in obj_names.iter().enumerate() {
                            ui.selectable_value(&mut self.obj_idx, i, name);
                        }
                    });
            }
        });

        let x_name = param_names[self.x_param_idx].clone();
        let y_name = param_names[self.y_param_idx].clone();
        let obj_name = obj_names[self.obj_idx].clone();
        let minimize = matches!(directions.get(self.obj_idx), Some(Direction::Minimize));

        // キャッシュキー: view（DataFrame）の恒等性 + 選択中の X/Y/目的 + カラーマップ。
        // いずれも変わっていなければ、ランクのソートや色グルーピングを再計算しない。
        let cache_key = (
            std::sync::Arc::as_ptr(&view.df) as usize,
            self.x_param_idx,
            self.y_param_idx,
            self.obj_idx,
            cmap_fingerprint(cmap),
        );
        let cache_valid = self.cache.as_ref().is_some_and(|c| c.key == cache_key);
        if !cache_valid {
            let obj_values: Vec<f64> = view
                .numeric_column(&obj_name)
                .map(|c| c.to_vec())
                .unwrap_or_default();
            // ランクは目的値列全体（COMPLETE trial 全件）から算出する。
            // パラメータが欠損している行があっても、他行のランクには影響しない。
            let ranks = compute_rank_percentiles(&obj_values, minimize);
            let points = collect_rank_points(view, &x_name, &y_name, &ranks);

            // 色ごとにグルーピングして描画バッチ数を抑える（MCDM 散布図と同じ手法）。
            let mut color_groups: HashMap<[u8; 4], Vec<[f64; 2]>> = HashMap::new();
            let hit_candidates: Vec<(u32, usize, [f64; 2])> = points
                .iter()
                .map(|p| (p.trial_id, p.row, [p.x, p.y]))
                .collect();
            for p in &points {
                let color = cmap.interpolate(p.rank.clamp(0.0, 1.0) as f32);
                let key = [color.r(), color.g(), color.b(), color.a()];
                color_groups.entry(key).or_default().push([p.x, p.y]);
            }

            self.cache = Some(RankPlotCache {
                key: cache_key,
                ranks,
                color_groups,
                hit_candidates,
            });
        }
        let cache = self.cache.as_ref().expect("cache just populated above");
        if cache.hit_candidates.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No finite points to plot.").weak());
            });
            return;
        }
        let ranks = &cache.ranks;
        let hit_candidates = &cache.hit_candidates;
        let color_groups = &cache.color_groups;

        let mut clicked_detail: Option<(u32, usize)> = None;
        let mut hovered_detail: Option<(u32, usize)> = None;

        ui.horizontal(|ui| {
            let avail = ui.available_size();
            const BAR_W: f32 = 14.0;
            const GUTTER: f32 = 20.0;
            let plot_w = (avail.x - BAR_W - GUTTER).max(200.0);

            let plot = egui_plot::Plot::new("rank_plot")
                .unified_nav()
                .width(plot_w)
                .height(avail.y)
                .x_axis_label(&x_name)
                .y_axis_label(&y_name);

            plot.show(ui, |plot_ui| {
                apply_wheel_zoom(plot_ui);
                (clicked_detail, hovered_detail) = resolve_click_hover(plot_ui, hit_candidates);
                for (key, pts) in color_groups {
                    let color =
                        egui::Color32::from_rgba_unmultiplied(key[0], key[1], key[2], key[3]);
                    plot_ui.points(
                        egui_plot::Points::new("", pts.clone())
                            .color(color)
                            .radius(2.5),
                    );
                }
            });

            ui.add_space(4.0);
            ui.vertical(|ui| {
                ui.add_space(4.0);
                let bar_h = avail.y.clamp(60.0, 160.0);
                let (bar_rect, _) =
                    ui.allocate_exact_size(egui::vec2(BAR_W, bar_h), egui::Sense::hover());
                draw_rank_legend(ui, bar_rect, cmap);
            });
        });

        // ホバー中の点があれば、ポインタ位置に概要ツールチップを表示する。
        if let Some((_, row)) = hovered_detail {
            let trial_number = view.df.get_trial_number(row).unwrap_or(row as u32);
            let rows = vec![
                axis_row(&x_name, view.numeric_column(&x_name), row),
                axis_row(&y_name, view.numeric_column(&y_name), row),
                axis_row(&obj_name, view.numeric_column(&obj_name), row),
                (
                    "Rank Percentile".to_string(),
                    fmt_opt(ranks.get(row).copied()),
                ),
            ];
            show_hover_tooltip(ui, "rank_plot_hover_tooltip", trial_number, &rows);
        }

        // 点クリックでトライアル詳細モーダルを開く（散布図共有・アーティファクト付き）。
        if let Some((trial_id, row)) = clicked_detail {
            let context = vec![(
                "Rank Percentile".to_string(),
                fmt_opt(ranks.get(row).copied()),
            )];
            self.detail_modal.open(TrialDetailTarget {
                trial_id,
                row_index: row,
                context,
            });
        }
        self.detail_modal
            .show(ui, view, param_names, obj_names, artifact_map);
    }
}

/// view から Rank Plot の描画点を集める（NaN/Inf の x/y は除外）。
fn collect_rank_points(
    view: &StudyView,
    x_name: &str,
    y_name: &str,
    ranks: &[f64],
) -> Vec<RankPoint> {
    let (Some(x_col), Some(y_col)) = (view.numeric_column(x_name), view.numeric_column(y_name))
    else {
        return Vec::new();
    };
    (0..view.row_count())
        .filter_map(|i| {
            let x = x_col.get(i).copied()?;
            let y = y_col.get(i).copied()?;
            if !x.is_finite() || !y.is_finite() {
                return None;
            }
            let rank = ranks.get(i).copied().unwrap_or(1.0);
            let trial_id = view.trial_ids.get(i).copied().unwrap_or(i as u32);
            Some(RankPoint {
                trial_id,
                row: i,
                x,
                y,
                rank,
            })
        })
        .collect()
}

/// ランクパーセンタイルの凡例（縦グラデーションバー + Best/Worst ラベル）を描画する。
///
/// `common::heatmap::draw_colorbar_simple` と同じグラデーション描画方式に倣うが、
/// ランクは値そのものではなく相対順位（0=最良〜1=最悪）であるため、数値目盛の
/// 代わりに Best / Worst ラベルを使う方が直感的と判断し、専用の簡易版を用意した。
fn draw_rank_legend(ui: &mut egui::Ui, bar_rect: egui::Rect, cmap: &ColorMap) {
    let painter = ui.painter();
    let n_steps = 32;
    let step_h = bar_rect.height() / n_steps as f32;
    for i in 0..n_steps {
        // i=0 がバー上端（Worst=1.0 の色）、末尾が下端（Best=0.0 の色）。
        let t = 1.0 - (i as f32 / (n_steps - 1).max(1) as f32);
        let step_rect = egui::Rect::from_min_size(
            egui::pos2(bar_rect.left(), bar_rect.top() + i as f32 * step_h),
            egui::vec2(bar_rect.width(), step_h + 1.0),
        );
        painter.rect_filled(step_rect, 0.0, cmap.interpolate(t));
    }
    painter.rect_stroke(
        bar_rect,
        0.0,
        egui::Stroke::new(0.5, egui::Color32::from_gray(90)),
        egui::StrokeKind::Inside,
    );

    let text_color = crate::theme::CLOSE_BTN_TEXT();
    let font = egui::FontId::proportional(9.0);
    painter.text(
        egui::pos2(bar_rect.center().x, bar_rect.top() - 2.0),
        egui::Align2::CENTER_BOTTOM,
        "Worst",
        font.clone(),
        text_color,
    );
    painter.text(
        egui::pos2(bar_rect.center().x, bar_rect.bottom() + 2.0),
        egui::Align2::CENTER_TOP,
        "Best",
        font,
        text_color,
    );
}

/// 目的値のランクパーセンタイルを計算する（0.0=最良 〜 1.0=最悪）。
///
/// - `minimize=true` のとき値が小さいほど 0 に近く、`false` のとき値が大きいほど
///   0 に近い（常に「良い解」が 0 側になるよう Direction を反映する）。
/// - 同値（タイ）は平均順位で扱う（順位の間を取る）。
/// - NaN は最悪（1.0）として扱う。
/// - 有限値が 1 件のみのときは 0.0（唯一の観測 = 最良として扱う）。
/// - 空配列は空を返す。
pub fn compute_rank_percentiles(values: &[f64], minimize: bool) -> Vec<f64> {
    let n = values.len();
    if n == 0 {
        return Vec::new();
    }

    let mut finite: Vec<(usize, f64)> = values
        .iter()
        .enumerate()
        .filter(|(_, v)| v.is_finite())
        .map(|(i, &v)| (i, v))
        .collect();

    if minimize {
        finite.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    } else {
        finite.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    }

    let m = finite.len();
    let mut ranks = vec![1.0f64; n]; // NaN 等の非有限値は最悪(1.0)のまま残す
    let mut i = 0;
    while i < m {
        let mut j = i;
        while j < m && finite[j].1 == finite[i].1 {
            j += 1;
        }
        // タイ区間 [i, j) の平均順位（0-indexed）を [0, m-1] から [0, 1] へ正規化する。
        let avg_rank = ((i + j - 1) as f64) / 2.0;
        let percentile = if m > 1 {
            avg_rank / (m - 1) as f64
        } else {
            0.0
        };
        for &(orig_idx, _) in &finite[i..j] {
            ranks[orig_idx] = percentile;
        }
        i = j;
    }
    ranks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_rank_percentiles_ascending_minimize() {
        let ranks = compute_rank_percentiles(&[1.0, 2.0, 3.0, 4.0, 5.0], true);
        assert_eq!(ranks, vec![0.0, 0.25, 0.5, 0.75, 1.0]);
    }

    #[test]
    fn compute_rank_percentiles_descending_minimize_reorders() {
        let ranks = compute_rank_percentiles(&[5.0, 4.0, 3.0, 2.0, 1.0], true);
        assert_eq!(ranks, vec![1.0, 0.75, 0.5, 0.25, 0.0]);
    }

    #[test]
    fn compute_rank_percentiles_maximize_flips_order() {
        let ranks = compute_rank_percentiles(&[1.0, 2.0, 3.0, 4.0, 5.0], false);
        assert_eq!(ranks, vec![1.0, 0.75, 0.5, 0.25, 0.0]);
    }

    #[test]
    fn compute_rank_percentiles_ties_are_averaged() {
        let ranks = compute_rank_percentiles(&[1.0, 1.0, 2.0], true);
        assert_eq!(ranks, vec![0.25, 0.25, 1.0]);
    }

    #[test]
    fn compute_rank_percentiles_nan_is_worst() {
        let ranks = compute_rank_percentiles(&[1.0, f64::NAN, 2.0], true);
        assert_eq!(ranks, vec![0.0, 1.0, 1.0]);
    }

    #[test]
    fn compute_rank_percentiles_single_element_is_best() {
        let ranks = compute_rank_percentiles(&[42.0], true);
        assert_eq!(ranks, vec![0.0]);
    }

    #[test]
    fn compute_rank_percentiles_single_nan_is_worst() {
        let ranks = compute_rank_percentiles(&[f64::NAN], true);
        assert_eq!(ranks, vec![1.0]);
    }

    #[test]
    fn compute_rank_percentiles_empty_is_empty() {
        assert!(compute_rank_percentiles(&[], true).is_empty());
    }

    #[test]
    fn rank_plot_chart_default() {
        let chart = RankPlotChart::default();
        assert_eq!(chart.x_param_idx, 0);
        assert_eq!(chart.y_param_idx, 1);
        assert_eq!(chart.obj_idx, 0);
    }
}
