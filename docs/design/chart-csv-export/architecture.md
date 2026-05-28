# chart-csv-export アーキテクチャ設計

**作成日**: 2026-05-28
**関連要件定義**: [requirements.md](../../spec/chart-csv-export/requirements.md)
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *ユーザヒアリング（要件定義）より*

Tunny Dashboard の各チャートセルの⋯ポップアップメニューに「Save as CSV」ボタンを追加する。既存の `CellToolbarAction` パターン・`io/export.rs` の `save_csv_to_file()` を最大限再利用し、チャート固有のCSV生成ロジックを `io/csv_export.rs` に一元管理する。

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存コードベースのアーキテクチャパターンより*

- **パターン**: 既存の `CellToolbarAction` イベント駆動パターンを踏襲し、`SaveAsCsv` バリアントを追加する
- **CSV生成**: `io/csv_export.rs` に ChartId ごとの dispatch 関数を集約（ウィジェットはUI専用）
- **ファイル保存**: 既存の `rfd` ファイルダイアログ + `save_csv_to_file()` を再利用

---

## 変更箇所一覧

### 変更ファイル 🔵

**信頼性**: 🔵 *コードベース調査より*

| ファイル | 変更種別 | 内容 |
|---------|---------|------|
| `egui-app/src/ui/grid_canvas.rs` | 変更 | `CellToolbarAction::SaveAsCsv` バリアント追加、⋯メニューに「Save as CSV」追加、`handle_toolbar_action()` に処理を追加 |
| `egui-app/src/io/csv_export.rs` | **新規** | チャート別CSV生成ロジックの一括dispatch |
| `egui-app/src/io/mod.rs` | 変更 | `pub mod csv_export;` を追加 |

### 変更不要ファイル 🔵

**信頼性**: 🔵 *既存コードベース調査より*

- `egui-app/src/io/export.rs`: 既存の `save_csv_to_file()`, `write_csv_to_path()`, `build_csv_string()` はそのまま再利用
- 各ウィジェット (`ui/widgets/*.rs`): UIロジックは変更不要（CSV生成はio層で担当）

---

## コンポーネント設計

### CellToolbarAction 拡張 🔵

**信頼性**: 🔵 *既存 `grid_canvas.rs` の設計パターンより*

```rust
// 変更前 (egui-app/src/ui/grid_canvas.rs)
pub enum CellToolbarAction {
    None,
    Close,
    Help(PanelItem),
    SaveAsPng(PanelItem),
}

// 変更後
pub enum CellToolbarAction {
    None,
    Close,
    Help(PanelItem),
    SaveAsPng(PanelItem),
    SaveAsCsv(PanelItem),   // 追加
}
```

### ⋯メニュー追加 🔵

**信頼性**: 🔵 *既存メニュー実装パターン（`show_cell_toolbar()`）より*

```rust
// show_cell_toolbar() 内の ⋯ メニューに追加
if ui.button("Save as CSV").clicked() {
    menu_action = Some(CellToolbarAction::SaveAsCsv(item.clone()));
    ui.close_menu();
}
```

`chart_cell_menu_items()` の戻り値も "Save as CSV" を追加する（テスト用）。

### handle_toolbar_action() 拡張 🔵

**信頼性**: 🔵 *既存 `handle_toolbar_action()` パターンより*

`handle_toolbar_action()` は現在 `app_state` を受け取っていないため、シグネチャを変更する必要がある。

