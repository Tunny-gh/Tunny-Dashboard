# SHAP（SHapley Additive exPlanations）によるパラメータ重要度

## 概要

SHAP（SHapley Additive exPlanations）は、ゲーム理論の **Shapley 値** に基づく特徴量重要度の計算手法（Lundberg & Lee, 2017）。
各パラメータが予測値にどれだけ寄与したかを、**すべての特徴量の組み合わせ順序の平均**として厳密に定義する。

ImportanceChart では **TreeSHAP**（Lundberg & Lee, 2018）を使用し、Random Forest の各木で Shapley 値を厳密かつ効率的に計算する。
本実装では TreeSHAP を自前で実装せず、**LightGBM の `predict_contrib`** に委譲している（後述）。
グローバル重要度は各サンプルの $|\phi_j(x)|$ を全サンプル・全木にわたって平均し、合計が 1 になるよう正規化する。

### 他手法との違い

| 比較軸         | SHAP (TreeSHAP)                                  | MDI                                        | RF-ANOVA                                 |
| -------------- | ------------------------------------------------ | ------------------------------------------ | ---------------------------------------- |
| 理論的根拠     | Shapley 公理（効率性・対称性・線形性・ダミー性） | 不純度減少量の集計                         | パーミュテーション後の精度変化           |
| 測定対象       | 予測値への各パラメータの貢献量                   | **学習中**の分岐品質                       | **学習後**のホールドアウト精度           |
| バイアス       | 高カーディナリティバイアスなし                   | 高カーディナリティ特徴量に過大評価傾向あり | 相関特徴量の影響が残る                   |
| 計算コスト     | 木の葉数 × 深さ² に比例（効率的）                | 木構築と同等                               | 木構築 + P 回のパーミュテーション評価    |
| 局所解釈可能性 | ◎（サンプルごとの寄与量も計算可能）              | ✕（グローバルのみ）                        | ✕（グローバルのみ）                      |

---

## 数式

### Shapley 値の定義

パラメータ $j$ の Shapley 値 $\phi_j(x)$ は、すべての部分集合 $S \subseteq F \setminus \{j\}$ に対して：

$$
\phi_j(x) = \sum_{S \subseteq F \setminus \{j\}} \frac{|S|!\,(|F| - |S| - 1)!}{|F|!} \left[f(S \cup \{j\}) - f(S)\right]
$$

ここで：

- $F$：全特徴量の集合
- $f(S)$：特徴量集合 $S$ のみ既知とした場合の期待予測値

$$
f(S) = \mathbb{E}\left[\hat{f}(x) \mid x_S\right]
$$

Shapley 値は以下の公理を満たす唯一の解：
- **効率性**：$\sum_j \phi_j(x) = \hat{f}(x) - \mathbb{E}[\hat{f}]$
- **対称性**：寄与が同等の特徴量には同じ値が割り当てられる
- **線形性**：複数モデルの線形和への重要度は各モデルの重要度の和に等しい
- **ダミー性**：予測に影響しない特徴量の重要度は 0

### グローバル SHAP 重要度

各サンプル $x$ の局所 Shapley 値の絶対値を全サンプル・全木で平均し、合計が 1 になるよう正規化する：

$$
\tilde{\phi}_j = \frac{\displaystyle\frac{1}{N \cdot T} \sum_{b=1}^{T} \sum_{i=1}^{N} |\phi_j(x_i, \text{tree}_b)|}{\displaystyle\sum_{j'} \frac{1}{N \cdot T} \sum_{b=1}^{T} \sum_{i=1}^{N} |\phi_{j'}(x_i, \text{tree}_b)|}
$$

---

## TreeSHAP（LightGBM の `predict_contrib`）

本実装では Shapley 値を自前で計算せず、**LightGBM の `predict_contrib`（C API: `C_API_PREDICT_CONTRIB`）** を利用する。
LightGBM は内部で TreeSHAP（Lundberg et al. 2018）を実装しており、木モデルに対する Shapley 値を厳密かつ多項式時間（単一の木で $O(L \cdot D^2)$、$L$: 葉数、$D$: 深さ）で計算する。

`predict_contrib` はサンプルごとに長さ $P+1$ のベクトルを返す。先頭 $P$ 要素が各特徴量の寄与 $\phi_j(x)$、末尾 1 要素がバイアス項（期待値 $\mathbb{E}[\hat{f}]$）であり、**バイアス項はグローバル重要度の集計から除外する**。

