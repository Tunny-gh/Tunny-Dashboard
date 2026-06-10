//! 実行可能性（制約充足）判定の一元化。
//!
//! `is_feasible` 派生列（1.0 = 実行可能 / 0.0 = 実行不可能、制約付きスタディのみ存在）
//! へのアクセスを `Feasibility` ビューに集約する。列名・閾値（> 0.5）・
//! 「列なし = 全行実行可能」のフォールバック規則をここ 1 箇所で定義し、
//! 各チャート・計算経路はこのビュー経由で判定する。

use super::model::DataFrame;

/// `is_feasible` 列の名前（DataFrame 構築時の派生列名と一致させること）。
pub(crate) const IS_FEASIBLE_COL: &str = "is_feasible";

/// 実行可能性ビュー。`DataFrame::feasibility()` から取得する。
/// 列スライスを直接ラップできるため、テストや列だけ持つ場面では
/// [`Feasibility::from_column`] で構築できる。
#[derive(Clone, Copy)]
pub struct Feasibility<'a> {
    col: Option<&'a [f64]>,
}

impl<'a> Feasibility<'a> {
    /// `is_feasible` 列スライス（無ければ `None`）から構築する。
    pub fn from_column(col: Option<&'a [f64]>) -> Self {
        Self { col }
    }

    /// スタディに制約が定義されているか（= `is_feasible` 列が存在するか）。
    pub fn has_constraints(&self) -> bool {
        self.col.is_some()
    }

    /// 指定行が実行可能か。列が無い（制約なし）・行が範囲外の場合は `true`。
    pub fn is_feasible(&self, row: usize) -> bool {
        self.col
            .and_then(|c| c.get(row))
            .map(|&v| v > 0.5)
            .unwrap_or(true)
    }

    /// 行 `0..n` を（実行可能, 実行不可能）のインデックス列に分割する。
    pub fn partition_indices(&self, n: usize) -> (Vec<usize>, Vec<usize>) {
        let mut feasible = Vec::with_capacity(n);
        let mut infeasible = Vec::new();
        for i in 0..n {
            if self.is_feasible(i) {
                feasible.push(i);
            } else {
                infeasible.push(i);
            }
        }
        (feasible, infeasible)
    }
}

impl DataFrame {
    /// この DataFrame の実行可能性ビューを返す。
    pub fn feasibility(&self) -> Feasibility<'_> {
        Feasibility::from_column(self.get_numeric_column(IS_FEASIBLE_COL))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_column_means_unconstrained_and_all_feasible() {
        let feas = Feasibility::from_column(None);
        assert!(!feas.has_constraints());
        assert!(feas.is_feasible(0));
        assert!(feas.is_feasible(999));
        let (f, inf) = feas.partition_indices(3);
        assert_eq!(f, vec![0, 1, 2]);
        assert!(inf.is_empty());
    }

    #[test]
    fn threshold_is_half() {
        let col = vec![1.0, 0.0, 0.6, 0.5];
        let feas = Feasibility::from_column(Some(&col));
        assert!(feas.has_constraints());
        assert!(feas.is_feasible(0));
        assert!(!feas.is_feasible(1));
        assert!(feas.is_feasible(2));
        assert!(!feas.is_feasible(3)); // ちょうど 0.5 は不可能側
    }

    #[test]
    fn out_of_range_row_defaults_to_feasible() {
        let col = vec![0.0];
        let feas = Feasibility::from_column(Some(&col));
        assert!(feas.is_feasible(5));
    }

    #[test]
    fn partition_indices_splits_correctly() {
        let col = vec![1.0, 0.0, 1.0];
        let feas = Feasibility::from_column(Some(&col));
        let (f, inf) = feas.partition_indices(3);
        assert_eq!(f, vec![0, 2]);
        assert_eq!(inf, vec![1]);
    }
}
