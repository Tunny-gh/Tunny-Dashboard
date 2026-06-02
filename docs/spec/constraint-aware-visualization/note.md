# 制約条件を考慮した可視化 — コンテキストノート

**生成日**: 2026-06-03

## 技術スタック

- **言語**: Rust 2021 edition
- **GUI**: egui + egui_plot（WASMなし、ネイティブデスクトップのみ）
- **グラフィクス**: wgpu（3D）、egui_plot（2D）
- **クレート構成**:
  - `rust_core` — データ処理ライブラリ（パーサー・DataFrame・分析）
  - `egui-app` — デスクトップ GUI

## 関連実装（既存コード）

| ファイル | 内容 |
|---|---|
| `rust_core/src/io/journal/parser/state.rs` L169–180, 289–295 | `system_attrs.constraints` をパースし `constraint_values` / `has_constraints` に格納 |
| `rust_core/src/data/dataframe/model.rs` L128–158 | `DataFrame::from_trials` で `is_feasible`（1.0/0.0）と `constraint_sum` 派生列を生成 |
| `egui-app/src/state/types.rs` L36 | `StudyMeta.has_constraints: bool` |
| `egui-app/src/state/types.rs` L151–255 | `StudyView`: `Arc<DataFrame>` + 並行配列（`pareto_rank`, `cluster_id`, `state`） |
| `egui-app/src/state/types.rs` L192–195 | `StudyView::numeric_column("is_feasible")` で実行可能性フラグを取得可能 |
| `egui-app/src/theme/chart_colors.rs` | カラー定数（Color32）の定義場所 |

## 制約値の仕様

- **JSON フォーマット**: `trial.system_attrs.constraints` = 数値配列
- **実行可能解**: 配列内の全値が `<= 0.0`
- **実行不可能解**: 配列内に `> 0.0` の値が1つ以上存在
- **`is_feasible` 列**: 実行可能 = `1.0`、実行不可能 = `0.0`
- **制約なし Study**: `has_constraints = false`。`is_feasible` 列は生成されない

## 既存チャートのレンダリング構造

各チャートは `render_chart.rs` の `render_chart()` から呼び出される。
チャートウィジェットは `egui-app/src/ui/widgets/` 配下の `.rs` ファイルに実装。
`StudyView` を通じて列データへアクセス（例: `ctx.view.numeric_column("is_feasible")`）。

## ウィジェット状態管理

グローバルなウィジェット状態は `widget_states.rs` の `WidgetStates` 構造体で管理。
各チャートごとのトグル状態（show_infeasible）は当該ウィジェット構造体のフィールドとして追加する。
