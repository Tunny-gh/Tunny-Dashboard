# 感度分析 Ridge回帰 — scikit-learn クロスチェック

- **実施日**: 2026-07-05
- **対象実装**: `rust_core/src/sensitivity/ridge.rs`(`compute_ridge` / `compute_ridge_from_standardized_columns` / `compute_xtx_matrix` / `compute_xty_vector` / `compute_r_squared`)
- **リファレンス**: scikit-learn 1.9.0 `sklearn.linear_model.Ridge`(Python 3.12, numpy 2.5.1)
- **結果**: ✅ **一致**(全ケースで係数の最大絶対差 4.5e-15、R² 差 0.0 = 倍精度丸め誤差)

## 対象実装の前処理・定義(読解結果)

`compute_ridge(x_matrix, y, alpha)` の処理は次の通り:

1. `X` の各列を `column_mean_std`(母集団標準偏差、分母は `n`。`sklearn.preprocessing.StandardScaler`
   と同じ規約)で平均0・分散1に標準化する。標準偏差が `f64::EPSILON` 未満(定数列)の
   場合は 0 除算を避けるため標準偏差を `1.0` に固定する(標準化後の値は `x - mean`
   のみになり、定数列なら全行 0 になる)。
2. `y` を平均で中心化する。切片(intercept)は明示的なパラメータとして持たず、
   中心化済み `y_c` に対して回帰する — つまり **切片は正則化しない**設計。
3. 正規方程式 `(X'X + αI) β = X' y_c` を faer の Cholesky 分解(`llt`)で解く。
4. `R²` は同じ `X_std, y_c` に対する in-sample の決定係数
   (`1 - Σ(y_c - ŷ)² / Σy_c²`、`ss_tot < EPSILON` なら `0.0`)。

これは sklearn の `Ridge` の目的関数 `||y - Xβ||² + α||β||²` と同じ正則化係数の
定義であり、`fit_intercept=True` の Ridge も切片自体は正則化しない。ただし sklearn は
X の標準化を自動では行わないため、Python 側で **同じ標準化(ddof=0、定数列は std=1.0
に固定)を手動で行った上で**、中心化済み `y_c` に対して `fit_intercept=False` の
Ridge を fit することで、Rust の計算と曖昧さなく同条件に揃えた。

なお `compute_ridge_result`(`SensitivityMetric` 経由の入口)は NaN/Inf 行フィルタ・
80/20 ホールドアウト分割・ホールドアウトR² を追加で行うが、これは PDP/Sobol/
サロゲート学習で使われる `compute_ridge` / `compute_ridge_from_vecs` という
汎用のin-sampleプリミティブとは別レイヤーである(コード内コメントに明記)。
本検証は sklearn との定義の揃えやすさを優先し、ホールドアウトを経由しない
`compute_ridge`(pub API)を対象にした。

## 検証方法

1. Rust 側ハーネス `rust_core/examples/verify_ridge.rs` が決定的な擬似乱数で
   3 パターンのテストデータを生成し、`compute_ridge` への入力(`x_matrix`, `y`,
   `alpha`)と出力(`beta`, `r_squared`)を JSON で出力する。
   - `linear_plus_irrelevant`: 4パラメータ・ノイズあり。x4 は真の係数 0(無関係変数)。
   - `noise_free_exact`: 2パラメータ・ノイズなしの厳密な線形関数(縮小推定バイアスと
     ノイズの影響を切り分けるため)。
   - `constant_column_guard`: 1列を定数(分散0)にして `column_mean_std` の
     `std < EPSILON` ガードを踏む経路を確認。
2. Python 側は上記の前処理を手動で再現し、`sklearn.linear_model.Ridge(alpha=alpha,
   fit_intercept=False, solver="cholesky")` を標準化済み `X`・中心化済み `y_c` に fit する。

```bash
cargo run -p tunny-core --example verify_ridge > verify_ridge.json
python check_ridge.py verify_ridge.json
```

## 検証に使った Python コード

