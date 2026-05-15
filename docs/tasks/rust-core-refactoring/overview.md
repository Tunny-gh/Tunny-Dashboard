# rust-core-refactoring タスク概要

**作成日**: 2026-05-14
**プロジェクト期間**: 2026-05-14 - 2026-05-22（7営業日）
**推定工数**: 52時間（13タスク × 4時間）
**総タスク数**: 13件

## 関連文書

- **要件定義書**: [📋 requirements.md](../../spec/rust-core-refactoring/requirements.md)
- **設計文書**: [📐 architecture.md](../../design/rust-core-refactoring/architecture.md)
- **データフロー**: [🔄 dataflow.md](../../design/rust-core-refactoring/dataflow.md)
- **型定義**: [📝 interfaces.rs](../../design/rust-core-refactoring/interfaces.rs)
- **ヒアリング記録**: [💬 design-interview.md](../../design/rust-core-refactoring/design-interview.md)
- **コンテキストノート**: [📝 note.md](../../spec/rust-core-refactoring/note.md)

## フェーズ構成

| フェーズ | 期間 | 成果物 | タスク数 | 工数 |
|---------|------|--------|----------|------|
| Phase 1 | 2026-05-14〜16 | SensitivityMetric トレイト・共通関数 | 5件 | 20h |
| Phase 2 | 2026-05-19〜20 | 責務分離リファクタリング | 4件 | 16h |
| Phase 3 | 2026-05-21〜22 | 効率改善・SamplingContext 移行 | 4件 | 16h |

## タスク番号管理

**使用済みタスク番号**: TASK-2258 〜 TASK-2270
**次回開始番号**: TASK-2271

## 全体進捗

- [ ] Phase 1: Epic A - コード重複排除
- [ ] Phase 2: Epic B - 責務分離
- [ ] Phase 3: Epic C - 効率改善

## マイルストーン

- **M1: トレイト基盤完成** (2026-05-16): SensitivityMetric トレイト・共通関数実装完了
- **M2: 責務分離完了** (2026-05-20): 全関数 50 行以内・単一責務達成
- **M3: 効率改善・リリース準備** (2026-05-22): SamplingContext 移行・egui-app 統合完了

---

## Phase 1: Epic A - コード重複排除

**期間**: 2026-05-14〜16
**目標**: SensitivityMetric トレイト導入、Pearson 共通化、k-means 初期化共通化
**成果物**: `metric_trait.rs` (新規)、共通関数群

### タスク一覧

- [x] [TASK-2258: SensitivityMetric トレイト定義と SensitivityKind リネーム](TASK-2258.md) - 4h (DIRECT) 🔵 ✅ 完了 (2026-05-15)
- [x] [TASK-2259: SpearmanMetric・RidgeMetric の SensitivityMetric 実装](TASK-2259.md) - 4h (TDD) 🔵 ✅ 完了 (2026-05-15)
- [x] [TASK-2260: RfAnovaMetric・MdiMetric・ShapMetric・PermutationMetric の SensitivityMetric 実装](TASK-2260.md) - 4h (TDD) 🔵 ✅ 完了 (2026-05-15)
- [x] [TASK-2261: Pearson 相関の core/math/stats.rs への移動](TASK-2261.md) - 4h (TDD) 🔵 ✅ 完了 (2026-05-15)
- [x] [TASK-2262: select_next_centroid による k-means 初期化共通化](TASK-2262.md) - 4h (TDD) 🔵 ✅ 完了 (2026-05-15)

### 依存関係

```
TASK-2258 → TASK-2259
TASK-2258 → TASK-2260
TASK-2261 (独立)
TASK-2262 (独立)
```

---

## Phase 2: Epic B - 責務分離

**期間**: 2026-05-19〜20
**目標**: 巨大関数の分割・各関数 50 行以内への削減
**成果物**: `compute_sensitivity_single_obj` 簡略化、クラスタ統計 3 分割、Ridge 3 分割、GpModel 分割

### タスク一覧

- [x] [TASK-2263: compute_sensitivity_single_obj の簡略化](TASK-2263.md) - 4h (TDD) 🔵 ✅ 完了 (2026-05-15)
- [x] [TASK-2264: compute_cluster_stats_on_data の 3 関数分割](TASK-2264.md) - 4h (TDD) 🔵 ✅ 完了 (2026-05-15)
- [x] [TASK-2265: Ridge 回帰 3 関数分割 + 行列フォーマット変換削減](TASK-2265.md) - 4h (TDD) 🔵 ✅ 完了 (2026-05-15)
- [x] [TASK-2266: GpModel → GpKernel + GpFittedModel への分割](TASK-2266.md) - 4h (TDD) 🟡 ✅ 完了 (2026-05-15)

### 依存関係

```
TASK-2259 → TASK-2263
TASK-2260 → TASK-2263
TASK-2261 → TASK-2265
TASK-2264 (独立)
TASK-2266 (独立)
```

---

## Phase 3: Epic C - 効率改善

**期間**: 2026-05-21〜22
**目標**: 不要アロケーション削減、グローバル状態廃止
**成果物**: k-means クローン削減、TOPSIS 単一アロケーション、SamplingContext 実装・統合

### タスク一覧

- [x] [TASK-2267: k-means 不要クローン削減](TASK-2267.md) - 4h (TDD) 🟡 ✅ 完了 (2026-05-15)
- [x] [TASK-2268: TOPSIS build_weighted_matrix 単一アロケーション化](TASK-2268.md) - 4h (TDD) 🔵 ✅ 完了 (2026-05-15)
- [x] [TASK-2269: SamplingContext 実装（rust_core 側）](TASK-2269.md) - 4h (TDD) 🔵 ✅ 完了 (2026-05-15)
- [x] [TASK-2270: egui-app 側の SamplingContext 統合](TASK-2270.md) - 4h (TDD) 🔵 ✅ 完了 (2026-05-15)

### 依存関係

```
TASK-2262 → TASK-2267
TASK-2268 (独立)
TASK-2269 → TASK-2270
```

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 13件
- 🔵 **青信号**: 11件 (85%)
- 🟡 **黄信号**: 2件 (15%)
- 🔴 **赤信号**: 0件 (0%)

### フェーズ別信頼性

| フェーズ | 🔵 青 | 🟡 黄 | 🔴 赤 | 合計 |
|---------|-------|-------|-------|------|
| Phase 1 | 5 | 0 | 0 | 5 |
| Phase 2 | 3 | 1 | 0 | 4 |
| Phase 3 | 3 | 1 | 0 | 4 |

**品質評価**: ✅ 高品質

## クリティカルパス

```
TASK-2258 → TASK-2259 → TASK-2263
```

または

```
TASK-2258 → TASK-2260 → TASK-2263
```

**クリティカルパス工数**: 12時間
**並行作業可能工数**: 40時間（TASK-2261, 2262, 2264, 2265, 2266, 2267, 2268, 2269 は並行実施可能）

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-2258`
