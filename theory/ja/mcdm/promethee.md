# PROMETHEE I / II（多基準意思決定法）

## 概要

PROMETHEE（Preference Ranking Organisation METHod for Enrichment Evaluations）は、1982年に J.-P. Brans が提案し、Brans & Vincke (1985) らにより発展した多基準意思決定（MCDM）手法。トライアル間の**ペアワイズ比較**に基づいて選好度を計算し、正のフロー（Φ+）と負のフロー（Φ-）からランキングを導出する。

PROMETHEE I は理論上は部分ランキング（Partial Ranking、比較不能なペアを許容）だが、本実装では Φ+ 降順・Φ- 昇順のタイブレークによる単一の全順序として返す（比較不能ペアの明示はしない）。PROMETHEE II は完全ランキング（Complete Ranking）を提供する。

Tunny Dashboard では以下の値を返す:

| 返り値              | 説明                                                    |
| ------------------- | ------------------------------------------------------- |
| `phi_plus[i]`       | トライアル i の正のフロー Φ+（他をどれだけ上回るか）    |
| `phi_minus[i]`      | トライアル i の負のフロー Φ-（他にどれだけ下回るか）    |
| `phi_net[i]`        | ネットフロー Φnet = Φ+ - Φ-                             |
| `ranked_indices_i`  | PROMETHEE I ランキング（Φ+ 降順、タイブレーク Φ- 昇順） |
| `ranked_indices_ii` | PROMETHEE II ランキング（Φnet 降順）                    |

---

## 理論背景

### 基本思想

PROMETHEE はすべてのトライアルペア (a, b) について「a は b よりどの程度好ましいか」を数量化し、それを集約してランキングを作る。距離測度（TOPSIS）や妥協指標（VIKOR）ではなく、**選好関数**を用いたペアワイズ比較が特徴。

### アルゴリズムの概要

m 個のトライアル × n 個の目的関数からなる決定行列を入力とし、以下のステップで計算する:

```
Step 1: 各目的関数の閾値（p）を自動算出
Step 2: 選好関数 P(d) の計算（Linear 型）
Step 3: 多指標選好度 π(a, b) の集約
Step 4: 正のフロー Φ+ と負のフロー Φ- の計算
Step 5: PROMETHEE I / II ランキングの生成
```

---

### Step 1: 閾値の自動算出

各目的関数 j について、有効トライアルの値域から閾値を設定する:

$$
\text{range}_j = \max_i f_{ij} - \min_i f_{ij}
$$

$$
p_j = 0.2 \times \text{range}_j
$$

- **q（無差別閾値）**: 0（わずかな差でも選好に反映）
- **p（厳格選好閾値）**: 値域の 20%（これを超える差は完全な選好）

> **本実装では Linear 型のみ対応**。他の選好関数（Usual, U-shape, Level, Gaussian 等）は将来拡張可能。

---

### Step 2: Linear 選好関数

トライアル a と b の目的 j における差分:

- 最小化目的: $d_j = f_{bj} - f_{aj}$（a が小さいほど d が正）
- 最大化目的: $d_j = f_{aj} - f_{bj}$（a が大きいほど d が正）

Linear 選好関数:

$$
P_j(d) = \begin{cases}
0 & d \leq 0 \\
\frac{d}{p_j} & 0 < d < p_j \\
1 & d \geq p_j
\end{cases}
$$

- $d \leq 0$: a は b より劣る（選好なし）
- $0 < d < p_j$: 差が閾値未満（部分選好）
- $d \geq p_j$: 差が閾値以上（完全選好）

---

### Step 3: 多指標選好度

すべての目的関数について重み付き和をとり、a と b の総合選好度を計算する:

$$
\pi(a, b) = \sum_{j=1}^{n} w_j \cdot P_j(d_j(a, b))
$$

$\pi(a, b) \in [0, 1]$

- $\pi(a, b) = 0$: a は b より全目的で劣るか同等
- $\pi(a, b) = 1$: a は b より全目的で厳格に優位
- $\pi(a, b) + \pi(b, a)$ は必ずしも 1 にならない（非対称）

---

### Step 4: フローの計算

各トライアル i について、正のフローと負のフローを計算する:

$$
\Phi^+(i) = \frac{1}{m-1} \sum_{b \neq i} \pi(i, b)
$$

$$
\Phi^-(i) = \frac{1}{m-1} \sum_{b \neq i} \pi(b, i)
$$

$$
\Phi^{\text{net}}(i) = \Phi^+(i) - \Phi^-(i)
$$

