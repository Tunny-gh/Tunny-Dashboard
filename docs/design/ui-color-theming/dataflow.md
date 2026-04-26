# UIカラーリング改善 データフロー図

**作成日**: 2026-04-18
**関連アーキテクチャ**: [architecture.md](architecture.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実なフロー
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測によるフロー
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測によるフロー

---

## テーマ適用フロー（初期化） 🔵

**信頼性**: 🔵 *eframe CreationContext API・egui Visuals API仕様より*

```mermaid
sequenceDiagram
    participant OS as OS/eframe
    participant Main as main.rs
    participant App as app.rs (TunnyApp::new)
    participant Theme as theme.rs
    participant Ctx as egui::Context

    OS->>Main: eframe::run_native() 呼び出し
    Main->>App: TunnyApp::new(cc) 呼び出し
    App->>Theme: tunny_light_visuals() 呼び出し
    Theme-->>App: egui::Visuals (カスタム設定済み) 返却
    App->>Ctx: cc.egui_ctx.set_visuals(visuals)
    Ctx-->>App: グローバルテーマ適用完了
    App-->>Main: TunnyApp インスタンス返却
    Main-->>OS: イベントループ開始
```

**詳細ステップ**:
1. `eframe::run_native()` がウィンドウを作成し、コールバックで `TunnyApp::new(cc)` を呼ぶ
2. `TunnyApp::new()` 内で `theme::tunny_light_visuals()` を呼んでカスタム `egui::Visuals` を取得
3. `cc.egui_ctx.set_visuals()` でegui Context にグローバル適用
4. 以降の全フレームでこのVisuals設定が使われる

---

## フレームごとのUI描画フロー 🔵

**信頼性**: 🔵 *egui フレームループ仕様・既存コード分析より*

```mermaid
sequenceDiagram
    participant eframe as eframe Runtime
    participant App as TunnyApp::update()
    participant Layout as layout.rs show_layout()
    participant Toolbar as toolbar.rs
    participant Grid as grid_canvas.rs
    participant Theme as theme.rs 定数

    eframe->>App: update(ctx, frame) 呼び出し
    App->>Layout: show_layout(app, ctx)

    Layout->>Theme: TOOLBAR_BG 参照
    Layout->>ctx: TopBottomPanel.frame(Frame::fill(TOOLBAR_BG))
    ctx->>Toolbar: ツールバーUI描画（TOOLBAR_TEXT色でテキスト）

    Layout->>ctx: SidePanel::left (PANEL_BG はVisuals.panel_fill から自動適用)
    Layout->>ctx: SidePanel::right (同上)

    Layout->>ctx: CentralPanel (CENTRAL_BG はVisuals.window_fill から自動適用)
    ctx->>Grid: show_grid_canvas()
    Grid->>Theme: BORDER_COLOR, ACCENT_BLUE, CELL_TOOLBAR_BG 参照
    Grid->>ctx: セル境界線描画 (BORDER_COLOR)
    Grid->>ctx: ホバーハイライト描画 (ACCENT_BLUE alpha40)
    Grid->>ctx: セルツールバー Frame.fill(CELL_TOOLBAR_BG)
```

---

## カラー定数の参照フロー 🔵

**信頼性**: 🔵 *Rust const 伝搬の仕組みより*

```mermaid
graph TD
    Theme[theme.rs\nconst 定数群]
    Visuals[tunny_light_visuals\negui::Visuals]
    AppNew[app.rs\nTunnyApp::new]
    Context[egui::Context\nグローバルVisuals]
    Layout[layout.rs\nToolbar Frame]
    Grid[grid_canvas.rs\nセル境界・ホバー]

    Theme -->|カラー定数| Visuals
    Theme -->|TOOLBAR_BG| Layout
    Theme -->|BORDER_COLOR\nACCENT_BLUE\nCELL_TOOLBAR_BG| Grid
    Visuals -->|set_visuals| AppNew
    AppNew -->|初期化時| Context
    Context -->|panel_fill\nwidget_bg等| Layout
    Context -->|selection\nhover等| Grid
```

---

## Visuals フィールドマッピング 🔵

**信頼性**: 🔵 *egui 0.30 Visuals struct API仕様より*

| egui Visuals フィールド | 設定値トークン | 影響するUI要素 |
|---|---|---|
| `dark_mode` | `false` | ライトテーマベース |
| `panel_fill` | `PANEL_BG` | 左パネル・右パネル背景 |
| `window_fill` | `CENTRAL_BG` | メインキャンバス背景 |
| `window_stroke` | `BORDER_COLOR` | パネル境界線 |
| `widgets.active.bg_fill` | `ACCENT_BLUE` | クリック中ボタン背景 |
| `widgets.active.fg_stroke` | `WHITE` | クリック中ボタンテキスト |
| `widgets.hovered.bg_fill` | `WIDGET_BG_HOVER` | ホバー時ウィジェット背景 |
| `widgets.inactive.bg_fill` | `WIDGET_BG` | 非アクティブウィジェット背景 |
| `widgets.inactive.bg_stroke` | `BORDER_COLOR` | 非アクティブ枠線 |
| `selection.bg_fill` | `ACCENT_BLUE_MUTED` | selectable_label選択背景 |
| `selection.stroke` | `ACCENT_BLUE` | 選択枠線 |
| `override_text_color` | `TEXT_PRIMARY` | 全テキストのデフォルト色 |
| `extreme_bg_color` | `WHITE` | テキスト入力・ComboBox背景 |

---

## ツールバー固有のテキスト色オーバーライドフロー 🔵

**信頼性**: 🔵 *egui ui.visuals_mut() API仕様より*

グローバルの `override_text_color = TEXT_PRIMARY (dark)` はツールバー内では不適切（背景がダークネイビーのため）。
`ui.visuals_mut()` でツールバースコープ内のテキスト色を上書きする。

```mermaid
sequenceDiagram
    participant Layout as layout.rs
    participant PanelUI as TopBottomPanel UI
    participant Toolbar as toolbar.rs

    Layout->>PanelUI: show(ctx, |ui| ...)
    Layout->>PanelUI: ui.visuals_mut().override_text_color = Some(TOOLBAR_TEXT)
    PanelUI->>Toolbar: show_toolbar(ui, ...) を呼び出し
    Note over Toolbar: ツールバー内テキストは TOOLBAR_TEXT (明るいグレー) で描画
    Note over PanelUI: スコープ外には影響しない
```

---

## 関連文書

- **アーキテクチャ**: [architecture.md](architecture.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **ヒアリング記録**: [design-interview.md](design-interview.md)

## 信頼性レベルサマリー

- 🔵 青信号: 6件 (100%)
- 🟡 黄信号: 0件 (0%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