```rust
// 変更前
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

`SaveAsCsv` のハンドリング:

```rust
CellToolbarAction::SaveAsCsv(target) => {
    let chart_id = match target {
        PanelItem::Chart(id) => id,
        PanelItem::TrialTable => {
            // TrialTable は全試行データをCSVとして出力
            // 既存の build_csv_string() を使用
            // ...
            return;
        }
    };
    let csv = crate::io::csv_export::build_chart_csv(chart_id, app_state, widgets);
    if let Some(csv_str) = csv {
        let filename = csv_export_filename(chart_id);
        if let Err(e) = crate::io::export::save_csv_to_file_named(&csv_str, &filename) {
            let _ = tx.try_send(AppMessage::Error(e));
        }
    }
}
```

### save_csv_to_file_named() 追加 🟡

**信頼性**: 🟡 *既存 `save_csv_to_file()` から妥当な拡張*

`export.rs` に、デフォルトファイル名を指定できるバリアントを追加する:

```rust
pub fn save_csv_to_file_named(csv: &str, default_name: &str) -> Result<(), String> {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("CSV", &["csv"])
        .set_file_name(default_name)
        .save_file()
    {
        write_csv_to_path(csv, &path)
    } else {
        Ok(())
    }
}
```

### io/csv_export.rs（新規） 🔵

**信頼性**: 🔵 *ユーザヒアリング（io層への一括dispatch）+ コードベース調査より*

中心となる dispatch 関数:

```rust
/// チャートIDに対応するCSV文字列を生成する。
/// データが存在しない場合は None を返す。
pub fn build_chart_csv(
    chart_id: &ChartId,
    app_state: &AppState,
    widgets: &WidgetStates,
) -> Option<String>
```

各 ChartId に対応した CSV 生成関数を内部に持つ:

- `build_optimization_history_csv()` → OptimizationHistory
- `build_hv_history_csv()` → HvHistory
- `build_importance_csv()` → ImportanceChart
- `build_pdp_csv()` → PdpChart
- `build_pdp_2d_csv()` → PdpChart2D
- `build_trial_based_csv()` → ParallelCoordinates / ScatterMatrix / ClusterScatter
- `build_pareto_csv()` → ParetoScatter2D / ParetoScatter3D
- `build_sensitivity_csv()` → SensitivityHeatmap
- `build_mcdm_rank_csv()` → McdmRankChart / McdmScatterChart / McdmTable
- `build_ahp_csv()` → AhpRankChart / AhpTable
- `build_slice_csv()` → SliceChart
- Surface Plot → `None`（スキップ）

データ可用性チェック関数:

```rust
/// チャートのCSVデータが利用可能かどうかを返す（ボタングレーアウト判定用）
pub fn has_csv_data(
    chart_id: &ChartId,
    app_state: &AppState,
    widgets: &WidgetStates,
) -> bool
```

### ボタングレーアウト実装 🔵

**信頼性**: 🔵 *ユーザヒアリング Q4（データなし時グレーアウト）より*

`show_cell_toolbar()` 内で `has_csv_data()` を呼び出し、`ui.add_enabled()` でグレーアウトを制御する。ただし `show_cell_toolbar()` は現在 `app_state` にアクセスできないため、データ可用性フラグを引数として受け取る形に変更する。

**代替案（現行シグネチャ維持）**: `render_cell_content()` 内でデータ可用性を評価し、`CellToolbarOptions` 構造体で渡す。 🟡

---

## データフロー概要

```
ユーザー ⋯クリック
    → show_cell_toolbar() でメニュー表示
    → 「Save as CSV」クリック
    → CellToolbarAction::SaveAsCsv(item) 返却
    → handle_toolbar_action() に到達
    → io::csv_export::build_chart_csv(chart_id, app_state, widgets)
        → chart_id に対応する CSV 生成関数を呼び出し
        → Option<String> を返す
    → Some(csv): export::save_csv_to_file_named(csv, filename) を呼び出し
        → rfd ファイルダイアログ
        → ユーザーが保存先を選択
        → write_csv_to_path() でファイル書き込み
    → None: 何もしない（ボタンはすでにグレーアウトされているが防衛的に処理）
    → Err: AppMessage::Error を送信
```

---

## ディレクトリ構造（変更箇所） 🔵

**信頼性**: 🔵 *既存プロジェクト構造より*

```
egui-app/src/
├── io/
│   ├── export.rs           (変更: save_csv_to_file_named() 追加)
│   ├── csv_export.rs       (新規: チャート別CSV生成ロジック)
│   └── mod.rs              (変更: pub mod csv_export; 追加)
└── ui/
    └── grid_canvas.rs      (変更: CellToolbarAction, メニュー, ハンドラー)
```

---

## 非機能要件の実現方法

### パフォーマンス 🟡

**信頼性**: 🟡 *NFR-001から妥当な推測*

- CSV生成はUIスレッドで同期的に実行（ファイルダイアログも同期）
- 10,000件以下の試行データはメモリ上で生成可能（既存 `build_csv_string()` の実績から）
- SensitivityHeatmap などの重い計算は既に非同期で完了している（CSV生成は結果データを読むだけ）

### ユーザビリティ 🔵

**信頼性**: 🔵 *NFR-101・ユーザヒアリングより*

- ボタン文言: "Save as CSV"（英語で統一、既存 "Save as PNG" と一貫）
- グレーアウト: `ui.add_enabled(has_csv_data, ...)` で制御
- ツールチップ: グレーアウト時 "No data available" を hover で表示

---

## 技術的制約

- **WASM非対応**: `rfd` のネイティブファイルダイアログを使用するため、ネイティブビルドのみ。WASMビルド不要のため問題なし 🔵
- **UIスレッドブロック**: `rfd::FileDialog::save_file()` はモーダルダイアログを開くが、eframe ではUIスレッドで呼び出すのが標準パターン 🔵
- **既存テストへの影響**: `chart_cell_menu_items()` の変更でテスト `menu_contains_save_as_png_and_help` が影響を受ける可能性がある → テストを更新する 🔵

---

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **Rust型定義**: [types.rs](types.rs)
- **要件定義**: [requirements.md](../../spec/chart-csv-export/requirements.md)

## 信頼性レベルサマリー

- 🔵 青信号: 18件 (82%)
- 🟡 黄信号: 4件 (18%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質
