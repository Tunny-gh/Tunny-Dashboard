# rayon-performance-optimization タスク概要

**作成日**: 2026-05-05
**プロジェクト期間**: 2026-05-05 - 2026-05-16（12日）
**推定工数**: 18時間
**総タスク数**: 6件

## 関連文書

- **要件定義書**: [📋 requirements.md](../../spec/rayon-performance-optimization/requirements.md)
- **設計文書**: [📐 architecture.md](../../design/rayon-performance-optimization/architecture.md)
- **データフロー図**: [🔄 dataflow.md](../../design/rayon-performance-optimization/dataflow.md)
- **設計ヒアリング記録**: [💬 design-interview.md](../../design/rayon-performance-optimization/design-interview.md)

## フェーズ構成

| フェーズ | 期間 | 成果物 | タスク数 | 工数 | ファイル |
|---------|------|--------|----------|------|----------|
| Phase 1 | 2026-05-05〜05-06 | Sensitivity 並列化 | 1件 | 2h | [TASK-2175](#phase-1-sensitivity-並列化) |
| Phase 2 | 2026-05-07〜05-08 | RandomForest 並列化 | 1件 | 3h | [TASK-2176](#phase-2-randomforest-並列化) |
| Phase 3 | 2026-05-09〜05-10 | Permutation 並列化 | 1件 | 3h | [TASK-2177](#phase-3-permutation-並列化) |
| Phase 4 | 2026-05-12〜05-14 | Sobol 並列化 | 2件 | 7h | [TASK-2178〜2179](#phase-4-sobol-並列化) |
| Phase 5 | 2026-05-15〜05-16 | criterion ベンチマーク追加 | 1件 | 3h | [TASK-2180](#phase-5-criterion-ベンチマーク追加) |

## タスク番号管理

**使用済みタスク番号**: TASK-2175 ~ TASK-2180
**次回開始番号**: TASK-2181

## 全体進捗

- [x] Phase 1: Sensitivity 並列化
- [x] Phase 2: RandomForest 並列化
- [x] Phase 3: Permutation 並列化
- [x] Phase 4: Sobol 並列化
- [x] Phase 5: criterion ベンチマーク追加

## マイルストーン

- **M1: Phase 1〜3 完了** (2026-05-10): 3 つの並列化で `cargo test` 通過
- **M2: Phase 4 完了** (2026-05-14): Sobol リファクタリング + 並列化で `cargo test` 通過
- **M3: Phase 5 完了** (2026-05-16): 全 4 ベンチマークで速度改善を定量計測

---

## Phase 1: Sensitivity 並列化

**期間**: 2026-05-05〜05-06（1日）
**目標**: `run_tree_metric_for_all_objectives` を `par_iter` で並列化
**成果物**: `sensitivity/analysis/common.rs` の変更

### タスク一覧

- [x] [TASK-2175: sensitivity common.rs 目的変数ループを par_iter 化](TASK-2175.md) - 2h (TDD) 🔵

### 依存関係

```
TASK-2175（独立）
```

---

## Phase 2: RandomForest 並列化

**期間**: 2026-05-07〜05-08（1日）
**目標**: `RandomForest::train` の木構築ループを `into_par_iter` で並列化
**成果物**: `core/random_forest/forest.rs` の変更

### タスク一覧

- [x] [TASK-2176: RandomForest::train 木構築ループを into_par_iter 化](TASK-2176.md) - 3h (TDD) 🔵

### 依存関係

```
TASK-2176（独立）
```

---

## Phase 3: Permutation 並列化

**期間**: 2026-05-09〜05-10（1日）
**目標**: `compute_from_prepared` の特徴量ループを `into_par_iter` で並列化
**成果物**: `sensitivity/permutation.rs` の変更

### タスク一覧

- [x] [TASK-2177: permutation.rs 特徴量ループを into_par_iter 化](TASK-2177.md) - 3h (TDD) 🔵

### 依存関係

```
TASK-2177（独立）
```

---

## Phase 4: Sobol 並列化

**期間**: 2026-05-12〜05-14（2日）
**目標**: Sobol 計算 3 段階（サロゲート構築・f_a/f_b・per-param 指標）を並列化
**成果物**: `sensitivity/sobol.rs` のリファクタリング + 並列化

### タスク一覧

- [x] [TASK-2178: sobol.rs 指標計算ヘルパー関数抽出](TASK-2178.md) - 3h (TDD) 🔵
- [x] [TASK-2179: sobol.rs 3 段階ループを par_iter / into_par_iter 化](TASK-2179.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-2178 → TASK-2179
```

---

## Phase 5: criterion ベンチマーク追加

**期間**: 2026-05-15〜05-16（1日）
**目標**: 全 4 並列化対象のベンチマークを criterion で追加し速度改善を定量計測
**成果物**: `rust_core/benches/` に 4 ファイル追加 + `Cargo.toml` 更新

### タスク一覧

- [x] [TASK-2180: criterion ベンチマーク 4 ファイル追加](TASK-2180.md) - 3h (DIRECT) 🔵

### 依存関係

```
TASK-2175, TASK-2176, TASK-2177, TASK-2179 → TASK-2180
```

