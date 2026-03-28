# TDD開発メモ: sensitivity

## 概要

- 機能名: 感度分析WASM実装（Spearman / Ridge / R²）
- 開発開始: 2026-03-27
- 現在のフェーズ: 完了

## 関連ファイル

- 実装ファイル:
  - `rust_core/src/sensitivity.rs`
- テストファイル:
  - `rust_core/src/sensitivity.rs` (インラインテスト)

## テストケース（16件）

| ID | 分類 | 内容 |
|---|---|---|
| TC-801-01 | 正常系 | 完全正相関データで Spearman = 1.0 |
| TC-801-02 | 正常系 | 完全負相関データで Spearman = -1.0 |
| TC-801-03 | 正常系 | 既知数値例で Spearman = 13/35 ≈ 0.3714 |
| TC-801-04 | 正常系 | タイあり（同値）でも正常計算 |
| TC-801-05 | 境界値 | n < 2 で 0.0 を返す |
| TC-801-06 | 正常系 | 完全線形データで R² > 0.99 |
| TC-801-07 | 正常系 | 正の関係でβ係数の符号が正 |
| TC-801-08 | 正常系 | 2変数で強い関係のβ絶対値が大きい |
| TC-801-09 | 境界値 | 空データで RidgeResult 空を返す |
| TC-801-10 | 正常系 | compute_sensitivity_all が P×M の行列を返す |
| TC-801-11 | 正常系 | x1↑→obj0↑、x2↑→obj0↓の相関符号が正しい |
| TC-801-12 | 正常系 | compute_sensitivity_selected がサブセットで計算 |
| TC-801-13 | 境界値 | 空インデックスで空の SensitivityResult |
| TC-801-P01 | 性能 | Spearman: debug=5k×10×4、release=50k×30×4 ≤500ms |
| TC-801-P02 | 性能 | Ridge: debug=5k×10×4、release=50k×30×4 ≤300ms |
| TC-801-P03 | 性能 | compute_sensitivity_selected: debug=5k、release=50k ≤50ms |

## 主要な設計決定

1. **Spearman 相関係数**
   - `rank()` 関数: O(n log n) ソート + 平均順位割り当て（タイ対応）
   - `pearson_correlation(rank_x, rank_y)` でピアソン → Spearman

2. **Ridge 回帰（列優先最適化）**
   - 行優先 x_matrix → 列優先フラット配列 `x_cols[j*n+i]` に変換
   - XTX を列ドット積で計算（連続メモリアクセス → キャッシュ効率向上）
   - 上三角のみ計算して対称性で補完（演算量を約1/2に削減）
   - 正規方程式をガウス消去法（部分ピボット付き）で解く

3. **compute_sensitivity_all の XTX 事前計算**
   - X の標準化とXTX計算を1回のみ実施
   - 全目的でXTXを共有して計算コストを削減

4. **パフォーマンステスト（debug/release 分岐）**
   - `#[cfg(debug_assertions)]` で debug 時はデータサイズを縮小
   - debug: n=5,000, p=10、release: n=50,000, p=30

## 最終テスト結果

```
Running unittests sensitivity
test result: ok. 16 passed; 0 failed
All Rust tests: 100 passed; 0 failed
```

## 品質評価

✅ **高品質**
- テスト: 100/100 通過（Rust全体）
- セキュリティ: 重大な脆弱性なし（パニックなし、NaN対応）
- パフォーマンス: debug/release 両モードで要件達成
- REQ-090〜REQ-092 準拠
