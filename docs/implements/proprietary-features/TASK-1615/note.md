# TASK-1615 実装コンテキストノート

**タスク**: `rust_core/src/topsis.rs` TOPSISアルゴリズム新規実装
**要件名**: mode-frontier-features
**作成日**: 2026-04-04

---

## 1. 技術スタック

- **言語**: Rust (edition 2021)
- **WASMターゲット**: wasm-bindgen（optional feature = "wasm"）
- **テストフレームワーク**: 標準 `#[test]`（cargo test）
- **シリアライゼーション**: serde / serde_wasm_bindgen（WASM境界用）
- 参照元: `rust_core/Cargo.toml`, `rust_core/src/lib.rs`

---

## 2. 開発ルール

### ファイル構造
- 新規ファイル: `rust_core/src/topsis.rs`
- lib.rs に `pub mod topsis;` を追加
- WASM export は TASK-1617 で実施（本タスクでは純粋Rust実装のみ）

### 命名規則
- テスト関数: `tc_1615_<seq>_<description>` 形式（既存: tc_201_*, tc_801_*）
- 公開関数: スネークケース、`pub fn compute_topsis(...)`
- 構造体: パスカルケース、`pub struct TopsisResult`

### コーディング規約
- `#[cfg(feature = "wasm")]` で WASM 依存部分を条件分岐
- エラーは `Result<T, String>` で返す
- NaN値は計算除外・スコア0.0・ランク末尾
- 参照元: `rust_core/src/pareto.rs`, `rust_core/src/sensitivity.rs`

---

## 3. 関連実装（参考パターン）

### pareto.rs の nd_sort()
```rust
// 非支配ソート — minimize/maximize フラグ処理のパターン
pub fn nd_sort(objs: &[Vec<f64>], is_min: &[bool]) -> Vec<usize>
```
参照元: `rust_core/src/pareto.rs`

### sensitivity.rs の ridge 回帰
```rust
// 行列演算パターン（手動実装、外部線形代数ライブラリなし）
pub fn compute_ridge(x_matrix: &[Vec<f64>], y: &[f64], lambda: f64) -> RidgeResult
```
参照元: `rust_core/src/sensitivity.rs`

### テストパターン例
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tc_201_01_two_obj_all_nondominated() {
        let objs = vec![vec![1.0, 4.0], vec![2.0, 3.0]];
        let is_min = [true, true];
        let ranks = nd_sort(&objs, &is_min);
        assert_eq!(ranks, vec![1, 1]);
    }
}
```
参照元: `rust_core/src/pareto.rs`

---

## 4. 設計文書

### TopsisResult 構造体
```rust
pub struct TopsisResult {
    pub scores: Vec<f64>,           // 全試行のスコア（trial順、0〜1）
    pub ranked_indices: Vec<u32>,   // スコア降順のインデックス
    pub positive_ideal: Vec<f64>,   // 正理想解（目的数次元）
    pub negative_ideal: Vec<f64>,   // 負理想解（目的数次元）
    pub duration_ms: f64,           // 計算時間（ms）
}
```

### compute_topsis() シグネチャ
```rust
pub fn compute_topsis(
    values: &[f64],        // [N × M] flattened（行major）
    n_trials: usize,
    n_objectives: usize,
    weights: &[f64],       // 目的数分の重み（合計1.0前提）
    is_minimize: &[bool],  // 各目的の最小化フラグ
) -> Result<TopsisResult, String>
```

### アルゴリズム手順
1. ベクトル正規化: `r_ij = v_ij / sqrt(sum_i(v_ij^2))`
2. 重み付き正規化: `w_ij = weights[j] * r_ij`
3. 理想解決定: minimize → A+= min, A-= max / maximize → A+= max, A-= min
4. ユークリッド距離: `D+_i = sqrt(sum_j((w_ij - A+_j)^2))`
5. スコア: `score_i = D-_i / (D+_i + D-_i)`, D++D-=0 の場合は 0.5
6. 降順ソート

参照元: `docs/design/mode-frontier-features/architecture.md`, `docs/tasks/mode-frontier-features/TASK-1615.md`

---

## 5. テスト関連情報

- **テストフレームワーク**: `cargo test`（標準Rustテスト）
- **テストファイル位置**: `rust_core/src/topsis.rs` 内の `#[cfg(test)] mod tests` ブロック
- **実行コマンド**: `cd rust_core && cargo test topsis`
- **パフォーマンステスト**: 50,000×4目的で100ms以内
- 参照元: `rust_core/src/pareto.rs`（テストパターン参照）

---

## 6. 注意事項

- **外部行列ライブラリなし**: nalgebra等は未使用。手動実装で対応
- **std::time::Instant**: WASM環境でも wasm-bindgen が shim を提供するため使用可能
- **DataFrame非依存**: compute_topsis は純粋関数（グローバルストア不使用）。値行列を直接受け取る
- **NaN処理**: NaN値を持つ試行はスコア0.0・ランク末尾（既存pareto.rsのNaN処理を参考）
- **エラーケース**:
  - `n_trials == 0` または `n_objectives == 0` → `Err(...)`
  - `values.len() != n_trials * n_objectives` → `Err(...)`
  - `weights.len() != n_objectives` → `Err(...)`
  - `is_minimize.len() != n_objectives` → `Err(...)`
- 参照元: `docs/design/mode-frontier-features/design-interview.md`（Q5残課題より）
