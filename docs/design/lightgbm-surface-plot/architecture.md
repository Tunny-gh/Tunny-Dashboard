# LightGBM Surface Plot アーキテクチャ設計

**作成日**: 2026-05-01
**関連要件定義**: [requirements.md](../../spec/lightgbm-surface-plot/requirements.md)
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *要件定義書 REQ-001〜014 より*

Tunny Dashboard の PDP Chart（1D・2D）にLightGBM RandomForest モデルを追加する。
バックエンドの 2D 計算（`compute_pdp_2d_lgbm`）は `rust_core/src/core/lgbm.rs` に実装済み。
本機能追加の変更は以下の 4 ファイルに限定される：

| ファイル | 変更内容 |
|---|---|
| `rust_core/src/core/lgbm.rs` | `compute_pdp_1d_lgbm()` 新規追加 |
| `rust_core/src/pdp/api.rs` | `compute_pdp_from_data()` に "random_forest" ディスパッチ追加 |
| `egui-app/src/ui/widgets/pdp_chart.rs` | `ModelType::RandomForest` 追加・1D ComboBox・n_grid 更新 |
| `egui-app/src/ui/widgets/pdp_2d.rs` | 2D ComboBox・n_grid 更新 |

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存プロジェクト Rust/egui アーキテクチャより*

- **パターン**: 既存の 2 層コア/UI 分離パターンを維持
  - `rust_core` クレート: 純粋な計算ロジック（no egui 依存）
  - `egui-app` クレート: UI・状態管理・非同期ディスパッチ

## コンポーネント構成

### rust_core 側（計算ロジック） 🔵

**信頼性**: 🔵 *既存 lgbm.rs・pdp/api.rs・ridge_core.rs 実装パターンより*

```
rust_core/src/
├── core/
│   └── lgbm.rs           ← compute_pdp_1d_lgbm() 追加
└── pdp/
    ├── api.rs             ← "random_forest" ディスパッチ追加
    ├── ridge_core.rs      （変更なし・フォールバック元）
    ├── kriging_core.rs    （変更なし）
    ├── types.rs           （変更なし）
    └── utils.rs           （変更なし）
```

**`compute_pdp_1d_lgbm()` 設計**（`lgbm.rs` に追加）:

```rust
// 戻り値タプル型（PdpResult1d を避けて循環依存を防ぐ）
type Pdp1dResult = Option<(Vec<f64>, Vec<f64>, f64)>;
// Returns: (grid, values, r_squared)

pub fn compute_pdp_1d_lgbm(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    param_idx: usize,
    n_grid: usize,
) -> Pdp1dResult;
```

**`pdp/api.rs` の `compute_pdp_from_data()` 拡張**:

```rust
"random_forest" => {
    let result = crate::core::lgbm::compute_pdp_1d_lgbm(
        &x_matrix, &y, target_param_idx, n_grid
    );
    match result {
        Some((grid, values, r_squared)) => PdpResult1d {
            param_name: ...,
            objective_name: ...,
            grid, values, r_squared,
            y_upper: None, y_lower: None,
        },
        None => compute_pdp_from_matrix(...)  // Ridge フォールバック
    }
}
```

### egui-app 側（UI） 🔵

**信頼性**: 🔵 *既存 pdp_chart.rs・pdp_2d.rs 実装パターンより*

```
egui-app/src/ui/widgets/
├── pdp_chart.rs    ← ModelType::RandomForest 追加・ComboBox・n_grid 更新
└── pdp_2d.rs       ← ComboBox・n_grid 更新
```

**`ModelType` 列挙型の拡張**（`pdp_chart.rs`）:

```rust
pub enum ModelType {
    Ridge,
    Kriging,
    SparseKriging,
    RandomForest,    // 追加
}

impl ModelType {
    pub fn label(&self) -> &'static str {
        match self {
            ModelType::RandomForest => "Random Forest (LightGBM)",
            // ...
        }
    }
    pub fn to_str(&self) -> &'static str {
        match self {
            ModelType::RandomForest => "random_forest",
            // ...
        }
    }
}
```

**1D PDP n_grid ロジック更新**（`pdp_chart.rs`）:

```rust
let n_grid = match self.model_type {
    ModelType::Ridge => 50,
    ModelType::RandomForest => 30,  // 追加（REQ-021）
    _ => 30,  // Kriging / SparseKriging
};
```

**2D PDP n_grid ロジック更新**（`pdp_2d.rs`）:

