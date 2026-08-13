# エントロピー重み — pymcdm クロスチェック

- **実施日**: 2026-07-05
- **対象実装**: `rust_core/src/mcdm/entropy.rs`(`compute_entropy_weights`)
- **リファレンス**: pymcdm(`pymcdm.weights.entropy_weights`)
- **結果**: ✅ **一致(正値のみの行列)**(重み最大絶対差 9.1e-16 = 倍精度丸め誤差)
  ⚠️ 負値/0を含む列の独自前処理は pymcdm に対応する実装がないため比較対象外(下記参照)

## 検証方法

1. Rust 側ハーネス `rust_core/examples/verify_entropy.rs` が決定的な擬似乱数(xorshift64\*)で
   20 alternatives × 4 objectives の **全値が正** の決定行列を生成する(列ごとにスケール・分散を変え、
   1列は分散をわざと小さくして重みの差が出るようにした)。
2. **入力データと計算結果**(重み、entropy、diversity)を JSON で出力する。
3. Python 側は同じ入力を `pymcdm.weights.entropy_weights` で再計算して突き合わせる。

```bash
cargo run -p tunny-core --example verify_entropy > verify_entropy.json
python check_entropy.py verify_entropy.json
```

## 実装読解での確認点

- 正値のみの列に対する Rust の計算は標準的なエントロピー重み法そのもの:
  比率正規化 `p_ij = x_ij / sum_i(x_ij)` → `e_j = -(1/ln m) * sum_i(p_ij * ln p_ij)` →
  `d_j = 1 - e_j` → `w_j = d_j / sum_k(d_k)`(`entropy.rs:78-118`)。
  pymcdm の `entropy_weights` も `sum_normalization`(=比率正規化)→
  `e_j = -sum(p*ln p)/ln(m)` → `E = 1-e` → `E/sum(E)` と同一の式であり、
  正値のみの入力では完全に一致する。
- **pymcdm 側の制約**: `pymcdm.normalizations.sum_normalization` は
  `if np.any(x <= 0): raise ValueError(...)` と、列に0以下の値が1つでもあると例外を送出する
  (`sum_normalization` 実装、pymcdm 側ソース確認済み)。そのため pymcdm と直接比較できるのは
  **正値のみの決定行列**に限られる。今回の検証はその制約に従って正値行列のみで実施した。
- **負値/0を含む列の扱いは Rust 独自のロジック**(`entropy.rs:39-76`)であり、pymcdm には対応する
  実装が存在しないため、今回のクロスチェック対象には含めていない。この独自ロジックについて
  実装を読んだ限りでの確認結果は以下の通り:
  - 列に負値が1つでもあれば、その列だけ min-max 正規化(`(x - min) / (max - min)`)を先に適用してから
    比率正規化に進む(正の列はそのまま比率正規化のみ)。
  - 監査 A3 で指摘されていた「負の定数列が最大重みを得るバグ」は、コード中のコメント
    (`entropy.rs:64-67` `// 定数列は全行 1.0 とする。0.0 だと比率正規化で p=0 -> e_j=0 -> d_j=1 となり、
    情報量ゼロの列が最大重みを得てしまう`)から見て **修正済み**であることを確認した。
    定数列(min-max のレンジが0)は min-max 後の値を `0.0` ではなく `1.0` に固定することで、
    比率正規化後にその列の全行が `p_ij = 1/m`(一様分布)となり、結果として
    `entropy_j = 1.0`, `diversity_j = 0.0`, `weight_j ≈ 0` となる。
    これは Rust の単体テスト `tc_entropy_11_negative_constant_column`
    (`entropy.rs:274-290`、`values = [-3.0, 1.0, -3.0, 2.0, -3.0, 3.0]` で
    `weights[0] ≈ 0`, `weights[1] ≈ 1` を assert)でも確認されている挙動と一致する。

## 検証に使った Python コード

```python
"""Rust (tunny-core) のエントロピー重みを pymcdm.weights.entropy_weights と突き合わせる。

pymcdm.weights.entropy_weights は内部で sum_normalization を使い、これは
「全値が正 (>0)」でないと ValueError を送出する。そのため、この比較は
正値のみの決定行列に限定する（負値/0を含む列の扱いは Rust 側の独自ロジック
であり、pymcdm には対応する実装がないため対象外）。
"""

import json
import sys

import numpy as np
from pymcdm.weights import entropy_weights

path = sys.argv[1]
with open(path) as f:
    data = json.load(f)

n_trials = data["n_trials"]
n_obj = data["n_objectives"]
matrix = np.array(data["values"]).reshape(n_trials, n_obj)

assert np.all(matrix > 0), "this comparison requires an all-positive matrix"

py_weights = entropy_weights(matrix)
rust_weights = np.array(data["weights"])

diff = np.abs(py_weights - rust_weights)
print(f"weights: max|diff| = {diff.max():.3e}")
print("rust   :", rust_weights)
print("pymcdm :", py_weights)

assert diff.max() < 1e-9, "entropy weights mismatch"
print("PASS: Rust entropy weights match pymcdm.weights.entropy_weights (all-positive matrix)")
```

## 実行結果

```text
weights: max|diff| = 9.120e-16
rust   : [0.41403401 0.20736338 0.00094692 0.37765569]
pymcdm : [0.41403401 0.20736338 0.00094692 0.37765569]
PASS: Rust entropy weights match pymcdm.weights.entropy_weights (all-positive matrix)
```
