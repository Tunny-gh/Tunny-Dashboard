//! 階層クラスタリング（Ward 法）のデンドログラムウィジェット。
//!
//! `tunny_core::clustering::ward_linkage` で併合木を構築し、$k$ スライダーで
//! カットして得られるクラスタを葉の色分けで示す。学習コストは軽い（$O(n^2)$、
//! 800 行上限のサブサンプル込み）ため SYNC ウィジェット。理論的背景は
//! theory/{en,ja}/clustering/hierarchical.md。
//!
//! 配線メモ（このファイルはまだ mod.rs に登録されていない。ChartId::Dendrogram /
//! label "Dendrogram" / icon dendrogram.svg として配線予定）。

use crate::state::types::StudyView;
use crate::theme::chart_colors::COLOR_EMPTY_STATE;
use crate::theme::colormap::ColorMap;
use tunny_core::clustering::{cut_tree, dendrogram_nodes, ward_linkage, HierarchicalResult, Merge};

/// 距離行列に使う特徴空間。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum DendrogramSpace {
    #[default]
    Params,
    Objectives,
    All,
}

impl DendrogramSpace {
    fn label(self) -> &'static str {
        match self {
            DendrogramSpace::Params => "Parameters",
            DendrogramSpace::Objectives => "Objectives",
            DendrogramSpace::All => "Parameters + Objectives",
        }
    }

    fn disc(self) -> u8 {
        match self {
            DendrogramSpace::Params => 0,
            DendrogramSpace::Objectives => 1,
            DendrogramSpace::All => 2,
        }
    }
}

/// (study_name, row_count, space disc)
type DendrogramCacheKey = (String, usize, u8);

/// デンドログラムウィジェットの UI 状態。
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct DendrogramChart {
    pub k: usize,
    pub space: DendrogramSpace,
    #[serde(skip)]
    cache: Option<(DendrogramCacheKey, HierarchicalResult)>,
}

impl Default for DendrogramChart {
    fn default() -> Self {
        Self {
            k: 3,
            space: DendrogramSpace::default(),
            cache: None,
        }
    }
}

fn feature_names(
    param_names: &[String],
    obj_names: &[String],
    space: DendrogramSpace,
) -> Vec<String> {
    match space {
        DendrogramSpace::Params => param_names.to_vec(),
        DendrogramSpace::Objectives => obj_names.to_vec(),
        DendrogramSpace::All => param_names
            .iter()
            .chain(obj_names.iter())
            .cloned()
            .collect(),
    }
}

/// view から距離行列を組み立てる。指定した全特徴が有限な行のみ採用する
/// （800 行超のサブサンプルは `ward_linkage` 内部が担う）。
fn build_matrix(view: &StudyView, features: &[String]) -> Vec<Vec<f64>> {
    super::feature_matrix(view, features)
}

/// カット閾値（`merges[cutoff-1].distance` と `merges[cutoff].distance` の中点、
/// `cutoff = n_leaves - k`）を返す。範囲外（`k < 2` または `k >= n_leaves`）は `None`。
pub fn cut_threshold(merges: &[Merge], n_leaves: usize, k: usize) -> Option<f64> {
    let cutoff = n_leaves.checked_sub(k)?;
    if cutoff == 0 || cutoff >= merges.len() {
        return None;
    }
    Some((merges[cutoff - 1].distance + merges[cutoff].distance) / 2.0)
}

/// 各ノード（葉 0..n、内部ノード n..2n-1）に対する「カット後クラスタラベル」を返す。
/// 葉のラベルは `cut_tree` の結果をそのまま使う。内部ノードは両子のラベルが
/// 一致すればそのラベル、そうでなければ `None`（カットで分かれた枝＝グレー表示）。
fn compute_node_labels(result: &HierarchicalResult, leaf_labels: &[usize]) -> Vec<Option<usize>> {
    let n = leaf_labels.len();
    if n == 0 {
        return Vec::new();
    }
    let mut node_label: Vec<Option<usize>> = vec![None; 2 * n - 1];
    for (i, &l) in leaf_labels.iter().enumerate() {
        node_label[i] = Some(l);
    }
    for (i, m) in result.merges.iter().enumerate() {
        node_label[n + i] = match (node_label[m.a], node_label[m.b]) {
            (Some(la), Some(lb)) if la == lb => Some(la),
            _ => None,
        };
    }
    node_label
}

/// 葉位置 `x`（`0..n_leaves-1`）・高さ `height`（`0..=max_height`）を画面座標へ写像する。
/// y は下が 0、上が `max_height`（画面は反転: 下端 = 0）。
fn to_screen(
    x: f64,
    height: f64,
    rect: egui::Rect,
    n_leaves: usize,
    max_height: f64,
) -> egui::Pos2 {
    let x_frac = if n_leaves <= 1 {
        0.5
    } else {
        x / (n_leaves - 1) as f64
    };
    let y_frac = if max_height <= 0.0 {
        0.0
    } else {
        height / max_height
    };
    egui::pos2(
        rect.left() + x_frac as f32 * rect.width(),
        rect.bottom() - y_frac as f32 * rect.height(),
    )
}

