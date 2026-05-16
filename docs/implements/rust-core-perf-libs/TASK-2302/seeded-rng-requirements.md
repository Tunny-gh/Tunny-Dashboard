# SeededRng 実装と乱数生成の rand 統一 — 要件定義書

- **タスクID**: TASK-2302
- **要件名**: rust-core-perf-libs
- **フェーズ**: Phase 1 - 基盤整備

---

## 1. 機能の概要

- 🔵 独自 PRNG（LCG）を `rand` + `rand_chacha` ベースの `SeededRng` に統一する
- 🔵 対象: `sensitivity/sobol.rs` の `lcg_next` 関数 → `SeededRng::next_f64()` に置き換え
- 🔵 `SeededRng` 自体（`core/math/rng.rs`）と `sampling/common.rs` の移行はすでに完了済み
- 🔵 決定論的シード再現性を保証（Sobol 感度解析のモンテカルロサンプリングにも適用）

**参照した設計文書**: docs/design/rust-core-perf-libs/architecture.md § core/math/rng.rs

---

## 2. 入力・出力の仕様

### SeededRng（実装済み）

| API | 入力 | 出力 |
|-----|------|------|
| `SeededRng::from_seed(seed: u64)` | u64 シード値 | SeededRng インスタンス |
| `next_f64(&mut self) -> f64` | なし | `[0, 1)` の f64 |
| `next_usize(&mut self, n: usize) -> usize` | 上限 n | `[0, n)` の usize |

### sobol.rs の移行（残作業）

**変更前**:
```rust
let mut rng_state: u64 = 0xDEAD_BEEF_1234_5678;
// mat_a の行を生成
.map(|(lo, hi)| lo + lcg_next(&mut rng_state) * (hi - lo))
// mat_b の行を生成
.map(|(lo, hi)| lo + lcg_next(&mut rng_state) * (hi - lo))
```

**変更後**:
```rust
let mut rng = SeededRng::from_seed(0xDEAD_BEEF_1234_5678);
// mat_a の行を生成
.map(|(lo, hi)| lo + rng.next_f64() * (hi - lo))
// mat_b の行を生成
.map(|(lo, hi)| lo + rng.next_f64() * (hi - lo))
```

**参照した設計文書**: docs/tasks/rust-core-perf-libs/TASK-2302.md

---

## 3. 制約条件

- 🔵 `SeededRng` は `pub(crate)` — クレート外部には公開しない
- 🔵 `lcg_next` は `pub(crate)` なので `sensitivity/tests.rs` から直接インポートされている → 削除時にテストも更新必要
- 🔵 rand 0.9 API: `gen()` → `random()`, `gen_range` → `random_range()`
- 🟡 Sobol 感度解析の結果が lcg_next と SeededRng で数値的に異なる可能性 → 許容（実装変更が目的）
- 🔵 `clustering/kmeans.rs` の xorshift64 は Phase 4 で linfa 化されるため本タスクでは対象外

---

## 4. 想定される使用例

### 基本ケース
- `SeededRng::from_seed(42)` → 同一シードで毎回同一乱数列を再現
- sobol.rs でのモンテカルロサンプリング: mat_a, mat_b の生成

### エッジケース
- seed=0: 動作すること
- 連続した `next_f64()` 呼び出し: 全て `[0, 1)` の範囲内

### 削除対象
- `sensitivity/sobol.rs` の `lcg_next` 関数定義（行 15-20）
- `sensitivity/sobol.rs` の `lcg_next` 使用（行 258, 267）
- `sensitivity/tests.rs` の `lcg_next` import
- `sensitivity/tests.rs` の `tc_1610_02_lcg_next_range` テスト（SeededRng のテストに置き換え）

---

## 5. EARS要件との対応関係

- **参照した機能要件**: REQ-301-01（シード再現性）, REQ-301-03（Fisher-Yates rand 化）
- **参照した設計文書**:
  - アーキテクチャ: docs/design/rust-core-perf-libs/architecture.md § core/math/rng.rs
  - タスク定義: docs/tasks/rust-core-perf-libs/TASK-2302.md

---

## 品質判定

✅ **高品質**
- 要件の曖昧さ: なし
- 入出力定義: 完全（変更前後のコードまで明示）
- 制約条件: 明確
- 実装可能性: 確実（残作業は sobol.rs 1ファイルのみ）
- 信頼性レベル: 🔵 90%, 🟡 10%
