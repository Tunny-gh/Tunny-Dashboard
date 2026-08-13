# 階層クラスタリング (Ward法) — scipy クロスチェック

- **実施日**: 2026-07-05
- **対象実装**: `rust_core/src/clustering/hierarchical.rs`(`ward_linkage` / `cut_tree`)
- **リファレンス**: scipy 1.18.0 `scipy.cluster.hierarchy.linkage(method='ward')` / `fcluster`(Python 3.12, numpy 2.5.1)
- **結果**: ✅ **一致**(併合距離の最大絶対差 1.8e-15、併合サイズ・ノード ID 対応・k=3 のフラットクラスタ分割すべて完全一致)

## 検証方法

1. Rust 側ハーネス `rust_core/examples/verify_hierarchical.rs` が決定的な擬似乱数で
   3 特徴・3 ブロブ(各 10 点、計 30 点)の合成データを生成する。標準化なしのケースと、
   第 3 列だけ 1000 倍にスケールを歪ませてから `standardize=true` で `ward_linkage` を
   呼ぶケースの両方を出力する。
2. Python 側は同じ入力に scipy の `linkage(method='ward', metric='euclidean')` を適用し、
   マージ距離列(`Z[:,2]`)・クラスタサイズ列(`Z[:,3]`)・ノード ID 対応(`Z[:,0:2]`)・
   `fcluster(t=3)` によるフラットクラスタ分割を突き合わせる。標準化ケースは
   Python 側で `(X - mean) / std(ddof=0)` により手動標準化してから `linkage` に渡す。

```bash
cargo run -p tunny-core --example verify_hierarchical > verify_hierarchical.json
python check_hierarchical.py verify_hierarchical.json
```

## 実装読解での確認点

- 距離の初期値はユークリッド距離の 2 乗(`d²`)。Lance-Williams 更新
  (`((sa+sk)*d(a,k) + (sb+sk)*d(b,k) - sk*d(a,b)) / (sa+sb+sk)`)で Ward 法の
  更新式そのものを実装しており、併合時に `sqrt` を取って distance フィールドに格納する。
  これは scipy の `linkage(method='ward')` が出力する `Z[:,2]` と同じ定義(Ward 距離の
  平方根スケール)。
- 最近傍チェーン(nearest-neighbor chain)アルゴリズムで O(n²) の Ward リンケージを計算後、
  距離昇順に安定ソートしてノード ID を re-mapping している。この re-mapping 規則
  (葉 = 0..n-1、i 番目の併合結果を `n+i` とする)が **scipy の `Z` 行列のノード ID 規則と
  完全に同一**であることを確認した(`{a, b}` 集合が全併合行で scipy の `{idx1, idx2}` と
  一致)。
- `standardize=true` は各列を **母分散(ddof=0)** で標準化する(`var = Σ(x-mean)²/n`)。
  これは `sklearn.preprocessing.StandardScaler` と同じ定義であり、scipy 側でも
  `ddof=0` で手動標準化して比較した。
- `cut_tree` は距離の大きい順に k-1 本の併合を無視して森を作り、葉順で 0 から
  ラベルを振り直す。ラベル番号そのものは scipy の `fcluster` と一致する保証がない
  (両者とも独自の番号付け規則を持つ)ため、ペアワイズ共クラスタ行列
  (`labels[i] == labels[j]` の真偽行列)が完全に一致するかで分割の同一性を判定した。

## 検証に使った Python コード

```python
"""Rust (tunny-core) の Ward 法階層クラスタリングを scipy.cluster.hierarchy と突き合わせる。"""

import json
import sys

import numpy as np
from scipy.cluster.hierarchy import linkage, fcluster
from scipy.spatial.distance import pdist

path = sys.argv[1]
with open(path) as f:
    data = json.load(f)

n = data["n"]


def merges_to_arrays(merges):
    a = np.array([m["a"] for m in merges])
    b = np.array([m["b"] for m in merges])
    dist = np.array([m["distance"] for m in merges])
    size = np.array([m["size"] for m in merges])
    return a, b, dist, size


def compare(label, X, rust_block, standardize):
    if standardize:
        # Rust 側の標準化は母分散 (ddof=0) を使う (sklearn StandardScaler と同じ)。
        mean = X.mean(axis=0)
        std = X.std(axis=0, ddof=0)
        X = (X - mean) / std

    Z = linkage(X, method="ward", metric="euclidean")
    ref_dist = Z[:, 2]
    ref_size = Z[:, 3].astype(int)

    a, b, rust_dist, rust_size = merges_to_arrays(rust_block["merges"])

    max_diff = np.max(np.abs(rust_dist - ref_dist))
    size_match = np.array_equal(rust_size, ref_size)

    # ノード ID 対応 (a, b) は scipy と同じ命名規則 (葉=0..n-1, 併合順に n, n+1, ...)
    # のはずなので、各行の {a, b} 集合と scipy の {idx1, idx2} 集合を比較する。
    pair_match = all(
        {int(a[i]), int(b[i])} == {int(Z[i, 0]), int(Z[i, 1])} for i in range(len(Z))
    )

    # フラットクラスタ (k=3) の分割が一致するか (ラベル番号の付け替えは許容)。
    ref_labels = fcluster(Z, t=3, criterion="maxclust")
    rust_labels = np.array(rust_block["labels_k3"])
    # 同じ分割かどうかはペアワイズ共クラスタ行列の一致で判定する。
    ref_co = ref_labels[:, None] == ref_labels[None, :]
    rust_co = rust_labels[:, None] == rust_labels[None, :]
    partition_match = np.array_equal(ref_co, rust_co)

    print(f"[{label}]")
    print(f"  merge distance max|diff| = {max_diff:.3e}")
    print(f"  merge size match         = {size_match}")
    print(f"  merge (a,b) pair match   = {pair_match}")
    print(f"  k=3 partition match      = {partition_match}")
    assert max_diff < 1e-9, f"{label}: distance mismatch"
    assert size_match, f"{label}: size mismatch"
    assert pair_match, f"{label}: node id pair mismatch"
    assert partition_match, f"{label}: k=3 partition mismatch"


X_raw = np.array(data["data"])
compare("raw (no standardize)", X_raw, data["raw"], standardize=False)

X_scaled = np.array(data["data_scaled"])
compare("standardized", X_scaled, data["standardized_via_rust"], standardize=True)

print("PASS: Rust の Ward 法階層クラスタリングは scipy と一致")
```

## 実行結果

```text
[raw (no standardize)]
  merge distance max|diff| = 3.331e-16
  merge size match         = True
  merge (a,b) pair match   = True
  k=3 partition match      = True
[standardized]
  merge distance max|diff| = 1.776e-15
  merge size match         = True
  merge (a,b) pair match   = True
  k=3 partition match      = True
PASS: Rust の Ward 法階層クラスタリングは scipy と一致
```
