# R2 indicator — numpy 標準定義とのクロスチェック

- **実施日**: 2026-07-05
- **対象実装**: `rust_core/src/multi_objective/indicators.rs`(`r2_indicator` / `simplex_lattice_weights`)
- **リファレンス**: pymoo 0.6.2 に R2 indicator の実装は存在しないため、
  Hansen & Jaszkiewicz (1998) の重み付き Tchebycheff スカラー化による標準定義
  `R2(A,W,z*) = (1/|W|) Σ_w min_{a∈A} max_j w_j(a_j - z*_j)` を numpy で独自実装し、
  リファレンスとした（Python 3.12, numpy 2.5.1）。
- **結果**: ✅ **一致**(手作りケース1件・2目的乱数(n=20)・3目的乱数(n=15)の全3ケースで最大絶対差 0。
  重みベクトルが単体（各成分の和=1）上に乗っていることも全ケースで確認)

## 検証方法

`r2_indicator` は公開関数（`pub fn`）だが、重みベクトル生成の `simplex_lattice_weights` は
private のため crate 外から直接呼べない。そこで Rust ハーネス側にこの関数と同一のアルゴリズム
（Das-Dennis 単体格子、個数が100以下になる最大の分割数 `h` を選び、各重みを `k/h`（下限 `1e-6`）
としてから合計1に再正規化）を複製し、生成した重みをそのまま公開関数 `r2_indicator` に渡した。
これにより「重み生成が同じであれば `r2_indicator` の計算式自体が正しいか」を検証する
（重み生成アルゴリズムそのものは pymoo に対応物がなく比較対象がないため、
Rust の実装読解に基づきこのハーネス内に複製したものを正とする）。

```bash
cargo run -p tunny-core --example verify_indicators > verify_indicators.json
python check_r2.py verify_indicators.json
```

### 検証ケース

| ラベル | n_approx | n_weights(m) | 内容 |
|---|---|---|---|
| `r2_hand_2d_near_ideal` | 1 | 100 (m=2) | 解が ideal（原点）上 → R2=0 のはず |
| `r2_random_2d_n20` | 20 | 100 (m=2) | 乱数 |
| `r2_random_3d_n15` | 15 | 91 (m=3) | 乱数（単体格子の分割数 h=12 で C(14,2)=91 点） |

## 実装読解での確認点

- `r2_indicator(approx, weights)` は各重み `w` について `min_{a∈approx} max_j w_j * a_j` を取り、
  全重みの平均を返す。ideal は `[0,1]` 空間の原点（`0`）固定であり、`a_j` は既に
  `[0,1]` へスケール済みの前提（呼び出し元 `compute_indicator_histories` で ideal/nadir 正規化後の
  値を渡す）。
- 重みベクトル生成 `simplex_lattice_weights(m)`: 個数 `C(h+m-1, m-1)` が 100 以下になる最大の
  分割数 `h` を線形探索し、Das-Dennis 単体格子点を再帰生成、各成分を `k/h` として
  ゼロ重みを避けるため下限 `1e-6` でクランプしてから合計が1になるよう再正規化する。
  m=2 では `h=99` → 100点、m=3 では `h=12` → `C(14,2)=91` 点になる
  （このハーネスの出力でも `r2_random_3d_n15` の `n_weights=91` として確認できた）。
- 下限クランプ `EPS=1e-6` の影響: 境界重み（例えば `w=(1,0)` → `(1-ε, ε)` 相当）でも
  正規化後は実質的にほぼ同じ値になるため、参照実装でも同じ重みをそのまま使う限り
  この下限処理自体の妥当性は問題にならない（Rust と Python が同じ重みで計算しているため）。
- 単位テスト `r2_zero_at_ideal` / `r2_decreases_as_set_approaches_ideal` /
  `simplex_lattice_sums_to_one` は既にこの数式・単体格子の基本性質を担保しており、
  今回の検証はこれを乱数データと外部の独立実装で補強する位置づけ。

## 検証に使った Python コード

```python
"""Rust (tunny-core) の r2_indicator を numpy の標準定義と突き合わせる。

pymoo には R2 indicator の実装がないため、Hansen & Jaszkiewicz (1998) の
重み付き Tchebycheff スカラー化による標準定義で参照実装とする:

    R2(A, W, z*) = (1/|W|) * sum_{w in W} min_{a in A} max_j w_j * (a_j - z*_j)

ideal z* は [0,1] 空間の原点（Rust 実装と同じ）。重みベクトルは Rust 側
(simplex_lattice_weights を複製したもの) が生成したものをそのまま使う
（重み生成アルゴリズム自体は pymoo に対応物がないため、Rust 実装読解に基づき
このハーネスで複製したものと同一である前提）。
"""

import json
import sys

import numpy as np

path = sys.argv[1]
with open(path) as f:
    data = json.load(f)

TOL = 1e-9


def r2_reference(approx, weights):
    if len(weights) == 0:
        return 0.0
    if len(approx) == 0:
        return float("inf")
    total = 0.0
    for w in weights:
        best = min(max(wj * aj for wj, aj in zip(w, a)) for a in approx)
        total += best
    return total / len(weights)


all_ok = True
for case in data["r2_cases"]:
    label = case["label"]
    approx = case["approx"]
    weights = case["weights"]
    rust_r2 = case["r2"]

    ref_r2 = r2_reference(approx, weights)
    diff = abs(rust_r2 - ref_r2)
    ok = diff < TOL
    all_ok &= ok
    status = "OK" if ok else "MISMATCH"
    # 重みが単体格子上に乗っている（各成分の和が1）ことも確認する。
    sums = [abs(sum(w) - 1.0) for w in weights]
    weight_ok = max(sums) < 1e-9 if sums else True
    print(
        f"{label}: n_approx={len(approx)} n_weights={len(weights)}  "
        f"rust={rust_r2:.10f} ref={ref_r2:.10f} diff={diff:.2e}  "
        f"weight_sum_ok={weight_ok}  {status}"
    )

print()
print("PASS: 全ケースで Rust と numpy 参照実装が一致" if all_ok else "FAIL: 不一致あり")
sys.exit(0 if all_ok else 1)
```

## 実行結果

```text
r2_hand_2d_near_ideal: n_approx=1 n_weights=100  rust=0.0000000000 ref=0.0000000000 diff=0.00e+00  weight_sum_ok=True  OK
r2_random_2d_n20: n_approx=20 n_weights=100  rust=0.0688104647 ref=0.0688104647 diff=0.00e+00  weight_sum_ok=True  OK
r2_random_3d_n15: n_approx=15 n_weights=91  rust=0.0538407894 ref=0.0538407894 diff=0.00e+00  weight_sum_ok=True  OK

PASS: 全ケースで Rust と numpy 参照実装が一致
```
