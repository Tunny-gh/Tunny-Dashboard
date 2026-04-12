# Free Layout Dashboard アーキテクチャ設計

**作成日**: 2026-04-12
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: ユーザーヒアリング・既存実装を参考にした確実な設計
- 🟡 **黄信号**: ヒアリングから妥当な推測による設計
- 🔴 **赤信号**: ヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *ユーザーヒアリングより*

Tunny Dashboard の egui-app に「フリーレイアウトキャンバス」を追加する。
ユーザーは右側パネルからチャート・テーブルを選び、中央キャンバスの任意のグリッドセルへ
ドラッグ&ドロップで配置できる。グリッドは行列を自由に追加でき、セルは右クリックメニューで結合できる。

## 変更スコープ 🔵

**信頼性**: 🔵 *ユーザーヒアリングより*

| コンポーネント | 変更種別 | 概要 |
|---|---|---|
| `layout_state.rs` | 大改修 | GridLayout / GridCell / PanelItem モデル追加 |
| `main_canvas.rs` | 大改修 | 固定2×2グリッド → 自由グリッドレンダラーへ |
| `left_panel.rs` | 小改修 | チャート選択チェックボックスを削除 |
| `layout.rs` | 改修 | 右パネル（SidePanel::right）を追加、BottomPanel を条件表示化 |
| `right_panel.rs` | 新規 | ウィジェット一覧 + D&Dソース |
| `grid_canvas.rs` | 新規 | グリッドレンダラー（セル描画・結合・右クリックメニュー） |
| `bottom_panel.rs` | 改修 | TrialTable を `PanelItem` として抽象化、省略可能化 |

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存 egui-app アーキテクチャと整合*

- **パターン**: Immediate Mode GUI（egui の設計哲学に準拠）
- `AppState` がモデル（データ）を保持
- `LayoutState` がビュー設定（グリッド配置・パネル幅）を保持
- 各 `show_*` 関数がフレームごとに UI を再描画

## データモデル 🔵

**信頼性**: 🔵 *ユーザーヒアリング + 既存 ChartId 設計より*

### PanelItem — キャンバスに配置できるウィジェットの統合型

```
PanelItem
├── Chart(ChartId)   — 既存の10種類のチャート
└── TrialTable       — トライアル一覧テーブル（bottom_panel から移植）
```

### GridCell — 1セルの状態

```
GridCell {
    content: Option<PanelItem>,  // None = 空スロット
    col_span: u8,                // 1〜(max_cols - col) の範囲
    row_span: u8,                // 1〜(max_rows - row) の範囲
    merged_into: Option<(usize, usize)>, // 結合元セル座標（被結合セル用）
}
```

### GridLayout — キャンバス全体のグリッド

```
GridLayout {
    rows: usize,                           // デフォルト 2
    cols: usize,                           // デフォルト 2
    cells: Vec<Vec<GridCell>>,             // [row][col]
}
```

### RightPanelState — 右パネルの状態

```
RightPanelState {
    is_open: bool,       // パネル開閉状態
    width: f32,          // パネル幅（ドラッグでリサイズ可）
}
```

## コンポーネント構成

### LayoutState の変更 🔵

**信頼性**: 🔵 *既存実装より*

```
LayoutState {
    left_panel_width: f32,       // 既存（フィルター用）
    right_panel: RightPanelState,  // 新規
    layout_mode: LayoutMode,     // 既存（将来的に削除候補）
    grid: GridLayout,            // 新規（visible_charts を置き換え）
}
```

`visible_charts: HashSet<ChartId>` は削除し、`grid` に置き換える。

### 右パネル (right_panel.rs) 🔵

**信頼性**: 🔵 *ユーザーヒアリングより*

- `egui::SidePanel::right("right_panel")` で実装
- ハンバーガーボタン（≡）で開閉切り替え
- 利用可能な `PanelItem` の一覧をボタン or ラベルで表示
- 各アイテムを `ui.dnd_drag_source(...)` でドラッグ開始
- すでにグリッドに配置済みのアイテムはグレーアウトで表示

### グリッドキャンバス (grid_canvas.rs) 🔵

**信頼性**: 🔵 *ユーザーヒアリングより（セル結合・D&D）*

- `egui::Grid` または手動ペインタで 行×列 のセルを描画
- 各セルは `ui.dnd_drop_zone(...)` でドロップ受け付け
- 行追加/削除ボタン、列追加/削除ボタン
- セルを右クリック → コンテキストメニュー

