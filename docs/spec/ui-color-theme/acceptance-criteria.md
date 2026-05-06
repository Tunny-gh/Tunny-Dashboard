# UIカラー設定一元化 受け入れ基準

**作成日**: 2026-05-07
**関連要件定義**: [requirements.md](requirements.md)
**関連ユーザストーリー**: [user-stories.md](user-stories.md)
**ヒアリング記録**: [interview-record.md](interview-record.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: ユーザヒアリング・既存実装を参考にした確実な基準
- 🟡 **黄信号**: 既存実装・設計から妥当な推測による基準
- 🔴 **赤信号**: ヒアリング・実装にない推測による基準

---

## REQ-001〜005: ディレクトリ構造の確立 🔵

**信頼性**: 🔵 *ユーザヒアリングより*

### Given（前提条件）
- `egui-app/src/theme.rs` が存在する
- `egui-app/src/render/colormap.rs` が存在する

### When（実行条件）
- 移行作業を完了させる

### Then（期待結果）
- `egui-app/src/theme/` ディレクトリが存在する
- `egui-app/src/theme/mod.rs` が存在する
- `egui-app/src/theme/colormap.rs` が存在する
- `egui-app/src/theme/chart_colors.rs` が存在する
- `egui-app/src/theme.rs`（旧ファイル）が存在しない

### テストケース

#### 正常系

- [ ] **TC-001-01**: ディレクトリ・ファイル存在確認 🔵
  - **確認コマンド**: `ls egui-app/src/theme/`
  - **期待結果**: `mod.rs`, `colormap.rs`, `chart_colors.rs` の 3 ファイルが存在する
  - **信頼性**: 🔵

- [ ] **TC-001-02**: 旧 theme.rs 不存在確認 🔵
  - **確認コマンド**: `ls egui-app/src/theme.rs` がエラーになる
  - **期待結果**: ファイルが存在しない
  - **信頼性**: 🔵

- [ ] **TC-001-03**: `crate::theme` モジュールパスが有効であること 🔵
  - **確認コマンド**: `cargo check`
  - **期待結果**: コンパイルエラーなし
  - **信頼性**: 🔵

---

## REQ-011〜013: theme/mod.rs の内容 🔵

**信頼性**: 🔵 *既存 theme.rs の内容より*

### Given（前提条件）
- `theme/mod.rs` が存在する

### When（実行条件）
- `cargo check` を実行する

### Then（期待結果）
- 下記の定数がすべて `crate::theme` から参照可能である

### テストケース

#### 正常系

- [ ] **TC-011-01**: 全 UIテーマ定数の存在確認 🔵
  - **確認内容**: TOOLBAR_BG / TOOLBAR_TEXT / PANEL_BG / CENTRAL_BG / ACCENT_BLUE / ACCENT_BLUE_HOVER / ACCENT_BLUE_MUTED / BORDER_COLOR / TEXT_PRIMARY / TEXT_SECONDARY / CELL_TOOLBAR_BG / WIDGET_BG / WIDGET_BG_HOVER / TOOLBAR_BTN_HOVER / TOOLBAR_BTN_ACTIVE / TOOLBAR_INPUT_BG / TOOLBAR_INPUT_STROKE がすべて定義されている
  - **信頼性**: 🔵

- [ ] **TC-011-02**: 色値が変わっていないこと 🔵
  - **確認内容**: TOOLBAR_BG = `Color32::from_rgb(26, 35, 50)` 等、旧 `theme.rs` と同値であること
  - **信頼性**: 🔵

- [ ] **TC-011-03**: `tunny_light_visuals()` が呼び出せること 🔵
  - **確認コマンド**: `cargo check`
  - **期待結果**: `crate::theme::tunny_light_visuals` が参照可能
  - **信頼性**: 🔵

- [ ] **TC-013-01**: `ERROR_COLOR` 定数の存在 🟡
  - **確認内容**: `crate::theme::ERROR_COLOR` が赤系色として定義されている
  - **信頼性**: 🟡 *複数ウィジェットでの Color32::RED 使用から推測*

---

## REQ-021〜025: theme/colormap.rs の内容 🔵

**信頼性**: 🔵 *既存 colormap.rs の内容より*

### Given（前提条件）
- `theme/colormap.rs` が存在する

### When（実行条件）
- `cargo test` を実行する

### Then（期待結果）
- 既存のすべてのテストが通過する
- ColorMap struct と全グラデーション関数が `crate::theme::colormap` から参照可能である

### テストケース

#### 正常系

- [ ] **TC-021-01**: 全グラデーション関数の存在確認 🔵
  - **確認内容**: viridis / plasma / blue_yellow / jet / turbo / inferno / coolwarm / spectral / cividis がすべて定義されている
  - **信頼性**: 🔵

- [ ] **TC-021-02**: `tab10_palette()` の存在 🔵
  - **確認内容**: 10 色を返すことをテストが検証する
  - **信頼性**: 🔵

- [ ] **TC-025-01**: 既存テスト全通過 🔵
  - **確認コマンド**: `cargo test`
  - **期待結果**: `interpolate_at_zero_returns_first_stop` 等の既存テストがすべて通過する
  - **信頼性**: 🔵

#### 循環依存確認

- [ ] **TC-EDGE-001-01**: 循環依存の不在 🔵
  - **確認コマンド**: `cargo check`
  - **条件**: `theme/colormap.rs` が `state::app_state` を参照しても、`state::app_state` が `theme` を参照しない
  - **信頼性**: 🔵

---

## REQ-031〜035: theme/chart_colors.rs の内容 🔵

**信頼性**: 🔵 *ウィジェットコード分析・ユーザヒアリングより*

### Given（前提条件）
- `theme/chart_colors.rs` が存在する

### When（実行条件）
- `cargo check` を実行する

### Then（期待結果）
- 下記の定数グループがすべて `crate::theme::chart_colors` から参照可能である

### テストケース

#### 正常系

- [ ] **TC-031-01**: Pareto 色定数の存在 🔵
  - **確認内容**: COLOR_PARETO（赤）/ COLOR_NON_PARETO（青）/ 各 DIM バリアントが定義されている
  - **信頼性**: 🔵

- [ ] **TC-033-01**: MCDM スコア色定数の存在 🔵
  - **確認内容**: COLOR_MCDM_SCORE_HIGH / COLOR_MCDM_SCORE_MID / COLOR_MCDM_SCORE_LOW / COLOR_MCDM_SCORE_NONE が定義されている
  - **信頼性**: 🔵

- [ ] **TC-034-01**: 重複していた Pareto 色が一意の定数であること 🟡
  - **確認内容**: `pareto_2d.rs` と `slice_chart.rs` で同一の `crate::theme::chart_colors::COLOR_PARETO` を参照している
  - **信頼性**: 🟡

- [ ] **TC-035-01**: 最適化履歴線色定数の存在 🟡
  - **確認内容**: `optimization_history.rs` が参照する青/赤/緑/金の色が定数として定義されている
  - **信頼性**: 🟡

---

## REQ-041〜043: インポートルール 🔵

**信頼性**: 🔵 *ユーザヒアリングより*

### Given（前提条件）
- theme ディレクトリへの全色移行が完了している

### When（実行条件）
- `grep -r "Color32::from_rgb" egui-app/src/ui --include="*.rs"` を実行する

### Then（期待結果）
- 動的計算色（変数アルファ等）を除き、マッチがゼロである

### テストケース

#### 正常系

- [ ] **TC-042-01**: ウィジェット内インラインリテラルゼロ確認 🔵
  - **確認コマンド**: `grep -rn "Color32::from_rgb" egui-app/src/ui --include="*.rs"`
  - **期待結果**: 結果が 0 件、または動的計算色のみが残る
  - **信頼性**: 🔵

- [ ] **TC-041-01**: ウィジェットが theme をインポートしていること 🔵
  - **確認内容**: 色定数を使用する各ウィジェットファイルが `use crate::theme` または `use crate::theme::chart_colors` のインポートを持つ
  - **信頼性**: 🔵

---

## NFR-001, NFR-011: ビルド・テスト 🔵

**信頼性**: 🔵 *プロジェクト品質基準より*

### テストケース

- [ ] **TC-NFR-001-01**: `cargo build` 警告ゼロ 🔵
  - **確認コマンド**: `cargo build 2>&1 | grep -i warning`
  - **期待結果**: 出力がゼロ行
  - **信頼性**: 🔵

- [ ] **TC-NFR-011-01**: `cargo test` 全通過 🔵
  - **確認コマンド**: `cargo test`
  - **期待結果**: 全テスト PASS（失敗ゼロ）
  - **信頼性**: 🔵

---

## NFR-021: 保守性の検証 🔵

### テストケース

- [ ] **TC-NFR-021-01**: 色変更が theme 配下のみで完結すること 🔵
  - **確認内容**: ACCENT_BLUE の RGB 値を変更した際に `egui-app/src/theme/` 配下のファイルのみの編集でアプリ全体の色が変わること
  - **信頼性**: 🔵

---

## テストケースサマリー

### カテゴリ別件数

| カテゴリ | 正常系 | 異常系 | 境界値 | 合計 |
|---------|--------|--------|--------|------|
| 機能要件 | 14 | 0 | 1 | 15 |
| 非機能要件 | 3 | 0 | 0 | 3 |
| Edgeケース | 1 | 0 | 0 | 1 |
| **合計** | 18 | 0 | 1 | 19 |

### 信頼性レベル分布

- 🔵 青信号: 15件 (79%)
- 🟡 黄信号: 4件 (21%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質

### 優先度別テストケース

- **Must Have**: 14件
- **Should Have**: 4件
- **Could Have**: 1件

---

## テスト実施計画

### Phase 1: ディレクトリ移行テスト
- TC-001-01〜03（構造確認）
- TC-011-01〜03（mod.rs 内容）
- TC-021-01〜02（colormap.rs 内容）
- TC-025-01（既存テスト通過）

### Phase 2: チャート色集約テスト
- TC-031-01（Pareto 色）
- TC-033-01（MCDM スコア色）
- TC-034-01（重複解消）
- TC-EDGE-001-01（循環依存）

### Phase 3: インラインリテラル除去テスト
- TC-042-01（インラインゼロ確認）
- TC-041-01（インポート確認）
- TC-NFR-001-01（ビルド警告ゼロ）
- TC-NFR-011-01（全テスト通過）
- TC-NFR-021-01（保守性検証）
