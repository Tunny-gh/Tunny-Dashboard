# 制約条件を考慮した可視化 実装ガイド

**作成日**: 2026-06-03
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連データフロー**: [dataflow.md](dataflow.md)

本ドキュメントは Rust 実装者向けの具体的な変更手順と実装例を示す。

---

## 変更 1: `COLOR_INFEASIBLE` の追加 🔵

**ファイル**: `egui-app/src/theme/chart_colors.rs`

```rust
// ========================================
// 制約違反（実行不可能解）
// ========================================

/// 実行不可能解のグレーアウト色（premultiplied）
/// premultiplied: rgb(180,180,180) × 80/255 ≈ 56,56,56
pub const COLOR_INFEASIBLE: Color32 = Color32::from_rgba_premultiplied(56, 56, 56, 80);
```

**追加位置**: ファイル末尾または「スキャッタ系」セクションの近く

---

## 変更 2: `compute_pareto_ranks()` の修正 🔵

**ファイル**: `rust_core/src/multi_objective/pareto/ranking.rs`

```rust
pub fn compute_pareto_ranks(is_minimize: &[bool]) -> ParetoResult {
    crate::dataframe::with_active_df(|df| {
        let obj_names = df.objective_col_names();
        let m = obj_names.len();
        let n = df.row_count();
        if n == 0 || m == 0 {
            return ParetoResult { ranks: vec![], pareto_indices: vec![], hypervolume: None };
        }

        let obj_cols: Vec<Option<&[f64]>> = obj_names
            .iter()
            .map(|name| df.get_numeric_column(name))
            .collect();

        // ★ 追加: feasibility 情報を取得 ★
        let is_feasible_col = df.get_numeric_column("is_feasible");
        let constraint_sum_col = df.get_numeric_column("constraint_sum");
        let has_constraints = is_feasible_col.is_some();

        if !has_constraints {
            // 制約なし: 従来フロー
            let objectives: Vec<Vec<f64>> = (0..n)
                .map(|row| obj_cols.iter()
                    .map(|col| col.and_then(|c| c.get(row)).copied().unwrap_or(f64::NAN))
                    .collect())
                .collect();
            let ranks = nd_sort(&objectives, is_minimize);
            let pareto_indices: Vec<u32> = ranks.iter().enumerate()
                .filter(|(_, &r)| r == 0).map(|(i, _)| i as u32).collect();
            let hypervolume = compute_hypervolume(&pareto_indices, &objectives, is_minimize, m);
            return ParetoResult { ranks, pareto_indices, hypervolume };
        }

        // ★ 制約あり: feasible/infeasible 分離フロー ★

        // 全行の目的値を構築
        let objectives: Vec<Vec<f64>> = (0..n)
            .map(|row| obj_cols.iter()
                .map(|col| col.and_then(|c| c.get(row)).copied().unwrap_or(f64::NAN))
                .collect())
            .collect();

        let is_feasible_vals = is_feasible_col.unwrap();

        // feasible 行のインデックスと目的値を抽出
        let feasible_indices: Vec<usize> = (0..n)
            .filter(|&i| is_feasible_vals.get(i).copied().unwrap_or(1.0) > 0.5)
            .collect();
        let feasible_objectives: Vec<Vec<f64>> = feasible_indices.iter()
            .map(|&i| objectives[i].clone())
            .collect();

        let mut ranks = vec![0u32; n];

        // feasible 行のみで nd_sort
        let max_feasible_rank = if feasible_objectives.is_empty() {
            0u32
        } else {
            let feasible_ranks = nd_sort(&feasible_objectives, is_minimize);
            let max_r = feasible_ranks.iter().max().copied().unwrap_or(0);
            for (k, &orig_idx) in feasible_indices.iter().enumerate() {
                ranks[orig_idx] = feasible_ranks[k];
            }
            max_r
        };

        // infeasible 行を constraint_sum 昇順でランク付け
        let mut infeasible_with_sum: Vec<(usize, f64)> = (0..n)
            .filter(|&i| is_feasible_vals.get(i).copied().unwrap_or(1.0) <= 0.5)
            .map(|i| {
                let sum = constraint_sum_col
                    .and_then(|col| col.get(i))
                    .copied()
                    .unwrap_or(0.0);
                (i, sum)
            })
            .collect();
        infeasible_with_sum.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        for (violation_rank, (orig_idx, _)) in infeasible_with_sum.iter().enumerate() {
            ranks[*orig_idx] = max_feasible_rank + 1 + violation_rank as u32;
        }

        // pareto_indices: feasible の rank == 0
        let pareto_indices: Vec<u32> = feasible_indices.iter()
            .filter(|&&i| ranks[i] == 0)
            .map(|&i| i as u32)
            .collect();

        let hypervolume = compute_hypervolume(&pareto_indices, &objectives, is_minimize, m);

        ParetoResult { ranks, pareto_indices, hypervolume }
    })
    .unwrap_or(ParetoResult { ranks: vec![], pareto_indices: vec![], hypervolume: None })
}

/// Hypervolume 計算のヘルパー（共通化）
fn compute_hypervolume(
    pareto_indices: &[u32],
    objectives: &[Vec<f64>],
    is_minimize: &[bool],
    m: usize,
) -> Option<f64> {
    if m >= 2 && pareto_indices.len() >= 2 {
        let pareto_objs: Vec<Vec<f64>> = pareto_indices.iter()
            .map(|&i| objectives[i as usize].clone())
            .collect();
        let norm_pareto = normalize_objectives(&pareto_objs, is_minimize);
        let ref_pt = compute_ref_point(&norm_pareto, m);
        let pts_2d: Vec<(f64, f64)> = norm_pareto.iter().map(|obj| (obj[0], obj[1])).collect();
        Some(hypervolume_2d(&pts_2d, ref_pt[0], ref_pt[1]))
    } else {
        None
    }
}
```

