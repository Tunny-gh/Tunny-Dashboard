---
name: chart-csv-export-note
description: 開発コンテキストノート - 各チャートへのCSVダウンロードボタン追加
metadata:
  type: project
---

# chart-csv-export 開発コンテキストノート

**作成日**: 2026-05-28

## 技術スタック

- **言語**: Rust (Edition 2021)
- **UIフレームワーク**: egui + eframe
- **ターゲット**: ネイティブデスクトップ (Windows/Mac/Linux)
- **WASMなし**: cfg(wasm32)不要、ネイティブAPIを自由に使用可
- **ファイルダイアログ**: `rfd` クレート（既存利用中）
- **ワークスペース**: Cargo workspace (`egui-app`, `rust_core`)

## 関連既存実装

### CSV エクスポートインフラ
- `egui-app/src/io/export.rs`: `build_csv_string()`, `save_csv_to_file()`, `write_csv_to_path()` が実装済み
  - `save_csv_to_file()`: rfd ファイルダイアログを開いて保存
  - `build_csv_string()`: TrialRow スライスから CSV 文字列を生成
  - 既存関数はトライアル全体データ用（TrialRow ベース）

### チャートセル構造
- `egui-app/src/ui/grid_canvas.rs`: セルツールバーの実装
  - `show_cell_toolbar()`: Move / ⋯メニュー（Save as PNG, Help） / × ボタン
  - `CellToolbarAction` enum: None / Close / Help / SaveAsPng
  - `handle_toolbar_action()`: アクション処理
  - 新しい `SaveAsCsv` バリアントを同パターンで追加する

### チャート描画
- `egui-app/src/ui/render_chart.rs`: `render_chart()` - ChartId に対応するウィジェットを描画
- `egui-app/src/ui/chart_registry.rs`: `show_chart()` / `show_cell_chart()`
- `egui-app/src/state/layout_state.rs`: `ChartId` enum (18種類)

### チャートウィジェット（データ保持）
- `egui-app/src/ui/widget_states.rs`: `WidgetStates` に各チャートのUI状態を保持
- 各ウィジェット (`ui/widgets/`) が計算結果を保持

### AppState（データ源）
- `egui-app/src/state/app_state.rs`: `AppState` - 全試行データ・解析結果を保持
  - `current_study`: StudyContext（trial_rows, meta）
  - `mcdm_result`, `ahp_result`, `hv_history`, `best_trial_history` 等

## チャート別 CSV 対象データ

| ChartId | CSV対象データ | 列例 |
|---------|------------|------|
| OptimizationHistory | 試行ごとの目的値系列 | trial_idx, all_trials, best_value, moving_avg |
| HvHistory | 試行ごとのHV値 | trial_idx, hypervolume |
| ImportanceChart | 変数重要度スコア | variable, importance_score, method |
| PdpChart | 変数値vs目的値予測 | variable, variable_value, predicted_objective |
| PdpChart2D | 2変数PDP（グリッドは難しいため生データ） | param1_value, param2_value, predicted_obj |
| ParallelCoordinates | 全試行データ | trial_id + params + objectives |
| ScatterMatrix | 全試行データ | trial_id + params + objectives |
| SensitivityHeatmap | 感度行列 | variable, objective, sensitivity_index |
| ClusterScatter | クラスタ付き試行 | trial_id + params + objectives + cluster_id |
| ParetoScatter2D | パレートフロント試行 | trial_id + objectives (filtered to pareto) |
| ParetoScatter3D | パレートフロント試行（3D） | trial_id + objectives (filtered to pareto) |
| McdmRankChart | MCDMランキング結果 | trial_id, rank, score |
| McdmScatterChart | MCDMスキャッタデータ | trial_id, x_score, y_score |
| McdmTable | MCDMテーブル全データ | trial_id + mcdm scores + rank |
| AhpRankChart | AHPランキング結果 | trial_id, rank, ahp_score |
| AhpTable | AHPテーブル全データ | trial_id + ahp values |
| SliceChart | スライスチャートデータ | variable, variable_value, objective |
| **SurfacePlot** | **スキップ（3Dグリッドデータ）** | - |

## 追加ルール

- チャートデータが未計算・空の場合はボタンをグレーアウトし、hover tooltip "No data available" を表示
- ファイルダイアログのデフォルトファイル名: `{chart_label}.csv`（例: `optimization_history.csv`）
- CSV ヘッダーは英語小文字スネークケース
- 既存の `save_csv_to_file()` を最大限再利用する
