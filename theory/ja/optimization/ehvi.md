# 期待ハイパーボリューム改善（EHVI: Expected Hypervolume Improvement）

EHVI は単目的の獲得関数の多目的アナログです。期待改善量（EI）が「新しい点はスカラーのベスト値をどれだけ改善するか」を問うのに対し、EHVI は「新しい点はパレートフロントが支配する**ハイパーボリューム**をどれだけ増やすか」を問います。多目的ベイズ最適化の標準的な獲得関数です。

Tunny Dashboard では、多目的モードで全目的のガウス過程サロゲート（GP-FITC、GP-VFE、または GP-MOE）を学習した後、**サロゲートオプティマイザ** ウィジェットで EHVI が利用できます。**Suggest next trials (EHVI)** ボタンをクリックすると、推奨パラメータ設定が提案されます。

---

## 前提条件: 目的ごとのガウス過程

各目的 $k$ は**独立した**専用の GP サロゲートを持ち、事後平均と事後分散を出力します。EHVI は全目的の事後分散を必要とするため、全目的が GP 系である必要があります：

| モデル | EHVI 対応 |
|-------|-----------|
| GP-FITC | あり |
| GP-VFE | あり |
| GP-MOE | あり |
| Ridge | なし |
| LightGBM | なし |

目的どうしは $x$ を与えたとき**独立**として扱われます（各目的が独自の GP を持ち、目的間の共分散はモデル化しません）。これは qEHVI や多くの実用的な EHVI 実装と同じ仮定です。

---

## ハイパーボリューム

点集合 $P$ の参照点 $r$ に対するハイパーボリュームとは、$P$ が支配し $r$ で上から区切られる目的空間の体積です（最小化規約: 点は全次元で $r$ より厳密に小さい領域にのみ寄与します）。支配体積が大きいほどパレートフロントが優れ、広がっていることを意味するため、ハイパーボリュームは多目的フロントの標準的なスカラー品質指標です。

候補ベクトル $v$ を現在のフロント $P$ に追加したときのハイパーボリューム改善は

$$
\text{HVI}(v) = \max\!\big(0,\; \text{HV}(P \cup \{v\}) - \text{HV}(P)\big).
$$

---

## z-score 最小化フレーム

すべての EHVI 計算は **z-score 化した最小化フレーム** で行います。これにより「常に小さいほど良い」が成り立ち、全目的が比較可能なスケールを共有します。

目的 $k$ について、正規化目的を

$$
g_k(x) = \text{sign}_k \cdot \hat{\mu}^{\text{norm}}_k(x),
\qquad
\text{sign}_k =
\begin{cases}
+1 & \text{目的 } k \text{ が最小化} \\
-1 & \text{目的 } k \text{ が最大化}
\end{cases}
$$

と定義します。$\hat{\mu}^{\text{norm}}_k$ は z-score 単位の GP 事後平均です。符号反転により最大化を最小化へ変換します。事後標準偏差は

$$
s_k(x) = \sqrt{\widehat{\text{Var}}^{\text{norm}}_k(x)}
$$

です（符号は標準偏差に影響しません）。

### 観測フロント $P$

目的ごとの raw 観測値 $y$ を取り、各目的を z-score 最小化フレーム $\text{sign}_k \cdot (y - \bar{y}_k)/\sigma_{y,k}$ へ変換し、最小化規約で非劣解（支配されない点）の集合に縮約します。

### 参照点 $r$

次元ごとに、参照点は観測フロントの **nadir** に小さなマージンを加えた値です：

$$
r_k = \max_{p \in P} g_k(p) + \text{REF\_MARGIN},
\qquad \text{REF\_MARGIN} = 0.1 \;\text{（z-score 単位）}.
$$

マージンにより、すべての観測フロント点が参照ボックスの内部に厳密に収まり、正のハイパーボリュームに寄与します。

---

## 共通乱数を用いたモンテカルロ推定

EHVI は目的が 3 つ以上では閉形式を持たないため、Tunny はモンテカルロで推定します。候補 $x$ について、同時事後の目的ベクトルを $S$ サンプル引き、ハイパーボリューム改善を平均します：

$$
\widehat{\text{EHVI}}(x) = \frac{1}{S} \sum_{s=1}^{S}
\max\!\big(0,\; \text{HV}(P \cup \{v_s\}) - \text{HV}(P)\big),
\qquad
v_s[k] = g_k(x) + s_k(x)\, Z[s][k],
$$

ここで $Z$ は $S \times n_{\text{obj}}$ の標準正規行列（$S = 128$）です。$\text{HV}(P)$ は各反復で固定なので一度だけ計算します。

### 固定サンプル行列を使う理由（共通乱数）

行列 $Z$ は **`suggest_candidates_multi` 呼び出しごとに一度だけ** 固定シード RNG（シード 42）から引き、**すべての $x$ 評価で再利用** します。これは *共通乱数（Common Random Numbers）* のテクニックで、2 つの理由で重要です：

1. **決定性** — 同じデータでの 2 回の実行が同一の提案を返します。
2. **滑らかさ** — ノイズを固定することで $\widehat{\text{EHVI}}(x)$ は $x$ の決定的で滑らかな関数になります。オプティマイザはこれをマルチスタート L-BFGS と数値（中心差分）勾配で最大化します。評価ごとにサンプルを引き直すと勾配にノイズが混入し、ラインサーチが破綻します。

$r$ より全次元で厳密に小さくないサンプル $v_s$ は寄与 0 になります（ハイパーボリューム計算が処理）。これがまさに $\max(0, \cdot)$ の挙動です。

---

## バッチ提案: Constant Liar

$n > 1$ のバッチ候補では、Tunny は **Constant Liar** 戦略を使います。各候補を選ぶごとに：

1. 候補のパラメータと目的ごとの**予測平均**（raw 単位）を「嘘」の観測値として各目的の $(x, y)$ 作業コピーへ追加します。
2. 各目的の GP サロゲートを拡張データで再フィットします。
3. 観測フロント $P$ と参照点 $r$ を再計算します。
4. 次の候補について EHVI を再最適化します。

これによりバッチが 1 点に潰れるのを防ぎます。正規化距離による重複ガードは、新候補が前の候補と一致した場合にランダムな開始点から 1 回だけ再試行します。再フィットが途中で失敗した場合は、それまでに収集した候補を返します。

---

## 参考文献

- M. Emmerich, A. Deutz, J. Klinkenberg, *Hypervolume-based expected improvement*（EHVI）, 2011.
- K. Yang, M. Emmerich, A. Deutz, T. Bäck, *Multi-objective Bayesian global optimization using expected hypervolume improvement gradient*, 2019.
- S. Daulton, M. Balandat, E. Bakshy, *Differentiable Expected Hypervolume Improvement for Parallel Multi-Objective Bayesian Optimization*（qEHVI）, NeurIPS 2020.
