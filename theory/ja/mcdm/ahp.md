# AHP（階層分析法）

## 概要

AHP（Analytic Hierarchy Process）は、**一対比較行列から目的関数の相対的重要度（重み）を導出し、加重和スコアでトライアルをランキング**する多基準意思決定（MCDM）手法。Saaty（1977）により提案された。

Tunny Dashboard では AHP スコアとランキングを提供する:

| 返り値              | 説明                                                           |
| ------------------- | -------------------------------------------------------------- |
| `priority_vector`   | 目的関数の重みベクトル（Σ = 1.0）                              |
| `scores[i]`         | トライアル i の AHP スコア（0〜1、高いほど良い）              |
| `ranked_indices`    | スコア降順に並べたトライアルインデックス配列                   |
| `lambda_max`        | 最大固有値                                                      |
| `ci`                | 整合性指標（Consistency Index）                                 |
| `ri`                | ランダム整合性指標（Random Index）                              |
| `cr`                | 整合性比（Consistency Ratio）                                   |
| `is_consistent`     | CR ≤ 0.10 かどうか                                              |

---

## 理論背景

### アルゴリズムの概要

n 個の目的関数について、Saaty 1-9 スケールの一対比較行列 A（n×n）を入力とし、以下の 4 ステップで各トライアルのスコアを計算する。

```
Step 1: 一対比較行列の構築（ユーザ入力）
Step 2: 優先度ベクトル（重み）の導出（固有ベクトル近似法）
Step 3: 整合性チェック（λmax / CI / RI / CR）
Step 4: トライアルスコア計算（Min-Max 正規化 + 加重和）
```

---

### Step 1: 一対比較行列

Saaty 1-9 スケールで目的関数間の重要度を比較する:

$$
A_{ij} = a_{ij} \quad \text{（i が j より } a_{ij} \text{ 倍重要）}
$$

$$
A_{ji} = \frac{1}{a_{ij}} \quad \text{（逆数の法則）}
$$

$$
A_{ii} = 1.0 \quad \text{（対角成分）}
$$

**Saaty スケール:**

| 強度 | 定義             |
| ---- | ---------------- |
| 1    | 同等に重要       |
| 3    | 少し重要         |
| 5    | かなり重要       |
| 7    | 非常に重要       |
| 9    | 極めて重要       |
| 2,4,6,8 | 中間値       |

---

### Step 2: 優先度ベクトルの導出（固有ベクトル近似法）

正確な固有値分解の代わりに、列正規化→行平均の近似法を用いる。

**Step 2-1: 列正規化**

各列 j を列合計で除算して正規化行列 B を作成:

$$
B_{ij} = \frac{A_{ij}}{\sum_k A_{kj}}
$$

**Step 2-2: 行平均**

各行の平均を計算して優先度ベクトル w を得る:

$$
w_i = \frac{1}{n} \sum_{j=1}^{n} B_{ij}
$$

これにより $\sum_i w_i = 1.0$ となる重みベクトルが得られる。

---

### Step 3: 整合性チェック

一対比較行列の論理的整合性を検証する。

**最大固有値の近似:**

$$
\lambda_{\max} = \sum_{j=1}^{n} \left(\sum_{i=1}^{n} A_{ij}\right) \times w_j
$$

**整合性指標（CI）:**

$$
CI = \frac{\lambda_{\max} - n}{n - 1}
$$

**ランダム整合性指標（RI）:**

Saaty の標準 RI テーブル:

| n  | RI   |
| -- | ---- |
| 1  | 0.00 |
| 2  | 0.00 |
| 3  | 0.58 |
| 4  | 0.90 |
| 5  | 1.12 |
| 6+ | 1.24 |

**整合性比（CR）:**

$$
CR = \frac{CI}{RI}
$$

- CR ≤ 0.10: 整合性あり（一対比較は論理的に矛盾していない）
- CR > 0.10: 整合性なし（一対比較の見直しが推奨される）
- n ≤ 2 の場合: RI = 0 となるため CR = 0 とし、常に整合ありと判定する

---

### Step 4: トライアルスコア計算（加重和法）

**Min-Max 正規化:**

各目的関数列 j について、方向（minimize / maximize）に応じて正規化する:

| 方向     | 正規化式                                              |
| -------- | ----------------------------------------------------- |
| minimize | $(V_{\max,j} - v_{ij}) / (V_{\max,j} - V_{\min,j})$ |
| maximize | $(v_{ij} - V_{\min,j}) / (V_{\max,j} - V_{\min,j})$ |

