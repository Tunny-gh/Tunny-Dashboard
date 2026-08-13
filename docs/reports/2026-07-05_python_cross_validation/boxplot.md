# ボックスプロット (five-number summary / Tukey フェンス) — numpy・matplotlib クロスチェック

- **実施日**: 2026-07-05
- **対象実装**: `rust_core/src/statistics/boxplot.rs`(`compute_boxplot` / `quantile`)
- **リファレンス**: numpy 2.5.1 `numpy.percentile`(`method="linear"`)、matplotlib 3.11.0 `matplotlib.cbook.boxplot_stats`(Python 3.12.11)
- **結果**: ✅ **一致**(quantile・five-number summary は最大絶対差 7.1e-15 = 倍精度丸め誤差、Tukey フェンス(whisker)・外れ値も全データセットで完全一致)

## 検証方法

四分位数の定義を実装から特定: `quantile()` はソート済み配列に対し `h = (n-1)*q`、`sorted[lo] + (h-lo)*(sorted[hi]-sorted[lo])` で線形補間しており、これは **numpy の `method="linear"`(旧称 "linear" interpolation、statistics 文献では type-7)と同一の定義**。コード内コメントにも明記されている。

1. Rust 側ハーネス `rust_core/examples/verify_boxplot.rs` が決定的な擬似乱数で 6 種類のデータセット
   (一様分布 n=99、正規風+明確な外れ値混入 n=58、離散タイ n=50、NaN/Inf 混入 n=51、単一要素、既知データ 1〜9)を生成し、
   **入力データ・`quantile()` の複数分位点(0, 0.1, 0.25, 0.5, 0.75, 0.9, 1.0)・`compute_boxplot()` の全フィールド**を JSON で出力する。
2. Python 側は同じ入力を `numpy.percentile(method="linear")` で再計算して five-number summary を突き合わせ、
   Tukey フェンス(`whisker_low`/`whisker_high`)と外れ値は `matplotlib.cbook.boxplot_stats(whis=1.5)` の `whislo`/`whishi`/`fliers` と突き合わせる
   (scipy には Tukey フェンス方式の箱ひげ統計を直接返す関数がないため、同じ 1.5×IQR 規約を実装している matplotlib を採用)。

```bash
cargo run -p tunny-core --example verify_boxplot > verify_boxplot.json
python check_boxplot.py verify_boxplot.json
```

## 実装読解での確認点

- 非有限値(NaN/Inf)は計算前に除外。全除外されたら `None`。
- `n == 1` は退化ケースとして全統計量がその値に一致し、外れ値なしとする特別扱い。
- `quantile()` は numpy の `method="linear"`(type-7)と同一の線形補間式。
- `mean` は単純平均(分母 `n`)。
- IQR = `q3 - q1`、Tukey フェンス = `q1 - 1.5*IQR` / `q3 + 1.5*IQR`。
- `whisker_low`/`whisker_high` は「フェンス内にある実データ点の最小値/最大値」(フェンスの値そのものではない) — これは matplotlib の箱ひげ図や R の `boxplot.stats` と同じ一般的な Tukey 方式の慣習。
- フェンス内にデータが一つも無い場合(理論上は起こらないが)は `min`/`max` にフォールバックするディフェンシブコードがある。
- 外れ値はフェンス外の点全てを昇順で返す。

## 検証に使った Python コード

