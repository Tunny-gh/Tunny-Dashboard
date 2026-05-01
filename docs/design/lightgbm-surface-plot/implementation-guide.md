# LightGBM Surface Plot 実装ガイド

**作成日**: 2026-05-01
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連データフロー**: [dataflow.md](dataflow.md)
**関連要件定義**: [requirements.md](../../spec/lightgbm-surface-plot/requirements.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 設計文書・ユーザヒアリングを参考にした確実な実装
- 🟡 **黄信号**: 設計文書・ヒアリングから妥当な推測による実装
- 🔴 **赤信号**: 推測による実装

---

## 変更ファイル一覧

| ファイル | 変更種別 | 対応要件 |
|---|---|---|
| `rust_core/src/core/lgbm.rs` | 関数追加 | REQ-001 |
| `rust_core/src/pdp/api.rs` | ディスパッチ追加 | REQ-002 |
| `egui-app/src/ui/widgets/pdp_chart.rs` | 列挙型拡張・UI 更新 | REQ-011〜013, REQ-021 |
| `egui-app/src/ui/widgets/pdp_2d.rs` | UI・n_grid 更新 | REQ-014, REQ-022 |

---

## 1. rust_core/src/core/lgbm.rs — compute_pdp_1d_lgbm() 追加 🔵

**信頼性**: 🔵 *REQ-001・compute_pdp_2d_lgbm パターン・ユーザヒアリング「全特徴量で学習」「iter=100」より*

`compute_pdp_2d_lgbm()` の直後に追加する。

```rust
// ── 1D PDP ────────────────────────────────────────────────────────────────────

type Pdp1dResult = Option<(Vec<f64>, Vec<f64>, f64)>;

/// 1D partial dependence curve using a LightGBM RandomForest.
///
/// Trains on the full feature matrix. For each grid point v the target column
/// is fixed to v in every row; the average prediction gives the PDP value.
///
/// Returns `(grid, values, r_squared)` where `grid` spans the data range of
/// the target column and `values.len() == grid.len() == n_grid`.
pub fn compute_pdp_1d_lgbm(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    param_idx: usize,
    n_grid: usize,
) -> Pdp1dResult {
    let n = y.len();
    if n < 2 || x_matrix.is_empty() || n_grid < 2 {
        return None;
    }
    let p = x_matrix[0].len();
    if param_idx >= p {
        return None;
    }

    let config = LgbmRfConfig {
        num_iterations: 100,
        ..Default::default()
    };
    let booster = train_lgbm_rf(x_matrix, y, &config)?;

    let col: Vec<f64> = x_matrix.iter().map(|r| r[param_idx]).collect();
    let min_j = col.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_j = col.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let grid = pdp_linspace(min_j, max_j, n_grid);

    let values: Vec<f64> = grid
        .iter()
        .map(|&v| {
            let rows: Vec<Vec<f64>> = x_matrix
                .iter()
                .map(|r| {
                    let mut row = r.clone();
                    row[param_idx] = v;
                    row
                })
                .collect();
            let preds = lgbm_predict(&booster, &rows);
            if preds.is_empty() {
                0.0
            } else {
                preds.iter().sum::<f64>() / preds.len() as f64
            }
        })
        .collect();

    let mse = lgbm_mse(&booster, x_matrix, y)?;
    let r_squared = mse_to_r_squared(mse, y);

    Some((grid, values, r_squared))
}
```

**テストケース追加**（`lgbm.rs` の `#[cfg(test)] mod tests` 内に追加）: 🔵

```rust
#[test]
fn pdp_1d_lgbm_shape() {
    let (x, y) = synthetic_data(30);
    let (grid, values, r_squared) =
        compute_pdp_1d_lgbm(&x, &y, 0, 5).expect("pdp_1d_lgbm should return Some");
    assert_eq!(grid.len(), 5);
    assert_eq!(values.len(), 5);
    assert!(r_squared.is_finite());
}

#[test]
fn pdp_1d_lgbm_returns_none_for_invalid_input() {
    let (x, y) = synthetic_data(30);
    assert!(compute_pdp_1d_lgbm(&x, &y, 0, 0).is_none());   // n_grid < 2
    assert!(compute_pdp_1d_lgbm(&x, &y, 99, 5).is_none());  // param_idx 越境
    assert!(compute_pdp_1d_lgbm(&[], &[], 0, 5).is_none()); // 空データ
}

#[test]
fn pdp_1d_lgbm_monotone_for_linear_data() {
    let n = 40;
    let x: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64, 0.0]).collect();
    let y: Vec<f64> = x.iter().map(|r| r[0] * 2.0).collect();
    let (_, values, _) = compute_pdp_1d_lgbm(&x, &y, 0, 10).unwrap();
    // 線形データなら PDP は単調増加
    for i in 0..values.len() - 1 {
        assert!(values[i] <= values[i + 1] + 1e-6, "PDP should be non-decreasing");
    }
}
```

---

## 2. rust_core/src/pdp/api.rs — "random_forest" ディスパッチ追加 🔵

**信頼性**: 🔵 *REQ-002・既存 kriging ディスパッチパターンより*

`compute_pdp_from_data()` の `match model_type` ブロックに追加する。

```rust
pub fn compute_pdp_from_data(
    x_matrix: Vec<Vec<f64>>,
    y: Vec<f64>,
    param_names: Vec<String>,
    objective_name: &str,
    target_param_idx: usize,
    n_grid: usize,
    model_type: &str,
) -> PdpResult1d {
    match model_type {
        // ── 追加ここから ──────────────────────────────────────────────────────
        "random_forest" => {
            let param_name = param_names
                .get(target_param_idx)
                .cloned()
                .unwrap_or_default();
            match crate::core::lgbm::compute_pdp_1d_lgbm(
                &x_matrix,
                &y,
                target_param_idx,
                n_grid,
            ) {
                Some((grid, values, r_squared)) => PdpResult1d {
                    param_name,
                    objective_name: objective_name.to_string(),
                    grid,
                    values,
                    r_squared,
                    y_upper: None,
                    y_lower: None,
                },
                None => compute_pdp_from_matrix(
                    &x_matrix,
                    &y,
                    &param_names,
                    objective_name,
                    target_param_idx,
                    n_grid,
                ),
            }
        }
        // ── 追加ここまで ──────────────────────────────────────────────────────
        "kriging" => compute_pdp_1d_kriging_raw(...).unwrap_or_else(|| ...),
        "sparse_kriging" => compute_pdp_1d_sparse_kriging_raw(...).unwrap_or_else(|| ...),
        _ => compute_pdp_from_matrix(...),
    }
}
```

---

## 3. egui-app/src/ui/widgets/pdp_chart.rs — ModelType 拡張・UI 更新 🔵

**信頼性**: 🔵 *REQ-011〜013, REQ-021・ユーザヒアリングより*

### 3-1. ModelType 列挙型に RandomForest を追加

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ModelType {
    Ridge,
    Kriging,
    SparseKriging,
    RandomForest,   // ← 追加
}

impl ModelType {
    pub fn label(&self) -> &'static str {
        match self {
            ModelType::Ridge => "Ridge",
            ModelType::Kriging => "Kriging",
            ModelType::SparseKriging => "Sparse Kriging",
            ModelType::RandomForest => "Random Forest (LightGBM)",  // ← 追加
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            ModelType::Ridge => "ridge",
            ModelType::Kriging => "kriging",
            ModelType::SparseKriging => "sparse_kriging",
            ModelType::RandomForest => "random_forest",  // ← 追加
        }
    }
}
```

### 3-2. 1D PDP の n_grid 分岐に RandomForest を追加

`show()` 内の n_grid 計算ロジック:

```rust
// 変更前
let n_grid = match self.model_type {
    ModelType::Ridge => 50,
    _ => 30,
};