```
コンテキストメニュー項目:
  ・右に拡張（col_span + 1）
  ・下に拡張（row_span + 1）
  ・縮小（元のサイズに戻す）
  ・クリア（コンテンツを削除し右パネルに戻す）
```

### 左パネル変更点 🔵

**信頼性**: 🔵 *ユーザーヒアリングより（フィルター専用化）*

- `show_chart_selection` を削除
- Study Info・Filters・Color Mode のみ残す

### BottomPanel 廃止と TrialTable の PanelItem 化 🔵

**信頼性**: 🔵 *ユーザーヒアリングより*

- `egui::TopBottomPanel::bottom("bottom_panel")` を layout.rs から削除
- `TrialTable` ウィジェットを `PanelItem::TrialTable` として右パネル経由で配置
- `bottom_panel.rs` → `widgets/trial_table.rs` にリネーム・移植

## ディレクトリ構造（変更後）🔵

**信頼性**: 🔵 *既存プロジェクト構造より*

```
egui-app/src/
├── state/
│   ├── layout_state.rs      // GridLayout / GridCell / PanelItem / RightPanelState を追加
│   └── ...
├── ui/
│   ├── layout.rs            // right_panel 追加、bottom_panel 削除
│   ├── left_panel.rs        // チャート選択削除、フィルター専用化
│   ├── right_panel.rs       // 新規
│   ├── grid_canvas.rs       // 新規
│   ├── main_canvas.rs       // grid_canvas への委譲に変更
│   ├── bottom_panel.rs      // 削除（trial_table.rs へ移植）
│   ├── toolbar.rs           // 変更なし
│   └── widgets/
│       ├── trial_table.rs   // 新規（bottom_panel.rs から移植）
│       └── ...（既存ウィジェット群）
└── ...
```

## D&D 実装方針 🔵

**信頼性**: 🔵 *egui の dnd API より*

egui 0.28+ は `ui.dnd_drag_source` と `ui.dnd_drop_zone` を提供する。

```
[右パネル: dnd_drag_source]
  PanelItem をペイロードとして drag 開始
  
[グリッドセル: dnd_drop_zone]
  ドロップ受け付け → GridCell.content に PanelItem を格納
  同一 PanelItem が別セルにあれば移動（元セルをクリア）
```

egui の `DragAndDrop` API は `InternallyMutable`（`Context` 経由）なので
`&mut GridLayout` を持ちながらドラッグ中 UI を描画できる。

## セル結合の実装方針 🟡

**信頼性**: 🟡 *ヒアリング方向性 + egui 実装制約から推測*

egui は CSS Grid のようなネイティブな span 機能を持たないため、
手動レイアウトで実装する。

```
1. 各行を ui.horizontal(|ui| {...}) で描画
2. セルの幅は (available_width / cols) * col_span - gap
3. セルの高さは (available_height / rows) * row_span - gap
4. merged_into != None のセルはスキップ（結合元が描画を担当）
```

行・列追加時は `cells` ベクターに新しい `GridCell::default()` を追加する。
削除時はコンテンツがあるセルが存在しない行/列のみ削除を許可（誤削除防止）。

## 非機能要件 🟡

**信頼性**: 🟡 *既存プロジェクト方針から推測*

- **レンダリング**: egui の Immediate Mode に準拠。毎フレーム全セルを再描画
- **状態永続化**: セッション保存（`io/session.rs`）に GridLayout を追加 🟡
- **パフォーマンス**: グリッドが 10×10 以上になった場合でもチャート描画コストが支配的で UI 自体は軽量

## 技術的制約 🔵

**信頼性**: 🔵 *egui / eframe の既知制約より*

- egui の `ui.columns()` は均等分割のみ。可変幅列には手動レイアウト（`ui.allocate_exact_size`）が必要
- D&D 中のペイロードは `egui::Context` のメモリに格納。`Clone` を実装した型のみ使用可能
- `PanelItem` は `Clone + PartialEq + Hash` を実装する必要がある

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **ヒアリング記録**: [design-interview.md](design-interview.md)
- **既存アーキテクチャ**: [../tunny-dashboard/architecture.md](../tunny-dashboard/architecture.md)

## 信頼性レベルサマリー

- 🔵 青信号: 14件 (74%)
- 🟡 黄信号: 4件 (21%)
- 🔴 赤信号: 1件 (5%)

**品質評価**: ⚠️ 要改善（セル結合の実装詳細は検証が必要）