```python
"""Rust (tunny-core) の compute_ridge を sklearn.linear_model.Ridge と突き合わせる。

Rust 実装 (ridge.rs) は:
  1. X の各列を平均0・分散1 に標準化する（分母は n、population std。std<EPSILON なら 1.0 に固定）。
  2. y を平均で中心化する（切片は明示的に持たず、中心化 y に対して回帰する = 切片を正則化しない）。
  3. (X'X + alpha*I) beta = X'y_c の正規方程式を Cholesky で解く。

sklearn 側で同条件を再現するには:
  - X は自前で標準化する（StandardScaler相当、ddof=0）。sklearn の Ridge は
    normalize や標準化を自動では行わないため、これを揃えないと係数が一致しない。
  - y は中心化した上で Ridge(fit_intercept=False) を使う。fit_intercept=True でも
    Ridge は切片を正則化しないため理論上は同じ係数になるはずだが、中心化 y +
    fit_intercept=False の方が Rust の計算式に直接対応し曖昧さがない。
  - alpha は同じ値をそのまま使う（sklearn の Ridge の alpha は L2 ペナルティ係数で
    Rust の alpha と定義が同じ: ||y - Xb||^2 + alpha*||b||^2）。
"""

import json
import sys

import numpy as np
from sklearn.linear_model import Ridge

path = sys.argv[1]
with open(path) as f:
    cases = json.load(f)

all_ok = True
for case in cases:
    label = case["label"]
    x = np.array(case["x_matrix"], dtype=float)
    y = np.array(case["y"], dtype=float)
    alpha = case["alpha"]
    rust_beta = np.array(case["beta"])
    rust_r2 = case["r_squared"]

    n, p = x.shape

    mean = x.mean(axis=0)
    std = x.std(axis=0, ddof=0)
    std = np.where(std < np.finfo(float).eps, 1.0, std)
    x_std = (x - mean) / std

    y_mean = y.mean()
    y_c = y - y_mean

    model = Ridge(alpha=alpha, fit_intercept=False, solver="cholesky")
    model.fit(x_std, y_c)
    ref_beta = model.coef_

    y_hat = x_std @ ref_beta
    ss_res = np.sum((y_c - y_hat) ** 2)
    ss_tot = np.sum(y_c**2)
    ref_r2 = 0.0 if ss_tot < np.finfo(float).eps else max(0.0, 1.0 - ss_res / ss_tot)

    beta_diff = np.max(np.abs(rust_beta - ref_beta))
    r2_diff = abs(rust_r2 - ref_r2)
    ok = beta_diff < 1e-6 and r2_diff < 1e-6
    all_ok &= ok

    print(f"--- {label} (n={n}, p={p}, alpha={alpha}) ---")
    print(f"  rust beta = {np.array2string(rust_beta, precision=8)}")
    print(f"  sklearn beta = {np.array2string(ref_beta, precision=8)}")
    print(f"  max|beta diff| = {beta_diff:.3e}")
    print(f"  rust R^2 = {rust_r2:.10f}  sklearn R^2 = {ref_r2:.10f}  diff = {r2_diff:.3e}")
    print(f"  {'OK' if ok else 'MISMATCH'}")
    print()

print("PASS: Rust compute_ridge と sklearn Ridge は一致" if all_ok else "FAIL: 不一致あり")
sys.exit(0 if all_ok else 1)
```

## 実行結果

```text
--- linear_plus_irrelevant (n=200, p=4, alpha=1.0) ---
  rust beta = [ 5.55645756 -4.4005994   1.43110768  0.01534207]
  sklearn beta = [ 5.55645756 -4.4005994   1.43110768  0.01534207]
  max|beta diff| = 4.441e-15
  rust R^2 = 0.9979330094  sklearn R^2 = 0.9979330094  diff = 0.000e+00
  OK

--- noise_free_exact (n=150, p=2, alpha=1.0) ---
  rust beta = [4.99962222 1.25801062]
  sklearn beta = [4.99962222 1.25801062]
  max|beta diff| = 1.776e-15
  rust R^2 = 0.9999588719  sklearn R^2 = 0.9999588719  diff = 0.000e+00
  OK

--- constant_column_guard (n=60, p=2, alpha=1.0) ---
  rust beta = [1.32494962 0.        ]
  sklearn beta = [1.32494962 0.        ]
  max|beta diff| = 0.000e+00
  rust R^2 = 0.9977749924  sklearn R^2 = 0.9977749924  diff = 0.000e+00
  OK

PASS: Rust compute_ridge と sklearn Ridge は一致
```

`constant_column_guard` ケースでは、定数列(分散0)に対応する係数が Rust・sklearn
双方でちょうど `0.0` になっており、`column_mean_std` の `std<EPSILON → 1.0` ガード
(標準化後の値が全行 `x - mean = 0` になる)を Python 側で再現した結果と完全一致した。
