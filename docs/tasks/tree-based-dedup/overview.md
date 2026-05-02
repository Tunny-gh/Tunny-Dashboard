# tree-based-dedup タスク概要

**作成日**: 2026-05-02
**推定工数**: 4.5時間
**総タスク数**: 6件

## 関連文書

- **要件定義書**: [📋 requirements.md](../../spec/tree-based-dedup/requirements.md)
- **設計文書**: [📐 architecture.md](../../design/tree-based-dedup/architecture.md)
- **データフロー図**: [🔄 dataflow.md](../../design/tree-based-dedup/dataflow.md)
- **コンテキストノート**: [📝 note.md](../../spec/tree-based-dedup/note.md)
- **元タスクファイル**: [📄 refactor-tree-based-dedup.md](../permutation-feature-importance/refactor-tree-based-dedup.md)

## フェーズ構成

| フェーズ | 成果物 | タスク数 | 工数 | ファイル |
|---------|--------|----------|------|----------|
| Phase 1 | rust_core ヘルパー共通化 | 1 | 1h | [TASK-2161](#phase-1---rustcore-ヘルパー共通化) |
| Phase 2 | Result 型統合（rust_core） | 1 | 0.5h | [TASK-2162](#phase-2---result-型統合-rust_core) |
| Phase 3 | transpose 関数統合 + full.rs 更新 | 1 | 1h | [TASK-2163](#phase-3---transpose-関数統合--fullrs-更新) |
| Phase 4 | Result 型統合（egui-app） | 1 | 0.5h | [TASK-2164](#phase-4---result-型統合-egui-app) |
| Phase 5 | UI match arm 統合 | 1 | 1h | [TASK-2165](#phase-5---ui-match-arm-統合) |
| Phase 6 | 最終検証 | 1 | 0.5h | [TASK-2166](#phase-6---最終検証) |

## タスク番号管理

**使用済みタスク番号**: TASK-2161 ~ TASK-2166
**次回開始番号**: TASK-2167

## 全体進捗

- [x] Phase 1: rust_core ヘルパー共通化
- [x] Phase 2: Result 型統合（rust_core）
- [x] Phase 3: transpose 関数統合 + full.rs 更新
- [x] Phase 4: Result 型統合（egui-app）
- [x] Phase 5: UI match arm 統合
- [x] Phase 6: 最終検証

## マイルストーン

- **M1: ヘルパー共通化完了**: tree_common.rs 新規作成・各ファイル import 切り替え (TASK-2161)
- **M2: rust_core 型統合完了**: TreeImportanceResult 導入・transpose 統合 (TASK-2162, TASK-2163)
- **M3: egui-app 統合完了**: results.rs 型統合・importance_chart.rs match arm 統合 (TASK-2164, TASK-2165)
- **M4: 全検証完了**: clippy + 全テスト通過 (TASK-2166)

---

## Phase 1 - rust_core ヘルパー共通化

**目標**: `permute_single_column` / `normalize` を `tree_common.rs` に集約し重複を解消する
**成果物**: `sensitivity/tree_common.rs`（新規）、rf_anova.rs / permutation.rs の import 切り替え、R² 計算統一

### タスク一覧

- [x] [TASK-2161: tree_common.rs 新規作成 + rf_anova/permutation の import 切り替え](TASK-2161.md) - 1h (DIRECT) 🔵

### 依存関係

```
TASK-2161 (独立)
```

---

## Phase 2 - Result 型統合（rust_core）

**目標**: `TreeImportanceResult` 単一型を導入し 4 つの重複型を型エイリアス化する
**成果物**: `sensitivity/types.rs` 更新、`sensitivity/mod.rs` pub use 追加

### タスク一覧

- [x] [TASK-2162: types.rs TreeImportanceResult 導入 + 型エイリアス化](TASK-2162.md) - 0.5h (DIRECT) 🔵

### 依存関係

```
TASK-2161 → TASK-2162
```

---

## Phase 3 - transpose 関数統合 + full.rs 更新

**目標**: 4 つの `transpose_*_importances` 関数を `transpose_to_tree_result` に統合し、全呼び出し箇所を更新する
**成果物**: `analysis/common.rs` 更新、`analysis/full.rs` 更新

### タスク一覧

- [x] [TASK-2163: transpose_to_tree_result 統合 + full.rs 呼び出し更新](TASK-2163.md) - 1h (DIRECT) 🔵

### 依存関係

```
TASK-2162 → TASK-2163
```

---

## Phase 4 - Result 型統合（egui-app）

**目標**: egui-app 側でも `TreeImportanceResult` を導入し 4 型を型エイリアス化する
**成果物**: `egui-app/src/state/results.rs` 更新

### タスク一覧

- [x] [TASK-2164: egui-app results.rs TreeImportanceResult 導入](TASK-2164.md) - 0.5h (DIRECT) 🔵

### 依存関係

```
TASK-2163 → TASK-2164
```

---

## Phase 5 - UI match arm 統合

**目標**: `compute_sorted_importance` の 4 つの重複 match arm を `extract_tree_importance` ヘルパーに集約する
**成果物**: `egui-app/src/ui/widgets/importance_chart.rs` 更新

### タスク一覧

- [x] [TASK-2165: importance_chart.rs extract_tree_importance 導入 + match arm 統合](TASK-2165.md) - 1h (DIRECT) 🔵

### 依存関係

```
TASK-2164 → TASK-2165
```

---

## Phase 6 - 最終検証

**目標**: clippy 警告の確認と全テストの通過を確認する
**成果物**: 全テスト通過・clippy 無警告の確認

### タスク一覧

- [x] [TASK-2166: cargo clippy + 全テスト最終確認](TASK-2166.md) - 0.5h (VERIFY) 🔵

### 依存関係

```
TASK-2165 → TASK-2166
```
