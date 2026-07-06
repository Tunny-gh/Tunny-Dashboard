//! ピン留めした trial を横並びで比較する表ウィジェット。
//!
//! 行 = 目的関数 + (トグルで) 数値パラメータ + (トグルで) 数値ユーザー属性。
//! 列 = ピン留めしたトライアル。目的関数の行は highlight_best が有効なとき、
//! 方向（最小化/最大化）を考慮して最良セルを強調表示する。
//! カテゴリカルパラメータは数値列を持たないため、`build_rows` で自動的に除外される
//! （`trial_table` の全件一覧も同様に数値列のみを表示している）。
//! 詳細は `theory/{en,ja}/widgets/comparison-table.md` を参照。

use crate::state::types::{Direction, StudyView};
use crate::theme::chart_colors::COLOR_EMPTY_STATE;
use crate::theme::ACCENT_BLUE;

/// 比較表ウィジェットの UI 状態。計算キャッシュは持たない（表示のたびに軽量な再構築で十分なため）。
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ComparisonTableChart {
    /// 数値パラメータも行に含めるか。
    pub show_params: bool,
    /// 数値ユーザー属性も行に含めるか。
    pub show_user_attrs: bool,
    /// 目的関数行について、方向を考慮した最良セルを強調表示するか。
    pub highlight_best: bool,
}

impl Default for ComparisonTableChart {
    fn default() -> Self {
        Self {
            show_params: true,
            show_user_attrs: false,
            highlight_best: true,
        }
    }
}

/// 行の種別。目的関数行だけが `highlight_best` の対象になる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    /// `directions` 内のインデックス（反転判定に使う）。
    Objective(usize),
    Parameter,
    UserAttr,
}

/// 表 1 行ぶんの情報（列の借用込み）。`show` と CSV エクスポートで共有する。
pub struct RowInfo<'a> {
    pub label: &'a str,
    pub col: &'a [f64],
    pub kind: RowKind,
}

/// 行リストを構築する（目的関数 → 数値パラメータ → 数値ユーザー属性の順）。
/// 数値列を持たない列（カテゴリカルパラメータなど）はスキップする。
pub fn build_rows<'a>(
    view: &'a StudyView,
    param_names: &'a [String],
    obj_names: &'a [String],
    show_params: bool,
    show_user_attrs: bool,
) -> Vec<RowInfo<'a>> {
    let mut rows = Vec::with_capacity(obj_names.len() + param_names.len());
    for (i, name) in obj_names.iter().enumerate() {
        if let Some(col) = view.numeric_column(name) {
            rows.push(RowInfo {
                label: name,
                col,
                kind: RowKind::Objective(i),
            });
        }
    }
    if show_params {
        for name in param_names {
            if let Some(col) = view.numeric_column(name) {
                rows.push(RowInfo {
                    label: name,
                    col,
                    kind: RowKind::Parameter,
                });
            }
        }
    }
    if show_user_attrs {
        for name in view.df.user_attr_numeric_col_names() {
            if let Some(col) = view.numeric_column(name) {
                rows.push(RowInfo {
                    label: name,
                    col,
                    kind: RowKind::UserAttr,
                });
            }
        }
    }
    rows
}

/// ピン留めトライアルのうち、現在の view に存在するものだけを `(trial_id, row_index)` で解決する。
pub fn resolve_pinned_rows(view: &StudyView, pinned_trials: &[u32]) -> Vec<(u32, usize)> {
    pinned_trials
        .iter()
        .filter_map(|&trial_id| {
            view.trial_ids
                .iter()
                .position(|&t| t == trial_id)
                .map(|row| (trial_id, row))
        })
        .collect()
}

/// ピン留め列順の値から、方向を考慮した最良セルのインデックスを返す。
/// 非有限値は無視する。有限値が 1 つもなければ `None`。
pub fn best_pin_index(values: &[f64], direction: &Direction) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for (i, &v) in values.iter().enumerate() {
        if !v.is_finite() {
            continue;
        }
        let is_better = match &best {
            None => true,
            Some((_, b)) => match direction {
                Direction::Minimize => v < *b,
                Direction::Maximize => v > *b,
            },
        };
        if is_better {
            best = Some((i, v));
        }
    }
    best.map(|(i, _)| i)
}

