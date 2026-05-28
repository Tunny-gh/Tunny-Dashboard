# chart-csv-export 設計ヒアリング記録

**作成日**: 2026-05-28
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

既存のコードベース（`grid_canvas.rs`・`io/export.rs`・各ウィジェット）を調査し、CSV エクスポート機能の技術設計における不明点・設計判断事項を明確化するためのヒアリングを実施しました。

---

## 質問と回答

### Q1: CSV生成ロジックの配置方針について

**カテゴリ**: アーキテクチャ
**背景**: チャート固有のCSV生成ロジックの配置には2つの選択肢があった。
1. `io/csv_export.rs` に一括dispatch（ウィジェットはUI専用）
2. 各ウィジェット struct に `to_csv()` メソッドを追加（凝集度は高いが io 層分離が崩れる）

既存コードを見ると、ウィジェットはUI描画に特化しており、データ変換は `io/` 層で行うパターンが取られていた（例: `io/export.rs` の `build_csv_string()`）。

**回答**: `io/csv_export.rs` に一括dispatch（推奨）— ウィジェットはUI専用とし、CSV生成ロジックは io 層に集約する。

**信頼性への影響**:
- アーキテクチャ設計の信頼性レベルが 🟡 → 🔵 に向上
- 既存の `io/export.rs` パターンとの一貫性が確保された

---

## コードベース調査で確認した設計上の注意事項

### 注意1: handle_toolbar_action() のシグネチャ変更が必要 🔵

`handle_toolbar_action()` は現在 `app_state: &AppState` を受け取っていない。
`SaveAsCsv` ハンドリングのために `app_state` が必要なため、シグネチャ変更が必要。

```rust
// 現在
fn handle_toolbar_action(
    action: &CellToolbarAction,
    help_language: HelpLanguage,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
)

// 変更後
fn handle_toolbar_action(
    action: &CellToolbarAction,
    help_language: HelpLanguage,
    widgets: &mut WidgetStates,
    app_state: &AppState,   // 追加
    tx: &mpsc::SyncSender<AppMessage>,
)
```

呼び出し元 `render_cell_content()` は既に `app_state: &mut AppState` を受け取っているため、変更は最小限。

### 注意2: has_csv_data() の渡し方に設計判断が必要 🟡

ボタングレーアウト判定のために `show_cell_toolbar()` が `has_csv_data` の結果を知る必要がある。しかし `show_cell_toolbar()` は現在 `app_state` も `widgets` も受け取らない。

**設計案A**: `show_cell_toolbar()` に `csv_available: bool` 引数を追加（シンプル）
**設計案B**: `CellToolbarOptions` 構造体を導入して拡張性を確保（将来の追加に強い）

今回はシンプルさを優先して案A（`bool` 引数追加）を採用。 🟡

```rust
fn show_cell_toolbar(
    ui: &mut egui::Ui,
    row: usize,
    col: usize,
    item: PanelItem,
    title: &'static str,
    csv_available: bool,    // 追加
) -> CellToolbarAction
```

### 注意3: 既存テスト `menu_contains_save_as_png_and_help` の更新 🔵

`chart_cell_menu_items()` の戻り値に "Save as CSV" を追加した後、既存テストが変わる可能性:
- `items.contains(&"Save as PNG")` → 影響なし（contains なので）
- `items.contains(&"Help")` → 影響なし
- 新規テスト: `items.contains(&"Save as CSV")` を追加する

### 注意4: PDP キャッシュの構造確認が必要 🟡

`widgets.pdp_chart` の内部にある PDP 結果キャッシュの型が `state/messages.rs` の `PdpResult` / `PdpResult1d` によって決まる。実装前に型を確認してから CSV 列設計を確定させること。

### 注意5: McdmScatterChart のデータ源 🟡

`McdmScatterChart` の `show()` が参照するデータが `app_state.mcdm_result` の `primary_scores()` なのか、特定のバリアントに依存するのかを実装時に確認する。

---

## ヒアリング結果サマリー

### 確認できた事項
- CSV生成ロジックは `io/csv_export.rs` に集約（io 層責務の一貫性）
- `handle_toolbar_action()` に `app_state: &AppState` を追加する必要がある
- `show_cell_toolbar()` に `csv_available: bool` を追加してグレーアウト制御
- Surface Plot はスキップ（`has_csv_data()` が false を返す）

### 設計方針の決定事項
- 新規ファイル: `egui-app/src/io/csv_export.rs`
- 変更ファイル: `grid_canvas.rs`, `export.rs`, `io/mod.rs`
- 既存ウィジェットファイルは変更不要

### 残課題
- `PdpChart` の内部キャッシュ型 (`PdpResult` / `PdpResult1d`) の詳細確認
- `McdmScatterChart` の正確なデータ源の確認
- `AhpTable` の列構成（`priority_vector` の扱い）

### 信頼性レベル分布

**ヒアリング前**:
- 🔵 青信号: 6
- 🟡 黄信号: 8
- 🔴 赤信号: 2

**ヒアリング後**:
- 🔵 青信号: 14 (+8)
- 🟡 黄信号: 4 (-4)
- 🔴 赤信号: 0 (-2)

---

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [types.rs](types.rs)
- **要件定義**: [requirements.md](../../spec/chart-csv-export/requirements.md)
