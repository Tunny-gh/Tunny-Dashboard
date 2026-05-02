# Permutation Feature Importance 実装ガイド

**作成日**: 2026-05-02
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連データフロー**: [dataflow.md](dataflow.md)

---

## 実装順序

以下の順序で実装することで、コンパイルエラーを最小化できる。

```
Step 1: rust_core/src/sensitivity/types.rs
Step 2: rust_core/src/sensitivity/permutation.rs（新規作成）
Step 3: rust_core/src/sensitivity/mod.rs
Step 4: rust_core/src/sensitivity/analysis/full.rs
Step 5: egui-app/src/state/results.rs
Step 6: egui-app/src/ui/widgets/importance_chart.rs
Step 7: egui-app/src/ui/chart_registry.rs
```

---

## Step 1: types.rs の変更

**ファイル**: `rust_core/src/sensitivity/types.rs`

### 1-A: SensitivityMetric に Permutation を追加

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum SensitivityMetric {
    Spearman,
    Ridge,
    RfAnova,
    Mdi,
    Shap,
    Permutation,   // ← 追加
}
```

### 1-B: PermutationResult 構造体を追加

```rust
#[derive(Debug, Clone)]
pub struct PermutationResult {
    pub importances: Vec<Vec<f64>>, // [param][objective]
    pub r_squared:   Vec<f64>,      // [objective]
}
```

### 1-C: SensitivityResult に permutation フィールドを追加

```rust
pub struct SensitivityResult {
    pub param_names:    Vec<String>,
    pub objective_names: Vec<String>,
    pub spearman:       Vec<Vec<f64>>,
    pub ridge:          Vec<RidgeResult>,
    pub rf_anova:       Option<RfAnovaResult>,
    pub mdi:            Option<MdiResult>,
    pub shap:           Option<ShapResult>,
    pub permutation:    Option<PermutationResult>,   // ← 追加
}
```

---

## Step 2: permutation.rs の新規作成

**ファイル**: `rust_core/src/sensitivity/permutation.rs`

```rust
use super::data::sample_rows;
use crate::core::lgbm::{lgbm_mse, mse_to_r_squared, train_lgbm_rf, LgbmRfConfig};
use crate::core::random_forest::Lcg;

const N_REPEATS:         usize = 5;
const PFI_MAX_ROWS:      usize = 2_000;
const PFI_SEED_BASE:     u64   = 42;
const PFI_SPLIT_SEED:    u64   = 43;
const PFI_TREES:         usize = 100;
const PFI_MAX_DEPTH:     i32   = 10;
const PFI_MIN_DATA_LEAF: i32   = 2;

