# CMA-ES（Covariance Matrix Adaptation Evolution Strategy）

## 概要

CMA-ES（共分散行列適応進化戦略）は、単一目的・連続最適化のための微分不要手法。多変量正規分布 $\mathcal{N}(m, \sigma^2 C)$ から候補解をサンプリングし、評価結果に基づいて分布の平均 $m$・ステップサイズ $\sigma$・共分散行列 $C$ を世代ごとに適応させる。共分散行列が問題の局所的な曲率（変数間の相関・スケール差）を学習するため、条件数の悪い（軸に対して歪んだ）目的関数にも強い。

Tunny Dashboard の実装は Hansen のチュートリアル（*The CMA Evolution Strategy: A Tutorial*）に準拠した標準形。

| 手法 | 微分情報 | 探索の単位 | 条件数への強さ |
| --- | --- | --- | --- |
| L-BFGS | 数値勾配を使用 | 単一点（マルチスタート） | 曲率（ヘッセ近似）を利用 |
| NSGA-II | 不要 | 集団（$n$ 個体、遺伝オペレータ） | 軸ごとの摂動のみ（相関を学習しない） |
| **CMA-ES** | **不要** | **単一の適応分布**（$\lambda$ サンプル/世代） | **共分散行列が相関・スケールを学習** |

---

## サンプリングと固有値分解

各世代で共分散行列 $C$ を固有値分解する:

$$
C = B D^2 B^\top
$$

$B$ は固有ベクトルからなる直交行列、$D = \mathrm{diag}(d_1, \dots, d_n)$ は固有値の平方根。候補解は次の手順でサンプリングする:

$$
z \sim \mathcal{N}(0, I), \qquad y = B D z, \qquad x = m + \sigma y
$$

標準正規乱数 $z$ の生成には Box-Muller 法を使用する（詳細は [Box-Muller 法](../statistics/box-muller.md) を参照）。

Tunny の実装では、固有値分解に faer の**対称（self-adjoint）固有値分解**を使う。数値誤差で $C$ が厳密に対称でなくなる場合があるため、分解前に $(C + C^\top)/2$ を取って対称性を保証し、さらに数値誤差で固有値が負になることを防ぐため $\max(\lambda, 10^{-20})$ でクランプしてから平方根を取る。

---

## 戦略パラメータ（Hansen 推奨値）

次元数を $n$ とする。

**集団サイズと重み**:

$$
\lambda = 4 + \lfloor 3 \ln n \rfloor, \qquad \mu = \left\lfloor \frac{\lambda}{2} \right\rfloor
$$

上位 $\mu$ 個体に対数重みを与える（正規化して合計 1）:

$$
w_i \propto \ln\!\left(\frac{\lambda+1}{2}\right) - \ln(i), \qquad i = 1, \dots, \mu
$$

有効サンプルサイズ:

$$
\mu_{\mathrm{eff}} = \frac{1}{\sum_i w_i^2}
$$

**ステップサイズ適応（CSA）のパラメータ**:

$$
c_\sigma = \frac{\mu_{\mathrm{eff}} + 2}{n + \mu_{\mathrm{eff}} + 5}, \qquad
d_\sigma = 1 + 2\max\!\left(0,\ \sqrt{\frac{\mu_{\mathrm{eff}}-1}{n+1}} - 1\right) + c_\sigma
$$

**共分散適応のパラメータ**:

$$
c_c = \frac{4 + \mu_{\mathrm{eff}}/n}{n + 4 + 2\mu_{\mathrm{eff}}/n}, \qquad
c_1 = \frac{2}{(n+1.3)^2 + \mu_{\mathrm{eff}}}, \qquad
c_\mu = \min\!\left(1 - c_1,\ \frac{2(\mu_{\mathrm{eff}} - 2 + 1/\mu_{\mathrm{eff}})}{(n+2)^2 + \mu_{\mathrm{eff}}}\right)
$$

これらは全て Hansen のチュートリアルの推奨式であり、Tunny の実装（`cma_es.rs`）はそのまま採用している。

---

## 平均の更新

$\lambda$ 個の候補を評価し、コストの小さい順に並べて上位 $\mu$ 個の $y_k$（$x_k = m + \sigma y_k$ の $y_k$）を重み付き平均する:

$$
y_w = \sum_{i=1}^{\mu} w_i\, y_{(i)}, \qquad m \leftarrow m + \sigma\, y_w
$$

$y_{(i)}$ はコスト順で $i$ 番目の個体の $y$。

---

## ステップサイズパスと CSA 更新

ステップサイズパス $p_\sigma$ は $C^{-1/2} y_w = B D^{-1} B^\top y_w$ を使って更新する:

$$
p_\sigma \leftarrow (1 - c_\sigma)\, p_\sigma + \sqrt{c_\sigma(2-c_\sigma)\mu_{\mathrm{eff}}}\ \, C^{-1/2} y_w
$$

ステップサイズの更新（CSA, Cumulative Step-size Adaptation）:

$$
\sigma \leftarrow \sigma \cdot \exp\!\left(\frac{c_\sigma}{d_\sigma}\left(\frac{\lVert p_\sigma \rVert}{E\lVert \mathcal{N}(0,I) \rVert} - 1\right)\right)
$$