```python
"""Rust (tunny-core) のボックスプロット統計を numpy.percentile / matplotlib と突き合わせる。"""

import json
import sys

import matplotlib
import matplotlib.cbook as cbook
import numpy as np

path = sys.argv[1]
with open(path) as f:
    data = json.load(f)


def to_arr(values):
    return np.array([np.nan if v is None else v for v in values], dtype=float)


print(f"matplotlib version: {matplotlib.__version__}")
print()

# --- 1. quantile() 単体を numpy.percentile(method='linear') と比較 ---
max_q_diff = 0.0
for chk in data["quantile_checks"]:
    label = chk["label"]
    qs = np.array(chk["qs"])
    rust_vals = to_arr(chk["values"])
    finite = to_arr(next(d["values"] for d in data["datasets"] if d["label"] == label))
    finite = np.sort(finite[np.isfinite(finite)])
    if finite.size == 0:
        continue
    ref_vals = np.percentile(finite, qs * 100, method="linear")
    diff = np.nanmax(np.abs(rust_vals - ref_vals))
    max_q_diff = max(max_q_diff, diff)
    print(f"{label}: quantile max|diff| = {diff:.3e}")

print()
print(f"quantile() 全体の最大絶対差: {max_q_diff:.3e}")
assert max_q_diff < 1e-9
print("PASS: quantile() は numpy.percentile(method='linear') (type-7) と完全一致")
print()

# --- 2. compute_boxplot() の five-number summary を numpy と比較 ---
max_summary_diff = 0.0
whisker_mismatches = []
outlier_mismatches = []

for ds in data["datasets"]:
    label = ds["label"]
    finite = to_arr(ds["values"])
    finite = finite[np.isfinite(finite)]
    rust = ds["boxplot"]
    n = finite.size

    if n == 0:
        assert rust is None
        continue

    if n == 1:
        assert rust["mean"] == finite[0]
        assert rust["q1"] == rust["median"] == rust["q3"] == finite[0]
        assert rust["outliers"] == []
        print(f"{label}: n=1 の退化ケース OK")
        continue

    mean = np.mean(finite)
    q1, med, q3 = np.percentile(finite, [25, 50, 75], method="linear")
    mn, mx = finite.min(), finite.max()

    diffs = [
        abs(rust["mean"] - mean),
        abs(rust["min"] - mn),
        abs(rust["q1"] - q1),
        abs(rust["median"] - med),
        abs(rust["q3"] - q3),
        abs(rust["max"] - mx),
    ]
    max_summary_diff = max(max_summary_diff, max(diffs))

    # --- matplotlib.cbook.boxplot_stats で Tukey フェンス/ウィスカー/外れ値を突き合わせる ---
    mstat = cbook.boxplot_stats(finite, whis=1.5)[0]

    whisker_ok = (
        abs(rust["whisker_low"] - mstat["whislo"]) < 1e-9
        and abs(rust["whisker_high"] - mstat["whishi"]) < 1e-9
    )
    if not whisker_ok:
        whisker_mismatches.append(label)

    rust_outliers = sorted(rust["outliers"])
    mpl_outliers = sorted(float(v) for v in mstat["fliers"])
    outliers_ok = len(rust_outliers) == len(mpl_outliers) and all(
        abs(a - b) < 1e-9 for a, b in zip(rust_outliers, mpl_outliers)
    )
    if not outliers_ok:
        outlier_mismatches.append(label)

    print(
        f"{label}: n={n} summary_max|diff|={max(diffs):.3e} "
        f"whisker_match={whisker_ok} (rust=[{rust['whisker_low']:.4f},{rust['whisker_high']:.4f}] "
        f"mpl=[{mstat['whislo']:.4f},{mstat['whishi']:.4f}]) "
        f"outliers_match={outliers_ok} (rust_n_outliers={len(rust_outliers)} mpl_n_outliers={len(mpl_outliers)})"
    )

print()
print(f"five-number summary 全体の最大絶対差: {max_summary_diff:.3e}")
assert max_summary_diff < 1e-9
print("PASS: mean/min/q1/median/q3/max は numpy と完全一致")
print(f"whisker 不一致データセット: {whisker_mismatches}")
print(f"outliers 不一致データセット: {outlier_mismatches}")
assert not whisker_mismatches
assert not outlier_mismatches
print("PASS: Tukey フェンス (whisker_low/high, outliers) は matplotlib.cbook.boxplot_stats(whis=1.5) と完全一致")
```

## 実行結果

```text
matplotlib version: 3.11.0

uniform_n99: quantile max|diff| = 0.000e+00
normal_with_outliers_n58: quantile max|diff| = 0.000e+00
ties_n50: quantile max|diff| = 0.000e+00
with_nonfinite_n51: quantile max|diff| = 0.000e+00
single_n1: quantile max|diff| = 0.000e+00
known_1to9: quantile max|diff| = 0.000e+00

quantile() 全体の最大絶対差: 0.000e+00
PASS: quantile() は numpy.percentile(method='linear') (type-7) と完全一致

uniform_n99: n=99 summary_max|diff|=7.105e-15 whisker_match=True (rust=[0.3924,49.6801] mpl=[0.3924,49.6801]) outliers_match=True (rust_n_outliers=0 mpl_n_outliers=0)
normal_with_outliers_n58: n=58 summary_max|diff|=0.000e+00 whisker_match=True (rust=[11.8889,26.3192] mpl=[11.8889,26.3192]) outliers_match=True (rust_n_outliers=4 mpl_n_outliers=4)
ties_n50: n=50 summary_max|diff|=0.000e+00 whisker_match=True (rust=[0.0000,4.0000] mpl=[0.0000,4.0000]) outliers_match=True (rust_n_outliers=0 mpl_n_outliers=0)
with_nonfinite_n51: n=41 summary_max|diff|=1.554e-15 whisker_match=True (rust=[-14.1898,14.9062] mpl=[-14.1898,14.9062]) outliers_match=True (rust_n_outliers=0 mpl_n_outliers=0)
single_n1: n=1 の退化ケース OK
known_1to9: n=9 summary_max|diff|=0.000e+00 whisker_match=True (rust=[1.0000,9.0000] mpl=[1.0000,9.0000]) outliers_match=True (rust_n_outliers=0 mpl_n_outliers=0)

five-number summary 全体の最大絶対差: 7.105e-15
PASS: mean/min/q1/median/q3/max は numpy と完全一致
whisker 不一致データセット: []
outliers 不一致データセット: []
PASS: Tukey フェンス (whisker_low/high, outliers) は matplotlib.cbook.boxplot_stats(whis=1.5) と完全一致
```