- **Φ+(i)**: トライアル i が他をどれだけ上回るか（高いほど優秀）
- **Φ-(i)**: トライアル i が他にどれだけ下回るか（低いほど優秀）
- **Φnet(i)**: ネットフロー。正なら概ね優位、負なら概ね劣位
- $\Phi^{\text{net}}(i) \in [-1, 1]$（すべてのフロー値は [0, 1] に正規化されるため）

---

### Step 5: ランキング

#### PROMETHEE I（理論上は部分ランキング）

Φ+ の降順でソート。Φ+ が同値の場合、Φ- の昇順でタイブレーク。

$$
a \succ b \iff \Phi^+(a) > \Phi^+(b) \;\text{or}\; \bigl(\Phi^+(a) = \Phi^+(b) \;\text{and}\; \Phi^-(a) < \Phi^-(b)\bigr)
$$

理論上は以下のケースで比較不能となる:

- $\Phi^+(a) > \Phi^+(b)$ かつ $\Phi^-(a) > \Phi^-(b)$ → 比較不能（一方が上回る面と下回る面の両方を持つ）

**本実装での扱い**: `rank_promethee_i` は比較不能の検出を行わず、上記のタイブレーク規則（Φ+ 降順・Φ- 昇順）によって常に単一の全順序を返す。比較不能ペアの明示的な出力はない。

#### PROMETHEE II（完全ランキング）

Φnet の降順でソート。すべてのトライアルが順位付けされる。

$$
a \succ b \iff \Phi^{\text{net}}(a) > \Phi^{\text{net}}(b)
$$

---

## TOPSIS / VIKOR との比較

| 項目       | TOPSIS             | VIKOR                               | PROMETHEE                    |
| ---------- | ------------------ | ----------------------------------- | ---------------------------- |
| アプローチ | 理想解との距離     | ギャップの線形結合                  | ペアワイズ選好比較           |
| 距離測度   | ユークリッド（L2） | マンハッタン（L1）+ Chebyshev（L∞） | 重み付き選好度               |
| ランキング | スコア降順（完全） | Q 値昇順（完全）                    | I: 部分 / II: 完全           |
| パラメータ | なし               | v（戦略的重み）                     | 閾値 p（本実装では自動算出） |
| 計算量     | O(m × n + m log m) | O(m × n + m log m)                  | **O(m² × n + m log m)**      |
| スコア範囲 | [0, 1]             | [0, 1]                              | Φnet ∈ [-1, 1]               |

---

## 数値例

### 問題設定

3 トライアル × 2 目的、両方最小化、重み $w = [0.5, 0.5]$

| トライアル | 目的1 | 目的2 |
| ---------- | ----- | ----- |
| 0          | 1     | 1     |
| 1          | 3     | 3     |
| 2          | 5     | 5     |

### Step 1: 閾値

目的1: range = 5 - 1 = 4, p₁ = 0.8
目的2: range = 5 - 1 = 4, p₂ = 0.8

### Step 2-3: 選好度行列 π

$\pi(0,1)$: 目的1 は $d = 3 - 1 = 2 \geq 0.8$ → $P = 1.0$、目的2 も同様 → $\pi = 0.5 \times 1.0 + 0.5 \times 1.0 = 1.0$

$\pi(0,2)$: 目的1 は $d = 5 - 1 = 4 \geq 0.8$ → $P = 1.0$、目的2 も同様 → $\pi = 1.0$

$\pi(1,0)$: 目的1 は $d = 1 - 3 = -2 \leq 0$ → $P = 0$、目的2 も同様 → $\pi = 0$

$\pi(1,2)$: 目的1 は $d = 5 - 3 = 2 \geq 0.8$ → $P = 1.0$、目的2 も同様 → $\pi = 1.0$

$\pi(2,0) = 0$, $\pi(2,1) = 0$

| π(a,b) | b=0 | b=1 | b=2 |
| ------ | --- | --- | --- |
| a=0    | -   | 1.0 | 1.0 |
| a=1    | 0   | -   | 1.0 |
| a=2    | 0   | 0   | -   |

### Step 4: フロー

$$\Phi^+(0) = (1.0 + 1.0) / 2 = 1.0, \quad \Phi^-(0) = (0 + 0) / 2 = 0$$

$$\Phi^+(1) = (0 + 1.0) / 2 = 0.5, \quad \Phi^-(1) = (1.0 + 0) / 2 = 0.5$$

$$\Phi^+(2) = (0 + 0) / 2 = 0, \quad \Phi^-(2) = (1.0 + 1.0) / 2 = 1.0$$

