# クラスタ統計 (重心・標準偏差・有意差判定) — numpy / scipy クロスチェック

- **実施日**: 2026-07-05
- **対象実装**: `rust_core/src/clustering/stats.rs`(`compute_global_stats` / `compute_cluster_centroid_std` / `compute_significant_features`)
- **リファレンス**: numpy 2.5.1(重心・標準偏差の再計算)、scipy 1.18.0 `scipy.stats.ttest_ind`(有意差判定の定義差の参考記録)(Python 3.12)
- **結果**: ✅ 重心・標準偏差は**一致**(最大絶対差 0)。⚠️ 有意差判定は **scipy の t 検定とは別定義**(固定閾値の独自ロジック)であり数値比較の対象外 — 定義差として記録

## 検証方法

1. Rust 側ハーネス `rust_core/examples/verify_cluster_stats.rs` が 3 特徴・3 クラスタ
   (サイズ 15/25/10、計 50 行)の合成データを生成する。第 1・第 2 特徴はクラスタごとに
   平均をずらし、**第 3 特徴だけは全クラスタで平均をずらさず**、有意差判定が
   「シフトあり→True」「シフトなし→False」を正しく弁別できるかを見られるようにした。
2. `compute_global_stats` / `compute_cluster_centroid_std` / `compute_significant_features`
   はいずれも DataFrame を経由しない flat_data 直接受け取りの公開 API のため、
   `tunny_core::dataframe` を介さずにそのまま呼び出せる。
3. Python 側は同じ入力を numpy で再計算し、重心・標準偏差を突き合わせる。
   有意差判定は Rust の定義式をそのまま Python で再現した値と、参考として
   scipy の Welch の t 検定(クラスタ vs 残り)の p 値を並べて出力した(後述の理由により
   両者を同一視した「一致」判定はしていない)。

```bash
cargo run -p tunny-core --example verify_cluster_stats > verify_cluster_stats.json
python check_cluster_stats.py verify_cluster_stats.json
```

## 実装読解での確認点

- `compute_global_stats`: 全体平均、および**不偏標準偏差**(Bessel 補正、分母 `n-1`、
  `n<2` の場合は分母を 1 にクランプ)。
- `compute_cluster_centroid_std`: クラスタごとの平均と、同じく不偏標準偏差(分母
  `nc-1`、`nc<2` の場合は分母 1 にクランプ)。空クラスタ(`size=0`)は重心に
  `global_mean` を代入し、標準偏差 0・有意フラグ全て `false` を返すフォールバックが
  ある(本検証では全クラスタが非空のため未実行)。
- `compute_significant_features`: 特徴 `j` に対し
  `SE = sqrt(var_cluster[j]/n_cluster + var_global[j]/n_total)` を計算し、
  `|centroid[j] - global_mean[j]| / SE > 3.0` で有意フラグを立てる**固定閾値の独自ロジック**。
  これは形式的には Welch の t 統計量に似ているが、比較対象が
  **「クラスタ vs 全体母集団(クラスタ自身を含む)」**であり、scipy の
  `ttest_ind(equal_var=False)` が行う**「クラスタ vs 残り(クラスタを除いた集合)の
  二標本検定」**とは異なる母集団定義である。さらに閾値は p 値ではなく統計量そのものに
  対する固定値 3.0 であり、有意水準(α)を介した p 値判定とも一致しない。
  そのため **scipy の t 検定との数値的な「一致」は定義上ありえず、本検証では
  一致を主張せず両者の値を並べて記録するに留めた**。
- 上記の定義差はあるが、「クラスタ平均が全体平均から明確にずれている特徴は
  True、ずれていない特徴は False になる」という**弁別能力**自体は、今回の合成データ
  (第 3 特徴のみ意図的にシフトなし)で scipy の Welch 検定の p 値の大小傾向と
  完全に整合していることを確認した(下記実行結果を参照)。

## 検証に使った Python コード

