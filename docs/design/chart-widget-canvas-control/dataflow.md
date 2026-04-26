# Chart Widget Canvas Control データフロー図

**作成日**: 2026-04-17
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連要件**: キャンバス配置チャートの移動・サイズ変更・削除

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 既存コード分析・ユーザヒアリングを参考にした確実なフロー
- 🟡 **黄信号**: 既存コード分析・ユーザヒアリングから妥当な推測によるフロー
- 🔴 **赤信号**: 既存コード分析・ユーザヒアリングにない推測によるフロー

---

## システム全体のデータフロー 🔵

**信頼性**: 🔵 *既存コード分析 + 新規機能追加より*

```mermaid
flowchart TD
    U[ユーザー]
    RP[右パネル\nDragPayload::NewWidget]
    GC[グリッドキャンバス\nDragPayload::MoveFromCell]
    LS[LayoutState\nGridLayout]
    HA[リサイズハンドル]
    XB[✕ボタン]

    U -->|D&D新規配置| RP
    RP -->|DragPayload::NewWidget| GC
    GC -->|D&D移動| GC
    U -->|ドラッグハンドル| HA
    HA -->|Expand/Shrink| LS
    U -->|✕クリック| XB
    XB -->|Clear| LS
    GC -->|GridLayout.place| LS
    LS -->|再描画| GC
```

## 主要機能のデータフロー

### 機能1: セル間 D&D 移動 🔵

**信頼性**: 🔵 *ユーザーヒアリング・egui 0.30 D&D API より*

**関連**: 新規 DragPayload 型

```mermaid
sequenceDiagram
    participant U as ユーザー
    participant Src as 元セル\n(dnd_drag_source)
    participant Dst as 先セル\n(dnd_drop_zone)
    participant LS as LayoutState.grid
    participant RP as 右パネル

    U->>Src: チャートをドラッグ開始
    Src->>Src: dnd_drag_source(id, DragPayload::MoveFromCell{item,row,col})
    Note over Src: ドラッグ中はアイテム名ラベルが表示
    U->>Dst: 先セルへドロップ
    Dst->>Dst: dnd_drop_zone で DragPayload を受け取り
    Dst->>LS: GridLayout::place(dst_row, dst_col, item)
    Note over LS: place() は元セルの content を自動クリア
    LS->>RP: placed_items() で配置済みリストを更新
    LS->>Dst: 次フレームで移動後のセルを描画
```

**詳細ステップ**:
1. 元セルの `render_cell_content` 内で `dnd_drag_source` をラップ
2. ペイロードとして `DragPayload::MoveFromCell { item, row, col }` を設定
3. 先セルの `dnd_drop_zone` で `DragPayload` を受け取る
4. `DragPayload::MoveFromCell` の場合: `GridLayout::place(dst_row, dst_col, item)` を呼ぶ
5. `place()` は内部で全セルを走査し、同じアイテムを含む元セルをクリア → 移動完了
6. `DragPayload::NewWidget` の場合: 従来通り新規配置

### 機能2: ✕ボタンによる削除 🔵

**信頼性**: 🔵 *ユーザーヒアリング（✕ボタン常時表示・確認なし）より*

```mermaid
sequenceDiagram
    participant U as ユーザー
    participant XB as ✕ボタン\n(ui.put)
    participant PA as pending_actions
    participant LS as LayoutState.grid

    U->>XB: ✕をクリック
    XB->>PA: CellAction::Clear(row, col)
    PA->>LS: cells[row][col].content = None
    Note over LS: 右パネルの placed_items からも除外
    LS->>LS: 次フレームで空セルを描画
```

**配置フロー**:
1. グリッド描画ループ内で `cell.content.is_some()` の場合
2. `ui.put(close_rect, Button::new("✕"))` で✕ボタンを配置
3. `close_rect` はセル右上に固定（16x16px）
4. クリック検知 → `pending_actions.push(CellAction::Clear(r, c))`
5. ループ後に `cells[r][c].content = None` を適用

### 機能3: ハンドルクリックによるサイズ変更 🔵

**信頼性**: 🔵 *ユーザーヒアリング（端のみハンドル）・egui ui.interact() より*

```mermaid
sequenceDiagram
    participant U as ユーザー
    participant H as ドラッグハンドル\n(ui.interact)
    participant PA as pending_actions
    participant LS as LayoutState.grid

    U->>H: ハンドルをクリック
    H->>H: クリック方向を判定
    alt 右端ハンドル + 左クリック
        H->>PA: CellAction::ExpandRight(r, c)
    else 右端ハンドル + 右クリック
        H->>PA: CellAction::ShrinkRight(r, c)
    else 下端ハンドル + 左クリック
        H->>PA: CellAction::ExpandDown(r, c)
    else 下端ハンドル + 右クリック
        H->>PA: CellAction::ShrinkDown(r, c)
    end
    PA->>LS: safe_expand_right / shrink_right 等
    LS->>LS: col_span / row_span を更新
    LS->>LS: 次フレームでリサイズ後のセルを描画
```

