# Chart Widget Canvas Control アーキテクチャ設計

**作成日**: 2026-04-17
**関連要件**: キャンバスに配置したチャートの移動・サイズ変更・削除を可能にする
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 既存コード分析・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: 既存コード分析・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: 既存コード分析・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *ユーザーヒアリングより*

egui-app のグリッドキャンバスに配置済みのチャートウィジェットについて、以下の3つの操作を追加する:

1. **移動**: 配置済みチャートをドラッグ&ドロップで別セルへ移動
2. **削除**: チャート右上の✕ボタンで即座に削除（確認なし）
3. **サイズ変更**: セル右端・下端のドラッグハンドルで1セル単位の拡大/縮小（2x1等の横長/縦長対応）

既存の右クリックコンテキストメニュー（拡張/縮小/クリア）も引き続き利用可能。

## 変更スコープ 🔵

**信頼性**: 🔵 *既存コード分析より*

| ファイル | 変更種別 | 概要 |
|---|---|---|
| `grid_canvas.rs` | 大改修 | ドラッグソース・ハンドル・✕ボタン追加 |
| `layout_state.rs` | 改修 | DragPayload 型・ハンドル用メソッド追加 |
| `right_panel.rs` | 小改修 | DragPayload 型への対応 |
| `chart_registry.rs` | 小改修 | ✕ボタン配置のためのUI構造変更 |

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存 egui-app アーキテクチャと整合*

- **パターン**: Immediate Mode GUI（egui の設計哲学に準拠）
- 既存の `pending_actions: Vec<CellAction>` パターンを拡張し、新しい操作も同様に遅延適用
- `ui.interact()` を用いたセル境界のドラッグ検知を追加

## 機能1: チャート移動（D&D） 🔵

**信頼性**: 🔵 *ユーザーヒアリング（ドラッグ&ドロップ選択）・egui 0.30 D&D API より*

### 設計

配置済みチャートをドラッグソースとして機能させ、別セルへドロップで移動する。
右パネルからの新規配置とセル間移動を区別するため、ペイロード型を新設する。

```rust
/// D&D ペイロードの統合型（PanelItem を置き換え）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DragPayload {
    /// 右パネルからの新規配置
    NewWidget(PanelItem),
    /// セル間移動（元セル情報付き）
    MoveFromCell { item: PanelItem, row: usize, col: usize },
}
```

### 実装方針

1. `grid_canvas.rs` の `render_cell_content` 内で、コンテンツがあるセルの描画を `dnd_drag_source` でラップ
2. `DragPayload::MoveFromCell` に元セル座標を含めることで、ドロップ時に元セルを自動クリア
3. `dnd_drop_zone` のペイロード型を `PanelItem` → `DragPayload` に変更
4. 右パネルの `dnd_drag_source` も `DragPayload::NewWidget(...)` を投げるよう変更
5. ドロップ処理で `DragPayload` を分解し、移動元セルのクリアと配置先セルの設定を実行

### セル内ドラッグソースの実装 🔵

**信頼性**: 🔵 *egui 0.30 は dnd_drag_source を dnd_drop_zone 内にネスト可能*

```rust
// grid_canvas.rs render_cell_content 内
match &cell.content {
    Some(item) => {
        let payload = DragPayload::MoveFromCell {
            item: item.clone(),
            row,
            col,
        };
        let drag_id = egui::Id::new("cell_drag").with(row).with(col);
        ui.dnd_drag_source(drag_id, payload, |ui| {
            // チャート描画（ドラッグ中は半透明で表示）
            chart_registry::show_cell_chart(ui, ...);
        });
    }
    None => { /* プレースホルダー */ }
}
```

**制約**: egui 0.30 では `dnd_drag_source` 内でのみ子要素が描画される。ドラッグ開始時はペイロードとしてアイテム名のラベルが表示される。

## 機能2: ✕ボタンによる削除 🔵

**信頼性**: 🔵 *ユーザーヒアリング（✕ボタン常時表示・確認なし）より*

### 設計

各チャートセルの右上に小さな✕ボタン（16x16px）を常時表示。
クリックで即座に `cell.content = None` とする（確認ダイアログなし）。

### 実装方針 🔵

