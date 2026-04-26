# sensitivity-all-models アーキテクチャ設計

**作成日**: 2026-04-15
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 既存実装・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: 既存実装・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: 推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *ユーザーヒアリングより*

以下の2点を実装する：

1. **rust_core リファクタリング**: `compute_sensitivity_all` が Spearman・Ridge・RF ANOVA を常に一括計算している問題を解消し、選択したメトリクスのみ計算できる `compute_sensitivity_for(metric)` を追加する
2. **egui-app 拡張**: `ImportanceChart` に Run ボタンと全メトリクス（Spearman・Ridge・RF ANOVA・Sobol）の表示対応を追加する

`SensitivityHeatmap` は Spearman の表示専用として現状維持。

---

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存コードベースの PdpChart2D 実装パターンより*

- **パターン**: `pending_compute: Option<ImportanceMetric>` を `ImportanceChart` に追加し、`grid_canvas.rs` でスレッドタスクを spawn する
- **rust_core パターン**: `SensitivityMetric` 列挙型を新設し、`compute_sensitivity_for` が指定されたメトリクスのみ計算する

---

## 変更コンポーネント

---

### Phase 1: rust_core リファクタリング

#### 1-A. `rust_core/src/sensitivity/types.rs` — SensitivityMetric 追加 🔵

**信頼性**: 🔵 *ユーザーヒアリング・既存実装より*

```rust
/// 計算するメトリクスを指定する列挙型
#[derive(Debug, Clone, PartialEq)]
pub enum SensitivityMetric {
    Spearman,
    Ridge,
    RfAnova,
}
```

`mod.rs` に `pub use types::SensitivityMetric;` を追加。

#### 1-B. `rust_core/src/sensitivity/analysis/full.rs` — 個別計算関数追加 🔵

**信頼性**: 🔵 *既存 compute_sensitivity_all の実装より抽出*

共通前処理（データ取得・バリデーション）を切り出し、メトリクス別の内部関数を追加する：

```rust
/// Spearman 相関のみ計算
pub fn compute_spearman_only(df: &DataFrame) -> SensitivityResult {
    // spearman のみ計算。ridge = vec![], rf_anova = None
}

/// Ridge 回帰係数のみ計算
pub fn compute_ridge_only(df: &DataFrame) -> SensitivityResult {
    // spearman = vec![] (ゼロ埋め), ridge のみ計算。rf_anova = None
}

/// RF ANOVA 重要度のみ計算
pub fn compute_rf_anova_only(df: &DataFrame) -> SensitivityResult {
    // spearman = vec![] (ゼロ埋め), ridge = vec![], rf_anova のみ計算
}

/// 既存関数（変更なし・後方互換）
pub fn compute_sensitivity_all(df: &DataFrame) -> SensitivityResult {
    // Spearman + Ridge + RF ANOVA を一括計算（変更なし）
}
```

#### 1-C. `rust_core/src/sensitivity/analysis.rs` — compute_sensitivity_for 追加 🔵

**信頼性**: 🔵 *既存 compute_sensitivity パターンより*

```rust
use super::{SensitivityMetric, SensitivityResult};

/// 指定されたメトリクスのみ計算するエントリーポイント
pub fn compute_sensitivity_for(metric: SensitivityMetric) -> Option<SensitivityResult> {
    dataframe::with_active_df(|df| match metric {
        SensitivityMetric::Spearman => full::compute_spearman_only(df),
        SensitivityMetric::Ridge    => full::compute_ridge_only(df),
        SensitivityMetric::RfAnova  => full::compute_rf_anova_only(df),
    })
}

/// 既存関数（変更なし・後方互換）
pub fn compute_sensitivity() -> Option<SensitivityResult> {
    dataframe::with_active_df(compute_sensitivity_all)
}
```

`mod.rs` に `pub use analysis::compute_sensitivity_for;` を追加。

`selected.rs` も同様に `compute_spearman_selected` / `compute_ridge_selected` / `compute_rf_anova_selected` を追加（設計は full.rs と同様のパターンで実装）。🟡

---

### Phase 2: egui-app 拡張

