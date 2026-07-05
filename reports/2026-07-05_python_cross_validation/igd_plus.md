# IGD+（Inverted Generational Distance Plus）— pymoo クロスチェック

- **実施日**: 2026-07-05
- **対象実装**: `rust_core/src/multi_objective/indicators.rs`(`igd_plus`)
- **リファレンス**: pymoo 0.6.2 `pymoo.indicators.igd_plus.IGDPlus`（内部で moocore の `igd_plus` を使用。Python 3.12, numpy 2.5.1）
- **結果**: ✅ **一致**(手作り境界ケース2件・2目的乱数(n_approx=20, n_ref=15)・3目的乱数(n_approx=15, n_ref=12)の全4ケースで最大絶対差 0)

## 検証方法

1. Rust 側ハーネス `rust_core/examples/verify_indicators.rs` が `[0,1]` 正規化済み空間の
   `approx`（近似集合）・`reference`（参照集合）を生成し、`igd_plus(approx, reference)` の
   結果を **入力データと計算結果の両方** JSON で出力する。
2. Python 側は `IGDPlus(reference)(approx)` で再計算する。`IGDPlus` はデフォルトで
   `zero_to_one=False` / `norm_by_dist=False` のため、Rust 側と同じ「追加の正規化なし」の
   条件で比較できる（内部で `moocore.igd_plus(F, ref=pf)` を直接呼ぶだけになる）。

```bash
cargo run -p tunny-core --example verify_indicators > verify_indicators.json
python check_igd_epsilon.py verify_indicators.json
```

### 検証ケース

| ラベル | n_approx | n_ref | m | 内容 |
|---|---|---|---|---|
| `hand_2d_identical_sets` | 2 | 2 | 2 | 近似集合=参照集合（IGD+=0 のはず） |
| `hand_2d_approx_dominates_reference` | 1 | 2 | 2 | 近似が参照を完全に支配（IGD+=0 のはず） |
| `random_2d_n20_ref15` | 20 | 15 | 2 | 乱数 |
| `random_3d_n15_ref12` | 15 | 12 | 3 | 乱数 |

## 実装読解での確認点

- Rust の `igd_plus(approx, reference)` は、参照集合 `reference` の各点 `z` について
  近似集合 `approx` 内の点 `a` への修正距離 `d+(a,z) = sqrt(Σ max(a_j - z_j, 0)^2)` の最小値を取り、
  その平均を返す（引数順は `(approx, reference)`）。
- pymoo 側は `IGDPlus(pf=reference)` としてコンストラクトし、`(F=approx)` を呼び出す形。
  `IGDPlus._do` は `norm_by_dist=False`（デフォルト）のとき `moocore.igd_plus(F, ref=self.pf)` を
  そのまま呼ぶだけで、追加の正規化は行わない。データの normalize（`[0,1]` スケール化）は
  呼び出し前に Rust 側で完了させている前提と一致しており、引数の対応関係
  （`approx` ↔ `F`、`reference` ↔ `pf`/`ref`）も一致している。
- `reference` が空のとき Rust は `0.0` を、`approx` が空のとき `f64::INFINITY` を返す
  ガード節があるが、pymoo 側は空集合の挙動が異なりうるため、今回の検証データには含めていない
  （空集合ガードは Rust 固有の防御的実装として別途ユニットテストで担保されている）。

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

（このスクリプトは ε-indicator も同時に検証している。詳細は `epsilon_indicator.md` を参照）

## 実行結果

```text
hand_2d_identical_sets: n_approx=2 n_ref=2 m=2  IGD+ rust=0.0000000000 pymoo=0.0000000000 diff=0.00e+00 | eps rust=0.0000000000 pymoo=0.0000000000 diff=0.00e+00  OK
hand_2d_approx_dominates_reference: n_approx=1 n_ref=2 m=2  IGD+ rust=0.0000000000 pymoo=0.0000000000 diff=0.00e+00 | eps rust=0.0000000000 pymoo=0.0000000000 diff=0.00e+00  OK
random_2d_n20_ref15: n_approx=20 n_ref=15 m=2  IGD+ rust=0.0010194421 pymoo=0.0010194421 diff=0.00e+00 | eps rust=0.0118850864 pymoo=0.0118850864 diff=0.00e+00  OK
random_3d_n15_ref12: n_approx=15 n_ref=12 m=3  IGD+ rust=0.0168989015 pymoo=0.0168989015 diff=0.00e+00 | eps rust=0.0860171880 pymoo=0.0860171880 diff=0.00e+00  OK

PASS: 全ケースで Rust と pymoo/moocore が一致
```