impl ComparisonTableChart {
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        param_names: &[String],
        obj_names: &[String],
        directions: &[Direction],
        pinned_trials: &[u32],
    ) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_params, "Parameters");
            ui.checkbox(&mut self.show_user_attrs, "User attrs");
            ui.checkbox(&mut self.highlight_best, "Highlight best");
        });

        let pinned_rows = resolve_pinned_rows(view, pinned_trials);
        if pinned_rows.is_empty() {
            ui.vertical_centered(|ui| {
                ui.colored_label(
                    COLOR_EMPTY_STATE(),
                    "Pin trials (📌) in the Trial Table to compare them here.",
                );
            });
            return;
        }

        let rows = build_rows(
            view,
            param_names,
            obj_names,
            self.show_params,
            self.show_user_attrs,
        );
        if rows.is_empty() {
            ui.vertical_centered(|ui| {
                ui.colored_label(
                    COLOR_EMPTY_STATE(),
                    "No numeric columns available to compare.",
                );
            });
            return;
        }

        use egui_extras::{Column, TableBuilder};

        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.visuals_mut().faint_bg_color = crate::theme::TABLE_STRIPE_BG();
            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .column(Column::initial(140.0).at_least(80.0))
                .columns(Column::initial(90.0).at_least(50.0), pinned_rows.len())
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("");
                    });
                    for &(trial_id, row) in &pinned_rows {
                        let number = view.df.get_trial_number(row).unwrap_or(trial_id);
                        header.col(|ui| {
                            ui.strong(format!("Trial #{number}"));
                        });
                    }
                })
                .body(|body| {
                    body.rows(18.0, rows.len(), |mut row_ui| {
                        let info = &rows[row_ui.index()];
                        row_ui.col(|ui| {
                            ui.label(info.label);
                        });

                        let values: Vec<f64> = pinned_rows
                            .iter()
                            .map(|&(_, r)| info.col.get(r).copied().unwrap_or(f64::NAN))
                            .collect();
                        let best_idx = if self.highlight_best {
                            match info.kind {
                                RowKind::Objective(obj_idx) => directions
                                    .get(obj_idx)
                                    .and_then(|d| best_pin_index(&values, d)),
                                _ => None,
                            }
                        } else {
                            None
                        };

                        for (ci, &v) in values.iter().enumerate() {
                            row_ui.col(|ui| {
                                let text = if v.is_finite() {
                                    format!("{v:.6}")
                                } else {
                                    "—".to_string()
                                };
                                if Some(ci) == best_idx {
                                    ui.label(
                                        egui::RichText::new(text).strong().color(ACCENT_BLUE()),
                                    );
                                } else {
                                    ui.label(text);
                                }
                            });
                        }
                    });
                });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_show_params_and_highlight_best() {
        let chart = ComparisonTableChart::default();
        assert!(chart.show_params);
        assert!(!chart.show_user_attrs);
        assert!(chart.highlight_best);
    }

    #[test]
    fn best_pin_index_minimize_picks_smallest() {
        let values = [3.0, 1.0, 2.0];
        assert_eq!(best_pin_index(&values, &Direction::Minimize), Some(1));
    }

    #[test]
    fn best_pin_index_maximize_picks_largest() {
        let values = [3.0, 1.0, 2.0];
        assert_eq!(best_pin_index(&values, &Direction::Maximize), Some(0));
    }

    #[test]
    fn best_pin_index_ignores_non_finite() {
        let values = [f64::NAN, 1.0, f64::INFINITY];
        assert_eq!(best_pin_index(&values, &Direction::Minimize), Some(1));
    }

    #[test]
    fn best_pin_index_all_non_finite_returns_none() {
        let values = [f64::NAN, f64::INFINITY];
        assert_eq!(best_pin_index(&values, &Direction::Minimize), None);
    }

    #[test]
    fn best_pin_index_empty_returns_none() {
        let values: [f64; 0] = [];
        assert_eq!(best_pin_index(&values, &Direction::Minimize), None);
    }
}
