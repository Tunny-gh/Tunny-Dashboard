# sensitivity-analysis-statistical-importance データフロー図

**作成日**: 2026-04-25
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連要件定義**: [requirements.md](../../spec/sensitivity-analysis-statistical-importance/requirements.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実なフロー
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測によるフロー
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測によるフロー

---

## システム全体のデータフロー 🔵

**信頼性**: 🔵 *既存4層アーキテクチャ・ユーザヒアリングより*

```
DataFrame（最適化トライアルデータ）
         │
         ▼
┌─────────────────────────────────┐
│   Layer 2: メトリクス計算        │
│  ┌──────────┐  ┌─────────────┐ │
│  │Spearman  │  │Ridge        │ │
│  │+Stats    │  │+Stats       │ │
│  └──────────┘  └─────────────┘ │
│  ┌──────────┐  ┌─────────────┐ │
│  │RF-ANOVA  │  │MDI / SHAP   │ │
│  │(per-tree)│  │(per-tree)   │ │
│  └──────────┘  └─────────────┘ │
│  ┌──────────────────────────┐  │
│  │Sobol + Jansen分散        │  │
│  └──────────────────────────┘  │
└─────────────────────────────────┘
         │ SensitivityResult / SobolResult
         │ (統計フィールド込み)
         ▼
┌─────────────────────────────────┐
│   Layer 1: 統計コア              │
│  statistics.rs                  │
│  ├── t_two_sided_p()            │
│  ├── z_two_sided_p()            │
│  └── bonferroni_adjust()        │
└─────────────────────────────────┘
         │ p_adjusted, CI
         ▼
┌─────────────────────────────────┐
│   Layer 4: egui UI              │
│  importance_chart.rs            │
│  ├── バー描画                    │
│  ├── エラーバーオーバーレイ       │
│  └── p値テキスト インライン       │
└─────────────────────────────────┘
```

---

## 主要機能のデータフロー

### フロー 1: Spearman統計計算 🔵

**信頼性**: 🔵 *REQ-STAT-010〜013・Spearman理論より*

**関連要件**: REQ-STAT-010, REQ-STAT-011, REQ-STAT-012, REQ-STAT-013

```
x: &[f64]  y: &[f64]
      │           │
      └─────┬─────┘
            │
            ▼
  compute_spearman(x, y) → rho: f64           [既存]
            │
            ▼
  n = x.len()
  if n < 4 → return SpearmanStats { p_value: None, ci: None }
            │
            ▼
  t = rho * sqrt(n-2) / sqrt(1 - rho²)
  ※ rho = ±1.0 の場合 → p = 0.0（退化処理）
            │
            ▼
  p_raw = t_two_sided_p(t, df=(n-2))
            │
            ▼
  Fisher z変換:
    z = atanh(rho)
    se_z = 1 / sqrt(n-3)
    ci_lower = tanh(z - 1.96*se_z)
    ci_upper = tanh(z + 1.96*se_z)
            │
            ▼
  SpearmanStats { rho, p_value_raw: Some(p_raw), ci_lower, ci_upper }
```

---

### フロー 2: Ridge統計計算（対角逆行列） 🔵

**信頼性**: 🔵 *REQ-STAT-020〜025・Ridge理論・ユーザヒアリング（対角成分選択）より*

**関連要件**: REQ-STAT-020, REQ-STAT-021, REQ-STAT-022, REQ-STAT-023

```
x_matrix: &[Vec<f64>]   y: &[f64]   alpha: f64
                │               │
                ▼               ▼
    transpose_and_standardize() → x_cols  [既存]
                │
                ▼
    XTX + αI を構築 (A: p×p)             [既存をキャッシュ]
    XTY を計算
                │
                ├──── ガウス消去 (A, XTY) → β    [既存]
                │         ↓
                │    y_hat = X_std × β
                │    RSS = Σ(y - y_hat)²          [既存]
                │    R² = 1 - RSS/SS_tot          [既存]
                │
                ├──── σ² = RSS / (n - p - 1)      [新規]
                │    ※ n ≤ p+2 → None
                │
                ├──── 対角逆行列 A^{-1}_{jj}      [新規]
                │    for j in 0..p:
                │      e_j = [0,..,1,..,0] (標準基底)
                │      Ax = e_j をガウス消去で解く
                │      A_inv_jj = x[j]
                │
                └──── SE_j = sqrt(σ² × A_inv_jj)  [新規]
                       t_j = β_j / SE_j
                       p_j = t_two_sided_p(t_j, n-p-1)
                       ci_j = β_j ± t_{0.025,df} × SE_j
                         ↓
                     RidgeResult { beta, r_squared, std_errors, p_values, ci_lower, ci_upper, is_approximate: true }
```

---

### フロー 3: RF-ANOVAツリー間分散 🔵

**信頼性**: 🔵 *REQ-STAT-030〜034・ユーザヒアリング（ツリー間分散選択）より*

**関連要件**: REQ-STAT-030, REQ-STAT-031, REQ-STAT-032

```
x_matrix: &[Vec<f64>]   y: &[f64]
            │
            ▼
  80/20 holdout分割    [既存]
  ランダムフォレスト訓練  [既存]
            │
            ▼
  ┌─ 変更点: 木ごとにimportanceを記録 ─┐
  │  tree_importances: Vec<Vec<f64>>   │
  │       [tree_idx][param_idx]        │
  │  for t in 0..n_trees (=100):      │
  │    for j in 0..p:                 │
  │      permuted_mse = eval_permuted(j)
  │      tree_importances[t][j] =     │
  │        max(permuted_mse - baseline_mse, 0)
  └────────────────────────────────────┘
            │
            ▼
  raw_mean_j = mean(tree_importances[*][j])   (正規化前)
  se_j = std(tree_importances[*][j]) / sqrt(100)
            │
            ▼
  if se_j < ε → p_j = None
  else:
    t_j = raw_mean_j / se_j
    p_j = t_one_sided_upper_p(t_j, df=99)
            │
            ▼
  sum_raw = sum(raw_mean_j for j)             (正規化スケール変換用)
  norm_j = raw_mean_j / sum_raw              (既存互換）
  ci_lower_j = max(norm_j - t_{0.025,99} × (se_j/sum_raw), 0)
  ci_upper_j = norm_j + t_{0.025,99} × (se_j/sum_raw)
            │
            ▼
  RfAnovaResult { importances (norm), r_squared, p_values, ci_lower, ci_upper }
```

---

### フロー 4: Sobol Jansen分散 🔵

**信頼性**: 🔵 *REQ-STAT-040〜044・Jansen (1999)より*

**関連要件**: REQ-STAT-040, REQ-STAT-041, REQ-STAT-042, REQ-STAT-043

```
(既存の Saltelli サンプリング完了済み)
f_A[k][j], f_B[k][j], f_AB_i[k][j]: Vec<Vec<f64>>
      k = obj_idx, j = sample_idx
                │
                ▼
  For each (param_idx pi, obj_idx k):
    ┌─ 既存: S_i, ST_i 計算 ─────────────┐
    │  var_y = Var(f_A[k])                │
    │  s_i  = Σ f_B*(f_AB-f_A) / (N*Var_Y)│
    │  st_i = Σ (f_A-f_AB)² / (2N*Var_Y) │
    └────────────────────────────────────┘
                │
                ▼  [新規追加]
    ┌─ Jansen分散 ───────────────────────┐
    │  d_j = f_B[k][j] * (f_AB_i[k][j] - f_A[k][j])
    │  var_d = Var(d_j over j=0..N)      │
    │  var_s_i = var_d / (N × Var_Y²)    │
    │                                    │
    │  se_s_i = sqrt(var_s_i)            │
    │  ci_lower = max(s_i - 1.96*se, 0) │
    │  ci_upper = s_i + 1.96*se          │
    │                                    │
    │  if se < ε → p = None              │
    │  else: z = s_i / se                │
    │         p = z_two_sided_p(z)       │
    └────────────────────────────────────┘
                │
                ▼
  SobolResult { first_order, total_effect, r_squared, n_samples,
                first_order_ci_lower, first_order_ci_upper, first_order_p_values,
                total_effect_ci_lower, total_effect_ci_upper, total_effect_p_values }
```

---

### フロー 5: Bonferroni補正と有意性マーク 🔵

**信頼性**: 🔵 *REQ-STAT-050〜052・ユーザヒアリング（Bonferroni選択）より*

```
raw p_values: Vec<Option<f64>>   n_params: usize
                │
                ▼
  for each p_raw in p_values:
    p_adj = p_raw.map(|p| (p * n_params as f64).min(1.0))
                │
                ▼
  significance_mark(p_adj):
    p_adj < 0.001 → "***"
    p_adj < 0.01  → "**"
    p_adj < 0.05  → "*"
    else          → ""
```

---

### フロー 6: egui UIでの統計情報表示 🔵

**信頼性**: 🔵 *ユーザヒアリング（エラーバーオーバーレイ選択）・egui実装より*

```
SensitivityResult（統計フィールド込み）
          │
          ▼
  compute_sorted_importance() → Vec<(name, score, ci_lower, ci_upper, p_adj, mark)>
          │
          ▼ (ScrollArea内のループ)
  ┌──────────────────────────────────────────────────────────┐
  │ [param_name]  [=========bar=========|CI|]  0.032 *      │
  │                                    │ ↑                   │
  │                           I字型エラーバー                 │
  │                           (薄青色、1.5px)                │
  └──────────────────────────────────────────────────────────┘

  描画手順:
  1. ラベル描画（変更なし）
  2. バー描画（変更なし）
  3. エラーバー（ci_lower, ci_upper が Some の場合）:
     lo_x = bar_min + ci_lower/max_score * bar_max_width
     hi_x = bar_min + ci_upper/max_score * bar_max_width
     横線: lo_x → hi_x（y = cy）
     縦線: (lo_x, cy±4) と (hi_x, cy±4)
  4. p値テキスト:
     Some(p_adj) → format!("{p_adj:.3} {mark}")
     None → format!("{score:.3}")     （統計情報なし）
     Ridge → format!("~{p_adj:.3} {mark}") （近似バイアス注記）
```

---

## エラーハンドリングフロー 🟡

**信頼性**: 🟡 *EDGE-STAT-001〜003から妥当な推測*

```
計算中のエラー状態
       │
       ▼
  n < 4（Spearman/Ridge）
  or se = 0（ツリー間分散）
  or Var_Y = 0（Sobol）
  or df ≤ 0（Ridge）
       │
       ▼ → 統計フィールドは None のまま
           UIは score のみ表示（マーク・CIなし）
           Spearman: "n/a"
           Sobol R² < 0.5: UI に "(low surrogate quality)" 警告
```

---

## 信頼性レベルサマリー

- 🔵 青信号: 11件 (92%)
- 🟡 黄信号: 1件 (8%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
