# 部分依存プロット（PDP）による応答曲面

## 概要

部分依存プロット（Partial Dependence Plot; PDP）は、**特定のパラメータが目的関数に与える限界効果**を可視化する手法。他のパラメータの影響を平均化することで、着目パラメータと目的関数の関係を分離して把握できる。

Tunny Dashboard では複数のサロゲートモデルで 1D PDP・2D PDP を計算し、egui のカスタム 3D 描画による応答曲面プロット（`PdpChart2D`）として表示する。

| 種別   | 入力                        | 出力                          |
| ------ | --------------------------- | ----------------------------- |
| 1D PDP | パラメータ 1 つ、目的関数 1 つ | グリッド配列 + 予測値配列      |
| 2D PDP | パラメータ 2 つ、目的関数 1 つ | グリッド2配列 + 2D 予測値行列  |

### 2D PDP のサロゲートモデル

| モデル | 速度 | 品質 | 適用 N 規模 |
| ------ | ---- | ---- | ----------- |
| Ridge 回帰 | < 100ms | 線形のみ | 全規模 |
| Random Forest | < 2,000ms | 非線形・不連続 | 全規模 |
| GP-FITC | < 10,000ms | 滑らか・最高品質（デフォルト GP） | 全規模（全 N 点で学習） |
| GP-VFE | < 10,000ms | 滑らか・保守的フィット | 全規模（GP-FITC が過学習の場合） |
| GP-MOE | < 30,000ms | 滑らか・多領域対応 | 不連続・レジームスイッチ |

すべての GP バリアントは egobox-gp / egobox-moe（Apache-2.0）バックエンドを使用し、M = min(N, 100) 誘導点を用いる。N ≤ 100 のとき、FITC/VFE はノイズ推定付き厳密 GP と等価になる。

---

## 理論背景

### 一般的な PDP の定義

モデル $f(x_S, x_C)$ において、着目変数の集合 $S$ と補完変数の集合 $C = X \setminus S$ に対して、PDP は:

$$
\bar f_S(x_S) = \mathbb{E}_{x_C}[ f(x_S, x_C) ]
\approx \frac{1}{N} \sum_i f(x_S, x_{C,i})
$$

すべてのトレーニングサンプルに対して x_C を周辺化（平均化）することで、x_S だけの純粋な効果を抽出する。

---

### Tunny Dashboard の近似実装

完全なモンテカルロ評価は N × グリッド数 の予測呼び出しが必要になるため、サロゲートモデルを使った近似計算を採用している。

#### 1D PDP（Ridge 回帰による解析的計算）

1D PDP は全パラメータで Ridge 回帰をフィッティング後、解析的に計算する:

$$
\hat y = y_{\mathrm{mean}} + \sum_k \beta_k \frac{x_k - \mathrm{mean}_k}{\mathrm{std}_k}
$$

パラメータ $j$ に着目した場合、他のパラメータ $k\ne j$ を平均値 $\mathrm{mean}_k$ で代入すると:

$$
\bar f_j(v) = y_{\mathrm{mean}} + \beta_j \frac{v - \mathrm{mean}_j}{\mathrm{std}_j}
$$

上式は、実装式

$$
\bar f_j(v)=\bar y + \beta_j\frac{v-\mu_j}{\sigma_j} + \sum_{k\ne j}\beta_k\,\mathbb{E}\left[\frac{x_k-\mu_k}{\sigma_k}\right]
$$

において、標準化項の平均が $\mathbb{E}[(x_k-\mu_k)/\sigma_k]=0$ となることを使って簡約した形である。

**Ridge 係数 β_j に比例する線形関数**として解析的に表現される。

#### 2D PDP（複数サロゲートモデルによる計算）

2D PDP は選択されたサロゲートモデルで、Random Forest は 30×30、それ以外（Ridge・GP-FITC・GP-VFE・GP-MOE）は 20×20 のグリッドを計算する（`egui-app/src/ui/widgets/pdp/pdp_2d.rs`）:

