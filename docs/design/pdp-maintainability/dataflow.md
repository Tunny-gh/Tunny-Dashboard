# pdp-maintainability データフロー図

**作成日**: 2026-05-04
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連要件定義**: [requirements.md](../../spec/pdp-maintainability/requirements.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実なフロー
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測によるフロー
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測によるフロー

---

## 全体変更概要 🔵

**信頼性**: 🔵 *要件定義書 + コード直接分析より*

```
リファクタリング前                      リファクタリング後
─────────────────────                  ─────────────────────────────────────
kriging_core.rs                        kriging_core.rs
  compute_pdp_1d_kriging_raw()           compute_pdp_1d_kriging_raw()
    [正規化ブロック 20行]          →       normalize_x_minmax() ← utils.rs
    [y正規化ブロック 5行]          →       normalize_y()        ← utils.rs
    [R²ブロック 8行]               →       r_squared()          ← utils.rs
  compute_pdp_1d_sparse_kriging_raw()
    [正規化ブロック 20行 ×2]       →       normalize_x_minmax() ← utils.rs (1回)
    [y正規化ブロック 5行 ×2]       →       normalize_y()        ← utils.rs (1回)
    [R²ブロック 8行]               →       r_squared()          ← utils.rs
  compute_pdp_2d_kriging_raw()
    [R²ブロック 8行]               →       r_squared()          ← utils.rs
  compute_pdp_2d_sparse_kriging_raw()
    [R²ブロック 8行]               →       r_squared()          ← utils.rs

api.rs
  compute_pdp()
    [x_matrix抽出 10行]            →       extract_xy() ← utils.rs
    [y抽出 5行]                    →       (同上)
  compute_pdp_2d()
    [x_matrix抽出 10行]            →       extract_xy() ← utils.rs
    [y抽出 5行]                    →       (同上)
```

---

## 1. normalize_x_minmax のデータフロー 🔵

**信頼性**: 🔵 *要件定義REQ-101 + コード分析より*

```
入力: x_matrix: &[Vec<f64>]   (N行 × D列)
        │
        ├─ パス1（列走査, O(N×D)）
        │    for d in 0..D:
        │      col[d] = x_matrix.iter().map(|r| r[d])
        │      min[d] = col[d].fold(INFINITY, f64::min)
        │      max[d] = col[d].fold(NEG_INFINITY, f64::max)
        │      range[d] = (max[d] - min[d]).max(EPSILON)
        │    → col_stats: Vec<(f64, f64)>  (min, range) per dim
        │
        └─ パス2（行走査, O(N×D)）
             for row in x_matrix:
               for d, v in row.enumerate():
                 x_norm[row][d] = (v - min[d]) / range[d]
             → x_norm: Vec<Vec<f64>>  (各列が[0,1]に正規化)

出力: (col_stats, x_norm)
```

**エッジケース**:
- 定数列: `range = 0.0` → `range.max(EPSILON)` でクランプ → `x_norm` は全て `0.0`
- 空スライス: `n_dims = 0` → 空の Vec を返す

---

## 2. normalize_y のデータフロー 🔵

**信頼性**: 🔵 *要件定義REQ-102 + 既存コードパターンより*

```
入力: y: &[f64]   (N個)
        │
        ├─ y_mean = sum(y) / N
        ├─ var    = sum((v - y_mean)²) / N
        ├─ y_std  = sqrt(var).max(EPSILON)
        └─ y_norm = y.iter().map(|&v| (v - y_mean) / y_std)

出力: (y_mean: f64, y_std: f64, y_norm: Vec<f64>)
```

**エッジケース**:
- 定数 y: `var = 0` → `y_std = EPSILON`（ゼロ除算ガード）
- 空スライス: `y_mean = 0`, `y_std = EPSILON`

---

## 3. r_squared のデータフロー 🔵

**信頼性**: 🔵 *要件定義REQ-201 + 既存コードパターンより*

```
入力: y_actual: &[f64], y_pred: &[f64]   (同一長 N)
        │
        ├─ y_mean  = sum(y_actual) / N
        ├─ ss_tot  = sum((v - y_mean)²)  for v in y_actual
        ├─ ss_res  = sum((y_actual[i] - y_pred[i])²)
        │
        └─ if ss_tot < EPSILON:
               r_squared = 1.0    (定数 y の場合)
           else:
               r_squared = 1.0 - ss_res / ss_tot

出力: r_squared: f64   ([-∞, 1.0])
```

---

## 4. extract_xy のデータフロー 🔵

**信頼性**: 🔵 *要件定義REQ-301 + コード直接分析より*

```
入力: df: &DataFrame, param_names: &[String], objective_name: &str
        │
        ├─ n = df.row_count()
        │
        ├─ x_matrix 構築 (O(N × P)):
        │    for i in 0..N:
        │      row[j] = df.get_numeric_column(param_names[j])
        │                  .and_then(|c| c.get(i))
        │                  .copied()
        │                  .unwrap_or(0.0)
        │    → x_matrix: Vec<Vec<f64>>  (N × P)
        │
        └─ y 構築 (O(N)):
             for i in 0..N:
               y[i] = df.get_numeric_column(objective_name)
                         .and_then(|c| c.get(i))
                         .copied()
                         .unwrap_or(0.0)
             → y: Vec<f64>  (N)

出力: (x_matrix, y)
```

---

## 5. compute_pdp_1d_kriging_raw のデータフロー（リファクタリング後） 🔵

**信頼性**: 🔵 *要件定義REQ-103 + コード分析より*

```
compute_pdp_1d_kriging_raw(x_matrix, y, param_names, objective_name, target_param_idx, n_grid)
  │
  ├─ 入力検証（n, n_dims チェック）
  │
  ├─ ★ normalize_x_minmax(x_matrix)
  │    → (col_stats, x_norm)
  │
  ├─ ★ normalize_y(y)
  │    → (y_mean, y_std, y_norm)
  │
  ├─ gaussian_process::train_gp(x_norm.clone(), y_norm, 100, 42)
  │    → GpModel
  │
  ├─ centroid_norm 計算（各次元の正規化平均）
  │
  ├─ グリッドループ（n_grid 点）
  │    for v in grid:
  │      ★ rayon::par_iter（meanのみ）
  │         x_norm.par_iter().map(|row| {
  │           pt = row.clone(); pt[target_idx] = v_norm
  │           predict_mean(&model, &pt)
  │         }).sum() / N  → mean_avg
  │
  │      （variance は centroid 単点: O(N²), 変更なし）
  │      var_centroid = predict_variance(&model, &centroid_pt)
  │
  │      pdp_orig = mean_avg * y_std + y_mean
  │      std_orig = sqrt(var_centroid) * y_std
  │
  ├─ ★ r_squared(y, y_pred_from_model)
  │    → r_squared: f64
  │
  └─ PdpResult1d { grid, values, r_squared, y_upper, y_lower }
```

---

## 6. compute_pdp_1d_sparse_kriging_raw のデータフロー（リファクタリング後） 🔵

**信頼性**: 🔵 *要件定義REQ-502 + コード分析より*

```
compute_pdp_1d_sparse_kriging_raw(x_matrix, y, param_names, objective_name, target_param_idx, n_grid)
  │
  ├─ 入力検証
  │
  ├─ ★ normalize_x_minmax(x_matrix)  [1回に統合, 従来は2回呼ばれていた]
  │    → (col_stats, x_norm)
  │
  ├─ ★ normalize_y(y)  [1回に統合]
  │    → (y_mean, y_std, y_norm)
  │
  ├─ Step 1: GP subsample（ハイパーパラメータ取得用）
  │    gaussian_process::train_gp(x_norm.clone(), y_norm.clone(), 100, 42)
  │
  ├─ Step 2: K-means 誘導点選択（M=20）
  │    sparse_fitc::select_inducing_points_kmeans(gp_x_flat, gp_n, n_dims, M_1D, 42)
  │
  ├─ Step 3: FITC 訓練（O(N × M²)）
  │    sparse_fitc::fitc_train(x_flat, z, y_norm, params, n, m)
  │
  ├─ PDP グリッドループ ★ rayon 並列化
  │    grid.par_iter().map(|&v| {
  │      mean_norm = x_norm.iter().map(|row| { pt=...; fitc_predict_mean }).sum() / N
  │      var_avg   = x_norm.iter().map(|row| { pt=...; fitc_predict_variance }).sum() / N
  │      pdp_orig  = mean_norm * y_std + y_mean
  │      std_orig  = sqrt(var_avg) * y_std
  │      (pdp_orig, pdp_orig + 1.96*std_orig, pdp_orig - 1.96*std_orig)
  │    }).collect::<Vec<(f64,f64,f64)>>()
  │
  │    → アンパック: pdp_values, y_upper_vec, y_lower_vec
  │      （par_iter().collect() は順序保持）
  │
  ├─ ★ r_squared(y, y_pred)
  │
  └─ PdpResult1d { grid, values, r_squared, y_upper, y_lower }
```

---

## 7. rayon の並列実行フロー 🔵

**信頼性**: 🔵 *rayon 公開API・Rustドキュメントより*

```
シングルスレッド（変更前）:
  grid[0] → [N回のpredict_mean] → mean_avg[0]
  grid[1] → [N回のpredict_mean] → mean_avg[1]
  ...
  grid[G-1] → [N回のpredict_mean] → mean_avg[G-1]
  全体: O(G × N × M) = 逐次実行

rayon par_iter（変更後）:
  grid[0..G] を rayon のスレッドプールに分割
  ┌──────────────────────────────────────────────┐
  │ Thread 1: grid[0..G/4]  → mean_avg[0..G/4]  │
  │ Thread 2: grid[G/4..G/2] → mean_avg[G/4..G/2]│
  │ Thread 3: grid[G/2..3G/4] → ...             │
  │ Thread 4: grid[3G/4..G] → ...               │
  └──────────────────────────────────────────────┘
  collect() で順序保持された Vec<_> に集約
  理論的高速化: ×(コア数)
```

---

## 8. compute_pdp_2d のデータフロー（api.rs リファクタリング後） 🔵

**信頼性**: 🔵 *要件定義REQ-301/302 + コード分析より*

```
compute_pdp_2d(param1_name, param2_name, objective_name, n_grid, model_type)
  │
  └─ with_active_df(|df| {
         param_names = df.param_col_names()
         n = df.row_count()
         p1_idx = param_names.position(param1_name)?
         p2_idx = param_names.position(param2_name)?
         _      = objective_names.position(objective_name)?

         ★ (x_matrix, y) = extract_xy(df, &param_names, objective_name)
         //  ↑ 従来の 15行のコードブロックを1行に置き換え

         match model_type { ... }   // 変更なし
     })
```

---

## 信頼性レベルサマリー

- 🔵 青信号: 8件 (100%)
- 🟡 黄信号: 0件 (0%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質

## 関連文書

- **アーキテクチャ**: [architecture.md](architecture.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **要件定義**: [requirements.md](../../spec/pdp-maintainability/requirements.md)
