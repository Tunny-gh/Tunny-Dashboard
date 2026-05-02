# Tree-based 感度分析の重複解消 要件定義書

## 概要

Permutation Feature Importance (TASK-2156〜2160) の実装で生じた、Tree-based 感度分析（RfAnova / MDI / SHAP / Permutation）の重複コードを解消するリファクタリング。同一フィールド構造の Result 型（4型）、同一ロジックの transpose 関数（4関数）、同一実装のヘルパー関数（2関数×2ファイル）、UI 側の match arm 重複（4箇所）を統合し、今後の新メトリクス追加コストを下げる。

## 関連文書

- **ヒアリング記録**: [💬 interview-record.md](interview-record.md)
- **ユーザストーリー**: [📖 user-stories.md](user-stories.md)
- **受け入れ基準**: [✅ acceptance-criteria.md](acceptance-criteria.md)
- **コンテキストノート**: [📝 note.md](note.md)
- **タスクファイル**: [refactor-tree-based-dedup.md](../../tasks/permutation-feature-importance/refactor-tree-based-dedup.md)

## 機能要件（EARS記法）

**【信頼性レベル凡例】**:
- 🔵 **青信号**: タスクファイル・既存実装から確実に導出される要件
- 🟡 **黄信号**: タスクファイル・既存実装から妥当な推測による要件
- 🔴 **赤信号**: タスクファイル・既存実装にない推測による要件

### Epic 1: Result 型の統合（rust_core）

#### 通常要件

- **REQ-001**: システムは `TreeImportanceResult` 構造体（`importances: Vec<Vec<f64>>`, `r_squared: Vec<f64>`）を定義しなければならない 🔵 *types.rs 既存フィールドより*
- **REQ-002**: システムは `MdiResult`, `RfAnovaResult`, `ShapResult`, `PermutationResult` を `TreeImportanceResult` への `pub type` エイリアスとして定義しなければならない 🔵 *タスクファイル方針より*
- **REQ-003**: システムは型エイリアス経由のフィールドアクセスが既存コードと同一に動作することを保証しなければならない 🔵 *Rust 型エイリアスの性質より*
- **REQ-004**: システムは `mod.rs` の `pub use` で `TreeImportanceResult` をエクスポートしなければならない 🔵 *既存 mod.rs pub use パターンより*

#### 制約要件

- **REQ-101**: `serde` derive が必要な場合は `TreeImportanceResult` に付与しなければならない 🟡 *タスクファイル注意事項から妥当な推測*
- **REQ-102**: 外部クレートからの参照先（型名）が変わらないことを保証しなければならない 🔵 *タスクファイル「pub type で公開」より*

### Epic 2: transpose 関数の統合

#### 通常要件

- **REQ-201**: システムは `transpose_to_tree_result` 関数を定義し、4つの `transpose_*_importances` 関数を置換しなければならない 🔵 *タスクファイル方針より*
- **REQ-202**: `transpose_to_tree_result` は戻り値として `TreeImportanceResult` を返さなければならない 🔵 *タスクファイル方針より*
- **REQ-203**: システムは `transpose_mdi_importances`, `transpose_rf_anova_importances`, `transpose_shap_importances`, `transpose_permutation_importances` の4関数を削除しなければならない 🔵 *タスクファイル方針より*

#### 条件付き要件

- **REQ-301**: `full.rs` が `transpose_*_importances` を呼び出している箇所（single_obj / all）はすべて `transpose_to_tree_result` に更新しなければならない 🔵 *analysis/full.rs 既存実装より*

### Epic 3: 共有ヘルパーモジュール（tree_common.rs）

#### 通常要件

