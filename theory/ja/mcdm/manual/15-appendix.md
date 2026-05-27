# 第15章 付録

## 数式一覧

本書で使用した代表的な数式を以下にまとめます。

### 重みの正規化

重み $w_j$ は、合計が1になるように扱います。

$$
\sum_{j=1}^{n} w_j = 1
$$

### パレート支配

最小化問題で個体 $a$ が個体 $b$ を支配する条件は、すべての目的で同等以下であり、少なくとも一つの目的で厳密に小さいことです。

$$
f_j(a) \le f_j(b) \quad \forall j
$$

$$
\exists k \quad f_k(a) < f_k(b)
$$

### Min-Max正規化

ベネフィット型基準:

$$
r_{ij} = \frac{x_{ij} - x_j^{\min}}{x_j^{\max} - x_j^{\min}}
$$

コスト型基準:

$$
r_{ij} = \frac{x_j^{\max} - x_{ij}}{x_j^{\max} - x_j^{\min}}
$$

### ベクトル正規化

$$
r_{ij} = \frac{x_{ij}}{\sqrt{\sum_{i=1}^{m} x_{ij}^2}}
$$

### 加重和スコア

$$
S_i = \sum_{j=1}^{n} w_j r_{ij}
$$

### TOPSISスコア

$$
D_i^+ = \sqrt{\sum_j (v_{ij} - A_j^+)^2}
$$

$$
D_i^- = \sqrt{\sum_j (v_{ij} - A_j^-)^2}
$$

$$
\mathrm{score}_i = \frac{D_i^-}{D_i^+ + D_i^-}
$$

### VIKORの妥協指標

$$
Q_i = v \cdot \frac{S_i - S^*}{S^- - S^*} + (1 - v) \cdot \frac{R_i - R^*}{R^- - R^*}
$$

### PROMETHEEのネットフロー

$$
\Phi^{+}(i) = \frac{1}{m-1} \sum_{b \neq i} \pi(i, b)
$$

$$
\Phi^{-}(i) = \frac{1}{m-1} \sum_{b \neq i} \pi(b, i)
$$

$$
\Phi^{net}(i) = \Phi^{+}(i) - \Phi^{-}(i)
$$

## 用語集

| 用語 | 説明 |
| --- | --- |
| MCDM | Multi-Criteria Decision Making。複数の評価基準に基づく意思決定手法の総称。 |
| 代替案 | 比較対象となる候補。多目的最適化では個体、トライアル、候補解など。 |
| 個体 | 最適化アルゴリズムが生成した一つの候補解。 |
| パレートフロント | 他の個体に支配されない非劣解の集合。 |
| パレート支配 | ある個体が別の個体に対して、すべての目的で同等以上かつ少なくとも一つの目的で優れる関係。 |
| 評価基準 | 代替案を評価する観点。コスト、性能、リスクなど。 |
| 評価行列 | 代替案と評価基準の評価値を並べた行列。 |
| 重み | 評価基準の相対的重要度。 |
| ベネフィット型基準 | 値が大きいほど望ましい評価基準。 |
| コスト型基準 | 値が小さいほど望ましい評価基準。 |
| 正規化 | 単位やスケールの異なる値を比較可能な形に変換する処理。 |
| 正理想解 | 各評価基準で最も望ましい値を集めた仮想的な解。 |
| 負理想解 | 各評価基準で最も望ましくない値を集めた仮想的な解。 |
| 感度分析 | 重みや入力値の変化が結果に与える影響を確認する分析。 |
| アウトランキング | 代替案間の優越関係を比較する考え方。 |

## サンプルデータ

以下は、MCDMの動作確認に使える小規模なサンプルデータです。

| 個体 | コスト | 性能 | リスク | 制約余裕 |
| --- | ---: | ---: | ---: | ---: |
| A | 100 | 70 | 20 | 0.30 |
| B | 120 | 90 | 40 | 0.10 |
| C | 80 | 60 | 30 | 0.50 |
| D | 110 | 85 | 25 | 0.20 |

評価方向は次のとおりです。

| 評価基準 | 評価方向 | 重み例 |
| --- | --- | ---: |
| コスト | 小さいほど良い | 0.30 |
| 性能 | 大きいほど良い | 0.40 |
| リスク | 小さいほど良い | 0.20 |
| 制約余裕 | 大きいほど良い | 0.10 |

このデータでは、Bは性能に優れ、Cはコストに優れ、Aはリスクに優れています。重みを変えることで、上位候補が変わる可能性があります。

## 参考文献

- Saaty, T. L. (1977). A scaling method for priorities in hierarchical structures. *Journal of Mathematical Psychology*, 15(3), 234–281.
- Saaty, T. L. (1980). *The Analytic Hierarchy Process*. McGraw-Hill.
- Hwang, C. L., & Yoon, K. (1981). *Multiple Attribute Decision Making: Methods and Applications*. Springer-Verlag.
- Opricovic, S. (1998). *Multicriteria Optimization of Civil Engineering Systems*. Faculty of Civil Engineering, Belgrade.
- Opricovic, S., & Tzeng, G. H. (2004). Compromise solution by MCDM methods: A comparative analysis of VIKOR and TOPSIS. *European Journal of Operational Research*, 156(2), 445–455.
- Brans, J. P., & Vincke, P. (1985). A preference ranking organisation method: The PROMETHEE method for multiple criteria decision-making. *Management Science*, 31(6), 647–656.

## 手法選定ガイド

MCDM手法を選ぶ際の目安を以下に示します。

| 目的 | 推奨手法 | 理由 |
| --- | --- | --- |
| まず単純に総合スコアを作りたい | WSM | 実装と説明が容易。 |
| 理想解に近い候補を選びたい | TOPSIS | 正理想解・負理想解との距離で説明しやすい。 |
| 妥協解を選びたい | VIKOR | 全体効用と最大後悔をバランスできる。 |
| 重みの根拠を整理したい | AHP | 一対比較と整合性確認ができる。 |
| 選好関係を詳しく見たい | PROMETHEE | 代替案間の勝ち負けを分析できる。 |
| 候補を選別・除外したい | ELECTRE | 優越関係に基づく絞り込みに向く。 |

最初の分析では、WSMやTOPSISのように説明しやすい手法から始め、必要に応じてVIKORやPROMETHEEで追加確認する方法が実務上扱いやすいです。

手法選定では、計算の複雑さよりも、意思決定者が結果を理解し、前提条件を説明できることを重視します。


---

[← 第14章 システム実装における設計ポイント](14-system-design.md) | [目次](TOC.md)
