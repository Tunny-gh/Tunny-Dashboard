# TOPSIS（多基準意思決定法）

## 概要

TOPSIS（Technique for Order Preference by Similarity to Ideal Solution）は、**複数の目的関数を同時に考慮してトライアルをランキング**する多基準意思決定（MCDM）手法。理想解に近く、反理想解から遠い解を最良とみなす。

Tunny Dashboard では TOPSIS スコアとランキングを提供する:

| 返り値             | 説明                                                |
| ------------------ | --------------------------------------------------- |
| `scores[i]`        | トライアル i の TOPSIS スコア（0〜1、高いほど良い） |
| `rankedIndices`    | スコア降順に並べたトライアルインデックス配列        |
| `positiveIdeal[j]` | 目的関数 j の理想解（正理想解 A+）                  |
| `negativeIdeal[j]` | 目的関数 j の反理想解（負理想解 A-）                |

---

## 理論背景

### アルゴリズムの概要

m 個のトライアル × n 個の目的関数からなる決定行列 V（m×n）を入力とし、以下の 6 ステップで各トライアルのスコアを計算する。

```
Step 1: 決定行列の正規化
Step 2: 重み付き正規化行列の構築
Step 3: 正理想解・負理想解の決定
Step 4: 各解から理想解・反理想解までの距離計算
Step 5: TOPSIS スコア（相対的近接度）の計算
Step 6: スコア降順によるランキング
```

---

### Step 1: ベクトル正規化

各目的関数列 $j$ について、ユークリッドノルムで正規化する:

$$
r_{ij} = \frac{v_{ij}}{\sqrt{\sum_i v_{ij}^2}}
$$

これにより異なるスケールの目的関数が比較可能になる。

---

### Step 2: 重み付き正規化行列

正規化値に目的関数ごとの重み $w_j$ を乗じる:

$$
w_{ij} = w_j r_{ij}
$$

重みはユーザが設定する。`compute_topsis()` は VIKOR と同様に、渡された `weights` を内部で合計 1 に正規化してから使用する（全ゼロや NaN などの退化した入力は均一重みにフォールバック）。重要な目的関数の影響が大きくなる点は変わらない。

---

### Step 3: 正理想解・負理想解の決定

各目的関数の方向（minimize / maximize）に応じて、列ごとの最善値と最悪値を選ぶ:

| 方向     | 正理想解 A+\_j | 負理想解 A-\_j |
| -------- | -------------- | -------------- |
| minimize | $\min_i w_{ij}$   | $\max_i w_{ij}$   |
| maximize | $\max_i w_{ij}$   | $\min_i w_{ij}$   |

---

### Step 4: ユークリッド距離の計算

各トライアル $i$ から正理想解・負理想解までのユークリッド距離:

$$
D_i^+ = \sqrt{\sum_j (w_{ij} - A_j^+)^2},\qquad
D_i^- = \sqrt{\sum_j (w_{ij} - A_j^-)^2}
$$

---

### Step 5: 相対的近接度（TOPSIS スコア）

$$
\mathrm{score}_i = \frac{D_i^-}{D_i^+ + D_i^-}
$$

- `score_i → 1`: 正理想解に近い（優れたトライアル）
- `score_i → 0`: 負理想解に近い（劣ったトライアル）
- `D+_i + D-_i = 0` の場合は `score_i = 0.5`（縮退ケース）

---

## 実装の詳細

### NaN / Inf トライアルの扱い（`mod.rs` の `filter_valid_indices`）

いずれかの目的関数値が非有限（`NaN` または `±Inf`）のトライアルは有効トライアルから除外され、スコアは `0.0` となりランキング末尾に配置される。

```rust
/// Return indices of trials whose objectives are all finite (excludes NaN and ±Inf).
pub(crate) fn filter_valid_indices(
    values: &[f64],
    n_trials: usize,
    n_objectives: usize,
) -> Vec<usize> {
    (0..n_trials)
        .filter(|&i| (0..n_objectives).all(|j| values[i * n_objectives + j].is_finite()))
        .collect()
}
```

補足: すべてのトライアルが非有限値の場合は `valid_indices.is_empty()` となり、実装は縮退ケースとして**全トライアルに `0.5`** を割り当てる（`uniform_score_result(..., score=0.5)`）。

```
if valid_indices.is_empty() {
  return Ok(uniform_score_result(n_trials, n_objectives, 0.5, &start));
}
```

### 全トライアルが同一値のとき

列ノルムが 0 になる場合（全トライアルの目的関数値が等しい）は `r_ij = 0.0` として処理し、正理想解と負理想解が一致するため `D+ = D- = 0` → スコアは `0.5` となる。

### 重みのスケール不変性

`[0.7, 0.3]` と `[7.0, 3.0]` は同じ結果になる。`compute_topsis()` が重みを内部で合計 1 に正規化することに加え、そもそもスコアは重みベクトルの定数倍に不変（比率のみが効く）ためである。

### `compute_topsis()` の処理フロー

```
1. validate_inputs()         → 入力サイズの整合性チェック
2. normalize_weights()       → 重みを合計 1 に正規化（退化時は均一重み）
3. NaN/Inf フィルタリング     → valid_indices を構築
4. build_weighted_matrix()   → 列ノルム計算 → r_ij → w_ij
5. find_ideal_solutions()    → is_minimize に応じて A+/A- を決定
6. compute_scores()          → D+, D- → score_i
7. NaN/Inf トライアルにスコア 0.0 を割り当て
8. スコア降順ソートで ranked_indices を生成
```

### 計算量

| ステップ   | 計算量                 |
| ---------- | ---------------------- |
| 正規化     | O(m × n)               |
| 理想解決定 | O(m × n)               |
| 距離計算   | O(m × n)               |
| ソート     | O(m log m)             |
| **合計**   | **O(m × n + m log m)** |

50,000 トライアル × 4 目的関数で 100ms 未満（実測）。

---

## 特性・限界

**強み:**

- 複数の目的関数を一つの総合スコアに集約できる
- minimize / maximize が混在していても対応可能
- 重みで目的関数の相対的重要度を調整できる
- スコアが [0, 1] に収まるため直感的に解釈しやすい

**弱み:**

- 重みの設定が恣意的になりやすい（どの目的関数をどれだけ重視するかはユーザが決める）
- 目的関数間のスケールが大きく異なると、正規化後も影響が均等にならない場合がある
- 選好関係の推移律が成立しない場合がある（TOPSIS スコアのランクが入れ替わるランキング逆転問題）
- 目的関数が真に非可換のトレードオフ（パレートフロント上）の場合、重み次第で恣意的な選択になる

---

## 使用場面の目安

```
多目的最適化結果から「総合的に優秀なトライアル」を選びたい
  ↓
目的関数の相対重要度をユーザが指定できる
  ↓
TOPSIS ランキング

各目的関数の重要度が不明な場合は均等重みから始め、
TopsisRankingChart のスライダーで感度確認するのが有効。
```

---

## UI での操作

`TopsisRankingChart` コンポーネントで以下を操作できる:

- **重みスライダー**: 各目的関数の重み（0〜1）をリアルタイム変更→スコア再計算
- **上位 N 件表示**: 5 / 10 / 20 件を切り替え
- **バークリック**: 選択されたトライアルをハイライト（selectionStore 経由）

---

## 参考文献

- Hwang, C.-L., & Yoon, K. (1981). _Multiple Attribute Decision Making: Methods and Applications_. Springer.
