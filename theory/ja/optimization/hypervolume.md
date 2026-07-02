# ハイパーボリューム（Hypervolume）

## 概要

ハイパーボリューム（HV）は、多目的最適化におけるパレートフロントの標準的なスカラー品質指標。点集合 $P$ が支配し、参照点 $r$ で上から区切られる目的空間の体積を測る（最小化規約: 点は全次元で $r$ より厳密に小さい領域にのみ寄与する）。支配体積が大きいほどフロントが優れ、広がっていることを意味する。

Tunny Dashboard では以下で使用される:

- パレートランキングの HV 表示（`compute_pareto_ranks`）
- HV 推移（Hypervolume History）ウィジェット
- 多目的指標（MoIndicator::Hypervolume）
- EHVI 獲得関数の内部計算（サロゲートオプティマイザ）

---

## 定義

最小化する $m$ 個の目的について、点集合 $P \subset \mathbb{R}^m$ と参照点 $r \in \mathbb{R}^m$ に対し

$$
\mathrm{HV}(P; r) = \mathrm{Leb}\left( \bigcup_{p \in P} [p, r] \right)
$$

ここで $[p, r] = \{ y : p_k \le y_k \le r_k \ \forall k \}$、$\mathrm{Leb}$ はルベーグ測度（体積）。$r$ より全次元で厳密に小さくない点は寄与 0 となる。

---

## 参照点の自動算出

参照点はフロントの nadir（各目的の最悪値）に観測範囲に比例したマージンを加えて算出する:

$$
r_j = \mathrm{nadir}_j + 0.1 \cdot (\mathrm{nadir}_j - \mathrm{ideal}_j)
$$

マージンを観測範囲に比例させることで、**目的値のスケールに対して不変**になる（目的値を定数倍しても HV の相対比較が変わらない）。範囲が退化している次元（全点が同値）は $|\mathrm{nadir}_j| \cdot 0.1$、それも 0 なら $1.0$ をマージンとする。

HV 推移ウィジェットではユーザーが参照点を明示指定でき、その場合は自動算出をスキップする。

> **注:** EHVI（`theory/ja/optimization/ehvi.md`）は z-score 正規化空間で固定マージン 0.1 を使う別経路であり、この式は適用されない。

---

## アルゴリズム

### m = 2: スイープ法

点を第 1 目的の昇順にソートし、隣接点間の矩形を加算する。$O(n \log n)$。

### m ≥ 3: WFG アルゴリズム

While, Bradstreet, Barone (2012) の WFG アルゴリズムを使用する。HV を点ごとの**排他的寄与**（exclusive hypervolume）の和として計算する:

$$
\mathrm{HV}(\{p_1, \dots, p_n\}; r) = \sum_{i=1}^{n} \mathrm{exclhv}(p_i \mid \{p_{i+1}, \dots, p_n\})
$$

各項は包含 HV（単独 box の体積）から「影」の HV を引いたもの:

$$
\mathrm{exclhv}(p_i \mid Q) = \underbrace{\prod_k (r_k - p_{ik})}_{\mathrm{inclhv}(p_i)} - \mathrm{HV}(\mathrm{nds}(\mathrm{limitset}(p_i, Q)); r)
$$

- **limitset**: 後続点 $q \in Q$ を $p_i$ の box 内へ射影した影 $\max(p_i, q)$（成分ごとの max）の集合
- **nds**: 非支配集合への縮約。ここで大量の影が支配されて削除されることが WFG の枝刈りの中核であり、実用計算量（経験的に $O(n^{m/2})$ 程度）を実現する

再帰の基底は点数 0（HV = 0）、1（inclhv）、2（包除原理の閉形式）。

旧実装（最後の次元でスライスして再帰する方式、概算 $O(n^m)$）はテスト内の検証用リファレンスとして残しており、ランダム前面に対する両実装の一致がプロパティテストで検証されている。

### 実装上の注意

- 入力に支配される点や重複点が含まれていてもよい（計算前に非支配集合へ縮約する）
- 最後の目的の昇順ソートは limitset の枝刈りを効かせるためのヒューリスティックで、正しさはソート順に依存しない
- 最大化目的は符号反転（$-y$）で最小化に変換してから計算する

---

## 出典

- L. While, L. Bradstreet, L. Barone, "A Fast Way of Calculating Exact Hypervolumes", IEEE Transactions on Evolutionary Computation, 16(1), 2012.
- E. Zitzler, L. Thiele, "Multiobjective evolutionary algorithms: a comparative case study and the strength Pareto approach", IEEE TEVC, 3(4), 1999.（HV 指標の原典）