// 変更後
let n_grid = match self.model_type {
    ModelType::Ridge => 50,
    ModelType::RandomForest => 30,  // ← REQ-021: 明示的に 30 を指定
    _ => 30,                        // Kriging / SparseKriging
};
```

### 3-3. 1D PDP のモデル選択 ComboBox に RandomForest を追加

```rust
// 変更前
for model in [
    ModelType::Ridge,
    ModelType::Kriging,
    ModelType::SparseKriging,
] {

// 変更後
for model in [
    ModelType::Ridge,
    ModelType::Kriging,
    ModelType::SparseKriging,
    ModelType::RandomForest,   // ← 追加
] {
```

---

## 4. egui-app/src/ui/widgets/pdp_2d.rs — ComboBox・n_grid 更新 🔵

**信頼性**: 🔵 *REQ-014, REQ-022・ユーザヒアリングより*

### 4-1. 2D PDP のモデル選択 ComboBox に RandomForest を追加

```rust
// 変更前
for model in [
    ModelType::Ridge,
    ModelType::Kriging,
    ModelType::SparseKriging,
] {

// 変更後
for model in [
    ModelType::Ridge,
    ModelType::Kriging,
    ModelType::SparseKriging,
    ModelType::RandomForest,   // ← 追加
] {
```

### 4-2. 2D PDP の n_grid に RandomForest 分岐を追加

```rust
// 変更前（"Run 2D PDP" クリック時）
self.pending_compute = Some(Pdp2dComputeRequest {
    param1: self.selected_param1.clone(),
    param2: self.selected_param2.clone(),
    objective: obj_name.clone(),
    n_grid: 20,   // ← ハードコード
    model_type: self.selected_model.to_str().to_string(),
});

// 変更後
let n_grid = match self.selected_model {
    ModelType::RandomForest => 30,  // REQ-022
    _ => 20,
};
self.pending_compute = Some(Pdp2dComputeRequest {
    param1: self.selected_param1.clone(),
    param2: self.selected_param2.clone(),
    objective: obj_name.clone(),
    n_grid,
    model_type: self.selected_model.to_str().to_string(),
});
```

---

## 実装順序（推奨） 🔵

**信頼性**: 🔵 *依存関係・テスト容易性より*

```
1. rust_core/src/core/lgbm.rs
   → compute_pdp_1d_lgbm() 追加
   → テスト: cargo test -p tunny-core pdp_1d_lgbm

2. rust_core/src/pdp/api.rs
   → "random_forest" ディスパッチ追加
   → テスト: cargo test -p tunny-core compute_pdp_from_data

3. egui-app/src/ui/widgets/pdp_chart.rs
   → ModelType::RandomForest 追加・label/to_str
   → n_grid 分岐更新
   → ComboBox 更新
   → テスト: cargo test -p tunny-desktop (ユニットテスト)

4. egui-app/src/ui/widgets/pdp_2d.rs
   → ComboBox 更新
   → n_grid 分岐更新

5. 統合確認
   → cargo build
   → cargo test
   → アプリ起動・手動 UI 確認
```

---

## Clippy 注意事項 🟡

**信頼性**: 🟡 *既存 lgbm.rs のパターン・clippy 警告経験から妥当な推測*

- `compute_pdp_1d_lgbm` 内の `rows.iter().map(...)` は `into_iter()` を使うと所有権問題が生じるため `.map(|r| { let mut row = r.clone(); ... })` を維持すること
- `ModelType` に `#[allow(dead_code)]` は不要（全バリアントが ComboBox で使われる）

---

## 信頼性レベルサマリー

- 🔵 青信号: 12件 (92%)
- 🟡 黄信号: 1件 (8%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質
