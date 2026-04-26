# UIカラーリング改善 アーキテクチャ設計

**作成日**: 2026-04-18
**ブランチ**: featura/egui
**関連設計**: [egui-migration/architecture.md](../egui-migration/architecture.md)
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *ユーザヒアリングより*

egui-appのUIに対してカラーテーマを適用する。
現在はegui デフォルトのダークテーマのみが使用されており、全パネルが均一な色調で視覚的な区別がつきにくい。
添付スクリーンショット（旧React版Tunny Dashboard）の雰囲気（白背景・ダークネイビーツールバー・青アクセントボタン）をeguiで再現する。

**対象範囲** 🔵:
- `egui-app/src/theme.rs` （新規）: カラー定数・Visuals構築
- `egui-app/src/app.rs`: 起動時テーマ適用
- `egui-app/src/ui/layout.rs`: ツールバーFlex包装
- `egui-app/src/ui/grid_canvas.rs`: セルツールバー・境界線色
- `egui-app/src/ui/left_panel.rs`: パネル内セクションスタイル
- `egui-app/src/ui/right_panel.rs`: パネル内セクションスタイル

---

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *egui Visuals API・ユーザヒアリング（ライトテーマ）より*

- **パターン**: 集中管理テーマモジュール + グローバルVisuals適用
- **選択理由**:
  - egui の `ctx.set_visuals()` で一括適用できるため、各ウィジェットへの個別適用が最小限で済む
  - `theme.rs` に全カラー定数を集約することで、将来的な色の変更が1ファイルで完結する
  - ツールバーなどパネル固有の背景色は `egui::Frame` でラップして個別指定する

## カラーパレット定義 🔵

**信頼性**: 🔵 *ユーザヒアリング（添付スクリーンショット分析）より*

| トークン名 | Color32 (RGB) | 用途 |
|---|---|---|
| `TOOLBAR_BG` | `(26, 35, 50)` = `#1a2332` | ツールバー背景（ダークネイビー） |
| `TOOLBAR_TEXT` | `(220, 230, 245)` = `#dce6f5` | ツールバーテキスト（明るいグレー） |
| `PANEL_BG` | `(245, 247, 250)` = `#f5f7fa` | サイドパネル背景（ライトグレー） |
| `CENTRAL_BG` | `(255, 255, 255)` = `#ffffff` | メインキャンバス背景（白） |
| `ACCENT_BLUE` | `(37, 99, 235)` = `#2563eb` | アクティブボタン・強調色 |
| `ACCENT_BLUE_HOVER` | `(29, 78, 216)` = `#1d4ed8` | ホバー時の青 |
| `ACCENT_BLUE_MUTED` | `(219, 234, 254)` = `#dbeafe` | 選択状態ラベル背景 |
| `BORDER_COLOR` | `(203, 213, 225)` = `#cbd5e1` | パネル・セル境界線 |
| `TEXT_PRIMARY` | `(30, 41, 59)` = `#1e293b` | メインテキスト |
| `TEXT_SECONDARY` | `(100, 116, 139)` = `#64748b` | サブテキスト・ラベル |
| `CELL_TOOLBAR_BG` | `(248, 250, 252)` = `#f8fafc` | チャートセルツールバー背景 |
| `WIDGET_BG` | `(255, 255, 255)` = `#ffffff` | ウィジェット背景 |
| `WIDGET_BG_HOVER` | `(241, 245, 249)` = `#f1f5f9` | ホバー時ウィジェット背景 |
| `SELECTION_BG` | `(37, 99, 235)` = `#2563eb` | selectable_label 選択背景 |

## コンポーネント別スタイリング計画 🔵

**信頼性**: 🔵 *ユーザヒアリング（全4パネル対象）・egui API調査より*

### theme.rs （新規ファイル） 🔵

**信頼性**: 🔵 *egui Visuals API仕様より*

```
egui-app/src/theme.rs
```

提供する関数:
- `pub fn tunny_light_visuals() -> egui::Visuals`: グローバルVisuals設定を返す
- カラー定数 `pub const XXX: egui::Color32`: 上記パレット全定義

