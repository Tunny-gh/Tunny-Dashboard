# Sobol 感度指数

## 概要

Sobol 感度指数は、目的関数の**分散分解**に基づいてパラメータの重要度を定量化するグローバル感度分析手法。線形・非線形を問わず、またパラメータ間の**交互作用**も含めた影響度を [0, 1] の値として返す。

Tunny Dashboard では 2 種類の指数を提供する:

| 指数                      | 記号 | 意味                                                    |
| ------------------------- | ---- | ------------------------------------------------------- |
| 一次指数 (First-Order)    | S_i  | パラメータ x_i の**単独**寄与率                         |
| 全効果指数 (Total-Effect) | ST_i | x_i の単独 + **他パラメータとの交互作用**を含む総寄与率 |

---

## 理論背景

### ANOVA 分散分解

Sobol 分解は、モデル出力 Y = f(X_1, ..., X_p) の分散を各パラメータの寄与に分解する:

```
Var(Y) = Σ_i V_i + Σ_{i<j} V_ij + ... + V_{1...p}
```

- V_i = Var(E[Y | X_i]) — x_i のみが変化したときの Y の期待値の分散
- V_ij = Var(E[Y | X_i, X_j]) - V_i - V_j — x_i と x_j の交互作用分散

### 一次指数

```
S_i = V_i / Var(Y)
```

x_i の変化だけで説明できる Y の分散の割合。S_i の合計は ≤ 1（交互作用がなければ = 1）。

### 全効果指数

```
ST_i = E[Var(Y | X_{~i})] / Var(Y) = 1 - Var(E[Y | X_{~i}]) / Var(Y)
```

X\_{~i} は「x_i 以外のすべてのパラメータ」。ST_i は x_i の単独効果と x_i が絡むすべての交互作用効果の合計。

**ST_i ≥ S_i** は常に成立する。差 (ST_i - S_i) が大きいほど、x_i が他パラメータとの交互作用に強く関与している。

---

## Tunny Dashboard の実装

### 全体フロー

直接 Monte Carlo 積分を行うと n × p 倍のシミュレーション実行が必要（最適化問題では不可能）。代わりに**二次 Ridge サロゲートモデル**を構築し、そのモデル上で Saltelli サンプリングを行う。

```
実際のトライアルデータ
    ↓
Step 1: 二次 Ridge サロゲート構築
    ↓
Step 2: Saltelli 行列 A, B を LCG64 で生成
    ↓
Step 3: A, B, AB_i でサロゲートを評価
    ↓
Step 4: Jansen 推定量で S_i, ST_i を計算
```

---

### Step 1: 二次 Ridge サロゲートモデル

#### 特徴量構成（`build_quad_features()`）

p 個の標準化済み線形入力から p(p+3)/2 次元の二次特徴量ベクトルを構築:

```
φ(x) = [x_1, ..., x_p,          ← 線形項: p 個
         x_1², ..., x_p²,        ← 二乗項: p 個
         x_1 x_2, ..., x_{p-1} x_p]  ← 交差項: p(p-1)/2 個
```

例: p = 3 → 3 + 3 + 3 = 9 次元 (= 3×6/2)

#### サロゲートの学習（`build_sobol_surrogate()`）

```
1. X の各列を Z スコア標準化: x̃_j = (x_j - μ_j) / σ_j
2. build_quad_features で二次特徴量 Φ を構築
3. Φ の各列を再度 Z スコア標準化
4. 各目的関数に対して Ridge(α=1.0) を適合:
   β_k = (Φ^T Φ + I)^{-1} Φ^T (y_k - ȳ_k)
```

#### サロゲートの評価（`surrogate_eval()`）

```
f̂(x) = β^T · φ̃(x) + intercept
```

ここで φ̃(x) は二次特徴量を訓練データの統計量で標準化したもの。

---

### Step 2: Saltelli サンプリング

#### 疑似乱数生成: LCG64（`lcg_next()`）

外部クレート（`rand` 等）を使わず、Linear Congruential Generator で一様乱数を生成:

```rust
state = state * 6364136223846793005 + 1442695040888963407  (mod 2^64)
u = (state >> 11) as f64 / 2^53
```