#### 2-A. `egui-app/src/state/app_state.rs` — SensitivityResult 拡張 🔵

**信頼性**: 🔵 *rust_core/src/sensitivity/types.rs の既存定義より*

egui-app 版 `SensitivityResult` を rust_core 版に揃える：

```rust
// 変更前（spearman のみ）
pub struct SensitivityResult {
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub spearman: Vec<Vec<f64>>,
}

// 変更後（ridge + rf_anova 追加）
pub struct SensitivityResult {
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub spearman: Vec<Vec<f64>>,        // [param][objective]
    pub ridge: Vec<RidgeResult>,         // [objective]
    pub rf_anova: Option<RfAnovaResult>,
}

pub struct RidgeResult {
    pub beta: Vec<f64>,
    pub r_squared: f64,
}

pub struct RfAnovaResult {
    pub importances: Vec<Vec<f64>>,  // [param][objective]
}
```

**代替案**: `pub use tunny_core::sensitivity::{SensitivityResult, RidgeResult, RfAnovaResult};` で re-export する方が rust_core との型齟齬を防ぎやすい。🟡

#### 2-B. `egui-app/src/ui/widgets/importance_chart.rs` — Run ボタン + 全メトリクス対応 🔵

**信頼性**: 🔵 *PdpChart2D の pending_compute パターン・ユーザーヒアリングより*

**ImportanceMetric に RfAnova を追加**:
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ImportanceMetric {
    Spearman,
    Ridge,
    RfAnova,   // 新規追加
    Sobol,
}
```

**ImportanceChart 構造体に pending_compute を追加**:
```rust
pub struct ImportanceChart {
    pub selected_objective: usize,
    pub selected_metric: ImportanceMetric,
    pub computing: bool,
    pub pending_compute: Option<ImportanceMetric>,  // 新規追加
}
```

**show() シグネチャ変更**:
```rust
pub fn show(
    &mut self,
    ui: &mut egui::Ui,
    sensitivity: Option<&SensitivityResult>,
    sobol: Option<&SobolResult>,      // Sobol 表示用に追加
    obj_names: &[String],
)
```

**コントロール行に Run ボタン追加**:
```rust
ui.horizontal(|ui| {
    // 既存: メトリクス選択（Spearman/Ridge/RfAnova/Sobol）+ 目的関数選択
    ui.separator();
    if ui.button("Run").clicked() {
        self.pending_compute = Some(self.selected_metric.clone());
    }
    if self.computing {
        ui.spinner();
    }
});
```

**compute_sorted_importance() をメトリクス別に対応**:
```rust
fn compute_sorted_importance(
    sensitivity: Option<&SensitivityResult>,
    sobol: Option<&SobolResult>,
    metric: &ImportanceMetric,
    obj_idx: usize,
    param_names: &[String],
) -> Vec<(String, f64)> {
    let scores: Vec<f64> = match metric {
        ImportanceMetric::Spearman => {
            // 既存ロジック: sensitivity.spearman[param][obj_idx]
        }
        ImportanceMetric::Ridge => {
            // sensitivity.ridge[obj_idx].beta
        }
        ImportanceMetric::RfAnova => {
            // sensitivity.rf_anova.importances[param][obj_idx]
        }
        ImportanceMetric::Sobol => {
            // sobol.first_order[param][obj_idx]
        }
    };
    // 絶対値降順ソートして (param_name, score) ペアを返す（既存ロジックを流用）
}
```

#### 2-C. `egui-app/src/ui/grid_canvas.rs` — タスクスポーン 🔵

**信頼性**: 🔵 *grid_canvas.rs の ChartId::PdpChart2D spawn_task パターンより*

```rust
ChartId::ImportanceChart => {
    let sobol = app_state.sobol_result.as_ref();
    widgets.importance.show(ui, sensitivity.as_ref(), sobol, &obj_names);
    if let Some(metric) = widgets.importance.pending_compute.take() {
        widgets.importance.computing = true;
        let tx = tx.clone();
        crate::app::spawn_task(tx, move || match metric {
            ImportanceMetric::Sobol => {
                tunny_core::sensitivity::compute_sobol(1024)
                    .map(AppMessage::SobolDone)
                    .unwrap_or(AppMessage::SensitivityError)
            }
            ImportanceMetric::Spearman => {
                tunny_core::sensitivity::compute_sensitivity_for(SensitivityMetric::Spearman)
                    .map(AppMessage::SensitivityDone)
                    .unwrap_or(AppMessage::SensitivityError)
            }
            ImportanceMetric::Ridge => {
                tunny_core::sensitivity::compute_sensitivity_for(SensitivityMetric::Ridge)
                    .map(AppMessage::SensitivityDone)
                    .unwrap_or(AppMessage::SensitivityError)
            }
            ImportanceMetric::RfAnova => {
                tunny_core::sensitivity::compute_sensitivity_for(SensitivityMetric::RfAnova)
                    .map(AppMessage::SensitivityDone)
                    .unwrap_or(AppMessage::SensitivityError)
            }
        });
    }
}
```

#### 2-D. `egui-app/src/state/messages.rs` — AppMessage 追加 🟡

**信頼性**: 🟡 *既存 AppMessage パターンから妥当な推測*

```rust
pub enum AppMessage {
    SensitivityDone(SensitivityResult),
    SobolDone(SobolResult),
    SensitivityError,   // 計算失敗時に computing = false にする（新規追加）
    // ...
}
```

#### 2-E. `egui-app/src/app.rs` — エラーハンドラ追加 🟡

**信頼性**: 🟡 *既存 poll_messages パターンから妥当な推測*

```rust
AppMessage::SensitivityError => {
    self.app_state.widgets.importance.computing = false;
}
```

---

## ディレクトリ構造（変更対象のみ） 🔵

**信頼性**: 🔵 *既存プロジェクト構造より*

```
rust_core/src/sensitivity/
├── types.rs              ← SensitivityMetric 列挙型追加
├── analysis.rs           ← compute_sensitivity_for 追加
├── analysis/
│   ├── full.rs           ← compute_spearman_only / compute_ridge_only / compute_rf_anova_only 追加
│   └── selected.rs       ← 同様の個別計算関数追加
└── mod.rs                ← compute_sensitivity_for / SensitivityMetric を pub use 追加