### TreeSHAP の考え方

決定木では、ある特徴量が経路の分岐に「使われる（hot）」か「使われない（cold）」かで予測が変わる。
TreeSHAP は欠損特徴量の期待値 $f(S) = \mathbb{E}[\hat{f}(x) \mid x_S]$ を、各ノードの訓練サンプルの分岐比率（`n_child / n_parent`）で近似することにより、$2^{|F|}$ 個の部分集合を列挙せずに Shapley 値を厳密計算する（path-dependent TreeSHAP）。

---

## アルゴリズムの手順

```
入力: X (N × P), y (N)

1. データ前処理
   ├── NaN/Inf 行を除外
   └── N > 1,000 の場合はランダムサンプリングで 1,000 行に削減

2. 80/20 ホールドアウト分割

3. LightGBM RandomForest の学習（訓練データ）
   └── boosting_type=rf、T=64 本の回帰木を一括学習
       （行・特徴量サブサンプリング bagging_fraction=0.8 / feature_fraction=0.8）

4. SHAP 寄与量の取得 — Importance を出すツリー
   ├── 学習済み booster の predict_contrib を訓練データ全サンプルに適用し φ_j(x_i) を取得
   ├── 各特徴量について mean |φ_j(x)| を集計（バイアス列は除外）
   └── 総和=1 になるよう正規化

5. R² の計算 — 4 と同一の booster を使用
   ├── 同じ booster を評価データで予測し MSE を計算
   └── R² = 1 - MSE_eval × N_eval / SS_total
```

> **重要**: 重要度（手順 4）と R²（手順 5）は MDI 同様、**同一の LightGBM RandomForest** から算出される。
> R² は「その重要度を出したモデルがどれだけ目的関数を説明できているか」を表す信頼度指標である。

### ハイパーパラメータ

| パラメータ         | 値    | 備考                                          |
| ------------------ | ----- | --------------------------------------------- |
| 木の本数           | 64    | MDI と同じ                                    |
| 最大深さ           | 10    | TreeSHAP の計算量 $O(L \cdot D^2)$ を考慮して設定 |
| 最小リーフサンプル | 2     | MDI/RF-ANOVA と同じ                           |
| 乱数シード         | 42    | 再現性確保                                    |
| 最大行数           | 1,000 | MDI と同じ（TreeSHAP は N 倍計算量のため抑制） |

---

## R² の解釈

R² は重要度を算出したランダムフォレスト（同一 booster）のホールドアウトデータ上の決定係数：

$$
R^2 = 1 - \frac{\sum_i (y_i - \hat{y}_i)^2}{\sum_i (y_i - \bar{y})^2}
$$

- $R^2 \geq 0.8$（緑）: モデルの当てはまりが良好。重要度の信頼性が高い
- $0.5 \leq R^2 < 0.8$（黄）: やや低め。参考程度として扱う
- $R^2 < 0.5$（赤）: モデルが目的関数を説明できていない。重要度の信頼性が低い

---

## 注意事項

**計算コスト**

TreeSHAP は各サンプルについて木のすべての葉を訪問するため、MDI・RF-ANOVA より計算時間が長い。
最大行数を 1,000 に制限しており、バックグラウンドスレッドで実行されるため UI はブロックされない。

**局所解釈と大域解釈**

本実装では `mean(|φ_j(x)|)` によるグローバル重要度を表示する。
個々のサンプルに対する局所的な寄与量（`φ_j(x)` の符号あり値）は現在表示していない。

**背景分布の近似**

TreeSHAP は $f(S) = \mathbb{E}[\hat{f}(x) \mid x_S]$ を訓練サンプルの経路比率で近似する（interventional ではなく path-dependent）。
特徴量間に強い相関がある場合、外挿サンプルの影響で結果が不安定になることがある。

---

## 計算コストの目安

| 試行数 N | SHAP 計算時間の目安                       |
| -------- | ----------------------------------------- |
| 50〜200  | < 500ms                                   |
| 500      | < 2,000ms                                 |
| 1,000+   | < 5,000ms（1,000 行にダウンサンプリング） |

---

## 参考文献

- Lundberg, S.M. & Lee, S.-I. (2017). "A Unified Approach to Interpreting Model Predictions." *NeurIPS 2017*.
- Lundberg, S.M. et al. (2020). "From local explanations to global understanding with explainable AI for trees." *Nature Machine Intelligence*.
