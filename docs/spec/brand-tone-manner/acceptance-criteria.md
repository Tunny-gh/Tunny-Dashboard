# ブランドトンマナ統一 受け入れ基準

**作成日**: 2026-05-25
**関連要件定義**: [requirements.md](requirements.md)
**関連ユーザストーリー**: [user-stories.md](user-stories.md)
**ヒアリング記録**: [interview-record.md](interview-record.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: TONMANUAL・ユーザヒアリングを参考にした確実な基準
- 🟡 **黄信号**: TONMANUAL・既存実装から妥当な推測による基準
- 🔴 **赤信号**: TONMANUAL・ユーザヒアリングにない推測による基準

---

## [SCOPE-1] egui テーマカラー

### REQ-001〜REQ-012: カラー定数の値変更 🔵

**信頼性**: 🔵 *TONMANUAL §2 カラーパレット・ユーザヒアリングより*

#### Given
- `egui-app/src/theme/ui_colors.rs` が存在する

#### When
- ファイルを編集し `cargo build` が完了する

#### Then
- 各定数が以下の HEX 値を持つ

| 定数名 | 期待 HEX | TONMANUAL 根拠 |
|--------|---------|--------------|
| `ACCENT_BLUE` | `#3B82F6` | blue-500 |
| `ACCENT_BLUE_HOVER` | `#2563EB` | blue-600 |
| `ACCENT_BLUE_MUTED` | `#BFDBFE` | blue-200 |
| `PANEL_BG` | `#F3F4F6` | gray-100 |
| `CENTRAL_BG` | `#FFFFFF` | white |
| `TEXT_PRIMARY` | `#111827` | gray-900 |
| `TEXT_SECONDARY` | `#4B5563` | gray-600 |
| `BORDER_COLOR` | `#E5E7EB` | gray-200 |
| `TOOLBAR_BG` | `#BFDBFE` | blue-200 |
| `TOOLBAR_TEXT` | `#374151` | gray-700 |
| `TOOLBAR_INPUT_BG` | `#F3F4F6` | gray-100 |
| `TOOLBAR_INPUT_STROKE` | `#E5E7EB` | gray-200 |
| `TOOLBAR_BTN_ACTIVE` | `#3B82F6` | blue-500 |

#### テストケース

##### 正常系

- [ ] **TC-001-01**: 各定数の HEX 値が上表と一致する 🔵
  - **確認方法**: `ui_colors.rs` を目視確認 + `cargo build` がエラーなく完了
  - **信頼性**: 🔵 *TONMANUAL §2 より*

- [ ] **TC-001-02**: `cargo build --workspace` がエラーなく完了する 🔵
  - **確認方法**: `cargo build --workspace` を実行して exit code 0
  - **信頼性**: 🔵 *CLAUDE.md ビルドコマンドより*

- [ ] **TC-001-03**: `cargo test --workspace` が全件成功する 🔵
  - **確認方法**: `cargo test --workspace` を実行して全テスト PASS
  - **信頼性**: 🔵 *CLAUDE.md テストコマンドより*

##### 視認性確認

- [ ] **TC-001-V01**: ツールバー（TOOLBAR_BG = #BFDBFE）上の TOOLBAR_TEXT (#374151) のコントラスト比 ≥ 4.5:1 🟡
  - **確認方法**: コントラスト比チェッカーで #BFDBFE 背景 / #374151 テキストを確認
  - **信頼性**: 🟡 *WCAG AA 基準から妥当な推測*

---

### REQ-013〜REQ-017: 新規カラー定数の追加 🔵

**信頼性**: 🔵 *TONMANUAL §2 カラーパレットより*

#### Given
- `ui_colors.rs` が存在する

#### When
- 新規定数を追加して `cargo build` が完了する

#### Then
- 以下の定数が追加されている

| 定数名 | 期待 HEX | TONMANUAL 根拠 |
|--------|---------|--------------|
| `HEADER_BG` | `#93C5FD` | blue-300 |
| `ANNOUNCE_BG` | `#60A5FA` | blue-400 |
| `ACTION_GREEN` | `#22C55E` | green-500 |
| `ACTION_GREEN_HOVER` | `#16A34A` | green-600 |
| `TEXT_SUB` | `#374151` | gray-700 |

#### テストケース

- [ ] **TC-013-01**: 上記 5 定数が `ui_colors.rs` に存在する 🔵
  - **確認方法**: ファイルを grep して全定数を確認
  - **信頼性**: 🔵

- [ ] **TC-013-02**: `cargo build --workspace` がエラーなく完了する 🔵
  - **信頼性**: 🔵

---

## [SCOPE-2] ヘルプ HTML スタイル

### REQ-101〜REQ-107: ヘルプ HTML CSS の更新 🔵

**信頼性**: 🔵 *TONMANUAL §3 タイポグラフィ・§2 カラーより*

#### Given
- `egui-app/build.rs` の `wrap_as_standalone_html` 関数が存在する
- `theory/` ディレクトリに Markdown ファイルが存在する

#### When
- `build.rs` の CSS を修正して `cargo build` が完了する

#### Then
- 生成された HTML ファイル（`OUT_DIR/help/**/*.html`）が以下のスタイルを含む

| CSS プロパティ | 期待値 | TONMANUAL 根拠 |
|-------------|-------|--------------|
| `body { color: ... }` | `#4B5563` | gray-600 本文 |
| `h1, h2, h3 { color: ... }` | `#111827` | gray-900 見出し |
| `h1, h2, h3 { font-weight: ... }` | `800` | font-extrabold |
| `table th { background: ... }` | `#F3F4F6` | gray-100 |
| `border color` 系 | `#E5E7EB` | gray-200 |
| `code, pre { background: ... }` | `#F3F4F6` | gray-100 |
| `a { color: ... }` | `#2563EB` | blue-600 |

#### テストケース

##### 正常系

- [ ] **TC-101-01**: `cargo build` 後に `OUT_DIR/help/ja/README.html` に上記 CSS 値が含まれる 🔵
  - **確認方法**: 生成ファイルを検索して CSS 値を確認
  - **信頼性**: 🔵 *TONMANUAL §3 より*

- [ ] **TC-101-02**: KaTeX による数式レンダリングが引き続き動作する 🔵
  - **確認方法**: ヘルプブラウザで数式ページを開き、数式が正しく表示される
  - **信頼性**: 🔵 *build.rs 既存仕様より*

- [ ] **TC-101-03**: Markdown テーブルの `th` が gray-100 (#F3F4F6) 背景で表示される 🔵
  - **確認方法**: ヘルプブラウザでテーブルを含むページを開いて目視確認
  - **信頼性**: 🔵

##### 異常系

- [ ] **TC-101-E01**: `katex.min.css` の色定義と競合しない 🟡
  - **入力**: KaTeX が提供するデフォルト CSS
  - **期待結果**: KaTeX の数式スタイルがブランドCSS変更で崩れない
  - **信頼性**: 🟡 *build.rs 既存実装から妥当な推測*

---

## [SCOPE-3] HTML エクスポートレポート

### REQ-201〜REQ-207: HTML レポート CSS・レイアウトの更新 🔵

**信頼性**: 🔵 *TONMANUAL §2 §3・ユーザヒアリングより*

#### Given
- `egui-app/src/io/html_report.rs` の `build_html_report` 関数が存在する

#### When
- CSS を更新して HTML レポートを出力する

#### Then
- 出力 HTML が以下のスタイルを含む

| CSS プロパティ | 期待値 | TONMANUAL 根拠 |
|-------------|-------|--------------|
| `h1, h2 { color: ... }` | `#111827` | gray-900 |
| `body { color: ... }` | `#4B5563` | gray-600 |
| `th { background: ... }` | `#F3F4F6` | gray-100 |
| `th, td { border-color: ... }` | `#E5E7EB` | gray-200 |
| `.card { border-radius: ... }` | `8px` | rounded-lg |
| `.card { border-color: ... }` | `#E5E7EB` | gray-200 |

#### テストケース

##### 正常系

- [ ] **TC-201-01**: `build_html_report` が上記 CSS 値を含む HTML を返す 🔵
  - **確認方法**: ユニットテストまたはブラウザで確認
  - **信頼性**: 🔵 *TONMANUAL §2 §3 より*

- [ ] **TC-201-02**: 散布図 SVG の Pareto 外点が `#3B82F6`（blue-500）で描画される 🔵
  - **確認方法**: 生成 HTML の SVG タグ内 `fill` 属性を確認
  - **信頼性**: 🔵 *TONMANUAL §2 メインブルーより*

- [ ] **TC-201-03**: HTML 文書が外部リソースへの参照を持たない（スタンドアロン） 🔵
  - **確認方法**: 生成 HTML 内に `http://`・`https://` 参照がないことを確認
  - **信頼性**: 🔵 *html_report.rs 既存仕様より*

- [ ] **TC-201-04**: ブラウザで開いたレポートに JS エラーがない 🟡
  - **確認方法**: Chrome DevTools で確認
  - **信頼性**: 🟡 *既存実装から妥当な推測*

##### 正常系（ブランドヘッダー REQ-206: Should Have）

- [ ] **TC-206-01**: H1 上部に blue-300 (#93C5FD) 背景のブランドヘッダーバーが存在する 🟡
  - **確認方法**: 生成 HTML を目視確認
  - **信頼性**: 🟡 *TONMANUAL §4 ドキュメントヘッダーから推測*

---

## テストケースサマリー

### カテゴリ別件数

| カテゴリ | 正常系 | 異常系 | 境界値 | 合計 |
|---------|--------|--------|--------|------|
| SCOPE-1 egui テーマ | 5 | 0 | 0 | 5 |
| SCOPE-2 ヘルプ HTML | 3 | 1 | 0 | 4 |
| SCOPE-3 エクスポート | 4 | 0 | 0 | 4 |
| **合計** | **12** | **1** | **0** | **13** |

### 信頼性レベル分布

- 🔵 青信号: 10件 (77%)
- 🟡 黄信号: 3件 (23%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質

### 優先度別テストケース

- **Must Have**: 12件
- **Should Have**: 1件（TC-206-01）
- **Could Have**: 0件

---

## テスト実施計画

### Phase 1: ビルド確認（SCOPE-1 カラー値変更）
- TC-001-01, TC-001-02, TC-001-03, TC-013-01, TC-013-02
- 優先度: Must Have
- 条件: `cargo build --workspace` + `cargo test --workspace` が通ること

### Phase 2: HTML スタイル確認（SCOPE-2・SCOPE-3）
- TC-101-01〜TC-101-E01, TC-201-01〜TC-201-04
- 優先度: Must Have
- 条件: ブラウザで生成 HTML を目視確認

### Phase 3: 視認性・オプション確認
- TC-001-V01（コントラスト比）, TC-206-01（ブランドヘッダー）
- 優先度: Should Have
- 条件: 実装後にデザインレビュー
