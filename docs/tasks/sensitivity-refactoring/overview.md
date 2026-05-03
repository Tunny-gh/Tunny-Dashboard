# sensitivity-refactoring タスク概要

**作成日**: 2026-05-04
**推定工数**: 36時間
**総タスク数**: 9件

## 関連文書

- **要件定義書**: [📋 requirements.md](../../spec/sensitivity-refactoring/requirements.md)
- **設計文書**: [📐 architecture.md](../../design/sensitivity-refactoring/architecture.md)
- **インターフェース定義**: [🦀 interfaces.rs](../../design/sensitivity-refactoring/interfaces.rs)
- **データフロー図**: [🔄 dataflow.md](../../design/sensitivity-refactoring/dataflow.md)
- **ヒアリング記録**: [💬 design-interview.md](../../design/sensitivity-refactoring/design-interview.md)
- **コンテキストノート**: [📝 note.md](../../spec/sensitivity-refactoring/note.md)

## フェーズ構成

| フェーズ | 成果物 | タスク数 | 工数 | ファイル |
|---------|--------|----------|------|----------|
| Phase 1 - 基盤整備 | stats.rs + constants.rs | 2 | 8h | [TASK-2084~2085](#phase-1-基盤整備) |
| Phase 2 - 型定義・Trait | Newtype + TreeMetric | 2 | 8h | [TASK-2086~2087](#phase-2-型定義trait) |
| Phase 3 - 重複排除 | 統一・ディスパッチ適用 | 4 | 16h | [TASK-2088~2091](#phase-3-重複排除) |
| Phase 4 - 検証 | テスト・API確認 | 1 | 4h | [TASK-2092](#phase-4-検証) |

## タスク番号管理

**使用済みタスク番号**: TASK-2084 ~ TASK-2092
**次回開始番号**: TASK-2093

## 全体進捗

- [ ] Phase 1: 基盤整備
- [ ] Phase 2: 型定義・Trait
- [ ] Phase 3: 重複排除
- [ ] Phase 4: 検証

## マイルストーン

- **M1: 基盤完成**: stats.rs + constants.rs 作成完了
- **M2: 型・Trait完成**: Newtype + TreeMetric 実装完了
- **M3: 統合完了**: 重複排除・ディスパッチ適用完了
- **M4: 検証完了**: 全テストパス・API維持確認

---

## Phase 1: 基盤整備

**目標**: 共通関数・定数の基盤を作成
**成果物**: `core/math/stats.rs`, `sensitivity/constants.rs`

### タスク一覧

- [ ] [TASK-2084: core::math::stats モジュールの新規作成](TASK-2084.md) - 4h (DIRECT) 🔵
- [ ] [TASK-2085: sensitivity/constants.rs 新規作成](TASK-2085.md) - 4h (DIRECT) 🔵

### 依存関係

```
(なし: TASK-2084 と TASK-2085 は並行実行可能)
```

---

## Phase 2: 型定義・Trait

**目標**: Newtype パターン移行と TreeMetric Trait 定義
**成果物**: `sensitivity/types.rs` 変更, `sensitivity/metrics.rs` 新規

### タスク一覧

- [ ] [TASK-2086: sensitivity/types.rs Newtype 変更](TASK-2086.md) - 4h (TDD) 🔵
- [ ] [TASK-2087: sensitivity/metrics.rs TreeMetric Trait 定義](TASK-2087.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-2085 → TASK-2087 (constants を使用)
TASK-2084 → TASK-2087 (PreparedData を使用)
TASK-2086 と TASK-2087 は Phase 1 完了後に並行実行可能
```

---

## Phase 3: 重複排除

**目標**: 重複コード排除・TreeMetric ディスパッチ適用
**成果物**: ridge.rs/common.rs/sobol.rs/pdp/utils.rs 統一, full.rs/selected.rs 変更

### タスク一覧

- [ ] [TASK-2088: mdi.rs/shap.rs prepare_training_data 統一](TASK-2088.md) - 4h (TDD) 🔵
- [ ] [TASK-2089: ridge.rs/common.rs/sobol.rs column_mean_std 統一](TASK-2089.md) - 4h (TDD) 🔵
- [ ] [TASK-2090: pdp/utils.rs col_mean_std 統一](TASK-2090.md) - 4h (TDD) 🔵
- [ ] [TASK-2091: analysis/full.rs + selected.rs TreeMetric 適用](TASK-2091.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-2084 → TASK-2089
TASK-2084 → TASK-2090
TASK-2085 → TASK-2088
TASK-2086 → TASK-2091
TASK-2087 → TASK-2091
TASK-2088 → TASK-2091
TASK-2088, TASK-2089, TASK-2090 は並行実行可能
TASK-2091 は TASK-2086 + TASK-2087 + TASK-2088 完了後に実行
```

---

## Phase 4: 検証

**目標**: 全テストパス確認・パフォーマンス検証・API維持確認
**成果物**: テスト結果レポート

### タスク一覧

- [ ] [TASK-2092: 全テスト実行・パフォーマンステスト検証・API維持確認](TASK-2092.md) - 4h (DIRECT) 🔵

### 依存関係

```
TASK-2084~2091 → TASK-2092
```

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 9件
- 🔵 **青信号**: 9件 (100%)
- 🟡 **黄信号**: 0件 (0%)
- 🔴 **赤信号**: 0件 (0%)

### フェーズ別信頼性

| フェーズ | 🔵 青 | 🟡 黄 | 🔴 赤 | 合計 |
|---------|-------|-------|-------|------|
| Phase 1 | 2 | 0 | 0 | 2 |
| Phase 2 | 2 | 0 | 0 | 2 |
| Phase 3 | 4 | 0 | 0 | 4 |
| Phase 4 | 1 | 0 | 0 | 1 |

**品質評価**: ✅ 高品質

## クリティカルパス

```
TASK-2084 → TASK-2089 → TASK-2091 → TASK-2092
                ↘         ↑
TASK-2085 → TASK-2087 ────┘
         ↘ TASK-2088 ─────┘
TASK-2086 ─────────────────→ TASK-2091
```

**クリティカルパス工数**: 16時間
**並行作業可能工数**: 20時間

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-2084`
