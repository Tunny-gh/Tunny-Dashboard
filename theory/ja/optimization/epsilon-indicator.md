# additive ε-indicator

## 概要

単項 additive ε-indicator $I_{\varepsilon+}$ は、近似集合を平行移動して参照集合の全点を弱支配するために必要な最小の移動量を測る収束指標。値が小さいほど（より負に近いほど）近似集合が参照集合に近い、あるいはそれを支配していることを意味する。他の平均型の指標（IGD+ など）とは異なり、**最悪ケース**（参照集合の中で最も到達しにくい点）によって値が決まる。

Tunny Dashboard では以下で使用される:

- 多目的指標（MoIndicator::Epsilon）
- 収束指標（Convergence）チャート

---

## 定義

最小化する $m$ 個の目的について、近似集合 $A \subset \mathbb{R}^m$ と参照集合 $Z \subset \mathbb{R}^m$ に対し

$$
I_{\varepsilon+}(A, Z) = \max_{z \in Z} \min_{a \in A} \max_j (a_j - z_j)
$$

内側の $\max_j (a_j - z_j)$ は、点 $a$ を全次元で $-\varepsilon$ だけ平行移動すれば $z$ を弱支配できる最小の $\varepsilon$（$a$ が $z$ よりどれだけ「劣っているか」の最大次元）。これを近似集合内の全点 $a$ について最小化し（最も近い $a$ を選ぶ）、さらに参照集合内の全点 $z$ について最大化する（最も到達しにくい参照点で決まる）。

### 最悪ケース指標であること

外側が $\max$ であるため、$I_{\varepsilon+}$ は「平均的にどれだけ近いか」ではなく「参照集合のうち最も苦手な点にどれだけ近いか」で決まる。IGD+（[igd-plus.md](igd-plus.md)、平均型）と対照的な性質を持ち、フロントの一部だけが劣っている（穴がある）場合に敏感に反応する。

### 符号

$I_{\varepsilon+}$ は負の値を取りうる。近似集合 $A$ が参照集合 $Z$ の全点を**厳密に**支配する場合、平行移動を「逆方向」（$a$ を良くする方向）にしてもなお支配が保たれるため $\varepsilon < 0$ になる。$A = Z$ なら $I_{\varepsilon+} = 0$。

ただし Tunny の収束チャート（下記）では、参照集合は全系列の観測点の和集合の非支配前面から作られるため、系列の前面が参照集合を厳密に支配することは起こらず、実際には $I_{\varepsilon+} \ge 0$ となる。$0$ への到達は「その系列が参照前面を完全にカバーした」ことを意味する。

---

## Tunny での適用

### 自己参照型の収束分析

真のパレートフロントは通常未知のため、Tunny では全系列（基準 Study と比較 Study）の観測点の**和集合の非支配前面**を参照集合として固定し、各系列の各試行ステップでそこへの収束を測る（IGD+・R2 と共通の設計）。参照集合とスケールを全系列で共有するため、複数 Study の収束曲線を同一グラフ上で比較できる。

### 正規化空間

各目的を最小化方向に統一（最大化目的は符号反転）した上で、和集合の ideal・nadir を使って各目的を $[0, 1]$ にスケールする（スケール不変性のため）。目的の範囲が退化している次元はスケール 1 とする。詳細な式は [IGD+ — 正規化空間](igd-plus.md#正規化空間) と共通。

### エッジケース

- 参照集合が空: $I_{\varepsilon+} = 0$
- 近似集合が空: $I_{\varepsilon+} = +\infty$
- 無効な点（NaN・無限大・次元数不一致）: 直前値を引き継ぐ

---

## 関連指標

- [ハイパーボリューム](hypervolume.md) — 支配体積による品質指標。
- [IGD+](igd-plus.md) — 平均型の収束指標。additive ε-indicator は最悪ケース型。
- [R2 indicator](r2-indicator.md) — 重み付きユーティリティ関数の期待値に基づく収束指標。

---

## 出典

- E. Zitzler, L. Thiele, M. Laumanns, C. M. Fonseca, V. Grunert da Fonseca, "Performance Assessment of Multiobjective Optimizers: An Analysis and Review", IEEE Transactions on Evolutionary Computation, 7(2), 2003.（additive ε-indicator の定義）
