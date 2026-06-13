# 獲得関数（Acquisition Functions）

獲得関数はベイズ最適化の中核です。サロゲートモデルの予測値と不確実性を組み合わせ、次に評価すべき点（試行するパラメータ設定）を決定します。

Tunny Dashboard では、**サロゲートオプティマイザ** ウィジェットでガウス過程サロゲート（GP-FITC、GP-VFE、または GP-MOE）を学習した後に獲得関数が利用できます。**Suggest next trials** ボタンをクリックすると、推奨パラメータ設定が提案されます。

---

## 前提条件: ガウス過程の事後分散

獲得関数は、サロゲートが事後分散（予測平均 μ(x) に加えて不確実性 σ(x)）を出力できることを必要とします。この条件を満たすのは GP の 3 種類のみです：

| モデル | 獲得関数対応 |
|-------|-------------|
| GP-FITC | あり |
| GP-VFE | あり |
| GP-MOE | あり |
| Ridge | なし |
| LightGBM | なし |

---

## 実装されている獲得関数

すべての計算は**正規化空間**（x ∈ [0, 1]^d、y は z-score 単位）で行われ、結果は元の単位へ変換して表示されます。

### 期待改善量（Expected Improvement, EI）

EI は、新しい点 x が現在の最良観測値 f*（インカンバント）をどれだけ改善するかの期待値を測ります。搾取（μ が f* に近い）と探索（σ が大きい）のバランスを取ります。

最小化問題の場合：

$$
\text{EI}(x) = I \cdot \Phi(z) + \sigma(x) \cdot \phi(z)
$$

ここで：
- I = f\* − μ(x) − ξ  （探索オフセット ξ = 0.01 を含む改善量）
- z = I / σ(x)
- Φ = 標準正規分布の CDF、φ = 標準正規分布の PDF

σ(x) < 10⁻¹²（確定的な領域）の場合は EI = max(I, 0)。

最大化問題では、μ と f* の符号を反転させて同じ式を使います。

**探索オフセット**: ξ = **0.01**（z-score 単位）。ξ を大きくすると探索重視、小さくすると搾取重視になります。

### 下限信頼境界（Lower Confidence Bound, LCB）

LCB は目的関数の下限が最も低い点を選択します（最大化問題では上限が最も高い点）：

$$
\text{LCB}(x) = \mu(x) - \kappa \cdot \sigma(x)
$$

最大化では符号を反転し、上限信頼境界（UCB）として動作します。

**探索重み**: κ = **2.0**。κ を大きくすると探索が促進されます。

---

## バッチ獲得: Constant Liar 戦略

n > 1 の候補を同時に要求する場合、Tunny は **Constant Liar** アルゴリズムを使用します：

1. 現在のサロゲートで獲得関数を最大化 → 候補 c₁ を取得。
2. (c₁, y_lie) を訓練データに追加する。y_lie = 現在の最良観測値（最小化なら最小値、最大化なら最大値）。
3. 拡張データで GP サロゲートを再フィット。
4. 新しいサロゲートで獲得関数を最大化 → 候補 c₂ を取得。
5. n 個の候補が揃うまで繰り返す。

「嘘」の観測値を加えることで、GP は選択済みの候補付近で過信するようになり、次の候補が別の領域を探索するよう促します。再フィットが途中で失敗した場合は、その時点までに収集した候補を返します。

**多様性ガード**: 新しい候補が既存の候補と L2 距離 10⁻⁶ 以内（正規化空間）にある場合、別のランダムスタート点で再最適化します。

---

## エクスポートした JSON を Optuna で使う

**Copy enqueue JSON** ボタンを押すと、`study.enqueue_trial()` が受け付ける形式の JSON 配列がクリップボードへコピーされます：

```json
[
  {"x": 1.5, "y": 2.0},
  {"x": 0.8, "y": 3.1}
]
```

各オブジェクトはパラメータ名を値にマップします。Python での使い方：

```python
import json, optuna

study = optuna.load_study(...)
candidates = json.loads("<クリップボードから貼り付け>")
for params in candidates:
    study.enqueue_trial(params)
```