$V_{\max,j} = V_{\min,j}$ の場合は正規化値 = 0.0 とする。

**加重和スコア:**

$$
\mathrm{score}_i = \sum_{j=1}^{n} w_j \times \hat{v}_{ij}
$$

ここで $\hat{v}_{ij}$ は正規化後の値。スコア降順でランキングする。

---

## 実装の詳細

### 上三角格納（`ahp.rs`）

一対比較行列の対称性（A[j][i] = 1/A[i][j]）を利用し、上三角成分のみを格納する:

$$
\text{インデックス: } \mathrm{upper\_tri\_index}(n, i, j) = \frac{i \times (2n - i - 1)}{2} + (j - i - 1)
$$

$$
\text{格納長: } \frac{n(n-1)}{2}
$$

### NaN トライアルの扱い

いずれかの目的関数値が `NaN` のトライアルは正規化時に除外され、スコアは `0.0`、ランキング末尾に配置される。

### `compute_ahp()` の処理フロー（`rust_core/src/mcdm/ahp.rs`）

```
1. validate_inputs()          → 入力サイズの整合性チェック
2. 上三角 → n×n 行列復元      → 逆数関係で下三角を補完
3. 列正規化 → 行平均          → 優先度ベクトル w を導出
4. λmax / CI / CR 計算        → 整合性チェック
5. Min-Max 正規化 + 加重和    → 各トライアルのスコア
6. スコア降順ソート            → ranked_indices を生成
```

### 計算量

| ステップ         | 計算量                 |
| ---------------- | ---------------------- |
| 行列復元         | O(n²)                  |
| 優先度ベクトル   | O(n²)                  |
| 整合性チェック   | O(n²)                  |
| スコア計算       | O(m × n)               |
| ソート           | O(m log m)             |
| **合計**         | **O(n² + m × n + m log m)** |

n（目的関数数）は通常 2-6 であり、m（トライアル数）に対して線形。

---

## 特性・限界

**強み:**

- 一対比較という直感的な方法で重みを導出できる（直接スライダー入力に比べて設定が容易）
- 整合性比（CR）により、矛盾した比較の検出が可能
- minimize / maximize が混在していても対応可能
- スコアが [0, 1] に収まるため直感的に解釈しやすい

**弱み:**

- 目的関数数 n が大きいと一対比較の回数が n(n-1)/2 に増加し、整合性維持が困難
- n ≤ 2 の場合、CR は常に 0 となり整合性チェックが無意味
- Saaty スケール（1-9）の離散値により、微妙な重要度差を表現できない
- Min-Max 正規化は外れ値に敏感

---

## 使用場面の目安

```
多目的最適化結果から「総合的に優秀なトライアル」を選びたい
  ↓
目的関数間の相対的重要度を一対比較で表現したい
  ↓
AHP ランキング

目的関数が 3〜5 個の場合に最も適している。
目的関数が 6 個以上の場合は重みの直接入力（TOPSIS/VIKOR）を検討する。
```

---

## UI での操作

`AhpRankChart` / `AhpTable` コンポーネントで以下を操作できる:

- **一対比較行列グリッド**: 上三角成分を DragValue（1.0-9.0）で入力、下三角は自動表示
- **Run ボタン**: AHP 計算を実行（バックグラウンドタスク）
- **CR 表示**: 緑（CR ≤ 0.10 整合あり）/ 赤（CR > 0.10 整合なし）
- **優先度ベクトルバーチャート**: 各目的関数の重みを可視化
- **上位 N 件表示**: 5 / 10 / 20 件を切り替え（`AhpTable`）

---

## 参考文献

- Saaty, T. L. (1977). A scaling method for priorities in hierarchical structures. *Journal of Mathematical Psychology*, 15(3), 234-281.
- Saaty, T. L. (1980). *The Analytic Hierarchy Process*. McGraw-Hill.

---

## 実装ファイル

- `rust_core/src/mcdm/ahp.rs` — AHP アルゴリズム本体
- `egui-app/src/state/results.rs` — `AhpResult` 型定義
- `egui-app/src/state/messages.rs` — `AppMessage::AhpDone` メッセージ
- `egui-app/src/state/message_handler.rs` — AhpDone ハンドリング
- `egui-app/src/ui/widgets/ahp_chart.rs` — AhpRankChart / AhpTable UI
- `egui-app/src/ui/chart_registry.rs` — ChartId::AhpRankChart / AhpTable ディスパッチ