- **REQ-401**: システムは新規ファイル `rust_core/src/sensitivity/tree_common.rs` を作成しなければならない 🔵 *タスクファイル方針・ヒアリング確認より*
- **REQ-402**: システムは `permute_single_column` を `tree_common.rs` に配置し、`pub(crate)` で公開しなければならない 🔵 *タスクファイル方針より*
- **REQ-403**: システムは `normalize` を `tree_common.rs` に配置し、`pub(crate)` で公開しなければならない 🔵 *タスクファイル方針より*
- **REQ-404**: `normalize` の実装スタイルは `for` ループを使用しなければならない 🔵 *ヒアリング確認より*
- **REQ-405**: システムは `rf_anova.rs` と `permutation.rs` からローカル定義を削除し、`tree_common` から import するように変更しなければならない 🔵 *タスクファイル方針より*
- **REQ-406**: システムは `mod.rs` に `mod tree_common;` を追加しなければならない 🔵 *タスクファイル注意事項より*

#### 条件付き要件

- **REQ-501**: `rf_anova.rs` の R² 計算を `mse_to_r_squared()` に統一しなければならない 🔵 *ヒアリング確認より*
- **REQ-502**: `rf_anova.rs` に `use crate::core::lgbm::mse_to_r_squared;` を追加しなければならない 🟡 *REQ-501 から妥当な推測*

### Epic 4: UI match arm の統合

#### 通常要件

- **REQ-601**: システムは `extract_tree_importance` ヘルパー関数を `importance_chart.rs` に定義しなければならない 🔵 *タスクファイル方針より*
- **REQ-602**: `compute_sorted_importance` 内の RfAnova / Mdi / Shap / Permutation の4 arm を `extract_tree_importance` 呼び出しに置換しなければならない 🔵 *タスクファイル方針より*
- **REQ-603**: Spearman / Ridge / Sobol の arm は変更しないことを保証しなければならない 🔵 *タスクファイル「影響範囲」より*

### Epic 5: Result 型の統合（egui-app）

#### 通常要件

- **REQ-701**: システムは `egui-app/src/state/results.rs` でも `TreeImportanceResult` 構造体を導入し、4つの Result 型を型エイリアス化しなければならない 🔵 *ヒアリング確認より*
- **REQ-702**: `egui-app` 側の `SensitivityResult` の各フィールドの型は `Option<TreeImportanceResult>` に統一されなければならない 🟡 *results.rs 既存構造から妥当な推測*

## 非機能要件

### パフォーマンス

- **NFR-001**: リファクタリング前後で全テストの実行結果が一致しなければならない 🔵 *タスクファイル「既存テストがすべて通ること」より*
- **NFR-002**: リファクタリングによるパフォーマンスの低下があってはならない 🔵 *内部実装の移動のみであるため*

### 保守性

- **NFR-101**: 新しい Tree-based メトリクス追加時に必要な変更箇所がリファクタリング前より減少しなければならない 🔵 *タスクファイル「追加コストを下げる」より*
- **NFR-102**: 各ステップ（Step 1〜6）は独立してコンパイルが通らなければならない 🔵 *タスクファイル実装順序より*

### 後方互換性

- **NFR-201**: 外部クレートから型名でのアクセス（`MdiResult` 等）が引き続き可能でなければならない 🔵 *型エイリアスによる互換性維持*

## Edgeケース

### エラー処理

- **EDGE-001**: `permute_single_column` に空行列が渡された場合、`None` を返す動作を維持しなければならない 🔵 *既存実装の `if n == 0 { return None; }` より*
- **EDGE-002**: `normalize` に合計値がゼロ以下の配列が渡された場合、全要素を 0.0 に設定する動作を維持しなければならない 🔵 *既存実装の `if sum < f64::EPSILON` より*

### 境界値

- **EDGE-101**: 単一目的関数（`objective_count = 1`）の場合でも `transpose_to_tree_result` が正しく動作しなければならない 🔵 *既存テスト（single_obj）より*
- **EDGE-102**: パラメータ数 0 の場合でも空の `TreeImportanceResult` が返されなければならない 🟡 *既存 empty_result パターンから妥当な推測*

## スコープ外

以下は今回の対象外とする（ヒアリング確認済み）:

- NaN/Inf フィルタリングの共通化 → 別 issue
- 80/20 holdout split の共通化 → 別 issue
