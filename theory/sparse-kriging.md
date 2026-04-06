# Sparse Kriging（FITC 近似）によるサロゲートモデル

## 概要

Sparse Kriging は、標準的なガウス過程（GP / Kriging）の **O(N³)** という計算コストを **O(N × M²)** に削減する近似手法。**FITC（Fully Independent Training Conditional）** 近似と **M 個の誘導点（Inducing Points）** を使い、N=5000 規模のデータセットでも高速に滑らかな応答曲面を計算できる。

標準 Kriging が N ≤ 500 のサブサンプリングに頼るのに対し、Sparse Kriging は **全 N 点の情報を近似的に活用**するため、大規模データで精度が上回る。

---

## 誘導点（Inducing Points）の概念

### アイデア

N 個の訓練点すべてを使う代わりに、**M 個（M ≪ N）の代表点 Z = {z_1, ..., z_M}** を選んでそこでの関数値を媒介変数とする。

```
u = f(Z) ～ GP(0, K_ZZ)   （誘導変数）
```

訓練点 f(X) と誘導変数 u の結合分布を使って近似推論を行う。

### FITC（Fully Independent Training Conditional）

FITC 近似は、誘導変数 u を条件としたとき各訓練点が**独立**であると仮定する:

```
p(f(X) | u) ≈ Π_i p(f(xᵢ) | u)    （完全独立）
```

これにより対角近似が成立し、行列演算をO(N×M²)に削減できる。

---

## FITC 行列の構成

### カーネル行列

| 行列 | サイズ | 内容 |
| ---- | ------ | ---- |
| K_ZZ | M × M | 誘導点間のカーネル行列 + jitter（1e-6） |
| K_XZ | N × M | 訓練点と誘導点間のカーネル行列 |

### Q 行列（低ランク近似）

```
Q_XX ≈ K_XZ · K_ZZ^{-1} · K_XZ^T   （N×N 行列の低ランク近似）
```

Q_XX の対角要素:

```
Q_diag[i] = K_XZ[i,:] · K_ZZ^{-1} · K_XZ[i,:]^T
```

### FITC ダイアゴナル行列 Λ

FITC 近似では、「真の対角」と「近似の対角」の差をノイズと見なす:

```
Λ = diag(σ_f² - Q_diag) + σ_n² I
```

- `σ_f² - Q_diag[i]`：訓練点 i の分散のうち誘導点で説明されない残差分散
- `σ_n² I`：観測ノイズ
- すべての要素が正になるよう clamp（数値安定性）

---

## Woodbury 恒等式による高速計算

完全 GP では K_full（N×N）の Cholesky 分解（O(N³)）が必要だが、FITC では **Woodbury 恒等式**を利用して M×M 行列の分解のみで済む。

### FITC 対数周辺尤度（LML）

```
(Q + Λ)^{-1} = Λ^{-1} − Λ^{-1} K_XZ · Σ^{-1} · K_XZ^T · Λ^{-1}

ここで Σ = K_ZZ + K_XZ^T · Λ^{-1} · K_XZ   （M×M）
```

計算ステップ:

```
1. K_ZZ を Cholesky 分解（M×M, O(M³)）
2. Λ = diag(σ_f² − Q_diag) + σ_n² I を構築（O(N×M) の K_XZ 必要）
3. Σ = K_ZZ + K_XZ^T Λ^{-1} K_XZ を構築（O(N×M²)）
4. Σ を Cholesky 分解（M×M, O(M³)）
5. Woodbury 式で (Q+Λ)^{-1}y を計算（O(N×M)）
6. LML = −½ y^T (Q+Λ)^{-1} y − ½ log|Q+Λ| − N/2 log(2π)
```

主要コスト: O(N×M²)（M=50, N=5000 → 12.5×10⁶ ops）

---

## K-means による誘導点選択

### なぜ K-means か

誘導点 Z はデータの分布をよく代表するように選ぶ必要がある。K-means クラスタリングの**セントロイド**は各クラスタの重心であり、データを均等にカバーする良い誘導点になる。

### Lloyd's アルゴリズム

