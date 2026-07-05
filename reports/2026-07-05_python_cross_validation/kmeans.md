# k-means — scikit-learn クロスチェック

- **実施日**: 2026-07-05
- **対象実装**: `rust_core/src/clustering/kmeans.rs`(`run_kmeans` / `estimate_k_elbow`、内部は linfa-clustering 0.8.1 の `KMeans`)
- **リファレンス**: scikit-learn 1.9.0 `sklearn.cluster.KMeans`(Python 3.12, numpy 2.5.1)
- **結果**: ✅ **一致**(4 クラスタの割当は adjusted Rand index = 1.0 で完全一致、inertia は単位変換後に相対差 6.6e-14。ただし `KmeansResult.wcss` の単位が sklearn と異なる点を発見・記録)

## 検証方法

1. Rust 側ハーネス `rust_core/examples/verify_kmeans.rs` が決定的な擬似乱数で
   2 特徴・4 ブロブ(各 40 点、計 160 点、中心間距離 20 に対し各ブロブの広がりは ±1 程度で
   明確に分離)の合成データを生成し、`run_kmeans(k=4, InitStrategy::KMeansPlusPlus)` と
   `estimate_k_elbow(max_k=8)` を実行する。
2. k-means++ はシードを Rust (`rand_xoshiro`) と sklearn (numpy RNG) で揃えられないため
   厳密な数値一致は前提にせず、「クラスタ割当がラベル置換を除いて一致するか
   (adjusted Rand index)」と「inertia (wcss) がほぼ一致するか」を検証方針とした。
   Python 側は `sklearn.cluster.KMeans(n_init=10)` で同じ `n_runs=10` 相当の設定にする。

```bash
cargo run -p tunny-core --example verify_kmeans > verify_kmeans.json
python check_kmeans.py verify_kmeans.json
```

## 実装読解での確認点・発見事項

- `run_kmeans_on_data` は linfa の `KMeans::params_with_rng(k, rng).max_n_iterations(300)
  .tolerance(1e-5).n_runs(10)` を使用。初期化戦略はいずれも k-means++(D² 比例確率
  サンプリング)で、`InitStrategy::KMeansPlusPlus`(データ形状由来のシード)と
  `InitStrategy::Deterministic`(固定シード 42)の違いはシードの決め方のみ。
- **発見**: linfa-clustering 0.8.1 の `KMeans::inertia()` は
  `min_inertia / F::cast(n_samples)`(= 全サンプルの二乗距離の総和を `n` で割った**平均**)
  を返す(`linfa-clustering-0.8.1/src/k_means/algorithm.rs` で確認)。一方 sklearn の
  `inertia_` は**総和**であり、`n` 倍しなければ数値が直接比較できない。
  `KmeansResult.wcss` はこの `model.inertia()` をそのまま代入しているため
  (`rust_core/src/clustering/kmeans.rs`)、**`wcss` という名前だが実体は
  「クラスタ内平均二乗距離」であり、一般的な WCSS(Within-Cluster Sum of Squares、総和)
  ではない**。エルボー法(`estimate_k_elbow`)は同じ定義の値を k 間で比較しているだけ
  なので相対的な形状(単調減少・二階差分による推奨 k)には影響しないが、絶対値を
  他のツールの WCSS と直接比較する用途では注意が必要。
- `iterations` フィールドは常に `max_n_iterations` の設定値 300 を返す(linfa が実際の
  反復回数を公開しないための既知の制約、監査 B2 で確認済み・検証対象外)。
- `estimate_k_elbow_on_data` は k=2..max_k の wcss を計算し、二階差分(前進差分)が
  最大の k を推奨する。合成データ(4 ブロブ)に対し `recommended_k = 4` が正しく
  選ばれることを確認した。

## 検証に使った Python コード