$$\Phi^{\text{net}} = [1.0, 0.0, -1.0]$$

### Step 5: ランキング

| ランキング   | 順位                   |
| ------------ | ---------------------- |
| PROMETHEE I  | 0 → 1 → 2（Φ+ 降順）   |
| PROMETHEE II | 0 → 1 → 2（Φnet 降順） |

---

## 実装の詳細

### NaN / Inf トライアルの扱い（`mod.rs` の `filter_valid_indices`）

いずれかの目的関数値が非有限（`NaN` または `±Inf`）のトライアルは有効トライアルから除外され、フローは `0.0`、ランキング末尾に配置される。

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

すべてのトライアルが非有限値の場合は `zero_result()` を返す（全フロー 0.0、ランキングはインデックス順）。

### 単一トライアルの場合

$m = 1$ のときペアが存在しないため、分母を `(n_valid - 1)` ではなく `max(n_valid - 1, 1)` としてゼロ除算を防ぎ、全フローは 0.0 となる。

### 値域ゼロ（全トライアル同一値）

$p_j = 0$ となり `linear_preference` は $d > 0$ の場合常に 1.0 を返すが、すべての差分が 0 なので $P_j = 0$、結果として全フロー 0.0 となる。

### 計算量

| ステップ   | 計算量                  |
| ---------- | ----------------------- |
| 閾値算出   | O(m × n)                |
| 選好度行列 | **O(m² × n)**           |
| フロー計算 | O(m²)                   |
| ソート     | O(m log m)              |
| **合計**   | **O(m² × n + m log m)** |

選好度行列の O(m²) がボトルネック。10,000 トライアル × 4 目的関数で Release ビルド 20ms 未満（実測）。

---

## UI での表示

### PROMETHEE I

トライアルごとに 2 本のバーを表示:

- **Φ+ バー**（青系）: 他をどれだけ上回るか
- **Φ- バー**（赤系）: 他にどれだけ下回るか

ランキング順（ranked_indices_i）に並ぶ。前述のとおり、本実装では比較不能ペアの明示的な区別は行われない。

### PROMETHEE II

トライアルごとに 1 本の Φnet バーを表示:

- **正値**（青系）: 概ね優位
- **負値**（アクセント色）: 概ね劣位
- バーの幅は |Φnet| に比例

PROMETHEE I ↔ II の切替時は同じキャッシュ（`app_state.mcdm_cache`）から即時復元する。Φ+/Φ-/Φnet は常に同時に計算されるため再計算不要。

---

## 特性・限界

**強み:**

- ペアワイズ比較に基づく直感的な解釈（a は b よりどれだけ好ましいか）
- PROMETHEE I は理論上は比較不能なペアを許容する部分ランキングだが、本実装では Φ+ 降順・Φ- 昇順のタイブレークによる全順序として返される（比較不能ペアの明示的な検出・出力はしない）
- PROMETHEE II で完全な順序付けが可能
- 選好関数の種類と閾値で目的関数ごとの選好の度合いを調整可能

**弱み:**

- O(m²) の計算量。トライアル数が増えると急激に遅くなる（10 万件で数秒）
- 閾値 p の設定が結果に影響する（本実装では固定で range×0.2）
- Linear 型のみ対応（他の選好関数は将来拡張）
- Φnet が負値をとり得るため、TOPSIS/VIKOR のような [0, 1] スケールと直接比較できない

---

## 使用場面の目安

```
多目的最適化結果からペアワイズの優劣関係を可視化したい
  ↓
トライアル間の「どちらがどの程度好ましいか」を定量化したい
  ↓
PROMETHEE I / II ランキング

トライアル数が多い（>1万）場合は計算時間に注意。
Entropy 重みと組み合わせることで客観的な重み設定が可能。
```

---

## 参考文献

- Brans, J.-P. (1982). L'ingénierie de la décision: élaboration d'instruments d'aide à la décision. _La méthode PROMETHEE_. Université Laval.
- Brans, J.-P., & Vincke, P. (1985). A Preference Ranking Organisation Method: The PROMETHEE Method for MCDM. _Management Science_, 31(6), 647–656.
- Brans, J.-P., Vincke, P., & Mareschal, B. (1986). How to select and how to rank projects: The PROMETHEE method. _European Journal of Operational Research_, 24(2), 228–238.
- Brans, J.-P., & Mareschal, B. (2005). PROMETHEE methods. In _Multiple Criteria Decision Analysis: State of the Art Surveys_ (pp. 163–186). Springer.