$E\lVert \mathcal{N}(0,I) \rVert$（標準正規ベクトルのノルムの期待値）は次の近似式を使う:

$$
E\lVert \mathcal{N}(0,I) \rVert \approx \sqrt{n}\left(1 - \frac{1}{4n} + \frac{1}{21n^2}\right)
$$

$\lVert p_\sigma \rVert$ がこの期待値より大きい（＝ステップが累積的に一方向に進んでいる）とステップサイズを増やし、小さい（＝ジグザグして打ち消し合っている）と減らす。

---

## 共分散パスと Rank-one + Rank-$\mu$ 更新

共分散パス $p_c$ の更新には、Heaviside 型の停止判定 $h_\sigma$ を使う:

$$
h_\sigma = \begin{cases} 1 & \dfrac{\lVert p_\sigma \rVert}{\sqrt{1-(1-c_\sigma)^{2(g+1)}}} < \left(1.4 + \dfrac{2}{n+1}\right) E\lVert \mathcal{N}(0,I) \rVert \\[6pt] 0 & \text{otherwise} \end{cases}
$$

（$g$ は現在の世代番号。分母は $p_\sigma$ の初期化バイアスを補正する正規化項。）$h_\sigma$ はステップサイズ $\sigma$ が急激に増大している局面で共分散パスの更新を止め、$C$ の過剰な膨張を防ぐ役割を持つ。

$$
p_c \leftarrow (1 - c_c)\, p_c + h_\sigma \sqrt{c_c(2-c_c)\mu_{\mathrm{eff}}}\ \, y_w
$$

共分散行列 $C$ は rank-one 更新（$p_c$ 方向）と rank-$\mu$ 更新（上位 $\mu$ 個体の分散）を合成して更新する:

$$
C \leftarrow (1 - c_1 - c_\mu)\, C + c_1\left(p_c p_c^\top + \delta(h_\sigma)\, C\right) + c_\mu \sum_{i=1}^{\mu} w_i\, y_{(i)} y_{(i)}^\top
$$

$\delta(h_\sigma) = (1 - h_\sigma)\, c_c (2 - c_c)$ は $h_\sigma = 0$ のときに rank-one 更新で失われる分散を補正する項（Hansen チュートリアルの標準補正）。

Tunny の実装は上三角のみを計算して下三角へ鏡映することで、数値誤差による非対称性を厳密に防いでいる。

---

## Tunny での適用：サロゲートオプティマイザ段階

CMA-ES は**サロゲートオプティマイザ**段階で、フィット済みの応答曲面上での単一目的最適化に使われる。数値勾配を必要とせず、分布全体を適応させながら探索するため、L-BFGS のような局所的な勾配追従よりも初期点への依存が少なく、応答曲面が緩やかに多峰性を持つ場合でも収束しやすい。

- **L-BFGS との使い分け**: L-BFGS は数値勾配とマルチスタートで局所探索を行うため、曲面が滑らかで単峰性が強い場合は少ない評価回数で収束する。CMA-ES は分布全体を適応させるため評価回数はやや多く必要だが、曲面に緩い多峰性やスケール・相関の歪みがある場合に頑健。
- **NSGA-II との使い分け**: NSGA-II は集団内の個体が独立に探索するため強い多峰性や多目的フロント算出に向く一方、CMA-ES は単一の適応分布に情報を集約するため、単峰〜緩い多峰性の連続最適化では収束が速い傾向がある。
- **Random Search との使い分け**: Random Search は常に動作するベースラインだが分布を適応させないため非効率。CMA-ES は同じ評価予算でより高精度な解に到達しやすい。

現在 CMA-ES は多目的モードには対応していない（単一目的専用）。

### 実装パラメータ

| 事項 | 値 |
| --- | --- |
| 初期ステップサイズ $\sigma_0$ | 0.3（$[0,1]^d$ 箱を前提とした標準値） |
| 最大世代数 | 設定 0 のとき $\min(100 + 20n,\ 500)$（$n$ = 次元数） |
| 乱数シード | 42（決定的） |
| 標準正規乱数の生成 | Box-Muller 法（[Box-Muller 法](../statistics/box-muller.md) 参照） |
| 停止条件 | $\sigma$ が非有限、または $\sigma \cdot \max_i d_i < 10^{-9}$ |
| 固有値のクランプ | $\max(\lambda, 10^{-20})$（数値誤差対策） |
| 戻り値 | 評価済み最良点（best-ever、各世代の全サンプル中の最小コスト点） |

初期平均 $m$ には観測ベスト点（正規化座標）を使う。

---

## 参考文献

- Hansen, N. (2016). The CMA Evolution Strategy: A Tutorial. *arXiv:1604.00772*.
- Hansen, N., & Ostermeier, A. (2001). Completely derandomized self-adaptation in evolution strategies. *Evolutionary Computation*, 9(2), 159–195.

## 関連ドキュメント

- [Box-Muller 法](../statistics/box-muller.md) — 標準正規乱数の生成方法。
- [L-BFGS](lbfgs.md) — サロゲートオプティマイザの勾配ベース手法。
- [NSGA-II](nsga2.md) — サロゲートオプティマイザの集団ベース手法。
- [サロゲートオプティマイザ（ウィジェット）](../widgets/surrogate-optimizer.md) — 本手法の利用文脈。