/// Permutation Feature Importance via LightGBM Random Forest (n_repeats=5).
/// Returns (importances_normalized, r_squared).
/// importances.sum() ≈ 1.0 when valid data exists.
pub fn compute_permutation_importances(x_matrix: &[Vec<f64>], y: &[f64]) -> (Vec<f64>, f64) {
    let n = y.len();
    if n == 0 || x_matrix.is_empty() || x_matrix.len() != n {
        return (vec![], 0.0);
    }

    let p = x_matrix[0].len();
    if p == 0 {
        return (vec![], 0.0);
    }

    // NaN/Inf フィルタリング
    let valid_indices: Vec<usize> = (0..n)
        .filter(|&i| y[i].is_finite() && x_matrix[i].iter().all(|v| v.is_finite()))
        .collect();

    let n_valid = valid_indices.len();
    if n_valid < 2 {
        return (vec![0.0; p], 0.0);
    }

    // フィルタリング & ダウンサンプリング
    let (x_data, y_data) = if n_valid < n {
        let x_clean: Vec<Vec<f64>> = valid_indices.iter().map(|&i| x_matrix[i].clone()).collect();
        let y_clean: Vec<f64> = valid_indices.iter().map(|&i| y[i]).collect();
        if n_valid > PFI_MAX_ROWS {
            sample_rows(&x_clean, &y_clean, PFI_MAX_ROWS, PFI_SEED_BASE)
        } else {
            (x_clean, y_clean)
        }
    } else if n > PFI_MAX_ROWS {
        sample_rows(x_matrix, y, PFI_MAX_ROWS, PFI_SEED_BASE)
    } else {
        (x_matrix.to_vec(), y.to_vec())
    };

    let n = y_data.len();

    // 80/20 holdout 分割
    const MIN_EVAL: usize = 2;
    const MIN_TRAIN: usize = 2;
    let use_holdout = n >= MIN_TRAIN + MIN_EVAL;
    let split_idx = if use_holdout {
        ((n * 4) / 5).max(MIN_TRAIN)
    } else {
        n
    };

    let mut shuffle_idx: Vec<usize> = (0..n).collect();
    let mut rng_split = Lcg::new(PFI_SPLIT_SEED);
    for i in (1..n).rev() {
        let j = rng_split.next_usize(i + 1);
        shuffle_idx.swap(i, j);
    }

    let x_shuffled: Vec<Vec<f64>> = shuffle_idx.iter().map(|&i| x_data[i].clone()).collect();
    let y_shuffled: Vec<f64> = shuffle_idx.iter().map(|&i| y_data[i]).collect();

    let (x_train, x_eval, y_train, y_eval) = if use_holdout {
        (
            &x_shuffled[..split_idx],
            &x_shuffled[split_idx..],
            &y_shuffled[..split_idx],
            &y_shuffled[split_idx..],
        )
    } else {
        (
            x_shuffled.as_slice(),
            x_shuffled.as_slice(),
            y_shuffled.as_slice(),
            y_shuffled.as_slice(),
        )
    };

    // LightGBM RF 学習
    let config = LgbmRfConfig {
        num_iterations: PFI_TREES as i32,
        max_depth: PFI_MAX_DEPTH,
        min_data_in_leaf: PFI_MIN_DATA_LEAF,
        seed: PFI_SEED_BASE as i32,
        ..Default::default()
    };

    let booster = match train_lgbm_rf(x_train, y_train, &config) {
        Some(b) => b,
        None => return (vec![0.0; p], 0.0),
    };

    // ベースライン MSE
    let baseline_mse = lgbm_mse(&booster, x_eval, y_eval)
        .unwrap_or(0.0)
        .max(f64::EPSILON);

    // n_repeats=5 パーミュテーションループ
    let mut importances = vec![0.0f64; p];
    for (feature_idx, importance) in importances.iter_mut().enumerate() {
        let mut delta_sum = 0.0f64;
        for repeat_idx in 0..N_REPEATS {
            let seed = PFI_SEED_BASE + (feature_idx as u64) * (N_REPEATS as u64) + (repeat_idx as u64);
            let permuted = match permute_single_column(x_eval, feature_idx, seed) {
                Some(p) => p,
                None => continue,
            };
            let permuted_mse = lgbm_mse(&booster, &permuted, y_eval).unwrap_or(baseline_mse);
            delta_sum += (permuted_mse - baseline_mse).max(0.0);
        }
        *importance = delta_sum / N_REPEATS as f64;
    }

    normalize(&mut importances);

    let r_squared = mse_to_r_squared(baseline_mse, y_eval);
    (importances, r_squared)
}

fn permute_single_column(
    x_matrix: &[Vec<f64>],
    feature_idx: usize,
    seed: u64,
) -> Option<Vec<Vec<f64>>> {
    let n = x_matrix.len();
    if n == 0 {
        return None;
    }
    let mut column: Vec<f64> = x_matrix.iter().map(|row| row[feature_idx]).collect();
    let mut rng = Lcg::new(seed);
    for i in (1..n).rev() {
        let j = rng.next_usize(i + 1);
        column.swap(i, j);
    }
    let mut permuted = x_matrix.to_vec();
    for (i, row) in permuted.iter_mut().enumerate() {
        row[feature_idx] = column[i];
    }
    Some(permuted)
}