| モデル | 計算方法 |
| ------ | -------- |
| Ridge | 2変数線形平面: `y_mean + β₁(v1−mean₁)/std₁ + β₂(v2−mean₂)/std₂` |
| Random Forest | CART+Bagging でグリッド各点を予測 |
| GP-FITC | ARD Matérn 5/2 GP（egobox-gp、FITC M=min(N,100)、全 N 点で学習） |
| GP-VFE | ARD Matérn 5/2 GP（egobox-gp、VFE 下界、M=min(N,100)、全 N 点で学習） |
| GP-MOE | 混合エキスパート GP（egobox-moe、GMM クラスタリング、最大 K=3 FITC エキスパート） |

すべてのモデルで `model_type` 引数を `compute_pdp_2d()` に渡すことでバックエンドのディスパッチが切り替わる。

---

## 実装の詳細

### パラメータ型に関する前提

`compute_pdp_from_data()` / `compute_pdp_2d()` は `DataFrame.get_numeric_column(...)` を使用して入力行列を構築している。したがって理論上の PDP 対象は数値パラメータに限定され、非数値列は `0.0` として扱われる（実装上のフォールバック値）。

### グリッドの構築

各パラメータの観測値の最小値〜最大値を `n_grid` 点で等間隔サンプリング（linspace）:

$$
\mathrm{grid}_j[k] = \mathrm{min}_j + (\mathrm{max}_j - \mathrm{min}_j) \frac{k}{n_{\mathrm{grid}} - 1}
\quad (k = 0, \ldots, n_{\mathrm{grid}}-1)
$$

`n_grid` はモデルと次元数（1D/2D）によって異なる。1D PDP は Ridge が 50、それ以外（Random Forest・GP-FITC・GP-VFE・GP-MOE）は 30（`egui-app/src/ui/widgets/pdp/pdp_chart.rs`）。2D PDP は Random Forest が 30、それ以外は 20（前掲）。

### Z スコア標準化

Ridge 回帰前に各パラメータ列を Z スコア標準化する:

$$
  ilde x_k = \frac{x_k - \mathrm{mean}_k}{\mathrm{std}_k}
$$

`std_k ≈ 0`（定数列）の場合は `std_k = 1.0` でゼロ除算を回避。

### `compute_pdp_2d_from_matrix()` の処理フロー

1. `compute_ridge(x_matrix, y, α=1.0)` で $\beta, r_{\mathrm{squared}}$ を算出
2. `col_mean_std(col1), col_mean_std(col2)` で $(\mathrm{mean}_1,\mathrm{std}_1),(\mathrm{mean}_2,\mathrm{std}_2)$ を算出
3. `linspace(min1, max1, n_grid)`, `linspace(min2, max2, n_grid)` でグリッド生成
4. 各グリッド点の予測値を計算:

$$
\mathrm{values}[i][j] = y_{\mathrm{mean}}
+ \beta_1\frac{\mathrm{grid}_1[i]-\mathrm{mean}_1}{\mathrm{std}_1}
+ \beta_2\frac{\mathrm{grid}_2[j]-\mathrm{mean}_2}{\mathrm{std}_2}
$$

5. `PdpResult2d { x_values, y_values, z_values, r_squared }` を返す

### 出力形式

`z_values[i][j]` は `x_values[i]`（X 軸）× `y_values[j]`（Y 軸）のグリッド点における予測目的関数値。egui-app 側で各グリッド点を 3D 頂点として展開し、カスタム 3D サーフェス描画に渡す。

### キャッシュ戦略

PDP の呼び出し元（egui-app 側）でモデル種別・パラメータ組み合わせ・目的・グリッド数をキーとしてキャッシュする。同一条件への再アクセスは再計算をスキップする。Study が変わると自動でキャッシュクリアされる。

---

## R² について

`r_squared` は各サロゲートモデルの訓練データへの適合度:

$$
R^2 = 1 - \frac{\sum_i (y_i - \hat y_i)^2}{\sum_i (y_i - \bar y)^2}
$$

