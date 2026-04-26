# SHAP（SHapley Additive exPlanations）によるパラメータ重要度

## 概要

SHAP（SHapley Additive exPlanations）は、ゲーム理論の **Shapley 値** に基づく特徴量重要度の計算手法（Lundberg & Lee, 2017）。
各パラメータが予測値にどれだけ寄与したかを、**すべての特徴量の組み合わせ順序の平均**として厳密に定義する。

ImportanceChart では **TreeSHAP**（Lundberg & Lee, 2018）を使用し、Random Forest の各木で Shapley 値を厳密かつ効率的に計算する。
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

## TreeSHAP アルゴリズム（Lundberg & Lee 2018）

Tree モデルに対して Shapley 値を **厳密に** 計算する多項式時間アルゴリズム。
単一の木に対して $O(L \cdot D^2)$ で動作する（$L$: 葉の数、$D$: 木の深さ）。

### 核心的アイデア

決定木では特徴量が「存在する（hot）」か「存在しない（cold）」かで 2 つの経路がある。
$f(S)$ を訓練サンプルの分岐比率（`n_child / n_parent`）で近似することで、
すべての $2^{|F|}$ 部分集合を列挙せずに Shapley 値を計算できる。

### PathElement による多項式重み管理

根からリーフまでの経路を以下の構造で管理する：

$$
\text{PathElement}_k = \{\text{feature}, \; z_k, \; o_k, \; w_k\}
$$

- $z_k$：特徴量 $k$ が**存在しない**場合の分岐比率（`n_cold / n_parent`）
- $o_k$：特徴量 $k$ が**存在する**場合の分岐比率（`n_hot / n_parent`）
- $w_k$：多項式係数（Shapley 重みの計算に使用）

**パス拡張 `extend_path`**（深さ $m$ の位置に新要素を追加）：

$$
w_{m} \leftarrow 0, \quad w_0 \leftarrow 1 \text{ (初回のみ)}
$$
$$
\text{for } i = m-1, \ldots, 0: \quad
w_{i+1} \mathrel{+}= o \cdot w_i \cdot \frac{i+1}{m+1}, \quad
w_i \mathrel{\times}= z \cdot \frac{m-i}{m+1}
$$

**Shapley 重みの計算 `unwound_sum`**（位置 $k$ の要素を取り除いた場合の重み総和）：

$$
W_k = \frac{1}{d+1} \sum_{i=0}^{d-1} \hat{w}_i
$$

ここで $\hat{w}_i$ は `unwind_path` によって位置 $k$ の要素を除いたときの再構成重み。

### 再帰的計算 `tree_shap_recurse`

```
tree_shap_recurse(node, x, depth, path, parent_z, parent_o, parent_feat, phi):
  extend_path(path, depth, parent_z, parent_o, parent_feat)

  if leaf:
    for k in 1..depth:
      w = unwound_sum(path, depth, k)
      phi[path[k].feature] += w * (path[k].o - path[k].z) * leaf_value
    return

  hot_child  = left if x[feat] <= threshold else right   # x が自然に進む方
  cold_child = the other child

  hot_z  = n_hot  / n_node
  cold_z = n_cold / n_node

  if feat already in path at index idx:
    # 同一特徴量が経路上に既存 → unwind してから再追加
    iz, io = path[idx].z, path[idx].o
    unwind_path(path, depth, idx)
    recurse(hot_child,  depth,   path, hot_z*iz,  io,  feat)
    recurse(cold_child, depth,   path, cold_z*iz, 0.0, feat)
  else:
    recurse(hot_child,  depth+1, path, hot_z,  1.0, feat)
    recurse(cold_child, depth+1, path, cold_z, 0.0, feat)
```

`path` は `Copy` 型の固定長配列として実装されており、各再帰ブランチは独立したコピーを受け取る。

---

## アルゴリズムの手順

```
入力: X (N × P), y (N)

1. データ前処理
   ├── NaN/Inf 行を除外
   └── N > 1,000 の場合はランダムサンプリングで 1,000 行に削減

2. 80/20 ホールドアウト分割（R² 計算用）

3. TreeSHAP 重要度の集計（T=64 本の木）
   各木 b について:
     3a. 訓練データから N_train 点を復元抽出（ブートストラップ）
     3b. CART 回帰木を構築（ShapNode: feature, threshold, value, n_samples を保持）
     3c. 訓練データの各サンプル x について tree_shap_recurse を実行
     3d. 各特徴量 j の |φ_j(x)| を累積

4. 正規化
   ├── サンプル数 × 木数で割って平均化
   └── 総和=1 になるよう正規化

5. R² の計算
   ├── 同じ訓練データで RandomForest を学習（MDI と同一ロジック）
   ├── 評価データで MSE を計算
   └── R² = 1 - MSE_eval × N_eval / SS_total
```

### ハイパーパラメータ

| パラメータ         | 値    | 備考                                       |
| ------------------ | ----- | ------------------------------------------ |
| 木の本数           | 64    | MDI と同じ                                 |
| 最大深さ           | 10    | TreeSHAP の計算量 O(2^D) を考慮して設定    |
| 最小リーフサンプル | 2     | MDI/RF-ANOVA と同じ                        |
| 乱数シード         | 42    | 再現性確保                                 |
| 最大行数           | 1,000 | MDI より少なめ（TreeSHAP は N 倍計算量）   |

---

## R² の解釈

R² はホールドアウトデータ上のランダムフォレストの決定係数：

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

---

## 実装ファイル

- `rust_core/src/sensitivity/shap.rs` — `compute_shap_importances()`、`tree_shap_recurse()`、`extend_path()`、`unwind_path()`、`unwound_sum()`
- `rust_core/src/sensitivity/types.rs` — `ShapResult` 構造体
- `rust_core/src/sensitivity/analysis/full.rs` — `SensitivityMetric::Shap` の計算
- `egui-app/src/ui/widgets/importance_chart.rs` — UI（`ImportanceMetric::Shap`）
