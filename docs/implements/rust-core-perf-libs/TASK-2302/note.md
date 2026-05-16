# TASK-2302 開発コンテキスト

## 1. 技術スタック

- **言語**: Rust (edition 2021)
- **RNG**: rand 0.9 + rand_chacha 0.9
  - `rand::Rng::random()` / `random_range()` (rand 0.9 API)
  - `rand_chacha::ChaCha8Rng` (決定論的シード対応)
  - `rand::SeedableRng::seed_from_u64(seed)`
- **テストフレームワーク**: Rust標準 `#[test]`
- 参照元: rust_core/Cargo.toml

## 2. 開発ルール

- WASM ビルド不要、ネイティブ API を自由に使用可
- SeededRng は `&mut self` を使用（RefCell 不要）
- rand 0.9 では `gen` がキーワードのため `r#gen` でエスケープ → ただし実際には `random()` / `random_range()` を使用
- テストは `#[test]` のみ（外部テストランナーなし）
- 参照元: docs/tasks/rust-core-perf-libs/TASK-2302.md

## 3. 関連実装

### 実装済み（変更不要）
- **SeededRng**: `rust_core/src/core/math/rng.rs`
  - `from_seed(seed: u64)` で ChaCha8Rng を初期化
  - `next_usize(n: usize)` — `random_range(0..n)` を使用
  - `next_f64()` — `random()` を使用
  - `pub(crate)` でクレート内公開

- **sampling/common.rs**: SeededRng 移行済み
  - `random_sample_fixed_seed` が `SeededRng::from_seed(42)` を使用
  - Fisher-Yates シャッフルを SeededRng で実装済み

### 未移行（TASK-2302 の残作業）
- **sensitivity/sobol.rs**:
  - `lcg_next(state: &mut u64)` 関数（行 15）: pub(crate) — テストから参照されている
  - `compute_sobol_from_df` で `lcg_next` を 2 箇所使用（行 258, 267）
  - `rng_state: u64 = 0xDEAD_BEEF_1234_5678` — この値を seed として SeededRng に移行

- **sensitivity/tests.rs**:
  - `lcg_next` を import して直接テスト（`tc_1610_02_lcg_next_range`）
  - 移行後はテストを SeededRng のテストに更新するか削除

## 4. 設計文書

- 移行後のシグネチャ:
  ```rust
  // sobol.rs: lcg_next の呼び出しを SeededRng に置き換え
  let mut rng = SeededRng::from_seed(0xDEAD_BEEF_1234_5678);
  // ... mat_a, mat_b の生成で rng.next_f64() を使用
  ```
- 参照元: docs/design/rust-core-perf-libs/architecture.md
- 参照元: docs/tasks/rust-core-perf-libs/TASK-2302.md

## 5. テスト関連情報

- **テストファイル**: `rust_core/src/sensitivity/tests.rs`
- **既存テスト**: `tc_1610_02_lcg_next_range` — lcg_next を直接テスト（移行後は SeededRng テストに更新）
- **追加すべきテスト**:
  - TC-301-01: SeededRng 同一シード → 同一乱数列（再現性）
  - TC-301-02: sobol.rs での SeededRng 使用確認
- **テスト実行**: `cargo test -p tunny-core -- sensitivity`

## 6. 注意事項

- `lcg_next` は `pub(crate)` のため tests.rs から直接 import されている
- 削除時には tests.rs の import と `tc_1610_02_lcg_next_range` テストも更新が必要
- rand 0.9 API: `rng.gen()` → `rng.random()`, `gen_range` → `random_range`
- SeededRng を sobol.rs で使用するため `use crate::core::math::rng::SeededRng;` を追加