```python
"""Rust (tunny-core, linfa k-means++) を sklearn.cluster.KMeans と突き合わせる。

k-means++ の乱数シードは Rust (rand_xoshiro) と sklearn (numpy RNG) で揃えられない
ため、初期化・収束経路が完全一致することは期待できない。よく分離した合成ブロブ
(4 クラスタ) を使い、「クラスタ割当がラベル置換を除いて一致 (adjusted Rand index
== 1.0)」「inertia (wcss) がほぼ一致」を検証する。
"""

import json
import sys

import numpy as np
from sklearn.cluster import KMeans
from sklearn.metrics import adjusted_rand_score

path = sys.argv[1]
with open(path) as f:
    data = json.load(f)

X = np.array(data["data"])
k = data["k"]
rust_labels = np.array(data["labels"])
rust_wcss = data["wcss"]

# linfa 側は n_runs=10 (最良を採用)。sklearn も n_init=10 で揃える。
model = KMeans(n_clusters=k, n_init=10, random_state=0)
ref_labels = model.fit_predict(X)
ref_wcss = model.inertia_

ari = adjusted_rand_score(rust_labels, ref_labels)
n = X.shape[0]
# linfa の KMeans::inertia() は「全サンプルの二乗距離の総和 / n_samples」(平均) を
# 返す (linfa-clustering 0.8.1 の algorithm.rs: `inertia: min_inertia / F::cast(n_samples)`)。
# sklearn の inertia_ は総和なので、比較前に rust 側を n 倍してスケールを揃える。
rust_wcss_sum = rust_wcss * n
rel_wcss_diff = abs(rust_wcss_sum - ref_wcss) / ref_wcss

print("[k-means: 4 separated blobs]")
print(f"  adjusted Rand index (labels vs sklearn) = {ari:.6f}")
print(f"  rust wcss (raw, = mean sq-dist) = {rust_wcss:.6f}")
print(f"  rust wcss * n (= sum, comparable to sklearn) = {rust_wcss_sum:.6f}")
print(f"  sklearn inertia (sum) = {ref_wcss:.6f}")
print(f"  relative |diff| after unit conversion = {rel_wcss_diff:.3e}")
assert ari == 1.0, "cluster assignment does not match up to label permutation"
assert rel_wcss_diff < 1e-3, "inertia differs too much"

# ── エルボー法: wcss 系列が k に対して単調非増加であること ──
wcss_per_k = data["elbow_wcss_per_k"]  # k=2..8
monotone = all(wcss_per_k[i] >= wcss_per_k[i + 1] - 1e-9 for i in range(len(wcss_per_k) - 1))
print(f"  elbow wcss_per_k (k=2..8) = {[round(v, 3) for v in wcss_per_k]}")
print(f"  monotone non-increasing = {monotone}")
print(f"  recommended_k = {data['elbow_recommended_k']}")
assert monotone

print("PASS: Rust の k-means は sklearn と同じ分割に到達し、inertia もほぼ一致")
```

## 実行結果

```text
[k-means: 4 separated blobs]
  adjusted Rand index (labels vs sklearn) = 1.000000
  rust wcss (raw, = mean sq-dist) = 0.682307
  rust wcss * n (= sum, comparable to sklearn) = 109.169125
  sklearn inertia (sum) = 109.169125
  relative |diff| after unit conversion = 6.626e-14
  elbow wcss_per_k (k=2..8) = [98.641, 49.247, 0.682, 0.585, 0.515, 0.443, 0.402]
  monotone non-increasing = True
  recommended_k = 4
PASS: Rust の k-means は sklearn と同じ分割に到達し、inertia もほぼ一致
```

> **補足**: `elbow_wcss_per_k` の値が k=3→4 で 49.2 → 0.68 と急落しているのは、
> 上記 `KmeansResult.wcss` の単位(平均二乗距離)がそのまま系列になっているため。
> 「総和」に換算し直しても急落の形状自体は変わらず、`recommended_k=4` の判定は
> 正しい(4 ブロブの合成データに対して妥当)。
