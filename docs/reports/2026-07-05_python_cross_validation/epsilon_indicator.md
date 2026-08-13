# additive ε-indicator — pymoo クロスチェック

- **実施日**: 2026-07-05
- **対象実装**: `rust_core/src/multi_objective/indicators.rs`(`additive_epsilon`)
- **リファレンス**: pymoo 0.6.2 `pymoo.indicators.epsilon.Epsilon`（内部で moocore の `epsilon_additive` を使用。Python 3.12, numpy 2.5.1）
- **結果**: ✅ **一致**(手作り境界ケース2件・2目的乱数(n_approx=20, n_ref=15)・3目的乱数(n_approx=15, n_ref=12)の全4ケースで最大絶対差 0)

## 検証方法

Rust ハーネス・Python スクリプトとも `igd_plus.md` と同一（`verify_indicators.rs` /
`check_igd_epsilon.py`）で、IGD+ と ε-indicator を同時に検証している。

```bash
cargo run -p tunny-core --example verify_indicators > verify_indicators.json
python check_igd_epsilon.py verify_indicators.json
```

検証ケースは `igd_plus.md` の表と共通。

## 実装読解での確認点

- Rust の `additive_epsilon(approx, reference)` は
  `I_ε+(A,Z) = max_{z∈Z} min_{a∈A} max_j (a_j - z_j)`（単項 additive ε-indicator、
  最小化前提）を素直に実装している。
- `moocore.epsilon_additive` の docstring 例（`dat`, `ref` の4点/6点サンプル）を
  上記の数式で手計算したところ `2.5` と一致し、**引数順序も `data=approx`・`ref=reference`
  で Rust と揃っている**ことを事前に確認した（`moocore.epsilon_additive(dat, ref=ref) == 2.5`）。
- pymoo の `Epsilon(pf=reference)` は `EpsilonIndicator.__init__` で
  `zero_to_one` を指定しないため追加の正規化は行われず（デフォルト `False`）、
  `_calc` は `moocore.epsilon_additive(F, ref=pf)` を素通しで呼ぶだけ。
  Rust 側も `[0,1]` に事前正規化した値をそのまま渡しているため、両者は同一の入力・同一の
  数式で比較していることになる。
- 符号（負の ε）についても確認: `approx` が `reference` を強く弱支配する場合
  ε は負になりうる（Rust 側ユニットテスト `additive_epsilon_can_be_negative_when_dominating`）。
  今回の乱数ケースでは正の値のみだったため、この境界は既存ユニットテストの手計算で
  別途担保されている。

## 検証に使った Python コード

```python
"""Rust (tunny-core) の igd_plus / additive_epsilon を pymoo (moocore) と突き合わせる。"""

import json
import sys

import numpy as np
from pymoo.indicators.igd_plus import IGDPlus
from pymoo.indicators.epsilon import Epsilon

path = sys.argv[1]
with open(path) as f:
    data = json.load(f)

TOL = 1e-9

all_ok = True
for case in data["igd_eps_cases"]:
    label = case["label"]
    approx = np.array(case["approx"], dtype=float)
    reference = np.array(case["reference"], dtype=float)

    igd_ind = IGDPlus(reference)
    ref_igd = float(igd_ind(approx))
    rust_igd = case["igd_plus"]

    eps_ind = Epsilon(reference)
    ref_eps = float(eps_ind(approx))
    rust_eps = case["epsilon"]

    igd_diff = abs(rust_igd - ref_igd)
    eps_diff = abs(rust_eps - ref_eps)
    ok = igd_diff < TOL and eps_diff < TOL
    all_ok &= ok
    status = "OK" if ok else "MISMATCH"
    print(
        f"{label}: n_approx={len(approx)} n_ref={len(reference)} m={approx.shape[1]}  "
        f"IGD+ rust={rust_igd:.10f} pymoo={ref_igd:.10f} diff={igd_diff:.2e} | "
        f"eps rust={rust_eps:.10f} pymoo={ref_eps:.10f} diff={eps_diff:.2e}  {status}"
    )

print()
print("PASS: 全ケースで Rust と pymoo/moocore が一致" if all_ok else "FAIL: 不一致あり")
sys.exit(0 if all_ok else 1)
```

## 実行結果

```text
hand_2d_identical_sets: n_approx=2 n_ref=2 m=2  IGD+ rust=0.0000000000 pymoo=0.0000000000 diff=0.00e+00 | eps rust=0.0000000000 pymoo=0.0000000000 diff=0.00e+00  OK
hand_2d_approx_dominates_reference: n_approx=1 n_ref=2 m=2  IGD+ rust=0.0000000000 pymoo=0.0000000000 diff=0.00e+00 | eps rust=0.0000000000 pymoo=0.0000000000 diff=0.00e+00  OK
random_2d_n20_ref15: n_approx=20 n_ref=15 m=2  IGD+ rust=0.0010194421 pymoo=0.0010194421 diff=0.00e+00 | eps rust=0.0118850864 pymoo=0.0118850864 diff=0.00e+00  OK
random_3d_n15_ref12: n_approx=15 n_ref=12 m=3  IGD+ rust=0.0168989015 pymoo=0.0168989015 diff=0.00e+00 | eps rust=0.0860171880 pymoo=0.0860171880 diff=0.00e+00  OK

PASS: 全ケースで Rust と pymoo/moocore が一致
```
