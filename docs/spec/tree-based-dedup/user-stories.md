# Tree-based 感度分析の重複解消 ユーザストーリー

**作成日**: 2026-05-02
**関連要件定義**: [requirements.md](requirements.md)
**ヒアリング記録**: [interview-record.md](interview-record.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: タスクファイル・既存実装から確実に導出されるストーリー
- 🟡 **黄信号**: タスクファイル・既存実装から妥当な推測によるストーリー
- 🔴 **赤信号**: タスクファイル・既存実装にない推測によるストーリー

---

## エピック1: Result 型の統合（rust_core）

### ストーリー 1.1: TreeImportanceResult 構造体の導入 🔵

**信頼性**: 🔵 *タスクファイル方針・types.rs 既存実装より*

**私は** Tunny Dashboard の開発者 **として**
**Tree-based 感度分析の4メトリクスで共通の Result 型を使いたい**
**そうすることで** 新メトリクス追加時に型定義の重複を防げる

**関連要件**: REQ-001, REQ-002, REQ-003, REQ-004

**詳細シナリオ**:
1. `types.rs` に `TreeImportanceResult` 構造体を定義
2. 既存の `RfAnovaResult`, `MdiResult`, `ShapResult`, `PermutationResult` を `pub type` エイリアスに変更
3. `mod.rs` の `pub use` に `TreeImportanceResult` を追加
4. `cargo build` が通ることを確認

**前提条件**:
- 既存の4型が同一フィールド（`importances: Vec<Vec<f64>>`, `r_squared: Vec<f64>`）を持つこと

**制約事項**:
- 型エイリアス経由でフィールドアクセスが可能（Rust の型エイリアスは透過的）
- 外部クレートからの参照先が変わらない

**優先度**: Must Have

---

## エピック2: transpose 関数の統合

### ストーリー 2.1: transpose_to_tree_result の導入 🔵

**信頼性**: 🔵 *タスクファイル方針・common.rs 既存実装より*

**私は** Tunny Dashboard の開発者 **として**
**4つの同一 transpose 関数を1つに統合したい**
**そうすることで** メトリクス追加時の transpose 関数の重複を防げる

**関連要件**: REQ-201, REQ-202, REQ-203, REQ-301

**詳細シナリオ**:
1. `common.rs` に `transpose_to_tree_result` 関数を定義
2. 戻り値を `TreeImportanceResult` に設定
3. 既存の4つの `transpose_*_importances` 関数を削除
4. `full.rs` の呼び出し箇所（single_obj / all）を更新
5. `cargo test` が全て通ることを確認

**前提条件**:
- Epic 1 の型エイリアス化が完了していること

**制約事項**:
- `full.rs` の `use` 文を更新する必要がある

**優先度**: Must Have

---

## エピック3: 共有ヘルパーモジュール（tree_common.rs）

### ストーリー 3.1: permute_single_column / normalize の移動 🔵

**信頼性**: 🔵 *タスクファイル方針・ヒアリング確認より*

**私は** Tunny Dashboard の開発者 **として**
**permute_single_column と normalize を共有モジュールに配置したい**
**そうすることで** rf_anova.rs と permutation.rs 間の重複を排除できる

**関連要件**: REQ-401, REQ-402, REQ-403, REQ-404, REQ-405, REQ-406

**詳細シナリオ**:
1. `rust_core/src/sensitivity/tree_common.rs` を新規作成
2. `permute_single_column`（`pub(crate)`）を移動
3. `normalize`（`pub(crate)`、for ループスタイル）を移動
4. `rf_anova.rs` と `permutation.rs` のローカル定義を削除し、`use super::tree_common::{...}` に変更
5. `mod.rs` に `mod tree_common;` を追加
6. `cargo test` が全て通ることを確認

**前提条件**:
- なし（Epic 1, 2 に依存しない独立ステップ）

**制約事項**:
- `rf_anova.rs` と `permutation.rs` の `use` 文に `Lcg` への依存が残るため、`tree_common.rs` でも `use crate::core::random_forest::Lcg;` が必要

**優先度**: Must Have

---

### ストーリー 3.2: R² 計算の統一 🔵

**信頼性**: 🔵 *ヒアリング確認より*

**私は** Tunny Dashboard の開発者 **として**
**rf_anova.rs の R² 計算を mse_to_r_squared() に統一したい**
**そうすることで** R² 計算ロジックの重複を排除できる

**関連要件**: REQ-501, REQ-502

**詳細シナリオ**:
1. `rf_anova.rs` の独自 R² 計算（行107-113）を `mse_to_r_squared(baseline_mse, y)` に置換
2. `use crate::core::lgbm::mse_to_r_squared;` を import に追加
3. `cargo test` で RF-ANOVA 関連テストが通ることを確認

**前提条件**:
- `mse_to_r_squared` が `core/lgbm` で定義されていること

**制約事項**:
- 計算結果は機能的に同一（数値精度の差異なし）

**優先度**: Should Have

---

## エピック4: UI match arm の統合

### ストーリー 4.1: extract_tree_importance ヘルパーの導入 🔵

**信頼性**: 🔵 *タスクファイル方針・importance_chart.rs 既存実装より*

**私は** Tunny Dashboard の開発者 **として**
**compute_sorted_importance 内の重複 match arm をヘルパー関数で統合したい**
**そうすることで** UI 側のコード重複を排除し、メトリクス追加時の変更箇所を減らせる

**関連要件**: REQ-601, REQ-602, REQ-603

**詳細シナリオ**:
1. `importance_chart.rs` に `extract_tree_importance` 関数を定義
2. RfAnova / Mdi / Shap / Permutation の match arm を `extract_tree_importance` 呼び出しに置換
3. Spearman / Ridge / Sobol の arm は変更しない
4. UI の動作が変わらないことを確認

**前提条件**:
- egui-app 側の `SensitivityResult` が `TreeImportanceResult` を使用していること（Epic 5）

**制約事項**:
- egui-app 側の `ImportanceMetric` enum は変更しない

**優先度**: Must Have

---

## エピック5: Result 型の統合（egui-app）

### ストーリー 5.1: egui-app results.rs の型統合 🔵

**信頼性**: 🔵 *ヒアリング確認・results.rs 既存実装より*

**私は** Tunny Dashboard の開発者 **として**
**egui-app 側の重複 Result 型も統合したい**
**そうすることで** rust_core と egui-app 間の型定義の整合性を保てる

**関連要件**: REQ-701, REQ-702

**詳細シナリオ**:
1. `egui-app/src/state/results.rs` に `TreeImportanceResult` を定義
2. 既存の4つの Result 型を型エイリアスに変更
3. `SensitivityResult` の各フィールドの型を `Option<TreeImportanceResult>` に更新
4. `cargo build` が通ることを確認

**前提条件**:
- rust_core 側の Epic 1 が完了していること（パターンの参照）

**制約事項**:
- egui-app の results.rs は rust_core とは独立した型定義（同一構造だが別型）

**優先度**: Must Have

---

## ストーリーマップ

```
エピック1: Result 型の統合（rust_core）
└── ストーリー 1.1 (🔵 Must Have)

エピック2: transpose 関数の統合
└── ストーリー 2.1 (🔵 Must Have)

エピック3: 共有ヘルパーモジュール
├── ストーリー 3.1 (🔵 Must Have)
└── ストーリー 3.2 (🔵 Should Have)

エピック4: UI match arm の統合
└── ストーリー 4.1 (🔵 Must Have)

エピック5: Result 型の統合（egui-app）
└── ストーリー 5.1 (🔵 Must Have)
```

## 信頼性レベルサマリー

- 🔵 青信号: 6件 (100%)
- 🟡 黄信号: 0件 (0%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質 — 全ストーリーがタスクファイル・既存実装・ヒアリング確認に基づいている
