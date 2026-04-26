# プロプライエタリ分析ツール不足機能 コンテキストノート

**生成日**: 2026-04-26

## プロジェクト基本情報

- **リポジトリ**: c:\Users\hiroa\Desktop\Tunny-Dashboard
- **技術スタック**: Rust (egui-app) / Rust WASM rust_core / Cargo workspace
- **UI フレームワーク**: egui（`egui-app` クレート）
- **分析コア**: rust_core クレート（WASM 共有ライブラリ）
- **ビルドターゲット**: デスクトップネイティブ（egui）

## 既実装チャート一覧（README 記載通り動作）

| ChartId | 実装ファイル | 状態 |
|---|---|---|
| `pareto-2d` | `egui-app/src/ui/widgets/pareto_2d.rs` | ✓ 完全 |
| `pareto-3d` | `egui-app/src/ui/widgets/pareto_3d.rs` | ✓ 完全 |
| `optimization-history` | `egui-app/src/ui/widgets/optimization_history.rs` | ✓ 完全 |
| `hv-history` | `egui-app/src/ui/widgets/hv_history.rs` | ✓ 完全 |
| `parallel-coords` | `egui-app/src/ui/widgets/parallel_coords.rs` | ✓ 完全 |
| `scatter-matrix` | `egui-app/src/ui/widgets/scatter_matrix.rs` | ✓ 完全 |
| `importance-chart` | `egui-app/src/ui/widgets/importance_chart.rs` | ✓ 完全 |
| `sensitivity-heatmap` | `egui-app/src/ui/widgets/sensitivity_heatmap.rs` | ✓ 完全 |
| `pdp-chart` | `egui-app/src/ui/widgets/pdp_chart.rs` | ✓ 完全 |
| `pdp-2d` | `egui-app/src/ui/widgets/pdp_2d.rs` | ✓ 完全 |
| `cluster-scatter` | `egui-app/src/ui/widgets/cluster_scatter.rs` | ✓ 完全 |
| `mcdm-chart` | `egui-app/src/ui/widgets/mcdm_chart.rs` | ✓ TOPSIS/VIKOR |
| `trial-table` | `egui-app/src/ui/widgets/trial_table.rs` | ✓ 完全 |

## 未実装機能の対応 Rust 関数

### trade-off Navigator

```rust
// rust_core/src/multi_objective/pareto/tradeoff.rs
pub fn score_tradeoff_navigator(weights: &[f64]) -> Option<TradeoffNavigatorResult>
// → 各 trial の重み付きチェビシェフスコアを返す
```

### Hypervolume 履歴

```rust
// rust_core/src/multi_objective/pareto/hypervolume.rs
// 世代別 HV 計算は既存関数で対応可能（HV 推移は試行ごとに計算済み）
```

### ライブ更新

```rust
// rust_core/src/io/journal/live_update.rs
pub fn append_journal_diff(data: &[u8]) -> Result<DiffResult, Error>
// → 差分バイト列を受け取り、新規 trial のみ追加
```

## 状態管理の現状（egui-app）

| State ファイル | 役割 | 状態 |
|---|---|---|
| `egui-app/src/state/app_state.rs` | アプリ全体の状態 | ✓ |
| `egui-app/src/state/filter.rs` | フィルタ範囲・選択インデックス | ✓ |
| `egui-app/src/state/layout_state.rs` | レイアウト構成（Grid） | ✓（保存機能なし） |
| `egui-app/src/state/results.rs` | LiveUpdate State | ⚠️ 部分 |

## 重要な実装注意事項

### アーティファクト連携のフォルダ検出

- Journal ファイルと同ディレクトリの `artifacts/` フォルダを自動検出するのがデフォルト挙動
- ToolBar に「Artifacts フォルダを選択」ボタンで手動上書きも可能
- `set_trial_system_attr` の `key="artifacts"` から artifact_id → ファイルパスのマッピングを構築

### セッション保存ファイル形式（拡張子 .tdash）

```json
{
  "version": "1.0",
  "created_at": "ISO8601",
  "journal_filename": "result.log",
  "selected_study_id": 1,
  "filter_ranges": { "x1": [2.0, 8.0] },
  "selected_indices": [1024, 2891],
  "color_mode": "cluster",
  "cluster_config": { "space": "objective", "k": 4 },
  "layout_mode": "FreeLayout",
  "layout_config": {},
  "pinned_trials": [],
  "tradeoff_weights": [0.5, 0.5]
}
```

### HTML レポートのスコープ

- インタラクティブ HTML は **スコープ外**（ユーザーヒアリングで「静的レポート（SVG+テーブル）」を選択）
- Export パネル（現行の CSV エクスポートの隣）に HTML エクスポートボタンを追加
- 内容: 現在のチャート SVG キャプチャ + 選択試行のサマリーテーブル + 統計情報

### Parallel Coordinates 軸制御

- 現在 `parallel_coords.rs` に軸の reorder・visibility toggle が未実装
- 各軸ヘッダーに目のアイコン（表示/非表示）と掴みハンドル（並び替え）を追加

### 複数 Study 比較

- 現在 Toolbar に Study セレクターがあるが、複数同時選択は未対応
- 比較用専用 Panel（ComparisonPanel）を Mode D に追加するか、専用 Layout Mode を追加する