初期シード: `0xDEADBEEF12345678`（固定再現性）

#### 行列 A, B の生成

各パラメータの値域 [lo_j, hi_j] は実際のトライアルの min/max から取得する。

```
A[s][j] = lo_j + u × (hi_j - lo_j)    (s = 0..n_samples, j = 0..p)
B[s][j] = lo_j + u × (hi_j - lo_j)    (A と B は独立サンプル)
```

#### AB_i 行列

A の第 i 列のみを B の第 i 列で置換した行列:

```
AB_i[s] = [A[s][0], ..., A[s][i-1], B[s][i], A[s][i+1], ..., A[s][p-1]]
```

---

### Step 3: サロゲート評価

```
f_A[k][s]    = surrogate_eval(A[s], objective=k)
f_B[k][s]    = surrogate_eval(B[s], objective=k)
f_AB_i[k][s] = surrogate_eval(AB_i[s], objective=k)
```

---

### Step 4: Jansen 推定量

#### 一次指数（Saltelli 2010）

```
S_i ≈ [ Σ_s f_B[s] · (f_AB_i[s] - f_A[s]) ] / (N · Var_Y)
```

#### 全効果指数（Jansen 1999）

```
ST_i ≈ [ Σ_s (f_A[s] - f_AB_i[s])² ] / (2 · N · Var_Y)
```

#### 分散の推定

```
Var_Y = mean(f_A²) - mean(f_A)²
```

Var_Y ≈ 0 の場合（サロゲートが定数に近い）は S_i = ST_i = 0 を返す。

#### クリッピング

推定誤差による範囲外値を防ぐため、最終値を [0, 1] にクリップ:

```
S_i  = clamp(S_i_raw,  0.0, 1.0)
ST_i = clamp(ST_i_raw, 0.0, 1.0)
```

---

### 複数目的関数への対応

ImportanceChart では、各パラメータのスコアとして全目的関数に対する指数の**平均**を表示:

```
display_score(p_j) = (1/m) Σ_k S_i[j][k]     // sobol_first の場合
display_score(p_j) = (1/m) Σ_k ST_i[j][k]    // sobol_total の場合
```

---

## パラメータ設定

| 設定項目            | 値                 | 変更箇所                                      |
| ------------------- | ------------------ | --------------------------------------------- |
| Saltelli サンプル数 | 1024               | `analysisStore.computeSobol(nSamples = 1024)` |
| Ridge 正則化強度 α  | 1.0                | `build_sobol_surrogate(..., alpha: f64)`      |
| 乱数シード          | 0xDEADBEEF12345678 | `compute_sobol()` 内 `rng_state`              |

---

## 必要データ量

- 最低 2 トライアル（n ≥ 2）、p ≥ 1、目的関数 ≥ 1 が必要
- サロゲートの精度は n とともに向上する。目安として n ≥ 10 × p(p+3)/2 を推奨

---

## 特性・限界

**強み:**

- 線形・非線形・交互作用をすべて扱える
- 値域が [0, 1] で複数パラメータ間の比較が容易
- ST_i - S_i で交互作用の強度を把握できる
- サロゲートを使うため、トライアル数が限られていても計算可能

**弱み:**

- サロゲートモデル（二次 Ridge）の精度に結果が依存する
  - 真の関数が高次非線形または強い不連続性を持つ場合は精度が落ちる
- p が大きくなると特徴量数 p(p+3)/2 が増加し、サロゲートの学習精度が低下する可能性がある
- 乱数シードが固定のため再現性はあるが、N の選択で結果が変わる

---

## 参考文献

- Saltelli, A. et al. (2010). Variance based sensitivity analysis of model output. Design and estimator for the total sensitivity index. _Computer Physics Communications_, 181(2), 259–270.
- Jansen, M. J. W. (1999). Analysis of variance designs for model output. _Computer Physics Communications_, 117(1), 35–43.
- Sobol, I. M. (1993). Sensitivity estimates for nonlinear mathematical models. _Mathematical Modelling and Computational Experiments_, 1(4), 407–414.
