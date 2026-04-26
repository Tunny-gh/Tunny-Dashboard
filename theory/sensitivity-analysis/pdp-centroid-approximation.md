# セントロイド近似による PDP 信頼帯の高速計算

## 概要

Kriging（ガウス過程）を使った 1D PDP では、**各グリッド点での予測分散**をトレーニングデータ全点で平均することが理論的に正確だが、計算コストが $O(G \times N \times N^2) = O(G \cdot N^3)$（$G$ = グリッド数、$N$ = 訓練点数）になる。

**セントロイド近似**は、非着目次元の代表点として**重心（centroid）**を1点だけ選び、分散をその1点で評価することで計算量を $O(G \times N^2)$ に削減する近似法。

| 手法 | 分散計算コスト | N=100, G=30 での呼び出し回数 |
| --- | --- | --- |
| 完全平均（理論値） | $O(G \cdot N \cdot N^2) = O(G N^3)$ | 30,000 回 |
| **セントロイド近似** | $O(G \cdot N^2)$ | 30 回 |

---

## 理論背景

### 完全 PDP 分散の定義

1D PDP の信頼帯には、グリッド点 $v$ における予測標準偏差が必要:

$$
\hat{\sigma}_{\mathrm{PDP}}(v) = \sqrt{ \frac{1}{N} \sum_{i=1}^{N} \sigma^2(v, x_{C,i}) }
$$

ここで:
- $x_{C,i}$ = $i$ 番目のトレーニング点の**非着目次元成分**
- $\sigma^2(v, x_{C,i})$ = GP の予測分散（後述）

GP の予測分散計算自体が $O(N^2)$ なので、これを全 $N$ 点で評価すると $O(N^3)$、グリッド $G$ 点分で $O(G N^3)$ になる。

---

## セントロイド近似

### 近似の定義

非着目次元の代表点として**各次元の算術平均（重心）** $\bar{x}_C$ を使う:

$$
\bar{x}_{C,d} = \frac{1}{N} \sum_{i=1}^{N} x_{C,i,d}, \qquad \forall d \ne j
$$

（$j$ = 着目パラメータの次元インデックス）

これを用いた近似:

$$
\hat{\sigma}_{\mathrm{centroid}}(v) = \sqrt{ \sigma^2(v, \bar{x}_C) }
$$

1回の分散評価（$O(N^2)$）で $G$ グリッド点の信頼帯を計算できる。

### 平均値の代表性

非線形関数 $g(x_C)$ の平均に対して、一般には $g(\bar{x}_C) \ne \mathbb{E}[g(x_C)]$（Jensen の不等式）。しかし、以下の条件が成立すると近似精度が高い:

1. **分散の空間的変化が緩やか**（典型的な GP では長さスケール $l_d$ が大きい）
2. **トレーニング点が概ね対称に分布**（一様サンプリングなど）
3. **非着目次元数が多い**（高次元の平均は集中することが多い、集中不等式）

ベイズ最適化の探索点はおおよそ空間を覆うように配置されるため、条件 1・2 は多くの場合成立する。

---

## GP の予測分散（Kriging）

GP の予測分散の計算式:

$$
\sigma^2(x^*) = k(x^*, x^*) - \mathbf{k}_*^T K^{-1} \mathbf{k}_*
$$

$$
= k(x^*, x^*) - \mathbf{k}_*^T (L^{-T} L^{-1}) \mathbf{k}_*
$$

$$
= k(x^*, x^*) - \mathbf{v}^T \mathbf{v}, \qquad \mathbf{v} = L^{-1} \mathbf{k}_*
$$

ここで:
- $K = L L^T$: Cholesky 分解（$O(N^3)$ の訓練時のみ計算）
- $\mathbf{k}_* = [k(x^*, x_1), \ldots, k(x^*, x_N)]^T$: 予測点とトレーニング点間のカーネルベクトル
- $\mathbf{v} = L^{-1}\mathbf{k}_*$: 前進代入（$O(N^2)$）

1回の分散予測は $O(N^2)$（カーネルベクトル計算 $O(N)$ + 前進代入 $O(N^2)$）。

---

## 実装

### 正規化空間でのセントロイド計算

```rust
// 非着目次元の重心（正規化済み x_norm 上で計算）
let centroid_norm: Vec<f64> = (0..n_dims).map(|d| {
    if d == target_param_idx {
        0.0  // 着目次元はグリッド値で上書きするのでダミー値
    } else {
        x_norm.iter().map(|row| row[d]).sum::<f64>() / n as f64
    }
}).collect();
```

### グリッドループ

```rust
for &v in &grid {
    let v_norm = (v - min_j) / range_j;

    // 平均: 全トレーニング点で周辺化（O(N²) 合計）
    let mean_avg: f64 = x_norm.iter().map(|row| {
        let mut pt = row.clone();
        pt[target_param_idx] = v_norm;
        gaussian_process::predict_mean(&model, &pt)
    }).sum::<f64>() / n as f64;

    // 分散: セントロイドで1回だけ評価（O(N²) × 1）
    let mut centroid_pt = centroid_norm.clone();
    centroid_pt[target_param_idx] = v_norm;
    let var = gaussian_process::predict_variance(&model, &centroid_pt).max(0.0);

    // 95% 信頼帯（元スケールに戻す）
    let pdp = mean_avg * y_std + y_mean;
    let std = var.sqrt() * y_std;
    y_upper.push(pdp + 1.96 * std);
    y_lower.push(pdp - 1.96 * std);
}
```

### サブサンプリングとの組み合わせ

平均計算（完全 MC）は $O(N \cdot N^2) = O(N^3)$ のままだが、**サブサンプリング** $N' = 100$ を組み合わせることで実際の計算は $O(N'^3)$ に収まる:

| 操作 | コスト（N=100, G=30） |
| --- | --- |
| GP 訓練（Cholesky） | $O(N^3) = 10^6$ 演算（1回） |
| 平均計算（全グリッド点） | $O(G \cdot N \cdot N^2) = 3 \times 10^7$（前進代入×3000回） |
| **分散計算（セントロイド近似）** | $O(G \cdot N^2) = 3 \times 10^5$（前進代入×30回） |

セントロイド近似なしの完全計算（分散を全 $N$ 点で評価）と比べ、**分散ステップだけで 100×高速化**。

---

## 精度の考察

### 過小評価傾向

セントロイドはトレーニング点の「内側」にあるため、データが疎な領域（探索空間の周辺部）での分散を過小評価する傾向がある。

$$
\sigma^2(v, \bar{x}_C) \le \frac{1}{N}\sum_i \sigma^2(v, x_{C,i}) \quad \text{（一般には成立しない）}
$$

実際には凸性・凹性に依存するが、GP では遠い点ほど分散が大きくなる傾向があり、重心は各点より近いためわずかに過小評価になりやすい。

### 実用的な正確さ

信頼帯はあくまで**視覚的な参考指標**として使われるため、実用上は問題なし。正確な信頼帯が必要な場合は、Monte Carlo 近似（$N$ をサブサンプリングして分散の平均を取る）に切り替える。

---

## 参考文献

- Friedman, J. H. (2001). Greedy function approximation: A gradient boosting machine. *Annals of Statistics*, 29(5), 1189–1232.（PDP の原典）
- Goldstein, A., et al. (2015). Peeking inside the black box: Visualizing statistical learning with plots of individual conditional expectation. *Journal of Computational and Graphical Statistics*, 24(1), 44–65.
