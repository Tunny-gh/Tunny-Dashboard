# HTML Help Browser データフロー図

**作成日**: 2026-05-14
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連要件定義**: [requirements.md](../../spec/html-help-browser/requirements.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実なフロー
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測によるフロー
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測によるフロー

---

## システム全体のデータフロー 🔵

**信頼性**: 🔵 *要件定義・アーキテクチャ設計より*

```
[ビルド時]
theory/en/*.md ──→ build.rs ──→ OUT_DIR/en/*.html
theory/ja/*.md ──→ build.rs ──→ OUT_DIR/ja/*.html
                                    │
                                    ▼
                            include_str! でバイナリ埋め込み

[実行時]
ユーザー → Help ボタン → HelpLauncher → 一時ファイル書き出し → open::that → ブラウザ
```

## 主要機能のデータフロー

### フロー1: ビルド時Markdown→HTML変換 🔵

**信頼性**: 🔵 *要件定義 REQ-001〜004・既存 build.rs パターンより*

**関連要件**: REQ-001, REQ-002, REQ-003, REQ-004

```
[build.rs 実行フロー]

1. CARGO_MANIFEST_DIR/theory/ をスキャン
   ├── theory/en/**/*.md → ファイルリストEN
   └── theory/ja/**/*.md → ファイルリストJA

2. 各 .md ファイルに対して:
   ├── pulldown-cmark で Markdown → HTML 変換
   │   ├── Heading → <h1>〜<h3>
   │   ├── **bold** → <strong>
   │   ├── `code` → <code>
   │   ├── - list → <ul><li>
   │   ├── | table | → <table>
   │   └── ```code``` → <pre><code>
   │
   ├── KaTeX 数式プレースホルダ処理
   │   ├── $...$ → <span class="katex">...</span>
   │   └── $$...$$ → <div class="katex-display">...</div>
   │
   └── HTML ラッパー生成
       ├── <!DOCTYPE html> + <html>
       ├── <style> ... ライトテーマCSS + KaTeX CSS ... </style>
       ├── <body> ... 変換済みHTML ... </body>
       ├── <script> ... KaTeX JS + auto-render ... </script>
       └── </html>

3. OUT_DIR に .html ファイル出力
   ├── OUT_DIR/help/en/sensitivity-analysis/overview.html
   ├── OUT_DIR/help/en/mcdm/topsis.html
   ├── OUT_DIR/help/ja/sensitivity-analysis/overview.html
   └── ...

4. cargo:rerun-if-changed=theory/ を出力
```

### フロー2: ヘルプボタン押下→ブラウザ起動 🔵

**信頼性**: 🔵 *要件定義 REQ-020〜023・既存 CellToolbarAction パターンより*

**関連要件**: REQ-020, REQ-021, REQ-022, REQ-023

```
[シーケンス: ヘルプ表示]

ユーザー          grid_canvas.rs       help_launcher.rs      OS/ブラウザ
  │                    │                      │                    │
  │ Help クリック      │                      │                    │
  │───────────────────>│                      │                    │
  │                    │                      │                    │
  │                    │ open_help(item, lang) │                    │
  │                    │─────────────────────>│                    │
  │                    │                      │                    │
  │                    │   1. HTML文字列取得   │                    │
  │                    │   2. 一時パス生成     │                    │
  │                    │   3. ファイル書き出し │                    │
  │                    │                      │                    │
  │                    │                      │ open::that(path)   │
  │                    │                      │───────────────────>│
  │                    │                      │                    │
  │                    │                      │    Ok / Err        │
  │                    │                      │<───────────────────│
  │                    │                      │                    │
  │                    │   Result<(), String>  │                    │
  │                    │<─────────────────────│                    │
  │                    │                      │                    │
  │  （ブラウザが別ウィンドウで開く）          │                    │
  │<─────────────────────────────────────────────────────────────│
```

**詳細ステップ**:
1. `grid_canvas.rs` が `CellToolbarAction::Help(panel_item)` を受け取る
2. `help_launcher::open_help(panel_item, app_state.help_language)` を呼び出す
3. `help_content::get_help_html(panel_item, lang)` で埋め込みHTML文字列を取得
4. `std::env::temp_dir()` / `"tunny-help-{widget}-{lang}.html"` に書き出す
5. `open::that(&path)` でデフォルトブラウザで開く
6. 失敗時は `Err` を返し、呼び出し元でトースト通知表示

### フロー3: 言語切替 🔵

**信頼性**: 🔵 *要件定義 REQ-030〜032・ヒアリング: メニュー内配置より*

**関連要件**: REQ-030, REQ-031, REQ-032

```
[状態遷移: 言語設定]

     ┌──────────────┐
     │  HelpLanguage │
     │  { En, Ja }  │
     └──────┬───────┘
            │
     AppState.help_language
            │
     ┌──────▼───────┐     メニュークリック     ┌───────────┐
     │   English    │─────────────────────────>│ Japanese  │
     │   (Default)  │<─────────────────────────│           │
     └──────────────┘     メニュークリック     └───────────┘
            │                                          │
            ▼                                          ▼
     help_content::get_help_html(     help_content::get_help_html(
         item, HelpLanguage::En)         item, HelpLanguage::Ja)
            │                                          │
            ▼                                          ▼
     include_str!(EN_HTML)              include_str!(JA_HTML)
```

### フロー4: 旧システムからの移行 🔵

**信頼性**: 🔵 *要件定義 REQ-050〜053・既存実装より*

**関連要件**: REQ-050, REQ-051, REQ-052, REQ-053

```
[移行前]                          [移行後]

help/                             help/
├── mod.rs                        ├── mod.rs              ◁ 変更
│   (4モジュール)                  │   (2モジュールに削減)
├── help_types.rs                  ├── help_types.rs       ◁ 変更
│   HelpModalState                 │   HelpLanguage enum
│   HelpContent                    │   HelpContent (再設計)
│   HelpTabDef                     │
├── help_content.rs                ├── help_content.rs     ◁ 変更
│   PanelItem → HelpContent       │   PanelItem → HTML文字列
│   (include_str! md)              │   (include_str! html)
├── help_modal.rs                  └── help_launcher.rs    ◀ 新規
│   show_help_modal()                  open_help()
│   (egui Window)                      write_temp_file()
└── md_renderer.rs                      open::that()
    render_markdown()
    (カスタムパーサー)

widget_states.rs:                  widget_states.rs:
  help_modal: HelpModalState         help_state: HelpLanguage
```

## エラーハンドリングフロー 🟡

**信頼性**: 🟡 *要件定義 EDGE-001, EDGE-002・既存パターンから妥当な推測*

```
[エラーハンドリング]

help_launcher::open_help()
        │
        ├── Ok(()) → 処理終了（ブラウザが開く）
        │
        └── Err(e) → エラー種別に応じた処理
              │
              ├── FileWriteError
              │   └── "一時ファイルの書き出しに失敗: {e}"
              │
              ├── BrowserLaunchError
              │   └── "ブラウザの起動に失敗: {e}"
              │
              └── ContentNotFoundError
                  └── "ヘルプコンテンツが見つかりません"
```

エラー表示は egui の `ui.label()` またはトースト通知で行う。`Result<(), String>` を返し、呼び出し元（grid_canvas.rs）で表示する。

## 状態管理フロー

### 言語設定の状態管理 🔵

**信頼性**: 🔵 *既存 AppState パターン・要件定義より*

```
AppState {
    ...既存フィールド...
    help_language: HelpLanguage,  // ◀ 新規追加
}

// new() で English をデフォルト
// clear() でリセットしない（selected_colormap と同じパターン）
```

### 一時ファイルライフサイクル 🟡

**信頼性**: 🟡 *既存 tempfile 使用パターンから妥当な推測*

```
1. ヘルプボタン押下
2. temp_dir() / "tunny-help-{widget}-{lang}.html" に書き出し
   （同名ファイルは上書き → 同一ヘルプの複数タブ防止）
3. open::that() でブラウザに読み込みさせる
4. ファイル自体はブラウザが読み込んだ後は不要
   （OS のテンポラリディレクトリクリーンアップに委ねる）
```

## データ整合性の保証 🔵

**信頼性**: 🔵 *ビルド時生成の特性より*

- **HTML一貫性**: 全HTMLはビルド時に一括生成されるため、実行時のデータ不整合は発生しない
- **言語一貫性**: EN/JA の両HTMLが同じビルドプロセスで生成される
- **数式一貫性**: LaTeX記法のパースは pulldown-cmark + KaTeX render で一括処理

## 関連文書

- **アーキテクチャ**: [architecture.md](architecture.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **設計ヒアリング**: [design-interview.md](design-interview.md)
- **要件定義**: [requirements.md](../../spec/html-help-browser/requirements.md)

## 信頼性レベルサマリー

- 🔵 青信号: 7件 (78%)
- 🟡 黄信号: 2件 (22%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質 — ビルド時生成パターンにより実行時のデータ整合性リスクを排除