- **R² ≈ 1.0**: サロゲートモデルがデータをよく説明しており、PDP の信頼度が高い
- **R² < 0.5**: モデルの説明力が低く、PDP は目安程度にとどめる
- R² が低い場合は、より表現力の高いモデル（GP-FITC / GP-MOE（滑らかな場合）、Random Forest / LightGBM（ノイジーな場合））への切り替え、または Spearman / Sobol による感度分析を推奨

---

## 特性・限界

### Ridge 回帰（1D PDP・2D PDP デフォルト）

**強み:** 解析解のため非常に高速（< 100ms）。外挿しない。R² で信頼度確認可能。

**弱み:** 線形のみ。非線形・U字型・交互作用は捉えられない。

### Random Forest

**強み:** 非線形・不連続な目的関数に対応。外挿しない。

**弱み:** 決定木境界のアーティファクト（段差）が現れやすい。少数サンプルでは不安定。

### GP-FITC（ガウス過程回帰、FITC 近似）

**強み:** 滑らかな補間。少数サンプル（N < 50）でも高品質。ARD で次元重要度を自動推定。全 N 点で学習（サブサンプリング不要）。egobox-gp バックエンド（COBYLA 10 点マルチスタート）。デフォルト GP。

**弱み:** N > 100 では M = 100 の誘導点上限によりコストを抑えるが近似誤差が生じる。ノイジーデータで過学習することがある。

### GP-VFE（Variational Free Energy 近似）

**強み:** GP-FITC と同じアーキテクチャ。VFE 下界によりやや保守的なノイズ推定 → より滑らかなフィット。GP-FITC 曲面が過学習気味の場合に推奨。

**弱み:** GP-FITC よりわずかにフィットが緩い（参考値であり環境により変動するが、ノイズなしベンチマークで GP-FITC よりやや低い R² が観測される傾向がある）。

### GP-MOE（混合エキスパート GP）

**強み:** 不連続・レジームスイッチング・多峰性の応答曲面に対応。クラスタ数を自動選択。滑らかな再結合で継ぎ目なし。GP の不確実性推定を保持。

**弱み:** GP-FITC / GP-VFE より学習コストが高い（おおよそ K 倍）。学習失敗時は GP-FITC にフォールバック（PDP 時）。

---

## 使用場面の目安

```
2つのパラメータが目的関数に与える複合的な影響を見たい
  ↓
ImportanceChart / SensitivityHeatmap で重要パラメータを絞り込む
  ↓
3D 応答曲面プロット（SurfacePlot3D）で上位 2 パラメータを可視化

サロゲートモデルの選択:
  まず高速確認したい              → Ridge（デフォルト）
  R² < 0.5 で非線形・ノイジー    → Random Forest または LightGBM
  滑らかな補間・最高品質          → GP-FITC（全 N 点で学習、デフォルト GP）
  GP-FITC が過学習気味            → GP-VFE（より滑らか・保守的）
  不連続・多領域の応答曲面        → GP-MOE
```

---

## 1D PDP と 2D PDP の比較

| 項目       | 1D PDP (`PdpChart`)       | 2D PDP (`PdpChart2DState`)            |
| ---------- | ------------------------- | ------------------------------------- |
| 着目変数   | パラメータ 1 つ           | パラメータ 2 つ                       |
| 可視化形式 | 折れ線グラフ（egui）       | egui カスタム 3D サーフェスプロット   |
| 出力       | `grid[k]`, `values[k]`   | `x_values[i]`, `y_values[j]`, `z_values[i][j]` |
| サロゲート | Ridge / Random Forest / GP-FITC / GP-VFE / GP-MOE（選択可、デフォルト Ridge） | Ridge / Random Forest / GP-FITC / GP-VFE / GP-MOE（選択可） |
| 用途       | 単一パラメータの傾向確認  | 2変数複合効果・最適領域の把握         |

---

## 参考文献

- Friedman, J. H. (2001). Greedy Function Approximation: A Gradient Boosting Machine. _Annals of Statistics_, 29(5), 1189–1232.
