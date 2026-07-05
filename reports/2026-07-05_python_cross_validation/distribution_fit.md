# 分布あてはめ (Normal / Log-normal / Weibull, MLE) — scipy クロスチェック

- **実施日**: 2026-07-05
- **対象実装**: `rust_core/src/statistics/distribution_fit.rs`(`fit_distribution`)
- **リファレンス**: scipy 1.18.0 `scipy.stats.norm.fit` / `scipy.stats.lognorm.fit` / `scipy.stats.weibull_min.fit`(Python 3.12.11, numpy 2.5.1)
- **結果**: ✅ **一致**(Normal / Log-normal は閉形式 MLE で最大絶対差 8.9e-16(パラメータ)・5.7e-14(対数尤度)= 倍精度丸め誤差。
  Weibull は反復法(二分法によるスコア方程式の解)で、scipy 既定の Nelder-Mead optimizer(収束許容誤差が粗い)との比較では最大絶対差 5.2e-5 だったが、
  scipy 側の収束許容誤差を締めて再フィットすると最大絶対差 3.4e-8 に縮み、同一の MLE 方程式を解いていることを確認した)

## 実装がモーメント法か MLE かの特定

`fit_distribution` を読むと、Normal・Log-normal は標本平均と標本分散(**分母 n**、不偏分散ではない)を使う閉形式の推定式だが、
併せて `log_likelihood` を「その推定値を代入した対数尤度の閉形式」`ll = -n/2 * (ln(2πσ²) + 1)`(Log-normal はヤコビアン項 `- Σ ln x` を追加)として計算しており、
これは正規分布において **モーメント法と MLE が一致する退化ケース**にあたる(母平均・母分散(ddof=0)の MLE 推定量そのもの)。
Weibull は形状パラメータ `k` を MLE のスコア方程式 `Σxᵏ ln x / Σxᵏ − 1/k − mean(ln x) = 0` の根として二分法で解いており、これも明確に MLE である。
したがって 3 分布とも **MLE** として扱い、scipy 側も `*.fit()`(デフォルトで MLE)と比較した。モーメント法固有のリファレンス計算は不要と判断した。

## 検証方法

1. Rust 側ハーネス `rust_core/examples/verify_distribution_fit.rs` が決定的な擬似乱数(Box-Muller・逆変換法)で 4 種類のデータセット
   (正規風 n=100、対数正規風 n=90、ワイブル風 n=80、歪んだ小サンプル n=40)を生成し、
   **入力データと Normal/LogNormal/Weibull 各分布の `params`・`log_likelihood`・`aic`** を JSON で出力する。
2. Python 側は同じ入力を `scipy.stats.norm.fit` / `scipy.stats.lognorm.fit(floc=0)` / `scipy.stats.weibull_min.fit(floc=0)` で再フィットし、
   パラメータと(`scipy.stats.*.logpdf` の総和で計算した)対数尤度を突き合わせる。
   Log-normal は scipy のパラメータ化(`s`=shape, `scale`=exp(mu))から `mu_ln = ln(scale)`, `sigma_ln = s` に変換して比較する。
   Weibull は scipy 既定の Nelder-Mead optimizer(`xtol`/`ftol` 既定 1e-4)に加えて、収束許容誤差を `1e-12` まで締めた optimizer でも再フィットし、両方をレポートする。

```bash
cargo run -p tunny-core --example verify_distribution_fit > verify_distribution_fit.json
python check_distribution_fit.py verify_distribution_fit.json
```

## 実装読解での確認点

- 有限値のみを使用。`n < 3` は `None`。
- Normal: `μ = mean(x)`、`σ² = mean((x-μ)²)`(分母 n)。`σ² <= 0` なら `None`。
- LogNormal: `x <= 0` を含めば `None`。`ln(x)` に対して Normal と同じ閉形式。対数尤度にヤコビアン `-Σln(x)` を加算(変数変換の密度補正)。
- Weibull: `x <= 0` を含めば `None`。スケール不変性を利用して `x / max(x)` に正規化してから形状 `k` を解くことでオーバーフローを回避。
  探索範囲は `[1e-2, 1e3]` で符号変化がなければ `None`。二分法 200 回反復(実質倍精度まで収束)。
