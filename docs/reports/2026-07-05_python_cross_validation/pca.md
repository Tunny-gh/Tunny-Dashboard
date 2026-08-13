# PCA (主成分分析) — sklearn / numpy クロスチェック

- **実施日**: 2026-07-05
- **対象実装**: `rust_core/src/clustering/pca.rs`(`run_pca_on_matrix_opts` 経由の `run_pca` / `run_pca_standardized`)
- **リファレンス**: scikit-learn 1.9.0 `sklearn.decomposition.PCA`(中心化のみ)、numpy 2.5.1 `numpy.corrcoef` + `numpy.linalg.eigh`(相関行列 PCA)(Python 3.12)
- **結果**: ✅ **一致**(固有値・loadings・射影のいずれも符号任意性を除いて最大絶対差 1.7e-10 以下)

## 検証方法

1. Rust 側ハーネス `rust_core/examples/verify_pca.rs` が、`tunny_core::dataframe` の
   公開 API(`store_dataframes` / `select_study`)で 3 パラメータ・1 目的関数・
   50 行の合成 study を 1 つ用意する(`run_pca` / `run_pca_standardized` はアクティブ
   DataFrame に依存する公開関数のため)。パラメータ 3 列のうち 1 列は他の 2 列と
   相関を持たせ、もう 1 列は桁違いのスケールにして標準化の効果を確認できるようにした。
2. `run_pca(3, PcaSpace::Param)`(中心化のみ)と `run_pca_standardized(3, PcaSpace::Param)`
   (相関行列 PCA)の両方を実行し、explained_variance・loadings・projections を出力する。
3. Python 側は「中心化のみ」ケースを `sklearn.decomposition.PCA` と直接比較し、
   「標準化」ケースを `numpy.corrcoef` の固有値分解と比較する(下記の実装読解を参照)。
   PCA の固有ベクトルは符号が任意なため、各成分ごとに Rust の loadings とリファレンスの
   内積の符号を見て揃えてから比較した。

```bash
cargo run -p tunny-core --example verify_pca > verify_pca.json
python check_pca.py verify_pca.json
```

## 実装読解での確認点

- `run_pca_on_matrix_opts` は「中心化 → (標準化オプション) → 標本共分散行列
  (分母 `n-1`)を構築 → `faer::self_adjoint_eigen` で固有値分解 → 降順に並べ替え」
  という古典的な共分散行列固有値分解方式の PCA。これは SVD ベースの sklearn PCA と
  数学的に等価(`explained_variance_ = s² / (n-1)`)であり、符号任意性を除いて
  数値一致することが期待できる。
- `standardize=true` は各列を **標本標準偏差(ddof=1, 分母 `n-1`)** で割ってから
  共分散(分母も `n-1`)を取る。両方が同じ `n-1` を使うため、この共分散行列は
  **相関行列そのもの**になる(相関行列は ddof の選び方に依らない)。したがって
  Python 側は `numpy.corrcoef` と直接比較でき、sklearn の `StandardScaler` を介した
  近似ではなく完全な数値一致を期待できる。
- 分散ゼロの列は標準化後に 0 に写像され、成分に寄与しない(ガード: `std > 1e-12`)。
- `faer::self_adjoint_eigen` は固有値を昇順で返す仕様のため、実装側で降順に
  並べ替えるインデックスソートを行っている。この並べ替えロジックが正しく
  「分散が大きい成分から順」になっていることも本検証で確認した。

## 検証に使った Python コード