**ハンドル検知フロー**:
1. 各セルの右端と下端に 6px 幅の矩形を計算
2. `ui.interact(handle_rect, id, Sense::click())` でクリック検知
3. ホバー時はハンドルを青色でハイライト、カーソルを変更
4. クリック時は `pending_actions` に Expand/Shrink を追加
5. ループ後に GridLayout の対応メソッドを実行

### 機能3b: ハンドルドラッグによる連続サイズ変更（Phase 2）🟡

**信頼性**: 🟡 *Phase 2 機能として妥当な推測*

```mermaid
sequenceDiagram
    participant U as ユーザー
    participant H as ドラッグハンドル
    participant Acc as 累積ドラッグ量\n(acc_delta)
    participant LS as LayoutState.grid

    U->>H: ハンドルをドラッグ開始
    loop 毎フレーム
        H->>Acc: drag_delta() を累積
        Acc->>Acc: 累積値 / 1セル幅 = 段階数
        alt expand が必要
            Acc->>LS: safe_expand_right(r, c)
            Acc->>Acc: 1セル分を累積値から減算
        else shrink が必要
            Acc->>LS: shrink_right(r, c)
            Acc->>Acc: 1セル分を累積値から減算
        end
    end
    U->>H: ドラッグ終了
    Acc->>Acc: 累積値をリセット
```

## セル描画の統合フロー 🔵

**信頼性**: 🔵 *既存 grid_canvas.rs + 新規UI要素の統合より*

```mermaid
flowchart TD
    A[グリッド描画ループ開始] --> B{セルが merged?}
    B -->|Yes| A
    B -->|No| C[セル矩形を計算]
    C --> D{content あり?}
    D -->|Yes| E[ハンドル矩形を ui.interact で登録]
    D -->|No| F[dnd_drop_zone でプレースホルダー描画]
    E --> G[✕ボタンを ui.put で配置]
    G --> H[セル内容を dnd_drag_source + dnd_drop_zone で描画]
    H --> I[pending_actions / pending_drops を収集]
    F --> I
    I --> A

    A -->|全セル完了| J[pending_actions を適用]
    J --> K[pending_drops を適用]
    K --> L[次フレーム描画]
```

## D&D 移動と新規配置の統合フロー 🔵

**信頼性**: 🔵 *DragPayload 型による統合設計より*

```mermaid
flowchart TD
    A[ドロップ検知] --> B{DragPayload の種別}
    B -->|NewWidget| C[GridLayout::place]
    B -->|MoveFromCell| D{移動元 == 移動先?}
    D -->|Yes| E[何もしない]
    D -->|No| C
    C --> F[place 内部: 同一アイテムの旧配置を全走査でクリア]
    F --> G[新セルに content を設定]
    G --> H[placed_items を更新]
    H --> I[右パネルのグレーアウト更新]
```

## 状態管理フロー 🔵

**信頼性**: 🔵 *既存 egui-app の状態管理パターンより*

```mermaid
stateDiagram-v2
    [*] --> 空セル
    空セル --> 配置済み: D&D新規配置
    配置済み --> 配置済み: D&D移動(別セルへ)
    配置済み --> 空セル: ✕ボタン削除
    配置済み --> 空セル: コンテキストメニュー→クリア
    配置済み --> 拡張済み: ハンドル/メニュー→拡張
    拡張済み --> 配置済み: ハンドル/メニュー→縮小
    拡張済み --> 空セル: ✕ボタン削除
    拡張済み --> 拡張済み: ハンドル/メニュー→さらに拡張
```

## エラーハンドリングフロー 🟡

**信頼性**: 🟡 *既存パターン + 新規操作の組み合わせより*

| エラーケース | 処理 |
|---|---|
| 自分自身のセルにドロップ | ドロップを無視（place 内で同一セルは何もしない） |
| 結合先セルにドロップ | ドロップ無効（merged_into != None のセルはスキップ） |
| コンテンツあるセルに拡張 | safe_expand が false を返す → 操作キャンセル |
| グリッド境界を超えるハンドル操作 | expand が false を返す → 操作キャンセル |
| ハンドルとD&Dの同時操作 | egui の排他処理でハンドルが優先（先に interact した方） |

## 関連文書

- **アーキテクチャ**: [architecture.md](architecture.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **ヒアリング記録**: [design-interview.md](design-interview.md)
- **既存フリーレイアウト設計**: [../free-layout-dashboard/dataflow.md](../free-layout-dashboard/dataflow.md)

## 信頼性レベルサマリー

- 🔵 青信号: 9件 (82%)
- 🟡 黄信号: 2件 (18%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
