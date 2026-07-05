# R2 indicator

## 概要

R2 indicator は、多数の重み付きユーティリティ関数（Tchebycheff スカラー化）の下で近似集合がどれだけ ideal 点に近づけるかを平均した収束指標。値が小さいほど、様々な選好方向のもとで近似集合が理想解に近いことを意味する。ハイパーボリュームより計算コストが低く、かつフロントの収束と分布の両方に感応する。

Tunny Dashboard では以下で使用される:

- 多目的指標（MoIndicator::R2）
- 収束指標（Convergence）チャート

---

## 定義

最小化する $m$ 個の目的について、近似集合 $A \subset \mathbb{R}^m$（$[0,1]$ に正規化済み、ideal はこの空間の原点）と重みベクトル集合 $W$ に対し

$$
R2(A; W) = \frac{1}{|W|} \sum_{w \in W} \min_{a \in A} \max_j w_j\, a_j
$$

各重み $w$ について、重み付き Tchebycheff スカラー化 $\max_j w_j a_j$（ideal = 原点からの重み付き最大距離）を近似集合内で最小化し、それを全重みで平均する。$A$ が原点（ideal）に近いほど各項が小さくなり、$R2$ は小さくなる。

### 重みベクトル: Das–Dennis 単体格子

$W$ は Das–Dennis の単体格子（simplex lattice）で生成する。各成分は $k / h$（$\sum_j k_j = h$、$\sum_j w_j = 1$）の形を取り、分割数 $h$ ステップの格子点全体が候補になる。分割数 $h$ は、生成される重みの個数 $\binom{h+m-1}{m-1}$ が 100 以下になる最大値を選ぶ（$m=2$ で $h=99$、$m=3$ で $h \approx 13$）。重みが 0 だとその目的を常に無視してしまうため、下限 $\varepsilon = 10^{-6}$ でクリップした後に再正規化する（$\sum_j w_j = 1$ を保つ）。

### 意味

R2 は、意思決定者がどの目的をどれだけ重視するかが事前に分からない状況を、多数の重みベクトルにわたる期待ギャップとして近似する。単一の重みでは見落とされる偏りも、格子状に張った重み全体で平均することで捉えられる。

---

## Tunny での適用

### 自己参照型の収束分析

真のパレートフロントは通常未知のため、Tunny では全系列（基準 Study と比較 Study）の観測点の**和集合の非支配前面**を参照集合として固定する（IGD+・ε-indicator と共通の設計）。ただし R2 自身の計算は参照集合そのものではなく重みベクトル $W$ と ideal（正規化空間の原点）のみを使う。参照集合は和集合の ideal/nadir（次項）を決めるために使われ、これを全系列で共有することで複数 Study の収束曲線を同一グラフ上で比較できる。

### 正規化空間

各目的を最小化方向に統一（最大化目的は符号反転）した上で、和集合の ideal・nadir を使って各目的を $[0, 1]$ にスケールする（スケール不変性のため）。目的の範囲が退化している次元はスケール 1 とする。詳細な式は [IGD+ — 正規化空間](igd-plus.md#正規化空間) と共通。R2 の ideal はこの正規化空間の原点（$[0,\dots,0]$）として扱う。

### エッジケース

- 重みベクトル集合が空（目的数 0 のとき）: $R2 = 0$
- 近似集合が空: $R2 = +\infty$
- 無効な点（NaN・無限大・次元数不一致）: 直前値を引き継ぐ

---

## 関連指標

- [ハイパーボリューム](hypervolume.md) — 支配体積による品質指標。
- [IGD+](igd-plus.md) — 参照集合への平均距離に基づく収束指標。
- [additive ε-indicator](epsilon-indicator.md) — 最悪ケース型の収束指標。

---

## 出典

- M. P. Hansen, A. Jaszkiewicz, "Evaluating the Quality of Approximations to the Non-dominated Set", Technical Report IMM-REP-1998-7, 1998.（R2 indicator の原典）
- D. Brockhoff, T. Wagner, H. Trautmann, "On the Properties of the R2 Indicator", GECCO 2012.（性質の分析）
- I. Das, J. E. Dennis, "Normal-Boundary Intersection: A New Method for Generating the Pareto Surface in Nonlinear Multicriteria Optimization Problems", SIAM Journal on Optimization, 8(3), 1998.（Das–Dennis 単体格子重みベクトルの生成法）