```python
"""Rust (tunny-core) の PCA を sklearn.decomposition.PCA / numpy.linalg.eigh と突き合わせる。"""

import json
import sys

import numpy as np
from sklearn.decomposition import PCA

path = sys.argv[1]
with open(path) as f:
    data = json.load(f)

X = np.array(data["data"])
n, p = X.shape


def align_signs(rust_loadings, ref_loadings):
    """列ごとに符号を合わせたリファレンス loadings を返す (PCA の符号任意性対応)。"""
    aligned = ref_loadings.copy()
    signs = np.ones(ref_loadings.shape[0])
    for c in range(ref_loadings.shape[0]):
        dot = np.dot(rust_loadings[c], ref_loadings[c])
        if dot < 0:
            aligned[c] = -aligned[c]
            signs[c] = -1
    return aligned, signs


# ── 中心化のみ (raw) ── sklearn PCA (内部で中心化、標準化はしない) と直接比較 ──
pca = PCA(n_components=p)
proj_ref = pca.fit_transform(X)  # 中心化込みの射影
rust = data["raw"]
rust_loadings = np.array(rust["loadings"])
rust_ev = np.array(rust["explained_variance"])
ref_loadings = pca.components_
ref_ev = pca.explained_variance_

aligned_loadings, signs = align_signs(rust_loadings, ref_loadings)
loadings_diff = np.max(np.abs(rust_loadings - aligned_loadings))
ev_diff = np.max(np.abs(rust_ev - ref_ev))

proj_rust = np.array(rust["projections"])
proj_ref_signed = proj_ref * signs[np.newaxis, :]
proj_diff = np.max(np.abs(proj_rust - proj_ref_signed))

print("[raw: centered-only PCA vs sklearn.decomposition.PCA]")
print(f"  explained_variance max|diff| = {ev_diff:.3e}")
print(f"  loadings max|diff| (sign-aligned) = {loadings_diff:.3e}")
print(f"  projections max|diff| (sign-aligned) = {proj_diff:.3e}")
assert ev_diff < 1e-9
assert loadings_diff < 1e-9
assert proj_diff < 1e-6

# ── 標準化 (相関行列 PCA) ── Rust は列を標本標準偏差 (ddof=1) で割ってから
# 標本共分散 (ddof=1) を取るので、これは相関行列そのものになる。
# 相関行列は ddof に依らないので numpy.corrcoef と直接比較できる。
corr = np.corrcoef(X.T)
eigvals, eigvecs = np.linalg.eigh(corr)  # 昇順
order = np.argsort(eigvals)[::-1]
ref_ev_std = eigvals[order]
ref_loadings_std = eigvecs[:, order].T  # 行 = 成分

rust_std = data["standardized"]
rust_loadings_std = np.array(rust_std["loadings"])
rust_ev_std = np.array(rust_std["explained_variance"])

aligned_std, signs_std = align_signs(rust_loadings_std, ref_loadings_std)
ev_diff_std = np.max(np.abs(rust_ev_std - ref_ev_std))
loadings_diff_std = np.max(np.abs(rust_loadings_std - aligned_std))

means = X.mean(axis=0)
stds = X.std(axis=0, ddof=1)
X_std = (X - means) / stds
proj_ref_std = X_std @ ref_loadings_std.T
proj_ref_std_signed = proj_ref_std * signs_std[np.newaxis, :]
proj_rust_std = np.array(rust_std["projections"])
proj_diff_std = np.max(np.abs(proj_rust_std - proj_ref_std_signed))

print("[standardized: correlation-matrix PCA vs numpy.corrcoef + eigh]")
print(f"  explained_variance max|diff| = {ev_diff_std:.3e}")
print(f"  loadings max|diff| (sign-aligned) = {loadings_diff_std:.3e}")
print(f"  projections max|diff| (sign-aligned) = {proj_diff_std:.3e}")
assert ev_diff_std < 1e-9
assert loadings_diff_std < 1e-9
assert proj_diff_std < 1e-6

print("PASS: Rust の PCA は sklearn / numpy と一致 (符号任意性を除く)")
```

## 実行結果

```text
[raw: centered-only PCA vs sklearn.decomposition.PCA]
  explained_variance max|diff| = 1.746e-10
  loadings max|diff| (sign-aligned) = 7.246e-13
  projections max|diff| (sign-aligned) = 5.806e-12
[standardized: correlation-matrix PCA vs numpy.corrcoef + eigh]
  explained_variance max|diff| = 1.110e-15
  loadings max|diff| (sign-aligned) = 6.661e-16
  projections max|diff| (sign-aligned) = 1.388e-15
PASS: Rust の PCA は sklearn / numpy と一致 (符号任意性を除く)
```
