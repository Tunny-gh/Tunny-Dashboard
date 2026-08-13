# 感度分析 Spearman順位相関 — scipy クロスチェック

- **実施日**: 2026-07-05
- **対象実装**: `rust_core/src/sensitivity/spearman.rs`(`compute_spearman`)
- **リファレンス**: scipy 1.18.0 `scipy.stats.spearmanr`(Python 3.12, numpy 2.5.1)
- **結果**: ✅ **一致**(全ケースで最大絶対差 1.1e-16 = 倍精度丸め誤差)

## 検証方法

`compute_spearman` が内部で使う順位付け・Pearson相関(`math::stats::rank` /
`spearman_correlation`)自体は別レポート([correlation.md](correlation.md))で
scipy と突き合わせ済みのため、本検証は `compute_spearman` 固有の前処理である
**NaN/Inf のペアワイズ除去**が正しく効いているかに焦点を当てた。

1. Rust 側ハーネス `rust_core/examples/verify_spearman.rs` が決定的な擬似乱数で
   6 パターンのテストケース(クリーンデータ、タイあり、NaN 混入、Inf 混入、
   NaN+Inf+負相関の混在、有効ペア数が 2 未満に落ちる極端ケース)を生成し、
   入力と `compute_spearman` の出力を JSON で出力する。
2. Python 側は同じ入力について、**両方が有限(is_finite)な行だけを手動でマスク**
   してから `scipy.stats.spearmanr` を呼ぶ。scipy の `nan_policy='omit'` は
   NaN のみを除去対象とし Inf は除去しないため、Rust の実装(`is_finite()` による
   NaN/Inf 両方のペアワイズ除去)と同条件にするには手動マスクが必要(`correlation.md`
   と同じ方式)。

```bash
cargo run -p tunny-core --example verify_spearman > verify_spearman.json
python check_spearman.py verify_spearman.json
```

## 実装読解での確認点

- `compute_spearman(x, y)` は `x[i]`・`y[i]` のいずれかが非有限(NaN または Inf)
  であるペアを事前に除去してから `spearman_correlation` を呼ぶ(コード内コメントに
  「scipy の `nan_policy='omit'` に相当」とあるが、Inf も対象に含む点は
  `nan_policy='omit'` より広い)。
- 除去後の有効ペア数が 2 未満の場合は `f64::NAN` を返す。
- `n = x.len().min(y.len())` で長さの短い方に合わせてから処理する(スライス長不一致時)。

## 検証に使った Python コード

```python
"""Rust (tunny-core) の compute_spearman を scipy.stats.spearmanr と突き合わせる。

compute_spearman は NaN/Inf を含む行をペアワイズに除去してから順位相関を計算する
(is_finite() でのフィルタ)。scipy の nan_policy='omit' は NaN のみを対象とし Inf は
除去しないため、参照値は「両方が有限な行だけを手動でマスクしてから spearmanr を呼ぶ」
方式で計算する(correlation.md と同じ方式)。
"""

import json
import sys

import numpy as np
from scipy import stats

path = sys.argv[1]
with open(path) as f:
    cases = json.load(f)

all_ok = True
for case in cases:
    label = case["label"]
    x = np.array(case["x"], dtype=float)
    y = np.array(case["y"], dtype=float)
    rust_rho = case["rho"]

    mask = np.isfinite(x) & np.isfinite(y)
    n_valid = int(mask.sum())

    if n_valid < 2:
        ref_rho = None
    else:
        ref_rho = float(stats.spearmanr(x[mask], y[mask]).statistic)

    if rust_rho is None and ref_rho is None:
        ok = True
        diff = 0.0
    elif rust_rho is None or ref_rho is None:
        ok = False
        diff = float("nan")
    else:
        diff = abs(rust_rho - ref_rho)
        ok = diff < 1e-10

    all_ok &= ok
    print(
        f"{label:32s} n_valid={n_valid:3d}  rust={rust_rho!s:>20s}  "
        f"scipy={ref_rho!s:>20s}  diff={diff:.3e}  {'OK' if ok else 'MISMATCH'}"
    )

print()
print("PASS: Rust compute_spearman と scipy.stats.spearmanr は一致" if all_ok else "FAIL: 不一致あり")
sys.exit(0 if all_ok else 1)
```

## 実行結果

```text
clean                            n_valid= 30  rust=   0.978642936596218  scipy=   0.978642936596218  diff=0.000e+00  OK
ties                             n_valid= 30  rust=-0.19616510718523328  scipy=-0.19616510718523328  diff=0.000e+00  OK
nan_in_x                         n_valid= 25  rust=  0.9815384615384616  scipy=  0.9815384615384616  diff=0.000e+00  OK
inf_in_y                         n_valid= 25  rust=  0.9684615384615385  scipy=  0.9684615384615385  diff=0.000e+00  OK
mixed_nan_inf_negative_corr      n_valid= 27  rust=                -1.0  scipy= -0.9999999999999999  diff=1.110e-16  OK
sparse_below_min_pairs           n_valid=  0  rust=                None  scipy=                None  diff=0.000e+00  OK

PASS: Rust compute_spearman と scipy.stats.spearmanr は一致
```

`sparse_below_min_pairs` ケースは NaN/Inf を多数混在させ、フィルタ後の有効ペア数を
0(< 2)に落としたケース。Rust・scipy いずれも「計算不能」(Rust は `NaN`、Python
側の判定ロジックも `None`)を返し、`fx.len() < 2` の早期リターンが scipy の
挙動と整合することを確認した。