**注意**: 既存の `compute_pareto_ranks()` のコードを上記に置き換える。`use` 文に変更は不要（既存 import を流用）。

---

## 変更 3: ウィジェット構造体への `show_infeasible` 追加 🔵

### 3-1: ParetoScatter2D

**ファイル**: `egui-app/src/ui/widgets/pareto_2d.rs`

```rust
pub struct ParetoScatter2D {
    pub x_axis: String,
    pub y_axis: String,
    pub use_downsample: bool,
    pub brush_start: Option<[f64; 2]>,
    pub brush_end: Option<[f64; 2]>,
    pub show_infeasible: bool,  // ★ 追加
}

impl Default for ParetoScatter2D {
    fn default() -> Self {
        Self {
            x_axis: "obj0".to_string(),
            y_axis: "obj1".to_string(),
            use_downsample: true,
            brush_start: None,
            brush_end: None,
            show_infeasible: true,  // ★ 追加
        }
    }
}
```

### 3-2: OptimizationHistoryChart

**ファイル**: `egui-app/src/ui/widgets/optimization_history.rs`

```rust
pub struct OptimizationHistoryChart {
    // ... 既存フィールド ...
    pub show_infeasible: bool,  // ★ 追加
}

impl Default for OptimizationHistoryChart {
    fn default() -> Self {
        Self {
            // ... 既存デフォルト値 ...
            show_infeasible: true,  // ★ 追加
        }
    }
}
```

### 3-3: ParallelCoordsChart / ScatterMatrix / ClusterScatter / Pareto3dChart

同様のパターンで `show_infeasible: bool` フィールドと `Default::default()` で `true` を追加。

---

## 変更 4: ParetoScatter2D 描画ロジックの修正 🔵

**ファイル**: `egui-app/src/ui/widgets/pareto_2d.rs`

`show()` メソッド内の描画ループを以下のように変更：

```rust
// ★ 追加: feasibility 情報取得
let is_feasible_col = view.numeric_column("is_feasible");
let has_constraints = ctx.meta.has_constraints;

// 点群の分類（既存変数に加え infeasible_pts を追加）
let mut pareto_pts: Vec<[f64; 2]> = Vec::new();
let mut pareto_pts_dim: Vec<[f64; 2]> = Vec::new();
let mut non_pareto_pts: Vec<[f64; 2]> = Vec::new();
let mut non_pareto_pts_dim: Vec<[f64; 2]> = Vec::new();
let mut infeasible_pts: Vec<[f64; 2]> = Vec::new();  // ★ 追加
let mut highlight_pt: Option<[f64; 2]> = None;
let mut displayed_points: Vec<(u32, [f64; 2])> = Vec::new();

for i in displayed {
    let x = x_col.and_then(|c| c.get(i)).copied().unwrap_or(0.0);
    let y = y_col.and_then(|c| c.get(i)).copied().unwrap_or(0.0);
    let pt = [x, y];
    let trial_id = view.trial_ids.get(i).copied().unwrap_or(i as u32);
    let rank = view.pareto_rank.get(i).copied().unwrap_or(0);

    // ★ 追加: feasibility チェック
    let feasible = is_feasible_col
        .and_then(|col| col.get(i))
        .map(|&v| v > 0.5)
        .unwrap_or(true);

    if !feasible {
        if self.show_infeasible {
            infeasible_pts.push(pt);
            displayed_points.push((trial_id, pt));
        }
        continue;
    }

    // 既存のロジック（変更なし）
    displayed_points.push((trial_id, pt));
    if highlighted == Some(trial_id) {
        highlight_pt = Some(pt);
        continue;
    }
    // ... 以下既存コード ...
}

// ★ 描画部分の変更: infeasible を最初（背面）に描画
plot_ui.points(
    egui_plot::Points::new(infeasible_pts)
        .name("Infeasible")
        .color(COLOR_INFEASIBLE)
        .radius(2.5),
);
// 続いて既存の non_pareto_pts_dim, non_pareto_pts, pareto_pts_dim, pareto_pts を描画
// ...
```