```python
"""Rust (tunny-core) のクラスタ統計を numpy / scipy.stats と突き合わせる。

- global_mean / global_std, cluster centroid / std_dev: numpy で再計算し数値一致を見る
  (標準偏差はいずれも不偏 (ddof=1))。
- significant_features: Rust は「|centroid - global_mean| / SE > 3.0」という固定閾値の
  独自ロジックであり、SE = sqrt(var_cluster/n_cluster + var_global/n_total) は
  「クラスタ vs 全体母集団」の比較である。これは scipy.stats.ttest_ind が行う
  「クラスタ vs 残り (二標本)」の t 検定とは異なる定義であるため、一致させる対象がなく
  参考として両方の値を出力するに留める。
"""

import json
import sys

import numpy as np
from scipy import stats

path = sys.argv[1]
with open(path) as f:
    data = json.load(f)

X = np.array(data["data"])
labels = np.array(data["labels"])
n, p = X.shape
k = data["k"]

# ── global stats ──
ref_global_mean = X.mean(axis=0)
ref_global_std = X.std(axis=0, ddof=1)
rust_global_mean = np.array(data["global_mean"])
rust_global_std = np.array(data["global_std"])

gm_diff = np.max(np.abs(rust_global_mean - ref_global_mean))
gs_diff = np.max(np.abs(rust_global_std - ref_global_std))
print("[global stats]")
print(f"  mean max|diff| = {gm_diff:.3e}")
print(f"  std  max|diff| = {gs_diff:.3e}")
assert gm_diff < 1e-9
assert gs_diff < 1e-9

# ── per-cluster centroid / std_dev ──
max_centroid_diff = 0.0
max_std_diff = 0.0
for stat in data["cluster_stats"]:
    cid = stat["cluster_id"]
    mask = labels == cid
    sub = X[mask]
    ref_centroid = sub.mean(axis=0)
    ref_std = sub.std(axis=0, ddof=1)
    max_centroid_diff = max(max_centroid_diff, np.max(np.abs(np.array(stat["centroid"]) - ref_centroid)))
    max_std_diff = max(max_std_diff, np.max(np.abs(np.array(stat["std_dev"]) - ref_std)))

print("[per-cluster centroid / std_dev]")
print(f"  centroid max|diff| = {max_centroid_diff:.3e}")
print(f"  std_dev  max|diff| = {max_std_diff:.3e}")
assert max_centroid_diff < 1e-9
assert max_std_diff < 1e-9

# ── significant_features: 定義差の記録 (一致を主張しない) ──
print("[significant_features: 定義差の参考記録 (scipy t検定とは別ロジック)]")
for stat in data["cluster_stats"]:
    cid = stat["cluster_id"]
    mask = labels == cid
    sub = X[mask]
    rest = X[~mask]
    for j in range(p):
        # Rust: |centroid_j - global_mean_j| / sqrt(var_cluster/n_c + var_global/n) > 3.0
        var_c = stat["std_dev"][j] ** 2
        var_g = rust_global_std[j] ** 2
        se_rust_def = np.sqrt(var_c / stat["size"] + var_g / n)
        stat_rust = abs(stat["centroid"][j] - rust_global_mean[j]) / se_rust_def
        rust_flag = stat_rust > 3.0

        # scipy: クラスタ vs 残り の Welch t検定 (二標本、不等分散)
        if len(rest) > 1 and len(sub) > 1:
            t_res = stats.ttest_ind(sub[:, j], rest[:, j], equal_var=False)
            p_value = t_res.pvalue
        else:
            p_value = float("nan")

        print(
            f"  cluster={cid} feature={j}: rust_stat={stat_rust:.3f} "
            f"rust_flag={rust_flag}({stat['significant_features'][j]}) "
            f"scipy_welch_p={p_value:.3e}"
        )
        assert rust_flag == stat["significant_features"][j]

print(
    "参考記録のみ: Rust の有意判定 (固定閾値3.0, クラスタ vs 全体母集団) は "
    "scipy.stats.ttest_ind (クラスタ vs 残り, Welch) と定義が異なるため数値比較の対象外。"
)
```

## 実行結果

```text
[global stats]
  mean max|diff| = 0.000e+00
  std  max|diff| = 0.000e+00
[per-cluster centroid / std_dev]
  centroid max|diff| = 0.000e+00
  std_dev  max|diff| = 0.000e+00
[significant_features: 定義差の参考記録 (scipy t検定とは別ロジック)]
  cluster=0 feature=0: rust_stat=4.803 rust_flag=True(True) scipy_welch_p=1.334e-06
  cluster=0 feature=1: rust_stat=3.556 rust_flag=True(True) scipy_welch_p=5.658e-04
  cluster=0 feature=2: rust_stat=0.873 rust_flag=False(False) scipy_welch_p=2.823e-01
  cluster=1 feature=0: rust_stat=6.563 rust_flag=True(True) scipy_welch_p=1.371e-24
  cluster=1 feature=1: rust_stat=3.395 rust_flag=True(True) scipy_welch_p=7.203e-04
  cluster=1 feature=2: rust_stat=0.057 rust_flag=False(False) scipy_welch_p=9.200e-01
  cluster=2 feature=0: rust_stat=8.797 rust_flag=True(True) scipy_welch_p=6.413e-15
  cluster=2 feature=1: rust_stat=13.699 rust_flag=True(True) scipy_welch_p=9.792e-20
  cluster=2 feature=2: rust_stat=0.773 rust_flag=False(False) scipy_welch_p=3.553e-01
参考記録のみ: Rust の有意判定 (固定閾値3.0, クラスタ vs 全体母集団) は scipy.stats.ttest_ind (クラスタ vs 残り, Welch) と定義が異なるため数値比較の対象外。
```

意図的にシフトを入れなかった `feature=2` は全クラスタで `rust_flag=False` かつ
scipy の p 値も 0.28〜0.92 と高く(有意でない)、シフトを入れた `feature=0/1` は
全クラスタで `rust_flag=True` かつ scipy の p 値も 1e-4〜1e-24 と極めて低い
(強く有意)。定義は異なるが、弁別の方向性は一貫している。