```
1. M 個のセントロイドをランダム初期化（シード 42 で再現性確保）
2. 各点を最近傍セントロイドに割り当て
3. セントロイドを割り当て点の平均で更新
4. 収束（セントロイドの移動 < ε）または最大 100 回繰り返す
```

データ: 正規化済み x_flat（列優先フラット配列、サイズ: n_dims × N）

---

## ハイパーパラメータ最適化

### パラメータ

```
θ = [log l₁, log l₂, log σ_f, log σ_n]   （標準 Kriging と同じ構造）
```

### 数値勾配 L-BFGS

FITC-LML の解析勾配は複雑なため、**有限差分による数値勾配**を使う:

```
∂LML/∂θⱼ ≈ (LML(θ + ε eⱼ) - LML(θ - ε eⱼ)) / (2ε)   （ε = 1e-5）
```

パラメータのクランプ: `[-6, 6]`（数値安定性）

### 反復回数の適応制御

N が大きいほど各イテレーションのコストが増大するため、反復数を自動調整:

| N | max_iter |
| - | -------- |
| N ≥ 2000 | 3 |
| N ≥ 500 | 10 |
| N < 500 | 20 |

---

## 事後予測

### 事後重みベクトル w

訓練後、グリッド予測に使う重みを計算:

```
w = K_ZZ^{-1} · K_XZ^T · (Q + Λ)^{-1} · y
```

これは M 次元ベクトル。

### グリッド点 x* での予測

```
μ(x*) = K_{x*,Z} · w = Σ_{j=1}^{M} k(x*, zⱼ) · wⱼ
```

1 点あたり O(M) の計算（M=50）。50×50=2500 グリッド点で 125,000 カーネル評価。

---

## フォールバック戦略

| 状況 | 動作 |
| ---- | ---- |
| N < M=50 | 標準 Kriging にフォールバック |
| Cholesky が失敗（数値不安定） | 標準 Kriging にフォールバック |
| 重みベクトル w に NaN/Inf | 標準 Kriging にフォールバック |

フォールバック先の標準 Kriging にも x/y 正規化が適用される。

---

## データ正規化

標準 Kriging と同じ正規化を適用（詳細は [kriging.md](kriging.md) 参照）:

- X: 各次元を [0,1] にスケーリング（min/max 正規化）
- Y: Z スコア正規化（平均 0、標準偏差 1）
- グリッド予測を逆変換して元スケールに戻す

---

## 計算量比較

| 手法 | 訓練コスト | N=5000, M=50 の概算 |
| ---- | ---------- | ------------------- |
| 標準 GP | O(N³) | 1.25×10¹¹ ops（不可能） |
| Kriging（サブサンプリング 500 点） | O(500³) | 1.25×10⁸ ops |
| **Sparse Kriging** | **O(N×M²)** | **1.25×10⁷ ops** |

Sparse Kriging は全 N 点の情報を使いながら、標準 Kriging のサブサンプリングより 1 桁速い。

---

## 特性

**強み:**
- 滑らかな応答曲面（Matérn 5/2 カーネル）
- 全 N 点の情報を活用（サブサンプリング不要）
- N=5000 規模で < 5,000ms（release ビルド）

**弱み:**
- 誘導点 M=50 による近似誤差（標準 Kriging より若干精度が落ちることがある）
- 数値勾配のため最適化が標準 Kriging より粗い
- M の選択（現在 50 固定）がパフォーマンスと精度のトレードオフ

---

## 実装ファイル

- `rust_core/src/sparse_kriging.rs` — `select_inducing_points_kmeans()`, `build_kzz()`, `build_kxz()`, `build_fitc_matrix()`, `fitc_lml()`, `fitc_predict_weights()`, `optimize_fitc_hyperparams()`
- `rust_core/src/pdp.rs` — `compute_pdp_2d_sparse_kriging_raw()`, `"sparse_kriging"` ディスパッチ
- `rust_core/src/kriging.rs` — `matern52_ard()`, `predict_mean()`（グリッド予測で再利用）
- `rust_core/src/lib.rs` — WASM バインディング（`surrogateModelType = "sparse_kriging"`）
- `frontend/src/components/charts/SurfacePlot3D.tsx` — UI（`{ value: 'sparse_kriging', label: 'Sparse Kriging' }`）
