# sensitivity-analysis-statistical-importance コンテキストノート

**生成日**: 2026-04-25

## プロジェクト基本情報

- **リポジトリ**: c:\Users\hiroa\Desktop\Tunny-Dashboard
- **開発ブランチ**: featura/egui（TS廃止・Rust/egui完全移行）
- **技術スタック**: Rust / egui / wgpu 3D / WASM不使用（後方互換性不要）
- **テスト**: Rustインラインテスト

## 感度分析の現状

### 実装済みメトリクス

| メトリクス | 実装ファイル | 現在の出力 | 統計情報 |
|----------|------------|----------|---------|
| Spearman | `rust_core/src/sensitivity/spearman.rs` | `Vec<Vec<f64>>` | なし |
| Ridge | `rust_core/src/sensitivity/ridge.rs` | `beta: Vec<f64>`, `r_squared: f64` | なし |
| RF-ANOVA | `rust_core/src/sensitivity/rf_anova.rs` | `importances`, `r_squared` | なし |
| MDI | `rust_core/src/sensitivity/mdi.rs` | `importances`, `r_squared` | なし |
| SHAP | `rust_core/src/sensitivity/shap.rs` | `importances`, `r_squared` | なし |
| Sobol | `rust_core/src/sensitivity/sobol.rs` | `first_order`, `total_effect`, `r_squared` | なし |

### 型定義 (`rust_core/src/sensitivity/types.rs`)

```rust
pub enum SensitivityMetric { Spearman, Ridge, RfAnova, Mdi, Shap }

pub struct RfAnovaResult {
    pub importances: Vec<Vec<f64>>, // [param][objective]
    pub r_squared: Vec<f64>,        // [objective]
}
// MDI, SHAP も同様

pub struct RidgeResult {
    pub beta: Vec<f64>,
    pub r_squared: f64,
}

pub struct SobolResult {
    pub first_order: Vec<Vec<f64>>,
    pub total_effect: Vec<Vec<f64>>,
    pub r_squared: Vec<f64>,
    pub n_samples: usize,
}
```

### UI表示 (`egui-app/src/ui/widgets/importance_chart.rs`)

- 重要度バーチャート（横棒）
- R²を右端に色分け表示（赤/黄/緑）
- p値・信頼区間・有意性マーク: **未実装**

## ヒアリング結果

**対象メトリクス**: 全メトリクス（Spearman, Ridge, RF-ANOVA, MDI, SHAP, Sobol）

**表示する統計情報**:
- p値
- 有意性マーク（p<0.05→*, p<0.01→**, p<0.001→***）
- 95%信頼区間

**p値計算方式**:
- Spearman/Ridge: 解析的計算（t分布近似）
- RF-ANOVA/MDI/SHAP: ツリー間分散（100木から標準誤差を推定）
- Sobol: Jansen分散ブートストラップ

**多重比較補正**: Bonferroni補正（閾値をパラメータ数で割る）

**UI表示**: バーにインライン表示（重要度バーの右側に「0.032 *」形式）

## 技術的制約

- 外部統計ライブラリ不可（WASMバイナリサイズ制約、現行方針引き継ぎ）
- p値のt分布・正規分布近似はpure Rustで実装
- ブートストラップ回数は既存サンプルサイズ制約内に収める

## 関連ファイル

- `rust_core/src/sensitivity/` — 全メトリクス実装
- `rust_core/src/sensitivity/types.rs` — 型定義
- `egui-app/src/ui/widgets/importance_chart.rs` — UI
- `theory/sensitivity-analysis/` — 手法別理論ドキュメント