egui-app/src/
├── state/
│   ├── app_state.rs      ← SensitivityResult に ridge/rf_anova フィールド追加
│   └── messages.rs       ← AppMessage::SensitivityError 追加
├── ui/
│   ├── widgets/
│   │   └── importance_chart.rs  ← pending_compute, Run ボタン, RfAnova メトリクス, 全メトリクス表示
│   └── grid_canvas.rs           ← spawn_task（メトリクス別分岐）、sobol_result 引き渡し
└── app.rs                        ← SensitivityError ハンドラ追加
```

---

## 非機能要件

### パフォーマンス 🔵

**信頼性**: 🔵 *ユーザーヒアリング・rust_core 実装より*

| メトリクス | 計算コスト | 備考 |
|------------|----------|------|
| Spearman | O(N log N) | ほぼ瞬時 |
| Ridge | O(N × P²) | ほぼ瞬時 |
| RF ANOVA | O(N × 100 木 × P) | 2000行以上は自動スキップ済み |
| Sobol | O(n_samples × P) | サロゲートモデル構築あり |

すべてバックグラウンドスレッドで実行 → UI はブロックされない。

### 後方互換性 🔵

**信頼性**: 🔵 *既存実装より*

- `compute_sensitivity()` / `compute_sensitivity_all()` は変更なし
- `SensitivityHeatmap` は変更なし（spearman フィールドを引き続き使用）
- `compute_sensitivity_for` は追加のみ（既存呼び出し箇所への影響なし）

---

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **ヒアリング記録**: [design-interview.md](design-interview.md)
- **既存実装**: `rust_core/src/sensitivity/analysis/full.rs`
- **既存実装**: `egui-app/src/ui/widgets/importance_chart.rs`
- **参照パターン**: `egui-app/src/ui/widgets/pdp_2d.rs`（pending_compute パターン）

## 信頼性レベルサマリー

- 🔵 青信号: 10件 (83%)
- 🟡 黄信号: 2件 (17%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