エンキューされた trial は、Optuna の任意のサンプラー（すべての組み込みサンプラーがキューを参照します）によって次に評価されます。

---

## 制約を考慮した獲得関数

Study に制約列が存在し、**Use constraints** が有効な場合、獲得関数は実行可能性を考慮するように変更されます。

### 実行可能性確率 P_feas(x)

各制約モデルは候補点 x で制約信号を予測します。モデルがガウス過程の場合（正規化空間の事後平均 μᵢ・標準偏差 σᵢ）、実行可能性確率は平滑になります：

$$
P(c_i \le 0 \mid x) = \Phi\!\left(\frac{z_{0,i} - \mu_i(x)}{\sigma_i(x)}\right), \qquad z_{0,i} = \frac{0 - \bar{c}_i}{s_{c_i}}
$$

ここで z₀ は実行可能境界（cᵢ = 0）を制約の z-score 空間で表したものです（c̄ᵢ, s_cᵢ はその平均・標準偏差）。事後分散を持たない決定論的モデル（Ridge、または下記のフォールバックで Ridge になった GP）の場合は、代わりにハード指標を用います：

$$
P(c_i \le 0 \mid x) = \begin{cases} 1 & \tilde{c}_i(x) \le 0 \text{ のとき} \\ 0 & \text{それ以外} \end{cases}
$$

制約が独立であると仮定し、積で全体の実行可能性確率を計算します：

$$
P_\text{feas}(x) = \prod_i P(c_i \le 0 \mid x)
$$

### 制約付き EI（Constrained EI）

$$
\text{EI}_c(x) = \text{EI}(x) \cdot P_\text{feas}(x)
$$

インカンバント f* は**最良の実行可能 trial**（全制約値 ≤ 0）から選択します。実行可能な trial が存在しない場合は全体の最良値を使います（Gardner et al., 2014）。

### 制約付き LCB（Constrained LCB）

$$
\text{LCB}_c(x) = \text{LCB}(x) + \lambda \cdot (1 - P_\text{feas}(x))
$$

λ = **10.0** は実行不可能性ペナルティです。このペナルティにより、実行不可能と予測される領域から離れるよう誘導します。

### 制約付き Constant Liar

バッチ獲得では、各 Constant Liar 反復で制約モデルも再フィットします。制約列の「嘘」値には、直前の候補点での制約予測平均値を使用します：

$$
c_i^\text{lie} = \tilde{c}_i(\mathbf{x}_\text{prev})
$$

これにより、拡張訓練データに制約情報が保持されます。

### 制約モデルについて

制約サロゲートは**目的関数と同じモデル種別**を使用します。これにより、GP 目的では制約境界付近の不確実性を考慮した平滑な実行可能性確率が得られます。完全に線形・ノイズゼロな制約（例: `c = 0.5 − x`）は GP のハイパーパラメータ最適化にとって退化ケースであり（最適 lengthscale が無限大に発散）、学習に失敗することがあります。その場合、当該制約のみ **Ridge 回帰へフォールバック**して上記のハード指標を用い、他の制約は GP の平滑な実行可能性確率を保ちます。事後分散を持たない目的モデル（Ridge・LightGBM）はハード指標を直接使用します。

---

## 参考文献

- Jones, D. R., Schonlau, M., & Welch, W. J. (1998). Efficient global optimization of expensive black-box functions. *Journal of Global Optimization*, 13, 455–492.
- Srinivas, N., Krause, A., Kakade, S. M., & Seeger, M. (2010). Gaussian process optimization in the bandit setting: No regret and experimental design. *ICML*.
- Ginsbourger, D., Le Riche, R., & Carraro, L. (2010). Kriging is well-suited to parallelize optimization. *Computational Intelligence in Expensive Optimization Problems*, 131–162.
- Gardner, J. R., Kusner, M. J., Xu, Z. E., Weinberger, K. Q., & Cunningham, J. P. (2014). Bayesian optimization with inequality constraints. *ICML*.