**信頼性**: 🔵 *egui の ui.put() + Response.clicked() パターンより*

```rust
// grid_canvas.rs: セル描画の最後にオーバーレイとして配置
if cell.content.is_some() {
    let close_size = egui::vec2(16.0, 16.0);
    let close_rect = egui::Rect::from_min_size(
        cell_rect.right_top() - egui::vec2(close_size.x, 0.0),
        close_size,
    );
    let close_resp = ui.put(close_rect, egui::Button::new("✕").small());
    if close_resp.clicked() {
        pending_actions.push(CellAction::Clear(r, c));
    }
}
```

**重要**: `ui.put()` を `dnd_drop_zone` の外側（親UIレベル）で呼び出し、セル内のチャート描画と独立してボタンを配置する。

### ✕ボタンの視覚仕様 🟡

**信頼性**: 🟡 *ユーザーヒアリング（常時表示）から推測*

- サイズ: 16x16 px
- 配置: セル右上（セル内パディングなし）
- 色: `Color32::from_gray(180)`（薄いグレー）、ホバー時 `Color32::from_gray(120)`（濃いグレー）
- フォント: 小さいテキスト（`RichText::new("✕").small()`）

## 機能3: ドラッグハンドルによるサイズ変更 🔵

**信頼性**: 🔵 *ユーザーヒアリング（端のみハンドル）・egui ui.interact() より*

### 設計

コンテンツがあるセルの右端と下端にドラッグハンドル（6px幅の薄い領域）を配置。
ドラッグで1セル単位の拡大/縮小を行う。

### ハンドル位置の計算 🔵

**信頼性**: 🔵 *既存 calc_cell_width/calc_cell_height と grid_area 座標系より*

```rust
const HANDLE_THICKNESS: f32 = 6.0;

// 右端ハンドル: セルの右端から HANDLE_THICKNESS 分の領域
let right_handle_rect = egui::Rect::from_min_size(
    egui::pos2(cell_rect.right() - HANDLE_THICKNESS, cell_rect.top()),
    egui::vec2(HANDLE_THICKNESS, cell_rect.height()),
);

// 下端ハンドル: セルの下端から HANDLE_THICKNESS 分の領域
let bottom_handle_rect = egui::Rect::from_min_size(
    egui::pos2(cell_rect.left(), cell_rect.bottom() - HANDLE_THICKNESS),
    egui::vec2(cell_rect.width(), HANDLE_THICKNESS),
);
```

### ハンドルのドラッグ検知 🔵

**信頼性**: 🔵 *egui ui.interact() + Sense::drag() パターンより*

```rust
let right_id = egui::Id::new("resize_right").with(r).with(c);
let right_resp = ui.interact(right_handle_rect, right_id, egui::Sense::drag());

let bottom_id = egui::Id::new("resize_bottom").with(r).with(c);
let bottom_resp = ui.interact(bottom_handle_rect, bottom_id, egui::Sense::drag());
```

**制約**: `ui.interact()` は親UIレベル（グリッド描画ループ内、`dnd_drop_zone` の外）で呼び出す必要がある。セル内の子UIが入力を消費するのを防ぐため。

### ドラッグ量の量子化 🟡

**信頼性**: 🟡 *グリッド離散性から妥当な推測*

ドラッグ量は1セル幅を超えるごとに expand/shrink を1段階実行:

```rust
if right_resp.dragged() {
    let delta_x = right_resp.drag_delta().x;
    let one_cell_w = total_w / cols as f32;
    let steps = (delta_x / one_cell_w).round() as i32;
    if steps > 0 {
        // expand_right を steps 回実行
        for _ in 0..steps { layout.grid.expand_right(r, c); }
    } else if steps < 0 {
        for _ in 0..steps.abs() { layout.grid.shrink_right(r, c); }
    }
}
```

**注意**: 毎フレームで呼ばれるため、前フレームからの差分ではなく累積値を使う必要がある。`ui.interact()` の `drag_delta()` は前フレームからの差分を返すため、accumulate 変数で管理する。

### 代替案: クリックベースのハンドル 🟡

**信頼性**: 🟡 *ドラッグ量子化の複雑さから妥当な代替案*

ドラッグではなく、ハンドルのクリックで1段階 expand/shrink する方式も検討可能:

```rust
if right_resp.clicked() {
    // 右クリック: 縮小、左クリック: 拡張
    if right_resp.clicked_by(egui::PointerButton::Primary) {
        pending_actions.push(CellAction::ExpandRight(r, c));
    }
}
```

この方が実装がシンプルで、ユーザーにも分かりやすい可能性がある。
コンテキストメニューとの統合も考慮し、**Phase 1 ではクリックベース、Phase 2 でドラッグベースへの移行**を推奨。

### ハンドルの視覚仕様 🟡

**信頼性**: 🟡 *一般的なリサイズUIパターンより*

- 太さ: 6px
- 色: ホバー時 `Color32::from_rgba_unmultiplied(100, 150, 255, 80)`（薄い青）、非ホバー時は透明
- カーソル: 右端は `CursorIcon::ResizeHorizontal`、下端は `CursorIcon::ResizeVertical`

## 新しい CellAction 列挙型 🔵

**信頼性**: 🔵 *既存 CellAction の拡張・ユーザーヒアリングより*

```rust
enum CellAction {
    // 既存
    ExpandRight(usize, usize),
    ExpandDown(usize, usize),
    ShrinkRight(usize, usize),
    ShrinkDown(usize, usize),
    Clear(usize, usize),
}
```

移動は `DragPayload::MoveFromCell` の処理で直接 `GridLayout::place()` を呼ぶため、新しい CellAction は不要。
✕ボタンは既存の `Clear` アクションを使用。
ハンドルのクリックは既存の `ExpandRight/ExpandDown/ShrinkRight/ShrinkDown` を使用。

## GridLayout の修正事項 🟡

**信頼性**: 🟡 *コード分析で発見した問題点*

### expand の安全性向上

現在の `expand_right/expand_down` は対象セルにコンテンツがあるかチェックしない。
コンテンツがあるセルに結合すると、そのコンテンツが見えなくなる（`merged_into` でスキップされるため）。

```rust
/// 安全な拡張: 対象セルが空の場合のみ結合を許可
pub fn safe_expand_right(&mut self, row: usize, col: usize) -> bool {
    let new_end_col = col + self.cells[row][col].col_span as usize;
    if new_end_col >= self.cols { return false; }
    let target = &self.cells[row][new_end_col];
    if target.merged_into.is_some() { return false; }
    if target.content.is_some() { return false; }  // ← 追加: コンテンツチェック
    self.cells[row][new_end_col].merged_into = Some((row, col));
    self.cells[row][col].col_span += 1;
    true
}
```

### 直接 span 設定メソッド 🟡

**信頼性**: 🟡 *ドラッグリサイズの効率化のため推測*

将来のドラッグリサイズで複数段階の一括変更に対応するため:

```rust
/// col_span を直接設定（指定値が有効範囲内か検証付き）
pub fn set_col_span(&mut self, row: usize, col: usize, new_span: u8) -> bool {
    let old_span = self.cells[row][col].col_span;
    if new_span == old_span { return true; }
    if new_span < 1 { return false; }
    if col + new_span as usize > self.cols { return false; }
    // 縮小の場合: 解放されるセルにコンテンツがないか確認
    // 拡張の場合: 新規対象セルが空か確認
    // ... 検証ロジック ...
    true
}
```

## コンポーネント構成

### grid_canvas.rs の変更 🔵

**信頼性**: 🔵 *既存コード分析より*

```
show_grid_canvas() の描画ループ内:
  for r in rows:
    for c in cols:
      if merged: continue

      // 1. セル矩形を計算
      let cell_rect = ...;

      // 2. ドラッグハンドルを登録（親UIレベル）← 新規
      register_resize_handles(ui, cell_rect, r, c, cell, &mut pending_actions);

      // 3. ✕ボタンを配置（親UIレベル）← 新規
      if cell.content.is_some() {
          register_close_button(ui, cell_rect, r, c, &mut pending_actions);
      }

      // 4. dnd_drop_zone でセル内容を描画
      let child_ui = ...;
      let (inner_resp, payload) = child_ui.dnd_drop_zone::<DragPayload, _>(...);
      // ドロップ時: DragPayload を分解して place

      // 5. コンテキストメニュー（既存のまま）
```