fn normalize(values: &mut [f64]) {
    let sum: f64 = values.iter().sum();
    if sum < f64::EPSILON {
        values.iter_mut().for_each(|v| *v = 0.0);
        return;
    }
    values.iter_mut().for_each(|v| *v /= sum);
}
```

> **注意**: `LgbmRfConfig` の `Default::default()` が `..Default::default()` で展開できない場合は、
> 既存の `rf_anova.rs` での `LgbmRfConfig { ... }` 構築パターンに合わせること。

---

## Step 3: mod.rs の変更

**ファイル**: `rust_core/src/sensitivity/mod.rs`

既存の `mod rf_anova;` 等のすぐ下に追加：

```rust
mod permutation;
pub use permutation::compute_permutation_importances;
```

---

## Step 4: full.rs の変更

**ファイル**: `rust_core/src/sensitivity/analysis/full.rs`

`compute_sensitivity_single_obj()` 内の match 文に Permutation ケースを追加。
**既存 RfAnova ケースの直後**に追加するのが最も安全：

```rust
SensitivityMetric::Permutation => {
    // x_matrix 構築（RF-Anova と同一パターン）
    let Some(ref x_mat) = x_matrix_opt else {
        return SensitivityResult {
            param_names: names,
            ..Default::default_empty()
        };
    };
    let (imp, r2) = crate::sensitivity::compute_permutation_importances(x_mat, &y);
    let importances: Vec<Vec<f64>> = imp.into_iter().map(|v| vec![v]).collect();
    SensitivityResult {
        param_names:    names,
        objective_names: vec![selected_obj],
        spearman:       vec![],
        ridge:          vec![],
        rf_anova:       None,
        mdi:            None,
        shap:           None,
        permutation:    Some(crate::sensitivity::types::PermutationResult {
            importances,
            r_squared: vec![r2],
        }),
    }
}
```

> **注意**: `Default::default_empty()` が存在しない場合は既存の RfAnova ケースの構造体構築を
> そのままコピーして `rf_anova: None` の代わりに `permutation: Some(...)` を設定する。

---

## Step 5: results.rs の変更

**ファイル**: `egui-app/src/state/results.rs`

### 5-A: PermutationResult 構造体を追加

既存の `ShapResult` 構造体の直後に追加：

```rust
#[derive(Debug, Clone)]
pub struct PermutationResult {
    pub importances: Vec<Vec<f64>>,
    pub r_squared:   Vec<f64>,
}
```

### 5-B: SensitivityResult に permutation フィールドを追加

```rust
pub struct SensitivityResult {
    pub param_names:    Vec<String>,
    pub objective_names: Vec<String>,
    pub spearman:       Vec<Vec<f64>>,
    pub ridge:          Vec<RidgeResult>,
    pub rf_anova:       Option<RfAnovaResult>,
    pub mdi:            Option<MdiResult>,
    pub shap:           Option<ShapResult>,
    pub permutation:    Option<PermutationResult>,   // ← 追加
}
```

---

## Step 6: importance_chart.rs の変更

**ファイル**: `egui-app/src/ui/widgets/importance_chart.rs`

### 6-A: ImportanceMetric に Permutation を追加

```rust
pub enum ImportanceMetric {
    Spearman,
    Ridge,
    RfAnova,
    Mdi,
    SobolFirst,
    SobolTotal,
    Shap,
    Permutation,   // ← 追加
}
```

### 6-B: label() に Permutation ケースを追加

```rust
ImportanceMetric::Permutation => "Permutation",
```

### 6-C: cache_id() に Permutation ケースを追加

```rust
ImportanceMetric::Permutation => 7,
```

### 6-D: is_sobol() は変更不要（既存の `_` または新規ケースで false を返す）

```rust
// is_sobol() は SobolFirst / SobolTotal のみ true を返す
// Permutation は既存の _ => false パターンで処理される（または明示追加）
ImportanceMetric::Permutation => false,
```

### 6-E: ComboBox の Tree-based グループに Permutation を追加

```rust
// 既存の Shap の selectable_value の直後
ui.selectable_value(
    &mut self.metric,
    ImportanceMetric::Permutation,
    ImportanceMetric::Permutation.label(),
);
```

### 6-F: compute_sorted_importance に Permutation ケースを追加

```rust
ImportanceMetric::Permutation => {
    let Some(ref perm) = result.permutation else {
        return vec![];
    };
    perm.importances
        .iter()
        .map(|param_imp| param_imp.get(obj_idx).copied().unwrap_or(0.0).abs())
        .collect()
}
```

### 6-G: R² 表示の match 文に Permutation ケースを追加

```rust
ImportanceMetric::Permutation => sensitivity
    .and_then(|r| r.permutation.as_ref())
    .and_then(|p| p.r_squared.get(obj_idx))
    .copied(),
