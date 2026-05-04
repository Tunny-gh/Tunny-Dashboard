# pdp-maintainability コンテキストノート

**作成日**: 2026-05-04

## 技術スタック

- **言語**: Rust 2021 edition
- **クレート名**: `tunny-core`（rlib）
- **線形代数**: `faer = "0.24.0"`
- **シリアライズ**: `serde = "1"`
- **ベンチマーク**: `criterion = "0.5"`

## 対象ディレクトリ構成

```
rust_core/src/pdp/
├── mod.rs        - pub re-export のみ
├── types.rs      - PdpResult1d, PdpResult2d 定義
├── utils.rs      - col_mean_std ラッパー（現在は薄いラッパーのみ）
├── api.rs        - compute_pdp, compute_pdp_2d, compute_pdp_from_data
├── ridge_core.rs - Ridge 回帰ベース PDP
├── kriging_core.rs - GP/FITC ベース PDP（最大ファイル: 610行）
└── tests.rs      - 統合テスト群
```

## 依存関係

```
pdp/api.rs → pdp/kriging_core.rs, pdp/ridge_core.rs
pdp/ridge_core.rs → core::math::grid::linspace, sensitivity::compute_ridge
pdp/kriging_core.rs → core::kriging::{gaussian_process, sparse_fitc}, core::math::grid::linspace
pdp/utils.rs → core::math::stats::column_mean_std
```

## 重複コードの箇所一覧（分析結果）

| # | 重複内容 | 出現箇所 | 推定削減行数 |
|---|---------|---------|------------|
| 1 | col_stats + y 正規化 + x 正規化 | kriging_core.rs x2 | ~40行 |
| 2 | R² 計算（ss_tot / ss_res / ガード） | kriging_core.rs x3 | ~30行 |
| 3 | x_matrix + y の DataFrame 抽出 | api.rs x2 | ~20行 |
| 4 | fold クロージャ形式 (|a,b| a.min(b)) | ridge_core.rs x4 | ~4行 |

## 注意事項

- `normalize_x_minmax` は `col_stats`（min/range タプル）と `x_norm` を同時に返す設計が効率的（2回ループを避ける）
- `r_squared` の引数は `y_actual: &[f64], y_pred: &[f64]` として純粋関数化できる
- rayon の `par_iter()` は `Sync` トレイトを要求するため、`GpModel`・`FitcModel` の `Sync` 実装を確認すること
- rayon を使う場合、結果の収集は `collect::<Vec<_>>()` で行い、後でアンパックする（順序保証が必要）

## 関連タスク

- 元タスク番号: TASK-803（PDP 基本実装）
- 参照: `docs/tasks/tunny-dashboard-tasks.md`