```rust
// Run ボタンクリック時
let n_grid = match self.selected_model {
    ModelType::RandomForest => 30,  // REQ-022
    _ => 20,
};
self.pending_compute = Some(Pdp2dComputeRequest {
    n_grid,
    model_type: self.selected_model.to_str().to_string(),
    // ...
});
```

## データ型の対応関係 🔵

**信頼性**: 🔵 *既存 types.rs・messages.rs・chart_registry.rs の変換コードより*

```
lgbm.rs
  compute_pdp_1d_lgbm() → Option<(Vec<f64>, Vec<f64>, f64)>
                              ↓ pdp/api.rs 変換
  compute_pdp_from_data() → rust_core::pdp::types::PdpResult1d { grid, values, r_squared, ... }
                              ↓ chart_registry.rs 変換（既存コード）
  AppMessage::PdpDone { result: PdpResult::OneDim(messages::PdpResult1d { x_values, y_values, r2, ... }) }
                              ↓ message_handler.rs（既存コード）
  widgets.pdp_chart.result → pdp_chart.rs show() で描画
```

## LightGBM リンク構成 🔵

**信頼性**: 🔵 *rust_core/build.rs・lgbm_sys.rs より*

- `libs/lib_lightgbm.dll` (Windows) / `libs/lib_lightgbm.dylib` (macOS) をワークスペースルートに配置
- `rust_core/build.rs` が `rustc-link-lib=dylib=lib_lightgbm` を設定
- Windows では DLL を `target/<profile>/` と `target/<profile>/deps/` にコピー
- `lgbm_sys.rs` が FFI バインディングを提供
- 新規コードは既存の `lgbm.rs` 関数（`train_lgbm_rf`, `lgbm_predict`, `lgbm_mse`, `mse_to_r_squared`, `pdp_linspace`）のみを使用

## 非機能要件の実現方法

### パフォーマンス 🟡

**信頼性**: 🟡 *NFR-001/002 要件・既存 lgbm テスト実績から妥当な推測*

- **1D PDP**: n=1000・n_grid=30 で約 n_grid 回の predict を実行
  - 各 predict: `lgbm_predict()` で n=1000 行 → O(n × n_grid) 計算
  - 目標: 2 秒以内（NFR-001）
- **2D PDP**: 既存 `compute_pdp_2d_lgbm` がそのまま n_grid=30 で動作
  - 目標: 5 秒以内（NFR-002）
- **UI**: バックグラウンドスレッドで実行（既存 `spawn_task` パターン）→ メインスレッドをブロックしない

### 後方互換性 🔵

**信頼性**: 🔵 *NFR-011 要件より*

- Ridge・Kriging・SparseKriging の n_grid・R²・動作を変更しない
- `ModelType` のデフォルト値を `Ridge` のまま維持
- 既存テストをすべてパスすること

## ディレクトリ構造（変更対象のみ） 🔵

**信頼性**: 🔵 *既存プロジェクト構造より*

```
rust_core/src/core/lgbm.rs      ← compute_pdp_1d_lgbm() 追加
rust_core/src/pdp/api.rs        ← "random_forest" ディスパッチ追加
egui-app/src/ui/widgets/
    pdp_chart.rs                ← ModelType 拡張・UI 更新
    pdp_2d.rs                   ← ComboBox・n_grid 更新
```

## 技術的制約

### LightGBM DLL 依存 🔵

**信頼性**: 🔵 *build.rs・lgbm_sys.rs より*

- `libs/lib_lightgbm.dll` が存在しない場合、コンパイルは成功するがリンクに失敗する
- テスト実行には DLL が必要（build.rs が自動コピー）
- `compute_pdp_1d_lgbm` が `None` を返した場合は Ridge にフォールバック（EDGE-002）

### 循環依存の回避 🔵

**信頼性**: 🔵 *既存の compute_pdp_2d_lgbm の設計パターンより*

- `lgbm.rs` は `pdp::types::PdpResult1d` を直接参照しない
- 戻り値は `Option<(Vec<f64>, Vec<f64>, f64)>` タプル
- `pdp/api.rs` がタプルを `PdpResult1d` に変換する（既存 2D パターンと同様）

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **設計ヒアリング記録**: [design-interview.md](design-interview.md)
- **要件定義**: [requirements.md](../../spec/lightgbm-surface-plot/requirements.md)

## 信頼性レベルサマリー

- 🔵 青信号: 14件 (88%)
- 🟡 黄信号: 2件 (12%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質
