# rayon 導入による並列化高速化 データフロー図

**作成日**: 2026-05-04  
**関連要件定義**: [requirements.md](../../spec/rayon-performance-optimization/requirements.md)  
**アーキテクチャ**: [architecture.md](architecture.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: 設計文書・ユーザヒアリングから妥当な推測による設計

---

## 1. Sensitivity 目的変数ループ（REQ-001） 🔵

**信頼性**: 🔵 *コードベース調査・ユーザヒアリングより*

```mermaid
flowchart TD
    A["compute_sensitivity_all(df)"] --> B["build x_matrix\n(n × p)"]
    B --> C["objective_columns\n[y_0, y_1, ..., y_N]"]

    C --> D["par_iter() — N スレッド同時実行"]

    D --> E0["Thread-0\nrun_tree_metric_for_objective\n(metric, x_matrix, y_0)"]
    D --> E1["Thread-1\nrun_tree_metric_for_objective\n(metric, x_matrix, y_1)"]
    D --> EN["Thread-N\nrun_tree_metric_for_objective\n(metric, x_matrix, y_N)"]

    E0 --> F0["prepare_training_data\n→ LightGBM train\n→ (importances_0, r²_0)"]
    E1 --> F1["prepare_training_data\n→ LightGBM train\n→ (importances_1, r²_1)"]
    EN --> FN["prepare_training_data\n→ LightGBM train\n→ (importances_N, r²_N)"]

    F0 --> G[".collect()"]
    F1 --> G
    FN --> G

    G --> H["transpose_to_tree_result\n→ TreeImportanceResult"]
```

**共有データ（読み取り専用）**:
- `x_matrix: &[Vec<f64>]` — 全スレッドから不変参照
- `metric: &M` — `M: TreeMetric + Sync`

**スレッドローカル**:
- `PreparedData`（LightGBM 訓練・評価データ）
- LightGBM Booster インスタンス

---

## 2. Sobol 指標計算（REQ-002） 🔵

**信頼性**: 🔵 *コードベース調査・ユーザヒアリングより*

### 2-A: サロゲート構築（per-objective Ridge 並列）

```mermaid
flowchart TD
    A["build_sobol_surrogate\n(x_matrix, y_matrix, n_params, alpha)"] --> B["x の標準化\n(param_means, param_stds)"]
    B --> C["quad_feats 計算\n(x_quad_std: n × n_quad)"]

    C --> D["y_matrix.par_iter() — N_obj スレッド同時"]

    D --> E0["Thread-0\ny_matrix[0]\n→ y_center\n→ compute_ridge(x_quad_std, y_centered)"]
    D --> E1["Thread-1\ny_matrix[1]\n→ y_center\n→ compute_ridge(...)"]
    D --> EN["Thread-N\ny_matrix[N]\n→ compute_ridge(...)"]

    E0 --> F["(beta_0, intercept_0, r²_0)"]
    E1 --> F2["(beta_1, intercept_1, r²_1)"]
    EN --> FN["(beta_N, intercept_N, r²_N)"]

    F --> G[".collect() → triplets"]
    F2 --> G
    FN --> G

    G --> H["SobolSurrogate { betas, intercepts, r_squared }"]
```

### 2-B: Monte Carlo サンプリング → 指標計算（per-param 並列）

```mermaid
flowchart TD
    A["surrogate 構築完了"] --> B["lcg_next で mat_a / mat_b 生成\n(直列 — RNG 状態が順次依存)"]

    B --> C["f_a, f_b 計算\n(0..n_objectives).into_par_iter()"]

    C --> D0["Thread-0: f_a[0], f_b[0]"]
    C --> D1["Thread-1: f_a[1], f_b[1]"]
    C --> DN["Thread-N: f_a[N], f_b[N]"]

    D0 --> E[".collect() → f_a, f_b"]
    D1 --> E
    DN --> E

    E --> F["(0..n_params).into_par_iter()"]

    F --> G0["Thread-pi=0\nmat_ab_0 生成\n→ f_ab_pi_0 (per-obj)\n→ first_order[0], total[0]"]
    F --> G1["Thread-pi=1\nmat_ab_1 生成\n→ f_ab_pi_1\n→ first_order[1], total[1]"]
    F --> GN["Thread-pi=p\n..."]

    G0 --> H[".collect() → sobol_pairs"]
    G1 --> H
    GN --> H

    H --> I["SobolResult { first_order, total_effect }"]
```

**共有データ（読み取り専用）**:
- `surrogate: SobolSurrogate`（全スレッドから `&` 参照）
- `mat_a`, `mat_b`（生成後は読み取り専用）
- `f_a`, `f_b`（per-param ループ内で読み取り専用）

**直列維持**:
- `mat_a` / `mat_b` の LCG サンプリング（`rng_state` が順次依存）

---

## 3. RandomForest 木構築（REQ-003） 🔵

**信頼性**: 🔵 *コードベース調査・ユーザヒアリングより*

```mermaid
flowchart TD
    A["RandomForest::train\n(x, y, n_trees, max_depth, min_samples_leaf, seed)"] --> B["feature_indices = 0..p"]

    B --> C["(0..n_trees).into_par_iter()"]

    C --> D0["Thread-tree_idx=0\nLcg::new(seed + 0)\nbootstrap サンプリング\nbuild_tree(...)"]
    C --> D1["Thread-tree_idx=1\nLcg::new(seed + 1)\nbootstrap サンプリング\nbuild_tree(...)"]
    C --> DN["Thread-tree_idx=T\nLcg::new(seed + T)\n..."]

    D0 --> E["DecisionTree { root: ... }"]
    D1 --> E2["DecisionTree { root: ... }"]
    DN --> EN["DecisionTree { root: ... }"]

    E --> F[".collect() → Vec<DecisionTree>"]
    E2 --> F
    EN --> F

    F --> G["RandomForest { trees }"]
```

**スレッドローカル**:
- `Lcg` インスタンス（`seed + tree_idx` で初期化）
- `x_boot`, `y_boot`（bootstrap サンプル）

**共有データ（読み取り専用）**:
- `x: &[Vec<f64>]`, `y: &[f64]`
- `feature_indices: &[usize]`

---

## 4. Permutation Feature Importance（REQ-004） 🔵

**信頼性**: 🔵 *コードベース調査・ユーザヒアリングより*

```mermaid
flowchart TD
    A["compute_from_prepared(data)"] --> B["LightGBM 訓練\n(直列維持)"]
    B --> C["baseline_mse 計算\n(直列維持)"]

    C --> D["(0..p).into_par_iter()"]

    D --> E0["Thread-feat=0\nx_work = x_eval.to_vec()\norig_col = col 0 のコピー\n×PFI_N_REPEATS: permute+mse"]
    D --> E1["Thread-feat=1\nx_work = x_eval.to_vec()\norig_col = col 1 のコピー\n×PFI_N_REPEATS: permute+mse"]
    D --> EN["Thread-feat=p\n..."]

    E0 --> F0["delta_sum_0 / N_REPEATS"]
    E1 --> F1["delta_sum_1 / N_REPEATS"]
    EN --> FN["delta_sum_p / N_REPEATS"]

    F0 --> G[".collect() → importances (raw)"]
    F1 --> G
    FN --> G

    G --> H["normalize(importances)"]
    H --> I["(importances, r_squared)"]
```

**スレッドローカル**:
- `x_work: Vec<Vec<f64>>`（各スレッドが `x_eval` の独立コピーを保持）
- `orig_col: Vec<f64>`

**共有データ（読み取り専用）**:
- `booster` — LightGBM prediction はスレッドセーフ
- `y_eval: &[f64]`
- `baseline_mse: f64`

---

## 全体データフロー（egui → 計算 → UI） 🔵

**信頼性**: 🔵 *egui-app コードベース調査より*

```mermaid
sequenceDiagram
    participant UI as egui UI スレッド
    participant BG as バックグラウンドスレッド
    participant Core as rust_core (rayon)

    UI->>BG: 計算リクエスト（非同期チャネル）
    BG->>Core: compute_sensitivity() など
    Core->>Core: rayon::par_iter() で並列実行
    Note over Core: CPU コア数分のスレッドプール<br/>rayon グローバル ThreadPool
    Core-->>BG: SensitivityResult / SobolResult
    BG-->>UI: 結果をステート更新（チャネル）
    UI->>UI: 次フレームで再描画
```

> egui UI スレッドとは独立して計算が実行されるため、UI のレスポンスは維持される。
> rayon スレッドプールのスレッド数制限は行わない（REQ-401）。
