# TOPSIS — pymcdm クロスチェック

- **実施日**: 2026-07-05
- **対象実装**: `rust_core/src/mcdm/topsis.rs`(`compute_topsis`)
- **リファレンス**: pymcdm(`pymcdm.methods.TOPSIS`)、numpy 2.5.1 / scipy 1.18.0 と同一 Python 環境
- **結果**: ✅ **一致**(スコア最大絶対差 2.2e-16 = 倍精度丸め誤差、ランキングも完全一致)

## 検証方法

1. Rust 側ハーネス `rust_core/examples/verify_topsis.rs` が決定的な擬似乱数(xorshift64\*)で
   20 alternatives × 4 objectives の決定行列を生成する。目的の方向は minimize/maximize を混在させ
   (`is_minimize = [true, false, true, false]`)、目的2は負値を含む(-50〜50)。
   重みはあえて未正規化(`[4.0, 1.0, 3.0, 2.0]`, 合計10)にして、Rust 内部の `normalize_weights` が
   正しく合計1に正規化することも同時に確認する。
2. **入力データ・重み・方向・計算結果**(スコア、ランキング、正/負の理想解)を JSON で出力する。
3. Python 側は同じ入力を pymcdm で再計算して突き合わせる。

```bash
cargo run -p tunny-core --example verify_topsis > verify_topsis.json
python check_topsis.py verify_topsis.json
```

## 実装読解での確認点

- `compute_topsis` は重みを呼び出し前に `super::normalize_weights` で合計1に正規化してから使用する
  (`topsis.rs:53`)。これは pymcdm 側には対応する処理がないため、Python に渡す前に
  `weights / weights.sum()` を計算して条件を揃えた。
- 正規化方式は **ベクトル正規化**: `r_ij = v_ij / sqrt(sum_i(v_ij^2))`(`build_weighted_matrix`、
  `topsis.rs:132-170`)。pymcdm の TOPSIS はデフォルトで `minmax_normalization` を使うため、
  `normalization_function=pymcdm.normalizations.vector_normalization` を明示的に渡して揃えた。
- **方向の扱いが pymcdm と表記上異なる点に注意**: pymcdm の `vector_normalization(x, cost=True)` は
  cost 列を `1 - x/‖x‖` へ反転してから常に `max` を正の理想解として扱う。一方 Rust 実装は反転せず、
  `is_minimize[j]` に応じて `positive_ideal[j]` を `col_min[j]`(minimize)または `col_max[j]`
  (maximize)から選択する(`find_ideal_solutions`、`topsis.rs:175-208`)。
  この2つは見た目は違うが数学的に等価: pymcdm の反転 `v'_ij = weight_j - v_ij` は列ごとの
  **定数だけの平行移動**であり、`D+_i = sqrt(sum_j(v'_ij - pis_j)^2)` を展開すると
  `pis_j`(pymcdm)と `positive_ideal_j`(Rust)の差の定数項が相殺され、`D+`/`D-` は完全に一致する
  (実測でも上記の通り diff は倍精度丸め誤差のみ)。
- スコアは重みのスケールに対して不変(`unnormalized_weights_match_normalized` テストで既に単体検証済み)。
  今回のクロスチェックでも同じ性質を Python 側の重み正規化と揃えることで確認した。

## 検証に使った Python コード

```python
"""Rust (tunny-core) の TOPSIS を pymcdm.methods.TOPSIS と突き合わせる。"""

import json
import sys

import numpy as np
from pymcdm.methods import TOPSIS
from pymcdm import normalizations

path = sys.argv[1]
with open(path) as f:
    data = json.load(f)

n_trials = data["n_trials"]
n_obj = data["n_objectives"]
matrix = np.array(data["values"]).reshape(n_trials, n_obj)
weights_raw = np.array(data["weights"])
weights = weights_raw / weights_raw.sum()  # Rust normalizes weights internally
is_minimize = data["is_minimize"]
types = np.array([-1 if m else 1 for m in is_minimize])

# Rust uses vector normalization (r_ij = v_ij / sqrt(sum_i v_ij^2)), not the
# pymcdm TOPSIS default (minmax). TOPSIS's D+/D- distances are invariant to
# the "1 - r" flip vector_normalization applies for cost columns (it is a
# per-column translation by a constant, which cancels out of both D+ and D-),
# so vector_normalization should reproduce Rust's positive/negative-ideal
# selection by min/max exactly.
body = TOPSIS(normalization_function=normalizations.vector_normalization)
scores = body(matrix, weights, types)

rust_scores = np.array(data["scores"])
diff = np.abs(scores - rust_scores)
print(f"max|diff| scores = {diff.max():.3e}")

rust_ranked = np.array(data["ranked_indices"])
py_ranked = np.argsort(-scores, kind="stable")
print("ranking match:", np.array_equal(rust_ranked, py_ranked))

assert diff.max() < 1e-9, "TOPSIS scores mismatch"
assert np.array_equal(rust_ranked, py_ranked), "TOPSIS ranking mismatch"
print("PASS: Rust TOPSIS matches pymcdm.methods.TOPSIS (vector_normalization)")
```

## 実行結果

```text
max|diff| scores = 2.220e-16
ranking match: True
PASS: Rust TOPSIS matches pymcdm.methods.TOPSIS (vector_normalization)
```
