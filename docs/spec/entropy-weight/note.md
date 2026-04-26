# Entropy Weight Method コンテキストノート

**作成日**: 2026-04-24
**要件名**: entropy-weight

## 技術スタック

- **コア計算**: Rust (tunny-core クレート)
- **UI**: egui (tunny-desktop クレート)
- **テスト**: cargo test、TDD方式
- **ブランチ**: featura/egui

## 開発ルール

- 外部線形代数ライブラリ不使用（Pure Rust実装）
- Result 型は `Result<T, String>`
- 構造体は `#[derive(Debug, Clone, serde::Serialize)]`
- テスト命名規則: `tc_entropy_<seq>_<description>`

## 関連実装

### rust_core/src/mcdm/
- `mod.rs`: 共通 validate_inputs, filter_valid_indices
- `topsis.rs`: TOPSIS アルゴリズム（参考パターン）
- `vikor.rs`: VIKOR アルゴリズム（参考パターン）

### egui-app/src/ui/widgets/mcdm_chart.rs
- `McdmRankChart`: 重みスライダー、v_param、Weight Mode 管理
- `McdmComputeRequest`: 計算リクエスト構造体
- `normalize_weights()`: 重み正規化ユーティリティ

### egui-app/src/ui/chart_registry.rs
- MCDM dispatch パターン（TOPSIS/VIKOR arm）

## 設計文書

- docs/theory/vikor.md（理論文書フォーマット参考）
- docs/spec/vikor/requirements.md（要件定義フォーマット参考）

## 注意事項

- エントロピー法は比例正規化が前提（非負データ）
- 負の値を含む場合は min-max 正規化で [0,1] に変換してから適用
- p_ij = 0 の場合、p_ij * ln(p_ij) = 0 として扱う（Shannon エントロピーの定義）
- 全目的 d_j = 0 の場合、均等重み 1/n を返す
- テストコマンド: `cd rust_core && cargo test`（コア）/ `cargo test --package tunny-desktop`（UI）
