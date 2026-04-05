# 部分依存プロット（PDP）による応答曲面

## 概要

部分依存プロット（Partial Dependence Plot; PDP）は、**特定のパラメータが目的関数に与える限界効果**を可視化する手法。他のパラメータの影響を平均化することで、着目パラメータと目的関数の関係を分離して把握できる。

Tunny Dashboard では Ridge 回帰サロゲートモデルを使って 1D PDP・2D PDP を高速に計算し、3D 応答曲面プロットとして表示する。

| 種別   | 入力                        | 出力                          |
| ------ | --------------------------- | ----------------------------- |
| 1D PDP | パラメータ 1 つ、目的関数 1 つ | グリッド配列 + 予測値配列      |
| 2D PDP | パラメータ 2 つ、目的関数 1 つ | グリッド2配列 + 2D 予測値行列  |

---

## 理論背景

### 一般的な PDP の定義

モデル f(x_S, x_C) において、着目変数の集合 S と補完変数の集合 C = X \ S に対して、PDP は:

```
f̄_S(x_S) = E_{x_C}[ f(x_S, x_C) ]
           ≈ (1/N) Σ_i f(x_S, x_{C,i})
```

すべてのトレーニングサンプルに対して x_C を周辺化（平均化）することで、x_S だけの純粋な効果を抽出する。

---

### Tunny Dashboard の近似実装

完全なモンテカルロ評価は N × グリッド数 の予測呼び出しが必要になるため、**Ridge 回帰サロゲートモデルによる解析的近似**を採用している。

#### Ridge 回帰モデルの構築

まず全パラメータで目的関数を Ridge 回帰でフィッティング（α = 1.0）:

```
ŷ = y_mean + Σ_k β_k × (x_k - mean_k) / std_k
```

#### 1D PDP の解析的計算

パラメータ j に着目した場合、他のパラメータ k≠j を平均値 mean_k で代入すると:

```
f̄_j(v) = y_mean + β_j × (v - mean_j) / std_j
         + Σ_{k≠j} β_k × (mean_k - mean_k) / std_k
       = y_mean + β_j × (v - mean_j) / std_j
```

つまり **Ridge 係数 β_j に比例する線形関数**として表現される。

#### 2D PDP の解析的計算

パラメータ j1, j2 に着目した場合:

```
f̄_{j1,j2}(v1, v2) = y_mean + β_j1 × (v1 - mean_j1) / std_j1
                            + β_j2 × (v2 - mean_j2) / std_j2
```

これは **2 変数の線形平面**として表現される。

---

## 実装の詳細

### グリッドの構築

各パラメータの観測値の最小値〜最大値を `n_grid` 点で等間隔サンプリング（linspace）:

```
grid_j[k] = min_j + (max_j - min_j) × k / (n_grid - 1)   (k = 0, ..., n_grid-1)
```

デフォルト `n_grid = 50`。

### Z スコア標準化

Ridge 回帰前に各パラメータ列を Z スコア標準化する:

```
x̃_k = (x_k - mean_k) / std_k
```

`std_k ≈ 0`（定数列）の場合は `std_k = 1.0` でゼロ除算を回避。

### `compute_pdp_2d_from_matrix()` の処理フロー（`rust_core/src/pdp.rs`）

```
1. compute_ridge(x_matrix, y, α=1.0)   → β, r_squared
2. col_mean_std(col1), col_mean_std(col2) → mean1/std1, mean2/std2
3. linspace(min1, max1, n_grid) → grid1
   linspace(min2, max2, n_grid) → grid2
4. values[i][j] = y_mean + β1×(grid1[i]-mean1)/std1
                          + β2×(grid2[j]-mean2)/std2
5. PdpResult2d { grid1, grid2, values, r_squared } を返す
```

### 出力形式

`values[i][j]` は `grid1[i]`（X 軸）× `grid2[j]`（Y 軸）のグリッド点における予測目的関数値。フロントエンドでは `values.flatMap((row, i) => row.map((val, j) => ({position: [grid1[j], grid2[i]], colorValue: val})))` の形で deck.gl `GridLayer` に渡す。

### キャッシュ戦略

`analysisStore.ts` で `surrogateModelType_param1_param2_objective_nGrid` をキーとしてキャッシュ。同一パラメータ組み合わせへの再アクセスは WASM 呼び出しをスキップ。Study が変わると自動でキャッシュクリア。

---

## R² について

`r_squared` は Ridge サロゲートモデルの適合度:

```
R² = 1 - Σ(y_i - ŷ_i)² / Σ(y_i - ȳ)²
```

- **R² ≈ 1.0**: パラメータ・目的関数間の関係が線形に近く、PDP の信頼度が高い
- **R² < 0.5**: 非線形な関係が強く、PDP は目安程度にとどめる
- R² が低い場合は、Spearman や Sobol による感度分析も参照することを推奨

---

## 特性・限界

**強み:**

- Ridge サロゲートによる解析解のため、グリッド点数に関わらず計算が非常に高速（O(np²) の Ridge フィッティング + O(n_grid²) のグリッド評価）
- 最小値・最大値から推定した観測範囲内でのみ予測するため外挿しない
- R² により信頼度を定量的に確認できる

**弱み:**

- **線形サロゲートの仮定**: 真のパラメータ・目的関数関係が非線形の場合、PDP は真の依存性を近似できない。特に U 字型の関係や急峻な変化は捉えられない
- **交互作用の無視**: 2D PDP は独立な足し合わせのため、パラメータ間の交互作用（相乗効果・拮抗効果）は表現されない
- **外挿なし**: 観測範囲外の予測は行わない（観測最小値〜最大値の範囲のみ表示）
- **サロゲートバイアス**: 実際の目的関数形状よりも平滑化された曲面になる

---

## 使用場面の目安

```
2つのパラメータが目的関数に与える複合的な影響を見たい
  ↓
どのパラメータ組み合わせを選べばよいか知りたい
  ↓
ImportanceChart / SensitivityHeatmap で重要パラメータを絞り込む
  ↓
3D 応答曲面プロット（SurfacePlot3D）で上位 2 パラメータを可視化

R² が低い場合は、非線形手法（Random Forest 等）によるサロゲートの将来対応を待つか、
Sobol 全効果指数でパラメータ影響度を確認する
```

---

## 1D PDP と 2D PDP の比較

| 項目       | 1D PDP (`PDPChart`)       | 2D PDP (`SurfacePlot3D`)         |
| ---------- | ------------------------- | -------------------------------- |
| 着目変数   | パラメータ 1 つ           | パラメータ 2 つ                  |
| 可視化形式 | 折れ線グラフ              | deck.gl GridLayer 3D ヒートマップ |
| 出力       | `grid[k]`, `values[k]`   | `grid1[i]`, `grid2[j]`, `values[i][j]` |
| サロゲート | Ridge（同一）             | Ridge（同一）                    |
| 用途       | 単一パラメータの傾向確認  | 2変数複合効果・最適領域の把握    |

---

## 実装ファイル

- `rust_core/src/pdp.rs` — PDP 計算ロジック（1D / 2D）
- `rust_core/src/lib.rs` — WASM バインディング（`computePdp2d`）
- `frontend/src/wasm/wasmLoader.ts` — JS ブリッジ（`Pdp2dWasmResult` 型）
- `frontend/src/stores/analysisStore.ts` — 状態管理・キャッシュ（`surface3dCache`）
- `frontend/src/components/charts/SurfacePlot3D.tsx` — 3D 応答曲面 UI（deck.gl GridLayer）
- `frontend/src/components/charts/PDPChart.tsx` — 1D PDP UI（ECharts 折れ線）
