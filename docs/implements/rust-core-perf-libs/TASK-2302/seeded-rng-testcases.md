# SeededRng 実装と乱数生成の rand 統一 — テストケース定義書

- **タスクID**: TASK-2302
- **要件名**: rust-core-perf-libs

---

## 1. 正常系テストケース

### TC-301-01: SeededRng 同一シードで同一乱数列を再現
- **何をテストするか**: `SeededRng::from_seed(seed)` で生成した RNG が同一シードで同一乱数列を生成すること
- **入力値**: `seed = 42`、`next_f64()` を 10 回呼び出し
- **期待される結果**: 2つの独立した SeededRng インスタンスが同一の乱数列を返す
- **テストの目的**: REQ-301-01（決定論的再現性）の確認
- 🔵 要件定義・設計文書より

```rust
#[test]
fn tc_301_01_seeded_rng_reproducibility() {
    // 【テスト目的】: 同一シードで同一乱数列が再現されることを確認
    let mut rng1 = SeededRng::from_seed(42);
    let mut rng2 = SeededRng::from_seed(42);
    for _ in 0..10 {
        let v1 = rng1.next_f64();
        let v2 = rng2.next_f64();
        assert_eq!(v1, v2, "同一シードで同一乱数列が再現されること");
        assert!(v1 >= 0.0 && v1 < 1.0, "next_f64 は [0,1) の範囲内");
    }
}
```

### TC-301-02: SeededRng 異なるシードで異なる乱数列
- **何をテストするか**: 異なるシードで異なる乱数列が生成されること
- **入力値**: `seed1 = 1`, `seed2 = 2`
- **期待される結果**: 最初の `next_f64()` の値が異なる
- 🔵 要件定義より

```rust
#[test]
fn tc_301_02_different_seeds_produce_different_sequences() {
    let mut rng1 = SeededRng::from_seed(1);
    let mut rng2 = SeededRng::from_seed(2);
    let v1 = rng1.next_f64();
    let v2 = rng2.next_f64();
    assert_ne!(v1, v2, "異なるシードで異なる乱数が生成されること");
}
```

### TC-301-03: SeededRng next_f64 の範囲確認
- **何をテストするか**: `next_f64()` が常に `[0, 1)` を返すこと
- **入力値**: seed = 0、1000 回呼び出し
- **期待される結果**: 全ての値が `>= 0.0` かつ `< 1.0`
- 🔵 要件定義より

### TC-301-04: SeededRng next_usize の範囲確認
- **何をテストするか**: `next_usize(n)` が `[0, n)` を返すこと
- **入力値**: n = 100、1000 回呼び出し
- **期待される結果**: 全ての値が `0..n` の範囲内
- 🔵 要件定義より

### TC-301-05: sobol.rs が SeededRng を使用していること（移行確認）
- **何をテストするか**: `sensitivity/sobol.rs` 内に `lcg_next` が存在しないこと
- **方法**: コンパイル確認（`lcg_next` への参照がコンパイルエラーとなること）
- **期待される結果**: `lcg_next` 関数が sobol.rs から削除されており、tests.rs の import もなくなっている
- 🔵 要件定義・移行要件より

### TC-301-06: compute_sobol_from_df が正常動作すること（回帰テスト）
- **何をテストするか**: lcg_next → SeededRng 移行後も Sobol 感度解析の計算が正常に動作すること
- **入力値**: 50行×2変数のデータフレーム
- **期待される結果**: `compute_sobol_from_df` が `Some(SobolResult)` を返す。インデックスが `[0, 1]` 範囲内
- 🔵 既存テスト tc_1610_04_sobol_indices_in_range を参考

---

## 2. 境界値テストケース

### TC-B01: seed = 0 での動作
- **何をテストするか**: seed=0 で動作すること
- **入力値**: `seed = 0`
- **期待される結果**: エラーなく `next_f64()` が `[0, 1)` を返す
- 🟡 境界値として妥当

### TC-B02: next_usize(1) での動作
- **何をテストするか**: bound=1 で next_usize が 0 を返すこと
- **入力値**: `n = 1`
- **期待される結果**: 常に `0`
- 🔵 `random_range(0..1)` は常に 0

---

## 3. 削除確認テストケース

### TC-DELETE-01: lcg_next が削除されていること
- **目的**: `sensitivity/sobol.rs` の `pub(crate) fn lcg_next` が削除されていること
- **方法**: `use super::sobol::lcg_next;` が tests.rs に存在しないこと
- 🔵 移行要件より必須

---

## 4. 開発言語・フレームワーク

- **プログラミング言語**: Rust (edition 2021)
- **テストフレームワーク**: Rust 標準 `#[test]`（cargo test）
- **テスト配置**: `rust_core/src/core/math/rng.rs` の `#[cfg(test)]` モジュール、および `rust_core/src/sensitivity/tests.rs`
- **テスト実行**: `cargo test -p tunny-core -- rng`
- 🔵 既存テスト構造より

---

## 5. テストケース実装の注意事項

- SeededRng は `pub(crate)` のため、テストは同一クレート内の `#[cfg(test)]` モジュールに記述
- `tc_1610_02_lcg_next_range` テストは lcg_next 削除後に SeededRng の next_f64 範囲テストに更新
- `use super::sobol::{build_quad_features, compute_sobol_index_pair, lcg_next};` の `lcg_next` import を削除

---

## 品質判定

✅ **高品質**
- 正常系・境界値・削除確認テストが網羅
- 期待値が明確（数値範囲、同一性、削除確認）
- Rust `#[test]` + cargo test で実現可能
- 信頼性: 🔵 90%, 🟡 10%

次のステップ: `/tsumiki:tdd-red rust-core-perf-libs TASK-2302` でテスト実装を開始。
