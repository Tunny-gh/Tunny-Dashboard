# Widget Help データフロー図

**作成日**: 2026-05-08
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連要件定義**: [requirements.md](../../spec/widget-help/requirements.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実なフロー
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測によるフロー
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測によるフロー

---

## システム全体のデータフロー 🔵

**信頼性**: 🔵 *要件定義・既存アーキテクチャより*

```
[theory/en/*.md]  ←コンパイル時埋め込み→  [include_str!]
                                            ↓
[? button] ──click──→ [HelpModalState.open = true]
                            ↓
                    [PanelItem::help_content()]
                            ↓
                    [HelpContent { tabs: [HelpTabDef] }]
                            ↓
                    [show_help_modal()]
                            ↓
                    ┌─────────────────────────┐
                    │   egui::Window          │
                    │  ┌─────────────────┐    │
                    │  │ Tab Bar         │    │
                    │  │ [Overview] [Sobol]   │
                    │  ├─────────────────┤    │
                    │  │ ScrollArea      │    │
                    │  │ render_markdown │    │
                    │  │ (md → egui UI)  │    │
                    │  └─────────────────┘    │
                    └─────────────────────────┘
                            ↓
                    [Esc / Close button]
                            ↓
                    [HelpModalState.open = false]
```

## 主要機能のデータフロー

### フロー1: 「?」ボタンクリック → モーダル表示 🔵

**信頼性**: 🔵 *ユーザストーリー 1.1・grid_canvas.rs 実装より*

**関連要件**: REQ-001, REQ-002, REQ-003

```
ユーザー               grid_canvas.rs         WidgetStates           help_modal.rs
  │                        │                       │                       │
  │  「?」クリック          │                       │                       │
  ├──────────────────────→│                       │                       │
  │                        │  show_cell_toolbar()  │                       │
  │                        │  help_resp.clicked()  │                       │
  │                        │                       │                       │
  │                        │  CellToolbarAction::Help(item)                │
  │                        ├──────────────────────→│                       │
  │                        │  help_modal.open=true │                       │
  │                        │  help_modal.item=item │                       │
  │                        │                       │                       │
  │                        │  ← return to main loop →                      │
  │                        │                       │                       │
  │                        │                       │  show_help_modal()    │
  │                        │                       ├──────────────────────→│
  │                        │                       │  state.open == true   │
  │                        │                       │                       │
  │                        │                       │  item.help_content()  │
  │                        │                       │  → HelpContent lookup │
  │                        │                       │                       │
  │                        │                       │  egui::Window::show() │
  │                        │                       │  タブバー描画          │
  │                        │                       │  render_markdown()    │
  │  ← モーダル表示 ──────────────────────────────────────────────────────│
```

### フロー2: タブ切替 🔵

**信頼性**: 🔵 *ユーザストーリー 2.2・要件定義 REQ-006, REQ-013 より*

**関連要件**: REQ-006, REQ-013, REQ-014, REQ-015

```
ユーザー               help_modal.rs          HelpModalState
  │                        │                       │
  │  タブ"Sobol"クリック   │                       │
  ├──────────────────────→│                       │
  │                        │  ui.selectable_label() │
  │                        │  .clicked()           │
  │                        │                       │
  │                        │  state.active_tab = 3 │
  │                        ├──────────────────────→│
  │                        │                       │
  │                        │  content.tabs[3]      │
  │                        │  → Sobol HelpTabDef   │
  │                        │                       │
  │                        │  render_markdown(     │
  │                        │    tab.markdown       │
  │                        │  )                    │
  │  ← Sobol 内容表示 ────│                       │
```

### フロー3: モーダルを閉じる 🔵

**信頼性**: 🔵 *要件定義 REQ-005・artifact_modal パターンより*

**関連要件**: REQ-005

```
ユーザー               help_modal.rs          HelpModalState
  │                        │                       │
  │  Esc / 閉じるボタン    │                       │
  ├──────────────────────→│                       │
  │                        │  still_open = false   │
  │                        │                       │
  │                        │  state.open = false   │
  │                        │  state.active_tab = 0 │
  │                        ├──────────────────────→│
  │                        │                       │
  │  ← モーダル非表示 ────│                       │
```

### フロー4: ヘルプ未定義ウィジェットのフォールバック 🟡

**信頼性**: 🟡 *EDGE-001 から妥当な推測*

**関連要件**: EDGE-001

```
ユーザー               help_modal.rs          PanelItem
  │                        │                       │
  │  「?」クリック          │                       │
  ├──────────────────────→│                       │
  │                        │  item.help_content()  │
  │                        │                       │
  │                        │  match arm なし        │
  │                        │  → fallback content   │
  │                        │    "Help content not  │
  │                        │     available"        │
  │  ← プレースホルダ表示 ─│                       │
```

## コンパイル時データフロー 🔵

**信頼性**: 🔵 *アーキテクチャ設計・include_str! 仕様より*

```
[cargo build]
    │
    ├── theory/en/sensitivity-analysis/overview.md
    │     ↓ include_str!()
    │     ↓ &static str (バイナリに埋め込み)
    │
    ├── theory/en/sensitivity-analysis/sobol.md
    │     ↓ include_str!()
    │     ↓ &static str
    │
    ├── ... (全25+ファイル)
    │
    └── help_content.rs
          ↓ PanelItem::help_content() の match arm
          ↓ 各 arm で HelpContent 構造体を返す
          ↓ HelpTabDef { markdown: &static str } に include_str! 結果を設定
          ↓
          [最終バイナリ]
            └── 静的データとして全ヘルプテキストが含まれる
```

## Markdown→egui レンダリングフロー 🔵

**信頼性**: 🔵 *アーキテクチャ設計・要件定義 REQ-015 より*

```
[Markdown 文字列]
    │
    ↓ lines() で行分割
    │
    ├── "# Heading"    → ui.heading(text)
    ├── "## Heading"   → ui.strong(RichText::new(text).size(16.0))
    ├── "### Heading"  → ui.strong(RichText::new(text).size(14.0))
    ├── "- item"       → ui.horizontal { ui.label("•"); render_inline(text) }
    ├── "| table |"    → parse_table_row() → ui.horizontal { ui.label(col)... }
    ├── "```"          → toggle code_block mode
    ├── "$formula$"    → ui.label(monospace_text)  ← プレーンテキスト
    ├── "**bold**"     → ui.strong(text)
    ├── "`code`"       → ui.label(RichText::new(text).monospace())
    ├── ""             → ui.add_space(4.0)  ← 空行 = 段落区切り
    └── "plain text"   → render_inline(ui, text)
```

## 状態管理フロー 🔵

**信頼性**: 🔵 *既存 WidgetStates・artifact_modal パターンより*

```
HelpModalState:
  ┌──────────────────────────────────────────┐
  │ open: bool (default: false)              │
  │ active_tab: usize (default: 0)          │
  │ item: Option<PanelItem> (default: None) │
  └──────────────────────────────────────────┘

状態遷移:
  [初期] open=false → [?クリック] open=true, item=Some(panel_item)
                                  → [タブクリック] active_tab=N
                                  → [Esc/Close] open=false, active_tab=0, item=None
```

## 変更影響範囲 🔵

**信頼性**: 🔵 *コード分析より*

| ファイル | 変更種別 | 変更内容 |
|---------|---------|---------|
| `egui-app/src/ui/help/mod.rs` | 新規 | モジュール公開 |
| `egui-app/src/ui/help/help_modal.rs` | 新規 | モーダルUI |
| `egui-app/src/ui/help/help_content.rs` | 新規 | コンテンツルックアップ |
| `egui-app/src/ui/help/md_renderer.rs` | 新規 | Markdown レンダラ |
| `egui-app/src/ui/help/help_types.rs` | 新規 | 型定義 |
| `egui-app/src/ui/grid_canvas.rs` | 変更 | `show_cell_toolbar` に ? ボタン追加 |
| `egui-app/src/ui/widget_states.rs` | 変更 | `help_modal: HelpModalState` 追加 |
| `egui-app/src/app.rs` | 変更 | `show_help_modal()` 呼び出し追加 |
| `theory/` | 変更 | en/ja フォルダ再構成 + 英語コンテンツ新規作成 |

## 関連文書

- **アーキテクチャ**: [architecture.md](architecture.md)
- **ヒアリング記録**: [design-interview.md](design-interview.md)
- **要件定義**: [requirements.md](../../spec/widget-help/requirements.md)

## 信頼性レベルサマリー

- 🔵 青信号: 10件 (91%)
- 🟡 黄信号: 1件 (9%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質 — 既存コードパターンに基づく確実なフロー設計