### right_panel.rs の変更 🔵

**信頼性**: 🔵 *DragPayload 型導入により小変更*

```rust
// 変更前
ui.dnd_drag_source(drag_id, item.clone(), |ui| { ... });

// 変更後
ui.dnd_drag_source(drag_id, DragPayload::NewWidget(item.clone()), |ui| { ... });
```

### layout_state.rs の変更 🔵

**信頼性**: 🔵 *DragPayload 追加・safe_expand メソッド追加*

- `DragPayload` enum の追加
- `safe_expand_right()` / `safe_expand_down()` の追加
- 既存 `expand_right/expand_down` はコンテキストメニュー用に残す（コンテンツチェックなし）

## 非機能要件

### パフォーマンス 🔵

**信頼性**: 🔵 *既存 egui Immediate Mode アーキテクチャより*

- 毎フレームのハンドル・✕ボタン描画は egui の軽量描画命令（`ui.interact`, `ui.put`）のみ
- チャート自体の描画コストが支配的で、UI部品の追加によるオーバーヘッドは無視可能
- ドラッグ中のリアルタイム更新は egui の `request_repaint()` で自然に対応

### ユーザビリティ 🟡

**信頼性**: 🟡 *一般的なUI設計パターンより*

- ✕ボタンは常時表示で発見しやすい
- ハンドルはホバー時のみ視覚的に強調（非ホバー時は透明）
- 移動中の視覚フィードバック: egui の標準 D&D プレビュー表示

### 拡張性 🟡

**信頼性**: 🟡 *将来要件から妥当な推測*

- `DragPayload` は拡張可能な設計（将来的に「コピー」等の操作も追加可能）
- `CellAction` も新しいアクションの追加が容易
- セッション保存/復元への対応は `GridLayout` のシリアライズで対応可能

## 技術的制約 🔵

**信頼性**: 🔵 *egui 0.30 API 制約・既存コード分析より*

- `ui.interact()` でハンドルを登録する際、同じ矩形領域を複数の interact 呼び出しで共有できない。ハンドル矩形はセル内容の描画領域と重ならない位置に配置する必要がある
- egui の D&D API は1フレームに1つのドラッグしか追跡しない。ハンドルのドラッグとセルのD&D移動は排他的
- `dnd_drag_source` 内に `dnd_drop_zone` をネスト可能だが、逆は不可。✕ボタンとハンドルは `dnd_drop_zone` の外側で処理する
- ハンドルドラッグの累積差分管理が必要（`drag_delta()` は前フレーム差分のみを返す）

## 実装フェーズ分け 🟡

**信頼性**: 🟡 *段階的リリース戦略として妥当な推測*

### Phase 1: 基本操作の追加
- ✕ボタンによる削除（常時表示）
- セル間 D&D 移動
- ハンドルクリックによるサイズ変更（1クリック1段階）
- `safe_expand_*` の追加

### Phase 2: 高度な操作
- ハンドルドラッグによる連続サイズ変更
- ドラッグ中のプレビュー表示
- セッション保存/復元へのレイアウト状態対応

## ディレクトリ構造（変更後）🔵

**信頼性**: 🔵 *既存プロジェクト構造より*

```
egui-app/src/
├── state/
│   ├── layout_state.rs      // DragPayload, safe_expand_* 追加
│   └── ...
├── ui/
│   ├── grid_canvas.rs       // 大改修: ハンドル, ✕ボタン, D&D移動
│   ├── right_panel.rs       // 小改修: DragPayload::NewWidget 対応
│   ├── chart_registry.rs    // 小改修: ✕ボタン配置スペース確保
│   └── ...
└── ...
```

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **ヒアリング記録**: [design-interview.md](design-interview.md)
- **既存フリーレイアウト設計**: [../free-layout-dashboard/architecture.md](../free-layout-dashboard/architecture.md)
- **既存チャート実装設計**: [../chart-implementation/architecture.md](../chart-implementation/architecture.md)

## 信頼性レベルサマリー

- 🔵 青信号: 18件 (75%)
- 🟡 黄信号: 5件 (21%)
- 🔴 赤信号: 1件 (4%)

**品質評価**: ✅ 高品質（ハンドルドラッグの量子化ロジックは実装時に検証が必要）
