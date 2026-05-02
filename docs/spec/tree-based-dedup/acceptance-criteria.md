# Tree-based 感度分析の重複解消 受け入れ基準

**作成日**: 2026-05-02
**関連要件定義**: [requirements.md](requirements.md)
**関連ユーザストーリー**: [user-stories.md](user-stories.md)
**ヒアリング記録**: [interview-record.md](interview-record.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: タスクファイル・既存実装から確実に導出される基準
- 🟡 **黄信号**: タスクファイル・既存実装から妥当な推測による基準
- 🔴 **赤信号**: タスクファイル・既存実装にない推測による基準

---

## Epic 1: Result 型の統合（rust_core）

### REQ-001〜004: TreeImportanceResult 導入 🔵

**信頼性**: 🔵 *タスクファイル方針・types.rs 既存実装より*

### Given（前提条件）
- `rust_core/src/sensitivity/types.rs` に4つの同一フィールド構造体が存在する

### When（実行条件）
- `TreeImportanceResult` 構造体を定義し、4型を型エイリアスに変更する

### Then（期待結果）
- `cargo build` が成功する
- 外部コードから `MdiResult`, `RfAnovaResult` 等の型名で引き続きアクセス可能

### テストケース

#### 正常系

- [ ] **TC-E1-01**: 型エイリアスでフィールドアクセス 🔵
  - **入力**: `let r: MdiResult = TreeImportanceResult { importances: vec![], r_squared: vec![] };`
  - **期待結果**: コンパイル成功、`r.importances`, `r.r_squared` にアクセス可能
  - **信頼性**: 🔵 *Rust 型エイリアスの性質より*

- [ ] **TC-E1-02**: mod.rs からのエクスポート 🔵
  - **入力**: 外部クレートからの `use tunny_core::sensitivity::{MdiResult, TreeImportanceResult};`
  - **期待結果**: コンパイル成功
  - **信頼性**: 🔵 *既存 pub use パターンより*

#### 回帰テスト

- [ ] **TC-E1-03**: 既存テストの通過 🔵
  - **条件**: `cargo test -p tunny-core`
  - **期待結果**: 全テストが PASS
  - **信頼性**: 🔵 *タスクファイル「各ステップで cargo test」より*

---

## Epic 2: transpose 関数の統合

### REQ-201〜203, 301: transpose_to_tree_result 導入 🔵

**信頼性**: 🔵 *タスクファイル方針・common.rs 既存実装より*

### Given（前提条件）
- Epic 1 の型エイリアス化が完了している
- 4つの `transpose_*_importances` 関数が存在する

### When（実行条件）
- `transpose_to_tree_result` を定義し、4関数を削除、full.rs の呼び出しを更新

### Then（期待結果）
- `cargo build` が成功する
- `cargo test` の全テストが PASS

### テストケース

#### 正常系

- [ ] **TC-E2-01**: 戻り値型が TreeImportanceResult 🔵
  - **入力**: `transpose_to_tree_result(&[vec![0.1, 0.2]], vec![0.95], 2, 1)`
  - **期待結果**: `TreeImportanceResult { importances: vec![vec![0.1], vec![0.2]], r_squared: vec![0.95] }`
  - **信頼性**: 🔵 *既存 transpose_importances_matrix の動作より*

- [ ] **TC-E2-02**: full.rs の single_obj 呼び出し 🔵
  - **条件**: `compute_sensitivity_single_obj` を実行
  - **期待結果**: 各メトリクスの結果がリファクタリング前と同一
  - **信頼性**: 🔵 *既存テスト（tc_801_*）で検証*

- [ ] **TC-E2-03**: full.rs の all 呼び出し 🔵
  - **条件**: `compute_sensitivity_all` を実行
  - **期待結果**: 全メトリクス結果がリファクタリング前と同一
  - **信頼性**: 🔵 *既存テスト（tc_801_10 等）で検証*

#### 回帰テスト

- [ ] **TC-E2-04**: 全テスト通過 🔵
  - **条件**: `cargo test -p tunny-core`
  - **期待結果**: 全テストが PASS
  - **信頼性**: 🔵 *タスクファイルより*

---

## Epic 3: 共有ヘルパーモジュール（tree_common.rs）

### REQ-401〜406: tree_common.rs 新規作成 🔵

**信頼性**: 🔵 *タスクファイル方針・ヒアリング確認より*

### Given（前提条件）
- `rf_anova.rs` と `permutation.rs` にそれぞれ `permute_single_column` / `normalize` が定義されている

### When（実行条件）
- `tree_common.rs` を新規作成し、両関数を移動。元ファイルは import に変更

### Then（期待結果）
- `cargo build` が成功する
- `cargo test` の全テストが PASS

### テストケース

#### 正常系

- [ ] **TC-E3-01**: permute_single_column の動作維持 🔵
  - **入力**: 3行2列の行列、feature_idx=0、seed=42
  - **期待結果**: 指定列がシャッフルされた行列が返る
  - **信頼性**: 🔵 *既存テスト（tc_pfi_001_*, tc_801_14, tc_801_15）で検証*

- [ ] **TC-E3-02**: normalize の動作維持 🔵
  - **入力**: `[1.0, 2.0, 3.0]`（合計=6.0）
  - **期待結果**: `[1/6, 2/6, 3/6]`（合計≈1.0）
  - **信頼性**: 🔵 *既存テスト（tc_pfi_001_01, tc_801_14）で検証*

- [ ] **TC-E3-03**: for ループスタイルの採用 🔵
  - **条件**: `tree_common.rs` の `normalize` 実装を確認
  - **期待結果**: `for v in values.iter_mut()` スタイルを使用
  - **信頼性**: 🔵 *ヒアリング確認より*

#### 境界値

- [ ] **TC-E3-04**: 空行列の処理 🔵
  - **入力**: `permute_single_column(&[], 0, 42)`
  - **期待結果**: `None`
  - **信頼性**: 🔵 *EDGE-001 対応*

- [ ] **TC-E3-05**: ゼロ合計配列の処理 🔵
  - **入力**: `normalize(&mut [0.0, 0.0, 0.0])`
  - **期待結果**: `[0.0, 0.0, 0.0]`
  - **信頼性**: 🔵 *EDGE-002 対応*

#### 回帰テスト

- [ ] **TC-E3-06**: RF-ANOVA テスト通過 🔵
  - **条件**: `tc_801_14`, `tc_801_15`
  - **期待結果**: PASS
  - **信頼性**: 🔵 *既存テストより*

- [ ] **TC-E3-07**: PFI テスト通過 🔵
  - **条件**: `tc_pfi_001_01` 〜 `tc_pfi_int_02`
  - **期待結果**: 全て PASS
  - **信頼性**: 🔵 *既存テストより*

### REQ-501〜502: R² 計算の統一 🔵

**信頼性**: 🔵 *ヒアリング確認より*

### テストケース

- [ ] **TC-E3-08**: R² 計算結果の一致 🔵
  - **条件**: rf_anova.rs の R² 計算を mse_to_r_squared() に変更
  - **期待結果**: tc_801_14, tc_801_15 が引き続き PASS
  - **信頼性**: 🔵 *機能的に同一の計算式であるため*

---

## Epic 4: UI match arm の統合

### REQ-601〜603: extract_tree_importance 導入 🔵

**信頼性**: 🔵 *タスクファイル方針・importance_chart.rs 既存実装より*

### Given（前提条件）
- Epic 5 の egui-app 側の型統合が完了している

### When（実行条件）
- `extract_tree_importance` ヘルパーを定義し、4 arm を置換

### Then（期待結果）
- egui-app の `cargo build` が成功する
- Importance Chart の表示が変わらない

### テストケース

#### 正常系

- [ ] **TC-E4-01**: RfAnova メトリクスの動作 🔵
  - **条件**: ImportanceMetric::RfAnova で compute_sorted_importance を実行
  - **期待結果**: リファクタリング前と同一のスコア順位
  - **信頼性**: 🔵 *既存コードの置換のみ*

- [ ] **TC-E4-02**: Permutation メトリクスの動作 🔵
  - **条件**: ImportanceMetric::Permutation で compute_sorted_importance を実行
  - **期待結果**: リファクタリング前と同一のスコア順位
  - **信頼性**: 🔵 *既存コードの置換のみ*

- [ ] **TC-E4-03**: Spearman/Ridge/Sobol の非変更確認 🔵
  - **条件**: Spearman, Ridge, SobolFirst, SobolTotal で compute_sorted_importance を実行
  - **期待結果**: 各メトリクスの動作が変更なし
  - **信頼性**: 🔵 *REQ-603（非変更保証）より*

---

## Epic 5: Result 型の統合（egui-app）

### REQ-701〜702: results.rs 型統合 🔵

**信頼性**: 🔵 *ヒアリング確認・results.rs 既存実装より*

### Given（前提条件）
- results.rs に4つの同一フィールド構造体が存在する

### When（実行条件）
- `TreeImportanceResult` を定義し、4型を型エイリアスに変更

### Then（期待結果）
- `cargo build` が成功する

### テストケース

- [ ] **TC-E5-01**: egui-app ビルド成功 🔵
  - **条件**: `cargo build -p tunny-desktop`
  - **期待結果**: コンパイル成功
  - **信頼性**: 🔵 *型エイリアスの透過性より*

- [ ] **TC-E5-02**: SensitivityResult フィールドアクセス 🔵
  - **条件**: `result.rf_anova.as_ref()?.importances` へのアクセス
  - **期待結果**: 型エイリアス経由でコンパイル成功
  - **信頼性**: 🔵 *型エイリアスの透過性より*

---

## 非機能要件テスト

### NFR-001: 全テスト通過 🔵

**信頼性**: 🔵 *タスクファイル「各ステップで cargo test」より*

- [ ] **TC-NFR-001-01**: ワークスペース全テスト
  - **測定項目**: テスト結果
  - **目標値**: 全テスト PASS
  - **測定条件**: `cargo test`（ワークスペース全体）
  - **信頼性**: 🔵 *タスクファイルより*

### NFR-102: 段階的コンパイル可能性 🔵

**信頼性**: 🔵 *タスクファイル実装順序より*

- [ ] **TC-NFR-102-01**: 各 Step の独立コンパイル
  - **条件**: Step 1〜5 の各完了時点で `cargo build` が成功
  - **期待結果**: 各ステップでコンパイル可能
  - **信頼性**: 🔵 *タスクファイル実装順序より*

---

## テストケースサマリー

### カテゴリ別件数

| カテゴリ | 正常系 | 異常系 | 境界値 | 回帰 | 合計 |
|---------|--------|--------|--------|------|------|
| Epic 1 (型統合) | 2 | 0 | 0 | 1 | 3 |
| Epic 2 (transpose) | 3 | 0 | 0 | 1 | 4 |
| Epic 3 (tree_common) | 3 | 0 | 2 | 3 | 8 |
| Epic 4 (UI arm) | 3 | 0 | 0 | 0 | 3 |
| Epic 5 (egui-app型) | 2 | 0 | 0 | 0 | 2 |
| 非機能要件 | 0 | 0 | 0 | 2 | 2 |
| **合計** | **13** | **0** | **2** | **7** | **22** |

### 信頼性レベル分布

- 🔵 青信号: 22件 (100%)
- 🟡 黄信号: 0件 (0%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質 — 全テストケースがタスクファイル・既存実装・ヒアリング確認に基づいている

### 優先度別テストケース

- **Must Have**: 20件（Epic 1〜5 の正常系・境界値・回帰 + NFR）
- **Should Have**: 2件（R² 計算の統一関連）

---

## テスト実施計画

### Phase 1: Step 1（tree_common.rs）
- TC-E3-01 〜 TC-E3-07
- 優先度: Must Have
- 実施タイミング: Step 1 完了時

### Phase 2: Step 2（types.rs 型統合）
- TC-E1-01 〜 TC-E1-03
- 優先度: Must Have
- 実施タイミング: Step 2 完了時

### Phase 3: Step 3（transpose 統合）
- TC-E2-01 〜 TC-E2-04
- 優先度: Must Have
- 実施タイミング: Step 3 完了時

### Phase 4: Step 4〜5（egui-app 型統合 + UI 統合）
- TC-E4-01 〜 TC-E4-03, TC-E5-01 〜 TC-E5-02
- 優先度: Must Have
- 実施タイミング: Step 4〜5 完了時

### Phase 5: Step 6（最終確認）
- TC-NFR-001-01, TC-NFR-102-01
- 優先度: Must Have
- 実施タイミング: 全 Step 完了時