```

### 6-H: 既存 match 文のコンパイルエラー修正

`Permutation` バリアントを追加したため、`_ => unreachable!()` が残っている箇所がある場合は
`ImportanceMetric::Permutation` のケースを明示追加する。

---

## Step 7: chart_registry.rs の変更

**ファイル**: `egui-app/src/ui/chart_registry.rs`

### 7-A: ImportanceMetric::Permutation → SensitivityMetric::Permutation マッピング

既存の match 文（`ImportanceMetric::Shap =>` の直後）に追加：

```rust
ImportanceMetric::Permutation => {
    tunny_core::sensitivity::SensitivityMetric::Permutation
}
```

### 7-B: SensitivityDone 変換に permutation フィールドを追加

`AppMessage::SensitivityDone` の `result` 構築部分に以下を追加：

```rust
permutation: r.permutation.map(|x| PermutationResult {
    importances: x.importances,
    r_squared:   x.r_squared,
}),
```

`PermutationResult` を use 宣言に追加：

```rust
use crate::state::results::{
    MdiResult, PermutationResult, RfAnovaResult, RidgeResult, SensitivityResult, ShapResult,
    SobolResult,
};
```

---

## 注意事項

### unreachable!() の削除

`chart_registry.rs` に `_ => unreachable!()` が残っている match アームがある場合、
`ImportanceMetric::Permutation` を追加後にコンパイルエラーになるため、
明示的なケースに変更する。

### LgbmRfConfig の Default

`LgbmRfConfig` に `Default` トレイトが実装されていない場合は、
既存の `rf_anova.rs` での構築方法（フィールドを明示的に列挙）を参照する。

### テスト追加

`rust_core/src/sensitivity/tests.rs` に以下のテストを追加する:

```rust
#[cfg(test)]
mod pfi_tests {
    use super::*;
    use crate::sensitivity::permutation::compute_permutation_importances;

    #[test]
    fn test_pfi_normal_case() {
        let n = 50;
        let p = 5;
        let x: Vec<Vec<f64>> = (0..n).map(|_| vec![1.0; p]).collect();
        let y: Vec<f64> = (0..n).map(|i| i as f64).collect();
        let (imp, _r2) = compute_permutation_importances(&x, &y);
        assert_eq!(imp.len(), p);
        let sum: f64 = imp.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6 || sum < f64::EPSILON);
    }

    #[test]
    fn test_pfi_single_feature() {
        let x: Vec<Vec<f64>> = (0..20).map(|i| vec![i as f64]).collect();
        let y: Vec<f64> = (0..20).map(|i| i as f64 * 2.0).collect();
        let (imp, _) = compute_permutation_importances(&x, &y);
        assert_eq!(imp.len(), 1);
        assert!((imp[0] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_pfi_nan_filtering() {
        let mut x: Vec<Vec<f64>> = (0..50).map(|i| vec![i as f64, (i * 2) as f64]).collect();
        let mut y: Vec<f64> = (0..50).map(|i| i as f64).collect();
        x[5][0] = f64::NAN;
        y[10] = f64::INFINITY;
        let (imp, _) = compute_permutation_importances(&x, &y);
        assert_eq!(imp.len(), 2);
        let sum: f64 = imp.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6 || sum < f64::EPSILON);
    }

    #[test]
    fn test_pfi_min_valid_rows() {
        let x = vec![vec![1.0], vec![2.0]];
        let y = vec![1.0, 2.0];
        let (imp, _) = compute_permutation_importances(&x, &y);
        assert_eq!(imp.len(), 1);
    }

    #[test]
    fn test_pfi_empty_input() {
        let (imp, r2) = compute_permutation_importances(&[], &[]);
        assert!(imp.is_empty());
        assert_eq!(r2, 0.0);
    }
}
```

---

## ビルド確認

実装後に以下を実行してコンパイルエラーがないことを確認する:

```bash
# rust_core のみビルド確認
rtk cargo build -p tunny-core

# 全ワークスペースのテスト
rtk cargo test

# クリップ
rtk cargo clippy
```