impl DendrogramChart {
    /// CSV エクスポート用: 葉順に (元 view の行インデックス, カット後クラスタラベル) を返す。
    pub fn leaf_assignments(&self) -> Option<Vec<(usize, usize)>> {
        let (_, result) = self.cache.as_ref()?;
        let n = result.leaf_order.len();
        if n == 0 {
            return None;
        }
        let labels = cut_tree(result, self.k.clamp(1, n));
        Some(
            result
                .leaf_order
                .iter()
                .map(|&leaf| (result.row_indices[leaf], labels[leaf]))
                .collect(),
        )
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        param_names: &[String],
        obj_names: &[String],
        study_name: &str,
    ) {
        ui.horizontal(|ui| {
            ui.label("Clusters (k):");
            ui.add(egui::Slider::new(&mut self.k, 1..=12));
            egui::ComboBox::from_id_salt("dendrogram_space")
                .selected_text(self.space.label())
                .show_ui(ui, |ui| {
                    for space in [
                        DendrogramSpace::Params,
                        DendrogramSpace::Objectives,
                        DendrogramSpace::All,
                    ] {
                        ui.selectable_value(&mut self.space, space, space.label());
                    }
                });
        });

        let features = feature_names(param_names, obj_names, self.space);
        if features.is_empty() {
            ui.colored_label(COLOR_EMPTY_STATE(), "No numeric columns available.");
            return;
        }

        let key: DendrogramCacheKey = (study_name.to_string(), view.row_count(), self.space.disc());
        if self.cache.as_ref().map(|(k, _)| k) != Some(&key) {
            let matrix = build_matrix(view, &features);
            self.cache = ward_linkage(&matrix, true).map(|r| (key, r));
        }

        let Some((_, result)) = &self.cache else {
            ui.colored_label(
                COLOR_EMPTY_STATE(),
                "Not enough data to build a dendrogram (need >= 2 rows).",
            );
            return;
        };

        let n = result.leaf_order.len();
        let k = self.k.clamp(1, n.max(1));
        let labels = cut_tree(result, k);
        let nodes = dendrogram_nodes(result);
        let node_label = compute_node_labels(result, &labels);
        let max_height = result.merges.last().map(|m| m.distance).unwrap_or(0.0);
        let threshold = cut_threshold(&result.merges, n, k);

        let cmap = ColorMap::turbo();
        let cluster_color = |label: usize| -> egui::Color32 { cmap.sample_categorical(label, k) };
        const ABOVE_CUT_COLOR: egui::Color32 = egui::Color32::from_gray(140);

        // サブサンプル時は下にキャプション行が続くため、その高さぶんを
        // 先に差し引いてからプロット領域を確保する（キャプションの見切れ防止）。
        let subsampled = result.row_indices.len() < view.row_count();
        let caption_h = if subsampled {
            ui.text_style_height(&egui::TextStyle::Body) + ui.spacing().item_spacing.y
        } else {
            0.0
        };
        let avail = ui.available_size();
        let size = egui::vec2(
            avail.x.max(240.0),
            (avail.y - caption_h).clamp(160.0, 420.0),
        );
        let (rect, _resp) = ui.allocate_exact_size(size, egui::Sense::hover());
        let painter = ui.painter_at(rect);

        for (i, node) in nodes.iter().enumerate() {
            let color = match node_label.get(n + i).copied().flatten() {
                Some(l) => cluster_color(l),
                None => ABOVE_CUT_COLOR,
            };
            let stroke = egui::Stroke::new(1.5, color);
            let a_bottom = to_screen(node.child_x.0, node.child_heights.0, rect, n, max_height);
            let a_top = to_screen(node.child_x.0, node.height, rect, n, max_height);
            let b_bottom = to_screen(node.child_x.1, node.child_heights.1, rect, n, max_height);
            let b_top = to_screen(node.child_x.1, node.height, rect, n, max_height);
            painter.line_segment([a_bottom, a_top], stroke);
            painter.line_segment([b_bottom, b_top], stroke);
            painter.line_segment([a_top, b_top], stroke);
        }

        if let Some(th) = threshold {
            let y = to_screen(0.0, th, rect, n, max_height).y;
            let dash_len = 6.0;
            let mut x = rect.left();
            while x < rect.right() {
                let x2 = (x + dash_len).min(rect.right());
                painter.line_segment(
                    [egui::pos2(x, y), egui::pos2(x2, y)],
                    egui::Stroke::new(1.0, crate::theme::chart_colors::COLOR_GRID_STROKE()),
                );
                x += dash_len * 2.0;
            }
        }

        if subsampled {
            ui.label(
                egui::RichText::new(format!(
                    "{} leaves (subsampled from {})",
                    result.row_indices.len(),
                    view.row_count()
                ))
                .weak(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dendrogram_chart_default_values() {
        let c = DendrogramChart::default();
        assert_eq!(c.k, 3);
        assert_eq!(c.space, DendrogramSpace::Params);
        assert!(c.cache.is_none());
    }

    #[test]
    fn dendrogram_space_disc_is_distinct() {
        let discs = [
            DendrogramSpace::Params.disc(),
            DendrogramSpace::Objectives.disc(),
            DendrogramSpace::All.disc(),
        ];
        assert_ne!(discs[0], discs[1]);
        assert_ne!(discs[1], discs[2]);
        assert_ne!(discs[0], discs[2]);
    }

    #[test]
    fn feature_names_variants() {
        let params = vec!["x".to_string()];
        let objs = vec!["obj".to_string()];
        assert_eq!(
            feature_names(&params, &objs, DendrogramSpace::Params),
            params
        );
        assert_eq!(
            feature_names(&params, &objs, DendrogramSpace::Objectives),
            objs
        );
        assert_eq!(
            feature_names(&params, &objs, DendrogramSpace::All),
            vec!["x".to_string(), "obj".to_string()]
        );
    }

    fn make_result(merges: Vec<Merge>, n: usize) -> HierarchicalResult {
        HierarchicalResult {
            merges,
            leaf_order: (0..n).collect(),
            row_indices: (0..n).collect(),
        }
    }

    #[test]
    fn cut_threshold_normal_case() {
        // 4 leaves -> 3 merges, ascending distance.
        let merges = vec![
            Merge {
                a: 0,
                b: 1,
                distance: 1.0,
                size: 2,
            },
            Merge {
                a: 2,
                b: 3,
                distance: 2.0,
                size: 2,
            },
            Merge {
                a: 4,
                b: 5,
                distance: 5.0,
                size: 4,
            },
        ];
        // k=2 -> cutoff = 4-2 = 2 -> midpoint(merges[1]=2.0, merges[2]=5.0) = 3.5
        let th = cut_threshold(&merges, 4, 2).unwrap();
        assert!((th - 3.5).abs() < 1e-9);
    }

    #[test]
    fn cut_threshold_k_one_is_none() {
        let merges = vec![Merge {
            a: 0,
            b: 1,
            distance: 1.0,
            size: 2,
        }];
        assert!(cut_threshold(&merges, 2, 1).is_none());
    }

    #[test]
    fn cut_threshold_k_at_least_n_is_none() {
        let merges = vec![Merge {
            a: 0,
            b: 1,
            distance: 1.0,
            size: 2,
        }];
        assert!(cut_threshold(&merges, 2, 2).is_none());
        assert!(cut_threshold(&merges, 2, 5).is_none());
    }

    #[test]
    fn compute_node_labels_propagates_matching_children() {
        // 4 leaves: (0,1) merge first, (2,3) merge second, then root merges both.
        let merges = vec![
            Merge {
                a: 0,
                b: 1,
                distance: 1.0,
                size: 2,
            }, // node 4
            Merge {
                a: 2,
                b: 3,
                distance: 2.0,
                size: 2,
            }, // node 5
            Merge {
                a: 4,
                b: 5,
                distance: 5.0,
                size: 4,
            }, // node 6 (root)
        ];
        let result = make_result(merges, 4);
        // k=2 cut: root merge (index 2) is dropped -> leaves split into {0,1} and {2,3}.
        let labels = cut_tree(&result, 2);
        let node_label = compute_node_labels(&result, &labels);
        // Leaves keep their own label.
        assert_eq!(node_label[0], Some(labels[0]));
        assert_eq!(node_label[2], Some(labels[2]));
        // node 4 = merge(0,1): both children share label -> Some.
        assert_eq!(node_label[4], Some(labels[0]));
        // node 5 = merge(2,3): both children share label -> Some.
        assert_eq!(node_label[5], Some(labels[2]));
        // node 6 = root merge(node4, node5): different clusters -> None (above cut).
        assert_eq!(node_label[6], None);
    }

    #[test]
    fn leaf_assignments_returns_none_without_cache() {
        let chart = DendrogramChart::default();
        assert!(chart.leaf_assignments().is_none());
    }

    #[test]
    fn build_matrix_skips_rows_with_non_finite_values() {
        use std::collections::HashMap;
        use std::sync::Arc;
        use tunny_core::dataframe::{DataFrame, TrialRow as CoreRow};

        let core_rows: Vec<CoreRow> = vec![
            CoreRow {
                trial_id: 0,
                trial_number: 0,
                param_display: [("x".to_string(), 1.0)].into(),
                param_category_label: HashMap::new(),
                objective_values: vec![],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            },
            CoreRow {
                trial_id: 1,
                trial_number: 1,
                param_display: [("x".to_string(), f64::NAN)].into(),
                param_category_label: HashMap::new(),
                objective_values: vec![],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            },
        ];
        let param_names = vec!["x".to_string()];
        let df = DataFrame::from_trials(&core_rows, &param_names, &[], &[], &[], 0);
        let view = StudyView::new(Arc::new(df), vec![0, 0]);
        let matrix = build_matrix(&view, &param_names);
        assert_eq!(matrix.len(), 1);
        assert_eq!(matrix[0], vec![1.0]);
    }
}
