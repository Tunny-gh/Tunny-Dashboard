# IGD+（Inverted Generational Distance Plus）

## 概要

IGD+ は、近似集合（パレートフロント）が参照集合にどれだけ収束しているかを測る指標。参照集合 $Z$ の各点から近似集合 $A$ 内の最も近い点までの「修正距離」を平均する。値が小さいほど、近似集合が参照集合に近い（収束している）ことを意味する。

古典的な IGD（Inverted Generational Distance）のユークリッド距離を、支配方向のみに限定した距離 $d^+$ に置き換えた修正版。

Tunny Dashboard では以下で使用される:

- 多目的指標（MoIndicator::IgdPlus）
- 収束指標（Convergence）チャート

---

## 定義

最小化する $m$ 個の目的について、近似集合 $A \subset \mathbb{R}^m$ と参照集合 $Z \subset \mathbb{R}^m$ に対し

$$
\mathrm{IGD}^+(A) = \frac{1}{|Z|} \sum_{z \in Z} \min_{a \in A} d^+(a, z)
$$

$$
d^+(a, z) = \sqrt{\sum_j \max(a_j - z_j, 0)^2}
$$

$d^+$ は、$a$ が $z$ より悪い（大きい）次元のみを二乗和に含める。$a$ が $z$ よりすべての次元で良い（小さい）場合、$d^+(a, z) = 0$ となる。つまり参照点より良い側にある近似点は、その参照点への寄与が 0 になる。

### IGD との違い

古典的な IGD は通常のユークリッド距離 $d(a, z) = \|a - z\|$ を使うため、$A$ が $Z$ を支配していても $A \ne Z$ なら距離は正のままで、指標が悪化しうる。この非整合性のため IGD は**パレート整合（Pareto compliant）ではない**。

IGD+ は $d^+$ を使うことで、$A$ が $Z$ を（弱）支配する限り指標が悪化しない **弱パレート整合（weakly Pareto compliant）** な指標になる（Ishibuchi et al. 2015）。収束指標として使う場合、IGD より IGD+ が理論的に妥当な選択となる。

---

## Tunny での適用

### 自己参照型の収束分析

真のパレートフロントは通常未知のため、Tunny では単一 Study の収束を測る際にも近似が必要になる。本実装では、比較対象となる全系列（基準 Study と、追加されたすべての比較 Study）の観測点の**和集合の非支配前面**を参照集合として固定し、各系列の各試行ステップでそこへの収束を IGD+ で測る。

参照集合とスケール（下記）を全系列で共有するため、複数 Study の収束曲線を同一グラフ上で直接比較できる。

### 正規化空間

各目的を最小化方向に統一（最大化目的は符号反転 $-y$）した上で、和集合の ideal（各目的の最良値）・nadir（最悪値）を使って $[0, 1]$ にスケールする:

$$
\hat{y}_j = \frac{y_j - \mathrm{ideal}_j}{\mathrm{nadir}_j - \mathrm{ideal}_j}
$$

これにより目的値のスケールに依存しない指標になる（スケール不変性）。ある目的の範囲が退化している（全点が同値で $\mathrm{nadir}_j = \mathrm{ideal}_j$）場合、その次元のスケールは 1 とする。

### エッジケース

- 参照集合が空: IGD+ = 0
- 近似集合（その時点までの前面）が空: IGD+ = $+\infty$
- 無効な点（NaN・無限大を含む、または次元数が不一致）: その試行では直前の値を引き継ぐ（収束曲線が途切れないようにするための挙動）

---

## 関連指標

- [ハイパーボリューム](hypervolume.md) — 支配体積による品質指標。参照点のみで計算でき、参照集合は不要。
- [additive ε-indicator](epsilon-indicator.md) — 最悪ケース（max）に基づく収束指標。IGD+ は平均（mean）に基づく。
- [R2 indicator](r2-indicator.md) — 重み付きユーティリティ関数の期待値に基づく収束指標。

---

## 出典

- H. Ishibuchi, H. Masuda, Y. Tanigaki, Y. Nojima, "Modified Distance Calculation in Generational Distance and Inverted Generational Distance", EMO 2015.（IGD+ の提案・弱パレート整合性の証明）
- C. A. Coello Coello, M. R. Sierra, "A Study of the Parallelization of a Coevolutionary Multi-Objective Evolutionary Algorithm", MICAI 2004.（IGD の起源の一つ）
