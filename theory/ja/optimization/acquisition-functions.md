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

## 参考文献

- Jones, D. R., Schonlau, M., & Welch, W. J. (1998). Efficient global optimization of expensive black-box functions. *Journal of Global Optimization*, 13, 455–492.
- Srinivas, N., Krause, A., Kakade, S. M., & Seeger, M. (2010). Gaussian process optimization in the bandit setting: No regret and experimental design. *ICML*.
- Ginsbourger, D., Le Riche, R., & Carraro, L. (2010). Kriging is well-suited to parallelize optimization. *Computational Intelligence in Expensive Optimization Problems*, 131–162.
