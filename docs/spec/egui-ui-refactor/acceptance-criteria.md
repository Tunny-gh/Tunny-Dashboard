# egui UI リファクタリング 受け入れ基準

**作成日**: 2026-05-08
**関連要件定義**: [requirements.md](requirements.md)
**関連ユーザストーリー**: [user-stories.md](user-stories.md)
**ヒアリング記録**: [interview-record.md](interview-record.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 既存コード分析・ユーザヒアリングを参考にした確実な基準
- 🟡 **黄信号**: 既存コード分析・ユーザヒアリングから妥当な推測による基準
- 🔴 **赤信号**: 既存コード・ユーザヒアリングにない推測による基準

---

## REQ-001: chart_registry.rs の描画・ディスパッチ分割 🔵

**信頼性**: 🔵 *ユーザヒアリング Q4・既存コード分析より*

### Given（前提条件）
- `egui-app/src/ui/chart_registry.rs` が ~750 行で `render_chart` + `poll_chart_work` を含んでいる

### When（実行条件）
- REQ-001 のリファクタリングを適用する

### Then（期待結果）
- `ui/render_chart.rs` が作成され、`render_chart` 関数が公開されている
- `ui/poll_chart.rs` が作成され、`poll_chart_work` 関数が公開されている
- `chart_registry.rs` は `show_chart` / `show_cell_chart` のみを含む薄いラッパーになっている
- `cargo build` がエラーなく完了する
- `cargo test` がグリーンのまま

### テストケース

#### 正常系

- [ ] **TC-001-01**: render_chart.rs にディスパッチコードが含まれない 🔵
  - **検証方法**: `grep -n "spawn_task\|tx\.clone\|\.send(" egui-app/src/ui/render_chart.rs` が 0 件
  - **期待結果**: マッチなし
  - **信頼性**: 🔵 *layer-contract.md・既存コード分析より*

- [ ] **TC-001-02**: poll_chart.rs に egui::Ui 操作が含まれない 🔵
  - **検証方法**: `grep -n "egui::Ui\|\.add(\|\.label(\|\.button(" egui-app/src/ui/poll_chart.rs` が 0 件
  - **期待結果**: マッチなし
  - **信頼性**: 🔵 *layer-contract.md・既存コード分析より*

- [ ] **TC-001-03**: chart_registry.rs の外部公開 API が変わらない 🔵
  - **検証方法**: `show_chart` / `show_cell_chart` のシグネチャが変更前と一致する
  - **期待結果**: シグネチャ変更なし（`cargo check` エラーなし）
  - **信頼性**: 🔵 *NFR-002・既存コード分析より*

- [ ] **TC-001-04**: cargo test グリーン 🔵
  - **検証方法**: `cargo test` 実行
  - **期待結果**: すべてのテストケースがパス
  - **信頼性**: 🔵 *NFR-001 より*

#### 異常系

- [ ] **TC-001-E01**: render_chart.rs が spawn_task を呼んだ場合はコンパイルエラー 🟡
  - **条件**: `poll_chart_work` の `spawn_task` を誤って render_chart.rs に移動した場合
  - **期待結果**: `cargo build` でコンパイルエラー（`tx` 引数がないため）
  - **信頼性**: 🟡 *Rust 型システムによる保証より*

---

## REQ-002: 計算ロジックを rust_core に移動 🔵

**信頼性**: 🔵 *ユーザヒアリング Q3・既存コード left_panel.rs 分析より*

### Given（前提条件）
- `left_panel.rs` に `normalize_weights`, `compute_improvement_rate`, `build_best_trial_history` が定義されている

### When（実行条件）
- REQ-002 のリファクタリングを適用する

### Then（期待結果）
- 3 関数の定義が `left_panel.rs` から消えている
- `rust_core` の適切なモジュールに 3 関数が定義されている
- `left_panel.rs` / ウィジェットから `tunny_core::xxx` 経由で呼び出せる
- `cargo test --workspace` がグリーン

### テストケース

#### 正常系

- [ ] **TC-002-01**: normalize_weights が tunny_core に存在する 🔵
  - **検証方法**: `grep -rn "pub fn normalize_weights" rust_core/src/` が 1 件ヒット
  - **期待結果**: 1 件ヒット
  - **信頼性**: 🔵 *ユーザヒアリング Q3 より*

- [ ] **TC-002-02**: normalize_weights の正規化精度テスト 🔵
  - **入力**: `weights = [1.0, 2.0, 1.0]`
  - **期待結果**: 合計が `1.0` (誤差 < 1e-10)
  - **信頼性**: 🔵 *既存テスト left_panel.rs::normalize_weights_sum_to_one より移植*

- [ ] **TC-002-03**: normalize_weights のゼロ合計テスト 🔵
  - **入力**: `weights = [0.0, 0.0, 0.0]`
  - **期待結果**: 各要素が `1/3`（均等分割）
  - **信頼性**: 🔵 *既存実装コードより*

- [ ] **TC-002-04**: compute_improvement_rate が tunny_core に存在する 🔵
  - **検証方法**: `grep -rn "pub fn compute_improvement_rate" rust_core/src/` が 1 件ヒット
  - **期待結果**: 1 件ヒット
  - **信頼性**: 🔵 *ユーザヒアリング Q3 より*

- [ ] **TC-002-05**: compute_improvement_rate の全改善ケース 🔵
  - **入力**: `history = [(0, 1.0), (1, 0.8), (2, 0.5)]`, `last_n = 100`
  - **期待結果**: `rate > 0.0`
  - **信頼性**: 🔵 *既存テスト improvement_rate_all_improving より移植*

- [ ] **TC-002-06**: compute_improvement_rate の空履歴ケース 🔵
  - **入力**: `history = []`, `last_n = 100`
  - **期待結果**: `rate == 0.0`
  - **信頼性**: 🔵 *既存テスト improvement_rate_empty_returns_zero より移植*

- [ ] **TC-002-07**: build_best_trial_history（最小化）のケース 🔵
  - **入力**: trial_ids = [0, 1, 2], obj_values = [1.0, 0.5, 0.8], is_minimize = true
  - **期待結果**: [(0, 1.0), (1, 0.5), (2, 0.5)]
  - **信頼性**: 🔵 *既存テスト build_best_trial_history_minimize より移植*

- [ ] **TC-002-08**: cargo test --workspace グリーン 🔵
  - **検証方法**: `cargo test --workspace` 実行
  - **期待結果**: すべてのテストがパス
  - **信頼性**: 🔵 *NFR-001 より*

---

## REQ-003: HTML レポート構築ロジックを io 層に移動 🔵

**信頼性**: 🔵 *ユーザヒアリング Q5・既存コード app.rs:77-108 分析より*

### Given（前提条件）
- `app.rs::apply_toolbar_actions` に ~30 行の `HtmlReportSnapshot` 構築ロジックが存在する

### When（実行条件）
- REQ-003 のリファクタリングを適用する

### Then（期待結果）
- `io/html_report.rs` に `build_and_send_report` 関数が追加されている
- `app.rs` の `GenerateHtmlReport` ハンドリングが 5 行以内に縮小している
- `app.rs` が `HtmlReportSnapshot` / `HtmlTrialRow` / `TrialStatistics` を import しない
- `cargo test` がグリーン

### テストケース

#### 正常系

- [ ] **TC-003-01**: app.rs が HtmlReportSnapshot をインポートしない 🔵
  - **検証方法**: `grep -n "HtmlReportSnapshot\|HtmlTrialRow\|TrialStatistics" egui-app/src/app.rs` が 0 件
  - **期待結果**: マッチなし
  - **信頼性**: 🔵 *REQ-003 仕様より*

- [ ] **TC-003-02**: io/html_report.rs に build_and_send_report が存在する 🔵
  - **検証方法**: `grep -n "pub fn build_and_send_report" egui-app/src/io/html_report.rs` が 1 件
  - **期待結果**: 1 件ヒット
  - **信頼性**: 🔵 *REQ-003 仕様より*

- [ ] **TC-003-03**: cargo test グリーン 🔵
  - **検証方法**: `cargo test` 実行
  - **期待結果**: すべてのテストがパス
  - **信頼性**: 🔵 *NFR-001 より*

---

## REQ-004: Trade-off Navigator と Convergence Card の widgets/ 分割 🔵

**信頼性**: 🔵 *ユーザヒアリング Q6・既存コード left_panel.rs:221-344 分析より*

### Given（前提条件）
- `left_panel.rs` に `show_tradeoff_navigator` と `show_convergence_card` が定義されている

### When（実行条件）
- REQ-004 のリファクタリングを適用する

### Then（期待結果）
- `ui/widgets/tradeoff_navigator.rs` が作成され、`pub fn show_tradeoff_navigator(...)` が含まれている
- `ui/widgets/convergence_card.rs` が作成され、`pub fn show_convergence_card(...)` が含まれている
- `left_panel.rs` からは両関数の定義が消えている
- `ui/widgets/mod.rs` が両モジュールを公開している
- `cargo test` がグリーン

### テストケース

#### 正常系

- [ ] **TC-004-01**: tradeoff_navigator.rs が存在する 🔵
  - **検証方法**: `test -f egui-app/src/ui/widgets/tradeoff_navigator.rs` (ファイル存在確認)
  - **期待結果**: ファイルが存在する
  - **信頼性**: 🔵 *REQ-004 仕様より*

- [ ] **TC-004-02**: convergence_card.rs が存在する 🔵
  - **検証方法**: `test -f egui-app/src/ui/widgets/convergence_card.rs` (ファイル存在確認)
  - **期待結果**: ファイルが存在する
  - **信頼性**: 🔵 *REQ-004 仕様より*

- [ ] **TC-004-03**: left_panel.rs が show_tradeoff_navigator を定義しない 🔵
  - **検証方法**: `grep -n "^pub fn show_tradeoff_navigator\|^fn show_tradeoff_navigator" egui-app/src/ui/left_panel.rs` が 0 件
  - **期待結果**: マッチなし
  - **信頼性**: 🔵 *REQ-004 仕様より*

- [ ] **TC-004-04**: left_panel.rs が show_convergence_card を定義しない 🔵
  - **検証方法**: `grep -n "^pub fn show_convergence_card\|^fn show_convergence_card" egui-app/src/ui/left_panel.rs` が 0 件
  - **期待結果**: マッチなし
  - **信頼性**: 🔵 *REQ-004 仕様より*

- [ ] **TC-004-05**: cargo test グリーン 🔵
  - **検証方法**: `cargo test` 実行
  - **期待結果**: すべてのテストがパス
  - **信頼性**: 🔵 *NFR-001 より*

---

## 非機能要件テスト

### NFR-001: テストのグリーン維持 🔵

- [ ] **TC-NFR-001-01**: 各 REQ 完了時に cargo test がグリーン 🔵
  - **測定項目**: `cargo test` の終了コード
  - **目標値**: exit code 0
  - **測定条件**: 各 REQ のコミット時点で実行

### NFR-003: 層境界の遵守 🔵

- [ ] **TC-NFR-003-01**: state/* が egui に依存しない 🔵
  - **検証方法**: `grep -rn "use egui\|egui::" egui-app/src/state/` が 0 件
  - **期待結果**: マッチなし
  - **信頼性**: 🔵 *layer-contract.md より*

- [ ] **TC-NFR-003-02**: rust_core の新規モジュールが egui に依存しない 🔵
  - **検証方法**: `grep -rn "egui" rust_core/src/convergence.rs` が 0 件
  - **期待結果**: マッチなし
  - **信頼性**: 🔵 *layer-contract.md より*

---

## テストケースサマリー

### カテゴリ別件数

| カテゴリ | 正常系 | 異常系 | 合計 |
|---------|--------|--------|------|
| REQ-001 | 4 | 1 | 5 |
| REQ-002 | 8 | 0 | 8 |
| REQ-003 | 3 | 0 | 3 |
| REQ-004 | 5 | 0 | 5 |
| 非機能要件 | 3 | 0 | 3 |
| **合計** | **23** | **1** | **24** |

### 信頼性レベル分布

- 🔵 青信号: 23件 (96%)
- 🟡 黄信号: 1件 (4%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質

### 優先度別テストケース

- **Must Have**: 24件
- **Should Have**: 0件
- **Could Have**: 0件
