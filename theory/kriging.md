# Kriging（ガウス過程回帰）によるサロゲートモデル

## 概要

Kriging（クリギング）は、**ガウス過程（Gaussian Process, GP）** を使った非パラメトリック回帰手法。訓練点の間を確率論的に補間し、予測の不確実性も同時に定量化できる。BoTorch・Spearmint など最新のベイズ最適化フレームワークでも標準的に採用されている手法。

本実装では **ARD（Automatic Relevance Determination）Matérn 5/2 カーネル** を採用し、**L-BFGS** によるハイパーパラメータ最適化を純 Rust で実装している。大規模データへの対応として **Sparse Kriging（FITC 近似）** も実装されており、N=5000 規模を高速に処理できる（[sparse-kriging.md](sparse-kriging.md) 参照）。

---

## ガウス過程の基礎

### 定義

ガウス過程は、任意の有限点集合 $\{x_1, \ldots, x_N\}$ 上の関数値が**多変量正規分布**に従うことを仮定する確率過程:

$$
f(x) \sim \mathcal{GP}(m(x), k(x,x'))
$$

- $m(x)$: 平均関数（実装では $m(x)=0$ を仮定）
- $k(x, x')$: カーネル関数（データ点間の類似度を表す共分散関数）

### 予測（事後分布）

訓練データ $(X, y)$ が与えられたとき、新しい点 $x^*$ に対する事後分布は:

$$
\mu(x^*) = k(x^*,X)K^{-1}y = k(x^*,X)\alpha
$$

$$
\sigma^2(x^*) = k(x^*,x^*) - k(x^*,X)K^{-1}k(X,x^*)
$$

ここで:

- $K$（$N\times N$）: 訓練点間のカーネル行列 $K[i,j] = k(x_i, x_j) + \sigma_n^2\delta_{ij}$
- $\alpha = K^{-1}y$: 重みベクトル

**実装では事後平均のみ**を計算（応答曲面の可視化に不確実性は不要）。

---

## ARD Matérn 5/2 カーネル

### カーネル関数

$$
k(x_1,x_2) = \sigma_f^2\left(1+\sqrt{5}\,r+\frac{5r^2}{3}\right)\exp(-\sqrt{5}\,r)
$$

$$
r^2 = \sum_d \left(\frac{x_{1,d}-x_{2,d}}{l_d}\right)^2
$$

| パラメータ | 意味                                      |
| ---------- | ----------------------------------------- |
| $σ_f$      | シグナル標準偏差（関数の振幅スケール）    |
| $l_d$      | 次元 d の長さスケール（大きいほど滑らか） |
| $σ_n$      | 観測ノイズ標準偏差                        |

**ARD（Automatic Relevance Determination）** とは: 次元ごとに独立な長さスケール $l_d$ を持つことで、重要な次元（小さい $l_d$）と無関係な次元（大きい $l_d$）を自動的に識別する仕組み。

### RBF ではなく Matérn 5/2 を選ぶ理由

| カーネル        | 滑らかさ           | 特徴                                                   |
| --------------- | ------------------ | ------------------------------------------------------ |
| RBF（Gaussian） | C∞（無限微分可能） | データから遠い領域で不確実性を過小評価しやすい         |
| Matérn 5/2      | C²（2 回微分可能） | 工学・ハイパーパラメータ最適化の現実的な滑らかさに合致 |

Optuna の目的関数（NNハイパーパラメータ・工学設計等）は C∞ ではなく C² 程度の滑らかさが現実的。RBF の使用は過度な仮定になりやすい。

### 同一点でのカーネル値

$$
k(x,x)=\sigma_f^2 \quad (r=0)
$$

これはカーネル行列の対角要素（信号分散）に相当する。

---

## Cholesky 分解による安定な線形システム求解

### なぜ Cholesky を使うか

`K^{-1} y` を直接計算するのではなく、`K = L · L^T`（Cholesky 分解）を使って安定に求解する。カーネル行列 K は対称正定値行列なので Cholesky 分解が存在する。

### 数値安定性のためのジッター

$$
K_{ii} \leftarrow K_{ii} + \mathrm{jitter},\qquad \mathrm{jitter}=10^{-6}
$$

浮動小数点誤差による K の非正定値化を防ぐ。

### alpha の計算

$$
\alpha = K^{-1}y = L^{-T}(L^{-1}y)
$$

前進代入（L · v = y）→ 後退代入（L^T · α = v）の 2 ステップ。計算量 O(N²)。

---

## 対数周辺尤度（ハイパーパラメータ最適化の目的関数）

### 対数周辺尤度

ハイパーパラメータ $\theta = \{\log l_d, \log \sigma_f, \log \sigma_n\}$ を、**対数周辺尤度（LML）** を最大化して推定する:

$$
L(\theta) = -\frac{1}{2}y^T\alpha - \sum_i \log L_{ii} - \frac{N}{2}\log(2\pi)
$$

対数周辺尤度は自動的にデータへの過学習を罰するため、適切なハイパーパラメータを選ぶ。

### 解析的勾配

L-BFGS のために解析的勾配を計算する:

$$
\frac{\partial L}{\partial \theta_j} = \frac{1}{2}\,\mathrm{tr}\!\left((\alpha\alpha^T-K^{-1})\frac{\partial K}{\partial \theta_j}\right)
$$

各パラメータに対する $\partial K/\partial \theta_j$:

**長さスケール** $\partial k/\partial \log l_d$:

$$
\frac{\partial k}{\partial \log l_d} = \sigma_f^2\frac{5}{3}\frac{(x_{1,d}-x_{2,d})^2}{l_d^2}(1+\sqrt{5}r)\exp(-\sqrt{5}r)
$$

**シグナル分散** $\partial k/\partial \log \sigma_f$:

$$
\frac{\partial k}{\partial \log \sigma_f} = 2k(x_1,x_2)
$$

**ノイズ分散** $\partial K/\partial \log \sigma_n$:

$$
\frac{\partial K}{\partial \log \sigma_n}=2\sigma_n^2 I,
\qquad
\frac{\partial L}{\partial \log \sigma_n}=\sigma_n^2\,\mathrm{tr}(\alpha\alpha^T-K^{-1})
$$

---

## L-BFGS ハイパーパラメータ最適化

### なぜ L-BFGS か

| 最適化手法 | 必要情報         | 収束速度           | メモリ            |
| ---------- | ---------------- | ------------------ | ----------------- |
| 勾配降下   | 1 次             | 遅い（1000+ 回）   | O(p)              |
| BFGS       | 2 次近似         | 速い（30〜100 回） | O(p²)             |
| **L-BFGS** | 2 次近似（近似） | 速い（30〜100 回） | **O(m·p)**（m=5） |

p = 4（`[log l₁, log l₂, log σ_f, log σ_n]`）の小問題では L-BFGS が最も効率的。

### Two-loop Recursion

L-BFGS の探索方向 $d=-H^{-1}\nabla L$ を、直近 $m$ ステップの差分履歴 $\{s_k, y_k\}$ から効率的に計算する:

$$
s_k = x_{k+1} - x_k
$$

$$
y_k = \nabla L_{k+1} - \nabla L_k
$$

```python
# First loop (backward): q ← ∇L
for i in reversed range(m):
    ρ_i = 1 / (s_i^T y_i)
    α_i = ρ_i · (s_i^T q)
    q   = q − α_i · y_i

# Scale: r ← H_0 · q   (H_0 = γ I, γ = s_{m-1}^T y_{m-1} / ‖y_{m-1}‖²)
r = γ · q

# Second loop (forward)
for i in range(m):
    β_i = ρ_i · (y_i^T r)
    r   = r + (α_i − β_i) · s_i

d = −r
```

### Armijo バックトラッキング線探索

十分減少条件（Armijo 条件）を満たすステップ幅 α を二分探索で決定:

$$
f(x+\alpha d) \le f(x) + c_1\alpha(\nabla f^T d),\qquad c_1=10^{-4}
$$

初期ステップ幅 $\alpha = 1.0$ から始め、条件を満たすまで $\alpha \leftarrow \alpha/2$ を繰り返す（最大 20 回）。

### 収束条件

$$
\lVert \nabla L \rVert_2 < 10^{-5}
$$

または最大 50 イテレーション（release ビルド）。debug ビルドでは 5 イテレーションに短縮してテスト時間を短縮。

---

## データ正規化

GP はハイパーパラメータの初期値 $\log l_s = 0$（長さスケール $=1$）を前提としているため、$x$ と $y$ が $[0,1]$ 程度のスケールでないと最適化が適切に収束しない。Optuna のパラメータ範囲は $[0,1000]$ のような任意のスケールを取り得るため、GP 学習前に正規化を行う。

### X の正規化（[0,1] スケーリング）

$$
\tilde x_d = \frac{x_d-\min_d}{\max(\max_d-\min_d,\varepsilon)}
$$

各次元を独立に最小値 0・最大値 1 にスケーリング。

### Y の正規化（Z スコア）

$$
\tilde y = \frac{y-\bar y}{\max(\sigma_y,\varepsilon)}
$$

目的関数の平均を引いて標準偏差で割る。

### 予測値の逆変換

グリッド予測結果を元のスケールに戻す:

$$
\hat f(x^*) = \tilde f(\tilde x^*)\,\sigma_y + \bar y
$$

---

## GP 学習の全体フロー

1. 入力: $(x_{2d}, y)$
2. X 正規化:

$$
\tilde x_d = \frac{x_d - \min_d}{\mathrm{range}_d}
$$

3. Y 正規化:

$$
\tilde y = \frac{y - \bar y}{\sigma_y}
$$

4. サブサンプリング（$N > 500$）: 無作為に 500 点を選択し、計算量を $O(500^3)$ に制限
5. ハイパーパラメータ初期化:

$$
\theta_0 = [\log l_1=0,\; \log l_2=0,\; \log \sigma_f=0,\; \log \sigma_n=-2]
$$

6. L-BFGS 最適化:

$$
\theta^* = \arg\max_\theta L(\theta)
$$

7. 最終モデル学習:

$$
K = \mathrm{build\_kernel\_matrix}(\tilde x_{\mathrm{sub}},\theta^*),\quad
L = \mathrm{cholesky}(K),\quad
\alpha = K^{-1}\tilde y
$$

8. グリッド予測（50×50）:

$$
\tilde f[i][j] = \sum_n \alpha_n\,k(\tilde x^*, \tilde x_n)
$$

9. 逆変換:

$$
\mathrm{values}[i][j] = \tilde f[i][j]\,\sigma_y + \bar y
$$

---

## 計算量

| 処理                  | 計算量      | N=500 の概算 |
| --------------------- | ----------- | ------------ |
| カーネル行列構築      | O(N²)       | 2.5×10⁵ ops  |
| Cholesky 分解         | O(N³)       | 4.2×10⁷ ops  |
| alpha 計算            | O(N²)       | 2.5×10⁵ ops  |
| 勾配計算（1 回）      | O(N²)       | 2.5×10⁵ ops  |
| グリッド予測（50×50） | O(2500 × N) | 1.25×10⁶ ops |

N > 500 の場合は自動サブサンプリングにより O(500³) ≈ 1.25×10⁸ ops に制限。**目標: 10,000ms 以内**（release ビルド）。

---

## 特性・限界

**強み:**

- **データが少なくても高品質な補間**（N=20 程度でも機能する）
- **滑らかな曲面**（Matérn 5/2 の C² 連続性）
- 不確実性定量化が原理的に可能（本実装では平均のみ）
- ARD により各次元の重要度を自動推定

**弱み:**

- **計算量が O(N³)**（N > 1000 でサブサンプリングが必要）
- ノイズが多いデータでは過学習しやすい
- L-BFGS が局所最適解に収束する場合がある（多点初期化で緩和可能）
- 外挿の信頼性は Random Forest より高いが、データ範囲外では不安定

---

## 使用場面の目安

```
目的関数の形が...

  線形に近い ──────────────────────────────────→ Ridge（最速）
  非線形・不連続 ──────────────────────────────→ Random Forest
  滑らかな非線形で少数サンプル（N ≤ 500） ────→ Kriging（最高品質）
  滑らかな非線形で多数サンプル（N > 5000） ───→ Random Forest（速度優先）
```

- N < 50 の Study では Kriging が最も信頼性の高い補間を提供する
- $R^2 > 0.8$ なら曲面はデータをよく説明できている
- 学習時間が 3 秒を超える場合は N が大きすぎる（1000点サブサンプリングを確認）

---

## 使用場面の更新

```
目的関数の形が...

  線形に近い ──────────────────────────────────→ Ridge（最速）
  非線形・不連続 ──────────────────────────────→ Random Forest
  滑らかな非線形で少数サンプル（N ≤ 500） ────→ Kriging（最高品質）
  滑らかな非線形で多数サンプル（N > 500） ────→ Sparse Kriging（速度と品質のバランス）
  滑らかな非線形で大量サンプル（N > 5000）───→ Random Forest（速度優先）
```

---

## 実装ファイル

- `rust_core/src/kriging.rs` — `GpModel`, `cholesky()`, `matern52_ard()`, `build_kernel_matrix()`, `compute_alpha()`, `log_marginal_likelihood()`, `log_ml_gradient()`, `log_ml_with_gradient()`, `lbfgs_direction()`, `armijo_line_search()`, `optimize_hyperparams()`, `train_gp()`, `predict_mean()`
- `rust_core/src/sparse_kriging.rs` — FITC 近似実装（[sparse-kriging.md](sparse-kriging.md) 参照）
- `rust_core/src/pdp.rs` — `compute_pdp_2d_kriging_raw()`, `compute_pdp_2d_kriging()`, `"kriging"` ディスパッチ
- `rust_core/src/lib.rs` — WASM バインディング（`computePdp2d` の `model_type` 引数）
- `frontend/src/wasm/wasmLoader.ts` — TypeScript ラッパー
- `frontend/src/stores/analysisStore.ts` — キャッシュ・状態管理（同期 WASM 呼び出し）
- `frontend/src/components/charts/SurfacePlot3D.tsx` — UI（モデル選択）
