# 制約条件を考慮した可視化 設計ヒアリング記録

**作成日**: 2026-06-03
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

既存の実装（パーサー・DataFrame・ウィジェット・Pareto 計算）を確認したうえで、設計の曖昧な部分（実装場所・ランク値・状態リセット・違反量ランキング方法）を明確化するためのヒアリングを実施した。

---

## 質問と回答

### Q1: Pareto ランク除外の実装場所

**カテゴリ**: アーキテクチャ
**背景**: `compute_pareto_ranks()` は `rust_core` 内で `with_active_df()` を通じて DataFrame にアクセスしている。feasibility フィルタを rust_core 内で行うか、呼び出し元（`study.rs`）で filtered objectives を渡すか判断が必要だった

**回答**: rust_core 内部で処理（推奨）

**信頼性への影響**:
- アーキテクチャ決定: `compute_pareto_ranks()` 自身が `is_feasible` 列と `constraint_sum` 列を読み取り、内部でフィルタと再ランキングを実施する設計が確定
- この回答により、アーキテクチャ設計の信頼性が 🔴 → 🔵 に向上

---

### Q2: 実行不可能解の pareto_rank 特別値

**カテゴリ**: データモデル
**背景**: 要件定義書 REQ-CAV-032 で「`u32::MAX` または `999`」と曖昧にしていた。実際の設計方針を確定させる必要があった

**回答**: max_rank に追加する形で違反量に応じてランク付けしてください

**信頼性への影響**:
- infeasible 試行は `u32::MAX` ではなく、feasible 試行の最大ランク + 1 + 違反量順位 という継続的なランク値を持つ設計に変更
- この回答により、Pareto ランキングがより情報豊富になる（UI 上でも違反の大小が視覚的に反映可能）
- 設計の信頼性が 🔴 → 🔵 に向上

---

### Q3: show_infeasible の Study 切替時リセット

**カテゴリ**: UI状態管理
**背景**: ユーザーが "Show Infeasible" を off にした後、別の Study に切り替えたとき、その設定を保持するかリセットするかが不明だった

**回答**: リセット（推奨）— Study 切替時に全チャートの show_infeasible = true に戻す

**信頼性への影響**:
- `Default::default()` で `show_infeasible = true` を返す設計が確定
- `message_handler.rs` の StudySelected ハンドラでウィジェットをリセットする実装方針が確定
- 信頼性が 🔴 → 🔵 に向上

---

### Q4: 実行不可能解の違反量ランキング計算方法

**カテゴリ**: データモデル（追加質問）
**背景**: Q2 の回答で「違反量に応じてランク付け」という方針が決まったため、具体的な計算方法を確認する必要があった

**回答**: constraint_sum 昇順（推奨）— 合計違反量の小さいものから優先的に小さいランク値を割り当て

**信頼性への影響**:
- `constraint_sum` 列（既実装）をソートキーとして使用する設計が確定
- infeasible 試行間でのランク計算が明確になった
- 信頼性が 🟡 → 🔵 に向上

---

## ヒアリング結果サマリー

### 確認できた事項

1. feasibility フィルタは `rust_core` の `compute_pareto_ranks()` 内で完結させる
2. infeasible 試行のランクは `max_feasible_rank + 1 + violation_rank`（constraint_sum 昇順）
3. Study 切替時に全チャートの `show_infeasible` を `Default::default()` でリセット
4. 違反量ランキングのソートキーは `constraint_sum`（既存の派生列）

### 設計方針の決定事項

| 決定事項 | 内容 |
|---|---|
| Pareto 計算の変更箇所 | `rust_core/src/multi_objective/pareto/ranking.rs:compute_pareto_ranks()` |
| infeasible の rank 値 | `max_feasible_rank + 1 + violation_rank` |
| 違反量のソート順 | `constraint_sum` 昇順 |
| show_infeasible のデフォルト | `true`（表示） |
| Study 切替時のリセット | `Default::default()` で自動リセット |
| トグル配置 | 各チャートの UI コントロール行 |
| グレーアウト色 | `COLOR_INFEASIBLE` = `rgba_premultiplied(56, 56, 56, 80)` |

### 残課題

- `ParetoScatter2D` を `Default::default()` でリセットすると `x_axis`/`y_axis` も "obj0"/"obj1" に戻ってしまう → `pareto_3d.show_infeasible = true` のように show_infeasible フィールドのみリセットする方が安全か検討が必要（🟡）
- `OptimizationHistoryChart::show()` のシグネチャに `has_constraints: bool` を追加するか、`view.df.constraint_col_names().is_empty()` で判断するかは実装時に確認（🟡）

### 信頼性レベル分布

**ヒアリング前**:
- 🔵 青信号: 6件（既存実装から確実なもの）
- 🟡 黄信号: 4件
- 🔴 赤信号: 8件（設計方針未決定）

**ヒアリング後**:
- 🔵 青信号: 16件（+10）
- 🟡 黄信号: 2件（-2）
- 🔴 赤信号: 0件（-8）

---

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
- **実装ガイド**: [implementation-guide.md](implementation-guide.md)
- **要件定義**: [requirements.md](../../spec/constraint-aware-visualization/requirements.md)