**UI コントロール行への追加**:

```rust
// X/Y 軸選択の horizontal ブロック内に追加
ui.horizontal(|ui| {
    ui.label("X Axis:");
    // ... 既存の ComboBox ...
    ui.label("Y Axis:");
    // ... 既存の ComboBox ...

    // ★ 追加: Show Infeasible トグル（制約あり Study のみ）
    if has_constraints {
        ui.separator();
        ui.checkbox(&mut self.show_infeasible, "Show Infeasible");
    }
});
```

---

## 変更 5: OptimizationHistory 描画ロジックの修正 🔵

**ファイル**: `egui-app/src/ui/widgets/optimization_history.rs`

`show_with_history()` 内で trial 点を描画するループに feasibility チェックを追加：

```rust
// ★ 追加
let is_feasible_col = view.numeric_column("is_feasible");
let has_constraints_flag = /* meta.has_constraints を引数で受け取るか view から判断 */;

// 点描画ループ内
let feasible = is_feasible_col
    .and_then(|col| col.get(i))
    .map(|&v| v > 0.5)
    .unwrap_or(true);

let point_color = if !feasible {
    if !self.show_infeasible { continue; }
    COLOR_INFEASIBLE
} else {
    COLOR_OPT_TRIAL // 通常色
};
```

**注意**: `OptimizationHistory` は現在 `view: &StudyView` を受け取るが、`has_constraints` の判断に `StudyMeta` も必要になる。`show()` のシグネチャに `has_constraints: bool` を追加するか、`view.df.constraint_col_names().is_empty()` で判断する。

---

## 変更 6: ParallelCoordinates / ScatterMatrix / ClusterScatter 🟡

**信頼性**: 🟡 *各ウィジェットのコード構造から妥当な推測（詳細はコード確認後に調整）*

それぞれのウィジェットで、線分または点の描画ループに同様の feasibility チェックを追加。
パターンは `pareto_2d.rs` と同じ：

```rust
let feasible = is_feasible_col
    .and_then(|col| col.get(i))
    .map(|&v| v > 0.5)
    .unwrap_or(true);

if !feasible {
    if !self.show_infeasible { continue; }
    // COLOR_INFEASIBLE で描画
}
```

---

## 変更 7: message_handler.rs での show_infeasible リセット 🔵

**ファイル**: `egui-app/src/state/message_handler.rs`

既存の `StudySelected` 処理に追加（L49–53 付近）：

```rust
AppMessage::StudySelected { meta, study_id, pareto_rank, pareto_indices } => {
    // ... 既存コード ...
    widget_states.hv_history.computing = false;
    widget_states.ahp_chart = Default::default();
    widget_states.cluster_scatter = Default::default();
    // ★ 追加: show_infeasible のリセット（Default::default() で true に戻る）
    widget_states.pareto_2d = Default::default();
    widget_states.opt_history = Default::default();
    widget_states.parallel_coords = Default::default();
    widget_states.scatter_matrix = Default::default();
    // pareto_3d は GPU リソースを保持するため show_infeasible のみリセット
    widget_states.pareto_3d.show_infeasible = true;
    // ...
}
```

**注意**: `ParetoScatter2D` に `Default::default()` を代入すると `x_axis`/`y_axis` もリセットされる。  
Study 切替時に軸選択をリセットしてよいか（既存の挙動と同じ）を確認のこと。

---

## テスト追加指針

### rust_core の unit test 🔵

**ファイル**: `rust_core/src/multi_objective/pareto/tests.rs`

```rust
#[test]
fn feasible_only_pareto_excludes_infeasible() {
    // DataFrame に constraint あり trial を含むセットアップ
    // compute_pareto_ranks() 呼び出し後
    // infeasible trial が pareto_indices に含まれないことを検証
}

#[test]
fn infeasible_ranked_by_constraint_sum() {
    // infeasible trial が constraint_sum 昇順にランク付けされることを検証
}
```

### egui-app の統合テスト 🟡

`egui-app/tests/` に visual/unit テストを追加（既存テスト構造に合わせる）。
