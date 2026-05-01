# LightGBM Surface Plot コンテキストノート

**生成日**: 2026-05-01

## プロジェクト基本情報

- **リポジトリ**: c:\Users\hiroa\Desktop\Tunny-Dashboard
- **技術スタック**: Rust + egui (eframe / egui_plot / wgpu)
- **ビルドツール**: Cargo (workspace)

## 実装対象チャートの現状

| 対象 | 状態 |
|---|---|
| `rust_core/src/core/lgbm.rs` の `compute_pdp_2d_lgbm()` | 実装済み |
| `rust_core/src/pdp/api.rs` の `compute_pdp_2d()` "random_forest" ディスパッチ | 実装済み |
| `rust_core/src/pdp/api.rs` の `compute_pdp_from_data()` "random_forest" ディスパッチ | **未実装** |
| `egui-app/src/ui/widgets/pdp_chart.rs` の `ModelType::RandomForest` | **未追加** |
| 1D PDP UI ComboBox に Random Forest 選択肢 | **未追加** |
| 2D PDP UI ComboBox に Random Forest 選択肢 | **未追加** |

## 実装済み Rust 関数シグネチャ

### `rust_core/src/core/lgbm.rs`

```rust
pub fn compute_pdp_2d_lgbm(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    param1_idx: usize,
    param2_idx: usize,
    n_grid: usize,
) -> Option<(Vec<f64>, Vec<f64>, Vec<Vec<f64>>, f64)>
// 戻り値: (grid1, grid2, z_values[n_grid][n_grid], r_squared)
```

### `rust_core/src/pdp/api.rs`

```rust
pub fn compute_pdp_from_data(
    x_matrix: Vec<Vec<f64>>,
    y: Vec<f64>,
    param_names: Vec<String>,
    objective_name: &str,
    target_param_idx: usize,
    n_grid: usize,
    model_type: &str,  // "ridge" | "kriging" | "sparse_kriging" (→ "random_forest" 追加必要)
) -> PdpResult1d

pub fn compute_pdp_2d(
    param1_name: &str,
    param2_name: &str,
    objective_name: &str,
    n_grid: usize,
    model_type: &str,  // "ridge" | "kriging" | "sparse_kriging" | "random_forest" ✅
) -> Option<PdpResult2d>
```

## ModelType 列挙型（現状）

```rust
// egui-app/src/ui/widgets/pdp_chart.rs
pub enum ModelType {
    Ridge,
    Kriging,
    SparseKriging,
    // RandomForest ← 追加が必要
}
```

## データフロー

### 1D PDP

```
pdp_chart.rs show() → PdpComputeRequest { model_type: req.model_type.to_str() }
→ chart_registry.rs spawn_task: compute_pdp_from_data(..., &model_type)
→ pdp/api.rs compute_pdp_from_data() → dispatch by model_type
→ AppMessage::PdpDone → message_handler → pdp_chart.result
```

### 2D PDP

```
pdp_2d.rs show() → Pdp2dComputeRequest { n_grid: 20, model_type: ... }
→ chart_registry.rs spawn_task: compute_pdp_2d(...)
→ pdp/api.rs compute_pdp_2d() → "random_forest" → lgbm::compute_pdp_2d_lgbm()
→ AppMessage::Pdp2dDone → message_handler → pdp_2d.result
```

## n_grid の現状

| 場所 | 現在値 |
|---|---|
| 1D PDP: Ridge | 50 |
| 1D PDP: Kriging / Sparse Kriging | 30 |
| 2D PDP: 全モデル共通 | 20 |

## 関連ファイル一覧

**変更対象（既存）**:
- `rust_core/src/core/lgbm.rs` — `compute_pdp_1d_lgbm()` 新規追加
- `rust_core/src/pdp/api.rs` — `compute_pdp_from_data()` に "random_forest" ディスパッチ追加
- `egui-app/src/ui/widgets/pdp_chart.rs` — `ModelType::RandomForest` 追加・1D ComboBox 更新・n_grid 分岐追加
- `egui-app/src/ui/widgets/pdp_2d.rs` — 2D ComboBox 更新・n_grid=30 分岐追加
