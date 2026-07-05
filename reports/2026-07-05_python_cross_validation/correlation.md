# 相関行列 (Pearson / Spearman) — scipy クロスチェック

- **実施日**: 2026-07-05
- **対象実装**: `rust_core/src/statistics/correlation.rs`(`compute_correlation_matrix`)、`rust_core/src/math/stats.rs`(`pearson_correlation` / `spearman_correlation` / `rank`)
- **リファレンス**: scipy 1.18.0 `scipy.stats.pearsonr` / `scipy.stats.spearmanr`(Python 3.12, numpy 2.5.1)
- **結果**: ✅ **一致**(最大絶対差 2.2e-16 = 倍精度丸め誤差、NaN の位置も完全一致)

## 検証方法

1. Rust 側ハーネス `rust_core/examples/verify_correlation.rs` が決定的な擬似乱数で
   n=60 のテストデータ 5 列(強相関ペア・独立列・タイを含む離散列・NaN 混入列)を生成し、
   **入力データと計算結果の両方**を JSON で出力する。
2. Python 側は同じ入力を scipy で再計算して突き合わせる(乱数生成器を揃える必要がない)。

```bash
cargo run -p tunny-core --example verify_correlation > verify_correlation.json
python check_correlation.py verify_correlation.json
```

## 実装読解での確認点

- Pearson: 標本共分散/√(分散積) の標準定義。分母 `sqrt(var_x * var_y) < ε` で NaN(定数列ガード)。
- Spearman: 順位化(タイは平均順位)→ Pearson。scipy の `spearmanr` と同じタイ処理。
- 行列構築時は **pairwise complete-case deletion**(両方が有限の行のみ使用、有効行 < 2 で NaN)。
  scipy 側もマスクを掛けて同条件で比較した。
- NaN は JSON では `null` として出力される(serde_json の仕様)ため、Python 側で nan に復元。

## 検証に使った Python コード

```python
"""Rust (tunny-core) の相関行列を scipy.stats と突き合わせる。"""

import json
import sys

import numpy as np
from scipy import stats

path = sys.argv[1]
with open(path) as f:
    data = json.load(f)

# serde_json は NaN を null で出力するので nan に戻す
cols = [
    (c["label"], np.array([np.nan if v is None else v for v in c["values"]]))
    for c in data["inputs"]
]
k = len(cols)


def ref_matrix(func):
    m = np.full((k, k), np.nan)
    np.fill_diagonal(m, 1.0)
    for i in range(k):
        for j in range(i + 1, k):
            xi, xj = cols[i][1], cols[j][1]
            mask = np.isfinite(xi) & np.isfinite(xj)
            if mask.sum() < 2:
                continue
            r = func(xi[mask], xj[mask]).statistic
            m[i, j] = m[j, i] = r
    return m


for name, func in [("pearson", stats.pearsonr), ("spearman", stats.spearmanr)]:
    rust = np.array(
        [[np.nan if v is None else v for v in row] for row in data[name]]
    )
    ref = ref_matrix(func)
    both = np.isfinite(rust) & np.isfinite(ref)
    max_diff = np.max(np.abs(rust[both] - ref[both]))
    nan_match = np.array_equal(np.isnan(rust), np.isnan(ref))
    print(f"{name}: max|diff| = {max_diff:.3e}  NaN位置一致 = {nan_match}")
    assert max_diff < 1e-10, f"{name} mismatch"
    assert nan_match

print("PASS: Rust と scipy の相関行列は一致")
```

## 実行結果

```text
pearson: max|diff| = 2.220e-16  NaN位置一致 = True
spearman: max|diff| = 2.220e-16  NaN位置一致 = True
PASS: Rust と scipy の相関行列は一致
```