- 3 分布とも `aic = 4 - 2*ln L`(全パラメータ数 2 で共通のため、比較上は `ln L` と同義)。
- scipy 側で確認: `scipy.stats.weibull_min.fit` の既定 optimizer(`scipy.optimize.fmin`, Nelder-Mead)は収束判定が粗く(`xtol=ftol=1e-4`)、
  Rust の二分法(実質倍精度)と比べてパラメータに `1e-5`〜`1e-4` オーダーの差が生じる。これは **Rust 実装の誤差ではなく scipy 側デフォルト optimizer の精度限界**であることを、
  optimizer の許容誤差を締めて再フィットすることで確認した(差が `3.4e-8` まで縮小)。

## 検証に使った Python コード

```python
"""Rust (tunny-core) の分布あてはめ (MLE) を scipy.stats.*.fit と突き合わせる。"""

import json
import sys

import numpy as np
import scipy
from scipy import optimize, stats


def _tight_optimizer(func, x0, args, disp):
    # scipy.stats.weibull_min.fit のデフォルト optimizer (Nelder-Mead, xtol/ftol=1e-4)
    # は収束許容誤差が粗く、Rust の二分法 (実質倍精度まで収束) と比べて
    # パラメータに ~1e-5 の差が出る。同じ MLE 方程式を解いていることを確認するため
    # 許容誤差を締めた optimizer で再フィットする。
    return optimize.fmin(
        func, x0, args=args, xtol=1e-12, ftol=1e-12, maxiter=10000, maxfun=10000, disp=disp
    )


path = sys.argv[1]
with open(path) as f:
    data = json.load(f)

print(f"scipy version: {scipy.__version__}")
print()


def to_arr(values):
    return np.array([np.nan if v is None else v for v in values], dtype=float)


max_param_diff = {"normal": 0.0, "lognormal": 0.0, "weibull": 0.0}
max_ll_diff = {"normal": 0.0, "lognormal": 0.0, "weibull": 0.0}

for ds in data:
    label = ds["label"]
    xs = to_arr(ds["values"])
    xs = xs[np.isfinite(xs)]
    n = xs.size
    print(f"--- {label} (n={n}) ---")

    # === Normal: scipy.stats.norm.fit は MLE (loc=mean, scale=population std, ddof=0) ===
    rust = ds["normal"]
    loc, scale = stats.norm.fit(xs)
    ref_ll = np.sum(stats.norm.logpdf(xs, loc=loc, scale=scale))
    p_diff = max(abs(rust["params"][0] - loc), abs(rust["params"][1] - scale))
    ll_diff = abs(rust["log_likelihood"] - ref_ll)
    max_param_diff["normal"] = max(max_param_diff["normal"], p_diff)
    max_ll_diff["normal"] = max(max_ll_diff["normal"], ll_diff)
    print(
        f"  normal:    rust=(mu={rust['params'][0]:.6f}, sigma={rust['params'][1]:.6f}) "
        f"scipy=(mu={loc:.6f}, sigma={scale:.6f}) param_diff={p_diff:.3e} "
        f"ll_diff={ll_diff:.3e} (rust_ll={rust['log_likelihood']:.6f} scipy_ll={ref_ll:.6f})"
    )

    # === LogNormal: scipy.stats.lognorm.fit(data, floc=0) -> shape=s(=sigma_ln), scale=exp(mu_ln) ===
    rust = ds["lognormal"]
    if (xs <= 0).any():
        assert rust is None
        print("  lognormal: 非正値を含むため Rust は None (scipyでも対象外)")
    else:
        s, floc, fscale = stats.lognorm.fit(xs, floc=0)
        mu_ln = np.log(fscale)
        sigma_ln = s
        ref_ll = np.sum(stats.lognorm.logpdf(xs, s, loc=0, scale=fscale))
        p_diff = max(abs(rust["params"][0] - mu_ln), abs(rust["params"][1] - sigma_ln))
        ll_diff = abs(rust["log_likelihood"] - ref_ll)
        max_param_diff["lognormal"] = max(max_param_diff["lognormal"], p_diff)
        max_ll_diff["lognormal"] = max(max_ll_diff["lognormal"], ll_diff)
        print(
            f"  lognormal: rust=(mu_ln={rust['params'][0]:.6f}, sigma_ln={rust['params'][1]:.6f}) "
            f"scipy=(mu_ln={mu_ln:.6f}, sigma_ln={sigma_ln:.6f}) param_diff={p_diff:.3e} "
            f"ll_diff={ll_diff:.3e} (rust_ll={rust['log_likelihood']:.6f} scipy_ll={ref_ll:.6f})"
        )

    # === Weibull: scipy.stats.weibull_min.fit(data, floc=0) -> c=k(形状), scale=lambda(尺度) ===
    rust = ds["weibull"]
    if (xs <= 0).any():
        assert rust is None
        print("  weibull:   非正値を含むため Rust は None (scipyでも対象外)")
    else:
        c_default, _, s_default = stats.weibull_min.fit(xs, floc=0)
        c, floc, fscale = stats.weibull_min.fit(xs, floc=0, optimizer=_tight_optimizer)
        ref_ll = np.sum(stats.weibull_min.logpdf(xs, c, loc=0, scale=fscale))
        p_diff = max(abs(rust["params"][0] - c), abs(rust["params"][1] - fscale))
        p_diff_default = max(abs(rust["params"][0] - c_default), abs(rust["params"][1] - s_default))
        ll_diff = abs(rust["log_likelihood"] - ref_ll)
        max_param_diff["weibull"] = max(max_param_diff["weibull"], p_diff)
        max_ll_diff["weibull"] = max(max_ll_diff["weibull"], ll_diff)
        print(
            f"  weibull:   rust=(k={rust['params'][0]:.6f}, lambda={rust['params'][1]:.6f}) "
            f"scipy_tight=(k={c:.6f}, lambda={fscale:.6f}) param_diff(tight)={p_diff:.3e} "
            f"param_diff(scipy既定optimizer)={p_diff_default:.3e} "
            f"ll_diff={ll_diff:.3e} (rust_ll={rust['log_likelihood']:.6f} scipy_ll={ref_ll:.6f})"
        )
    print()

print("=== まとめ (最大絶対差) ===")
for dist in ["normal", "lognormal", "weibull"]:
    print(f"{dist}: param_max|diff|={max_param_diff[dist]:.3e}  ll_max|diff|={max_ll_diff[dist]:.3e}")

# 反復法 (Weibull) は 1e-6, 閉形式 (Normal/LogNormal) は 1e-9 を許容誤差とする
assert max_param_diff["normal"] < 1e-9
assert max_ll_diff["normal"] < 1e-6
assert max_param_diff["lognormal"] < 1e-9
assert max_ll_diff["lognormal"] < 1e-6
assert max_param_diff["weibull"] < 1e-6
assert max_ll_diff["weibull"] < 1e-4
print()
print(
    "PASS: Normal/LogNormal/Weibull の MLE パラメータ・対数尤度は "
    "scipy.stats.*.fit (Weibull は締めた収束許容誤差の optimizer) と一致"
)
```

