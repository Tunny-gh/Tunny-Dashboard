# PROMETHEE I/II — pymcdm クロスチェック

- **実施日**: 2026-07-05
- **対象実装**: `rust_core/src/mcdm/promethee.rs`(`compute_promethee`)
- **リファレンス**: pymcdm(`pymcdm.methods.PROMETHEE_II`, 選好関数 `'vshape'`)
- **結果**: ✅ **一致**(Φ+/Φ-/Φnet 最大絶対差 2.2e-16、PROMETHEE I / II のランキングも完全一致)

## 検証方法

1. Rust 側ハーネス `rust_core/examples/verify_promethee.rs` が TOPSIS/VIKOR と同じ生成方式
   (xorshift64\*, 20 alternatives × 4 objectives, minimize/maximize 混在, 負値を含む目的あり)で
   決定行列を作り、重みは未正規化(`[4.0, 1.0, 3.0, 2.0]`)で内部正規化も同時に確認する。
   Rust が内部で計算する閾値 `p_j = 0.2 * range_j`(有効行内の max-min)も
   Python 側で再計算せずに済むよう JSON に含めた。
2. **入力データ・重み・方向・p閾値・計算結果**(Φ+, Φ-, Φnet, PROMETHEE I/II のランキング、
   非両立ペア数)を JSON で出力する。
3. Python 側は同じ入力・同じ p 閾値を pymcdm の `PROMETHEE_II` に渡して再計算する。

```bash
cargo run -p tunny-core --example verify_promethee > verify_promethee.json
python check_promethee.py verify_promethee.json
```

## 実装読解での確認点

- 選好関数は **線形(V-shape)、無差別閾値 q=0**、優先閾値は `p_j = 0.2 * range_j` で自動設定
  (`compute_thresholds`、`promethee.rs:103-125`、モジュール冒頭のコメントにも明記)。
  pymcdm では `PROMETHEE_II('vshape', p=p_thresholds, q=None)` が同じ区分線形関数
  (`d<=0→0`, `d>=p→1`, それ以外 `d/p`)を実装しており、`p=0` の場合の境界挙動
  (`d>0` なら1、そうでなければ0)も両実装で完全に一致する。
- 差分の符号規約は pymcdm と同じ: minimize は `d=vb-va`、maximize は `d=va-vb`
  (`pairwise_preference`、`promethee.rs:143-161`)。pymcdm 内部の `diff_tables` も
  `types` に応じて `crit[i]-crit[j]`(profit)/`crit[j]-crit[i]`(cost)を使っており、同一の規約。
- Φ+ は行方向・Φ- は列方向の集約で、どちらも `/(N-1)` で正規化(`compute_flows`、
  `promethee.rs:168-225`)。pymcdm の `F_plus = sum(pi_table, axis=1)/(N-1)`,
  `F_minus = sum(pi_table, axis=0)/(N-1)` と同一の定義。
- 重みは呼び出し前に `super::normalize_weights` で合計1に正規化される(`promethee.rs:37`)。
  pymcdm には対応する処理がないため、Python 側で `weights / weights.sum()` を渡して条件を揃えた。
- Φ+/Φ- を直接取り出すため、pymcdm の `PROMETHEE_II.__call__`(公開 API は Φnet のみ返す)ではなく
  `_method()` を直接呼び出した。ライブラリの非公開メソッドだが、内部実装を変更せず読み取り専用で
  利用しているだけなので比較目的としては問題ない。
- PROMETHEE I のランキング(`ranked_indices_i`)は Φ+ 降順・タイは Φ- 昇順のトータルオーダー
  (`rank_promethee_i`、`promethee.rs:227-252`)。pymcdm には PROMETHEE I 用の総順序 API がないため、
  Python 側で同じキー `(-phi_plus, phi_minus)` によるソートを実装して突き合わせた。

## 検証に使った Python コード

```python
"""Rust (tunny-core) の PROMETHEE I/II を pymcdm.methods.PROMETHEE_II と突き合わせる。"""

import json
import sys

import numpy as np
from pymcdm.methods import PROMETHEE_II

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
p_thresholds = data["p_thresholds"]

# Rust uses a linear (V-shape) preference function with q=0 and p_j = 0.2 *
# range_j, computed per-objective from the valid-row min/max. pymcdm's
# 'vshape' preference function implements the same piecewise-linear formula
# (d<=0 -> 0, d>=p -> 1, else d/p), so passing the same p thresholds and
# q=None should reproduce Rust's flows exactly.
body = PROMETHEE_II("vshape", p=p_thresholds, q=None)
# Call the private _method directly to get phi_plus/phi_minus/phi_net
# separately (the public __call__ only returns the net flow).
_, _, phi_plus, phi_minus, phi_net = body._method(
    matrix.astype(float), weights, types
)

for name, rust_key, py_val in [
    ("phi_plus", "phi_plus", phi_plus),
    ("phi_minus", "phi_minus", phi_minus),
    ("phi_net", "phi_net", phi_net),
]:
    rust_val = np.array(data[rust_key])
    diff = np.abs(rust_val - py_val)
    print(f"{name}: max|diff| = {diff.max():.3e}")
    assert diff.max() < 1e-9, f"{name} mismatch"

rust_ranked_ii = np.array(data["ranked_indices_ii"])
py_ranked_ii = np.argsort(-phi_net, kind="stable")
print("PROMETHEE II ranking match:", np.array_equal(rust_ranked_ii, py_ranked_ii))
assert np.array_equal(rust_ranked_ii, py_ranked_ii)

# PROMETHEE I ranking: descending phi_plus, ties broken by ascending phi_minus.
order = sorted(range(n_trials), key=lambda i: (-phi_plus[i], phi_minus[i]))
rust_ranked_i = np.array(data["ranked_indices_i"])
print("PROMETHEE I ranking match:", np.array_equal(rust_ranked_i, np.array(order)))
assert np.array_equal(rust_ranked_i, np.array(order))

print("PASS: Rust PROMETHEE I/II matches pymcdm.methods.PROMETHEE_II (vshape)")
```

## 実行結果

```text
phi_plus: max|diff| = 2.220e-16
phi_minus: max|diff| = 0.000e+00
phi_net: max|diff| = 2.220e-16
PROMETHEE II ranking match: True
PROMETHEE I ranking match: True
PASS: Rust PROMETHEE I/II matches pymcdm.methods.PROMETHEE_II (vshape)
```