`egui::Visuals` のカスタマイズ対象フィールド:
```
visuals.dark_mode = false
visuals.panel_fill = PANEL_BG
visuals.window_fill = CENTRAL_BG
visuals.window_stroke = Stroke::new(1.0, BORDER_COLOR)
visuals.widgets.active.bg_fill = ACCENT_BLUE
visuals.widgets.active.fg_stroke = Stroke::new(1.0, WHITE)
visuals.widgets.hovered.bg_fill = WIDGET_BG_HOVER
visuals.widgets.inactive.bg_fill = WIDGET_BG
visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER_COLOR)
visuals.selection.bg_fill = ACCENT_BLUE_MUTED
visuals.selection.stroke = Stroke::new(1.0, ACCENT_BLUE)
visuals.override_text_color = Some(TEXT_PRIMARY)
visuals.extreme_bg_color = Color32::WHITE  // テキスト入力背景
```

### app.rs の変更 🔵

**信頼性**: 🔵 *eframe CreationContext API仕様より*

```rust
// TunnyApp::new() 内
pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
    cc.egui_ctx.set_visuals(crate::theme::tunny_light_visuals());
    // ...既存コード...
}
```

### layout.rs の変更 🔵

**信頼性**: 🔵 *egui TopBottomPanel Frame API仕様より*

ツールバーパネルをダークネイビーFrameでラップ:
```rust
egui::TopBottomPanel::top("toolbar")
    .frame(
        egui::Frame::default()
            .fill(theme::TOOLBAR_BG)
            .inner_margin(egui::Margin::symmetric(8.0, 4.0))
    )
    .show(ctx, |ui| {
        // toolbar内テキスト色をオーバーライド
        ui.visuals_mut().override_text_color = Some(theme::TOOLBAR_TEXT);
        show_toolbar(...)
    });
```

### grid_canvas.rs の変更 🔵

**信頼性**: 🔵 *既存コード分析・egui API仕様より*

- セル境界線: `Color32::from_gray(100)` → `theme::BORDER_COLOR`
- D&Dホバーハイライト: `Color32::from_rgba_unmultiplied(100, 150, 255, 40)` → `theme::ACCENT_BLUE` (alpha 40)
- リサイズハンドルホバー: 同上 (alpha 80)
- `show_cell_toolbar()`: `fill` を `theme::CELL_TOOLBAR_BG`、`stroke` を `theme::BORDER_COLOR` で明示指定

### left_panel.rs / right_panel.rs の変更 🟡

**信頼性**: 🟡 *egui Separator・Heading API仕様から妥当な推測*

- `ui.heading()` にスタイルが不要な場合はglobalVisuals適用のみで充分
- セパレータの色はglobal `visuals.widgets.noninteractive.bg_stroke` で制御される

## ディレクトリ構造（変更箇所） 🔵

**信頼性**: 🔵 *既存プロジェクト構造より*

```
egui-app/src/
├── theme.rs          ← 新規追加（カラー定数・Visuals構築）
├── main.rs           ← 変更なし
├── app.rs            ← set_visuals() 追加
└── ui/
    ├── layout.rs     ← ツールバーFrame変更
    ├── grid_canvas.rs ← セル色定数置き換え
    ├── left_panel.rs  ← (グローバルVisuals適用で対応可能なら変更なし)
    └── right_panel.rs ← (同上)
```

## 非機能要件の実現方法

### パフォーマンス 🔵

**信頼性**: 🔵 *egui Visuals適用の仕組みより*

- `set_visuals()` はフレームごとに呼ばれるのではなく初期化時のみ呼ぶ（`new()`内）
- カラー定数は全て `const` なので実行時コストゼロ
- `ui.visuals_mut()` でツールバー内テキスト色をオーバーライドするが、これはUI構築時のみ（フレームに1回）

### セキュリティ 🔵

**信頼性**: 🔵 *変更はUI表示のみ・データ処理に関係なし*

- テーマ変更はUI描画のみに影響し、データ処理・ファイルI/Oに変更なし

### 互換性 🔵

**信頼性**: 🔵 *egui 0.30系 API仕様より*

- `egui::Visuals` は `Clone`/`Default` 実装済み
- `egui::Color32::from_rgb()` / `from_rgba_unmultiplied()` は安定API
- `cc.egui_ctx.set_visuals()` はeframe 0.30で利用可能

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **ヒアリング記録**: [design-interview.md](design-interview.md)
- **egui-migration設計**: [../egui-migration/architecture.md](../egui-migration/architecture.md)

## 信頼性レベルサマリー

- 🔵 青信号: 12件 (86%)
- 🟡 黄信号: 2件 (14%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