## 実行結果

```text
scipy version: 1.18.0

--- normal_like_n100 (n=100) ---
  normal:    rust=(mu=9.806695, sigma=1.907370) scipy=(mu=9.806695, sigma=1.907370) param_diff=0.000e+00 ll_diff=2.842e-14 (rust_ll=-206.466386 scipy_ll=-206.466386)
  lognormal: rust=(mu_ln=2.261379, sigma_ln=0.218485) scipy=(mu_ln=2.261379, sigma_ln=0.218485) param_diff=8.882e-16 ll_diff=2.842e-14 (rust_ll=-215.928053 scipy_ll=-215.928053)
  weibull:   rust=(k=5.794023, lambda=10.573395) scipy_tight=(k=5.794023, lambda=10.573395) param_diff(tight)=1.595e-08 param_diff(scipy既定optimizer)=3.052e-05 ll_diff=5.684e-14 (rust_ll=-206.635408 scipy_ll=-206.635408)

--- lognormal_like_n90 (n=90) ---
  normal:    rust=(mu=2.561514, sigma=2.107904) scipy=(mu=2.561514, sigma=2.107904) param_diff=4.441e-16 ll_diff=2.842e-14 (rust_ll=-194.816925 scipy_ll=-194.816925)
  lognormal: rust=(mu_ln=0.676078, sigma_ln=0.739768) scipy=(mu_ln=0.676078, sigma_ln=0.739768) param_diff=1.110e-16 ll_diff=2.842e-14 (rust_ll=-161.423793 scipy_ll=-161.423793)
  weibull:   rust=(k=1.383044, lambda=2.829631) scipy_tight=(k=1.383044, lambda=2.829631) param_diff(tight)=1.919e-08 param_diff(scipy既定optimizer)=1.701e-05 ll_diff=0.000e+00 (rust_ll=-166.978197 scipy_ll=-166.978197)

--- weibull_like_n80 (n=80) ---
  normal:    rust=(mu=4.490906, sigma=1.939712) scipy=(mu=4.490906, sigma=1.939712) param_diff=8.882e-16 ll_diff=0.000e+00 (rust_ll=-166.518237 scipy_ll=-166.518237)
  lognormal: rust=(mu_ln=1.400224, sigma_ln=0.470139) scipy=(mu_ln=1.400224, sigma_ln=0.470139) param_diff=4.441e-16 ll_diff=5.684e-14 (rust_ll=-165.154892 scipy_ll=-165.154892)
  weibull:   rust=(k=2.484276, lambda=5.072977) scipy_tight=(k=2.484276, lambda=5.072977) param_diff(tight)=2.488e-08 param_diff(scipy既定optimizer)=2.822e-05 ll_diff=8.527e-14 (rust_ll=-163.678733 scipy_ll=-163.678733)

--- skewed_small_n40 (n=40) ---
  normal:    rust=(mu=6.454215, sigma=1.548876) scipy=(mu=6.454215, sigma=1.548876) param_diff=8.882e-16 ll_diff=0.000e+00 (rust_ll=-74.258716 scipy_ll=-74.258716)
  lognormal: rust=(mu_ln=1.830977, sigma_ln=0.270936) scipy=(mu_ln=1.830977, sigma_ln=0.270936) param_diff=0.000e+00 ll_diff=1.421e-14 (rust_ll=-77.761737 scipy_ll=-77.761737)
  weibull:   rust=(k=5.200501, lambda=7.047189) scipy_tight=(k=5.200501, lambda=7.047189) param_diff(tight)=3.398e-08 param_diff(scipy既定optimizer)=5.162e-05 ll_diff=0.000e+00 (rust_ll=-72.594832 scipy_ll=-72.594832)

=== まとめ (最大絶対差) ===
normal: param_max|diff|=8.882e-16  ll_max|diff|=2.842e-14
lognormal: param_max|diff|=8.882e-16  ll_max|diff|=5.684e-14
weibull: param_max|diff|=3.398e-08  ll_max|diff|=8.527e-14

PASS: Normal/LogNormal/Weibull の MLE パラメータ・対数尤度は scipy.stats.*.fit (Weibull は締めた収束許容誤差の optimizer) と一致
```
