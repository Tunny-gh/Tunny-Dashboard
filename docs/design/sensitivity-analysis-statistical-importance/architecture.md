# sensitivity-analysis-statistical-importance アーキテクチャ設計

**作成日**: 2026-04-25
**関連要件定義**: [requirements.md](../../spec/sensitivity-analysis-statistical-importance/requirements.md)
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *要件定義書・ユーザヒアリングより*

感度分析の全6メトリクス（Spearman, Ridge, RF-ANOVA, MDI, SHAP, Sobol）に、帰無仮説検定に基づく統計的有意性指標を追加する。
既存の点推定値に加え、p値（Bonferroni補正済み）・95%信頼区間・有意性マーク（*/**/**）を算出し、
egui UIのバーチャートにエラーバーとインラインテキストで表示する。

---

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存プロジェクト構造・ユーザヒアリングより*

- **パターン**: 既存の4層クライアントサイドアーキテクチャへの加算的変更
- **選択理由**: featura/eguiブランチはRust/egui完全移行済み。既存の `SensitivityResult`, `RfAnovaResult` 等の型を拡張して後方互換性を維持しつつ統計情報を追加する

```
Layer 1: 統計計算コア（新規追加）
  rust_core/src/core/math/statistics.rs
  ├── StudentT CDF（不完全ベータ関数）
  ├── NormalCDF（Abramowitz & Stegun近似）
  └── Bonferroni補正

Layer 2: メトリクス計算関数（既存を拡張）
  rust_core/src/sensitivity/
  ├── spearman.rs        → compute_spearman_with_stats() 追加
  ├── ridge.rs           → compute_ridge_with_stats() 追加（対角逆行列）
  ├── rf_anova.rs        → 木ごと重要度記録に変更
  ├── mdi.rs             → 正規化前の木ごと記録に変更
  ├── shap.rs            → 木ごとSHAP記録に変更
  ├── sobol.rs           → Jansen分散計算を追加
  └── types.rs           → 統計フィールドを Optional で追加

Layer 3: 分析結合（既存を拡張）
  rust_core/src/sensitivity/analysis/
  ├── full.rs            → 統計フィールドを埋めるよう更新
  └── selected.rs        → 同上

Layer 4: UI（既存を拡張）
  egui-app/src/ui/widgets/importance_chart.rs
  ├── エラーバーオーバーレイ描画
  └── p値インラインテキスト表示
```

---

## コンポーネント構成

### Layer 1: 統計計算コア（新規） 🔵

**信頼性**: 🔵 *ユーザヒアリング（高精度選択・配置選択）より*

**ファイル**: `rust_core/src/core/math/statistics.rs`

```rust
// t分布のCDF（Gauss-Legendre積分または不完全ベータ関数による高精度実装）
// 誤差 < 10^{-6}
pub fn student_t_cdf(t: f64, df: f64) -> f64

// 両側p値（t分布）
pub fn t_two_sided_p(t_stat: f64, df: f64) -> f64

// 片側p値（t分布、重要度は非負なので上側）
pub fn t_one_sided_upper_p(t_stat: f64, df: f64) -> f64

// 正規分布CDF（Abramowitz & Stegun 近似、誤差 < 1.5×10^{-7}）
pub fn normal_cdf(z: f64) -> f64

// 正規分布の両側p値
pub fn z_two_sided_p(z_stat: f64) -> f64

// Bonferroni補正
pub fn bonferroni_adjust(p_values: &[f64], n_params: usize) -> Vec<f64>

// 有意性マーク（補正済みp値から）
pub fn significance_mark(p_adjusted: f64) -> &'static str
// returns: "***" | "**" | "*" | ""
```

**t分布CDF実装方式**: 不完全ベータ関数の継続分数展開（Lentz法）
- `x = df / (df + t²)` として `I_x(df/2, 1/2)` を計算
- 自由度が大きい場合（df > 30）は正規近似にフォールバック

---

### Layer 2: メトリクス計算関数の拡張 🔵

**信頼性**: 🔵 *ユーザヒアリング・既存実装より*

#### Spearman（`spearman.rs` 拡張）

```rust
pub struct SpearmanStats {
    pub rho: f64,
    pub p_value_raw: Option<f64>,    // 両側p値
    pub ci_lower: Option<f64>,       // 95% CI 下限（Fisher z変換）
    pub ci_upper: Option<f64>,       // 95% CI 上限
}

pub fn compute_spearman_with_stats(x: &[f64], y: &[f64]) -> SpearmanStats
```

アルゴリズム:
1. `rho = compute_spearman(x, y)` （既存）
2. `n < 4` なら `None` を返す
3. `t = rho * sqrt(n-2) / sqrt(1 - rho²)` を計算
4. `p = t_two_sided_p(t, (n-2) as f64)`
5. Fisher z: `z = atanh(rho)`, `se_z = 1/sqrt(n-3)`
6. `ci = [tanh(z - 1.96*se_z), tanh(z + 1.96*se_z)]`

---

#### Ridge（`ridge.rs` 拡張）

```rust
pub struct RidgeStats {
    pub beta: Vec<f64>,
    pub r_squared: f64,
    pub std_errors: Option<Vec<f64>>,
    pub p_values_raw: Option<Vec<f64>>,
    pub ci_lower: Option<Vec<f64>>,
    pub ci_upper: Option<Vec<f64>>,
    pub is_approximate: bool,   // α>0 バイアス注意フラグ
}

pub fn compute_ridge_with_stats(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    alpha: f64,
) -> RidgeStats
```

アルゴリズム（対角成分のみ追加ガウス消去）:
1. 既存の Ridge 計算（β, R²）を実行
2. `A = X^T X + αI`（p×p、既に計算済み）を保存
3. 残差分散: `σ² = RSS / (n - p - 1)` （`n ≤ p+2` なら `None`）
4. 対角逆行列: 各 j = 0..p について `A^{-1}_{jj}` をガウス消去で取得
   - `e_j = [0,..,1,..,0]`（第j標準基底）を右辺として `Ax = e_j` を解く
   - j番目の要素のみを使用 → p 回の O(p²) ガウス消去
5. `SE_j = sqrt(σ² × A^{-1}_{jj})`
6. `t_j = β_j / SE_j`、`df = n - p - 1`
7. `p_j = t_two_sided_p(t_j, df)`, `ci_j = β_j ± t_{0.025,df} × SE_j`

---

#### RF-ANOVA / MDI / SHAP（ツリー間分散） 🔵

**信頼性**: 🔵 *ユーザヒアリング（ツリー間分散選択）・既存実装より*

変更方針: 現在の `compute_rf_anova_importances()` は正規化後の重要度（合計=1）を返すが、
統計計算では **正規化前の raw 重要度**（木ごとの置換MSE差分）を使用する必要がある。

```rust
pub struct TreeStats {
    pub mean_importance: Vec<f64>,      // 正規化後（既存互換）
    pub std_error: Option<Vec<f64>>,    // 正規化前 raw の SE
    pub p_values_raw: Option<Vec<f64>>, // 片側t検定（df=99）
    pub ci_lower: Option<Vec<f64>>,     // mean_raw ± t_{0.025,99} × SE（正規化後スケール）
    pub ci_upper: Option<Vec<f64>>,
}
```

アルゴリズム変更:
1. 各木 t = 0..100 で `imp_t[param_j]` を個別記録（`Vec<Vec<f64>>` → [tree][param]）
2. `mean_raw_j = mean(imp_t[*][j])` （正規化前）
3. `se_j = std(imp_t[*][j]) / sqrt(n_trees)` （標準誤差）
4. `t_j = mean_raw_j / se_j`（`se_j ≈ 0` なら `None`）
5. `p_j = t_one_sided_upper_p(t_j, 99.0)`
6. 最終正規化後の CI: `ci_j = normalized_mean_j ± (se_j / sum_all_means) × t_{0.025,99}`

**MDI固有**: `imp_t[t][j]` = 木tのパラメータjの加重不純度低下量（正規化前）

---

#### Sobol（Jansen分散） 🔵

**信頼性**: 🔵 *ユーザヒアリング（Jansen分散選択）より*

```rust
// sobol.rs 内の compute_sobol() 関数を拡張
// 既存の f_A, f_B, f_AB_i の評価値から追加計算

pub struct SobolIndexStats {
    pub s_i: f64,              // 既存（一次指標）
    pub var_s_i: Option<f64>,  // Jansen分散
    pub ci_lower: Option<f64>, // s_i - 1.96 × sqrt(var_s_i)（クリップ≥0）
    pub ci_upper: Option<f64>, // s_i + 1.96 × sqrt(var_s_i)
    pub p_value_raw: Option<f64>, // 正規近似 z = s_i / sqrt(var_s_i)
}
```

Jansen分散公式（f_A, f_B, f_AB_i が揃っている状態で追加計算）:
```
d_j = f_B[j] * (f_AB_i[j] - f_A[j])   （一次指標の被積分量）
Var_S_i = Var(d) / (N × Var_Y²)         （Jansen 1999）
p_value = z_two_sided_p(s_i / sqrt(Var_S_i))
```

---

### Layer 3: types.rs の型拡張 🔵

**信頼性**: 🔵 *ユーザヒアリング・要件定義REQ-STAT-060〜063より*

**変更方針**: 既存フィールドはすべて保持し、`Option<...>` フィールドを追加することで後方互換性を保つ。

```rust
// rust_core/src/sensitivity/types.rs

pub struct SensitivityResult {
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub spearman: Vec<Vec<f64>>,
    // 追加
    pub spearman_p_values: Option<Vec<Vec<f64>>>,   // [obj][param]
    pub spearman_ci_lower: Option<Vec<Vec<f64>>>,
    pub spearman_ci_upper: Option<Vec<Vec<f64>>>,
    pub ridge: Vec<RidgeResult>,
    pub rf_anova: Option<RfAnovaResult>,
    pub mdi: Option<MdiResult>,
    pub shap: Option<ShapResult>,
}

pub struct RidgeResult {
    pub beta: Vec<f64>,
    pub r_squared: f64,
    // 追加
    pub std_errors: Option<Vec<f64>>,
    pub p_values: Option<Vec<f64>>,
    pub ci_lower: Option<Vec<f64>>,
    pub ci_upper: Option<Vec<f64>>,
    pub is_approximate: bool,
}

pub struct RfAnovaResult {
    pub importances: Vec<Vec<f64>>,
    pub r_squared: Vec<f64>,
    // 追加
    pub p_values: Option<Vec<Vec<f64>>>,    // [param][objective]
    pub ci_lower: Option<Vec<Vec<f64>>>,
    pub ci_upper: Option<Vec<Vec<f64>>>,
}
// MdiResult, ShapResult も同様

pub struct SobolResult {
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub first_order: Vec<Vec<f64>>,
    pub total_effect: Vec<Vec<f64>>,
    pub r_squared: Vec<f64>,
    pub n_samples: usize,
    // 追加
    pub first_order_ci_lower: Option<Vec<Vec<f64>>>,
    pub first_order_ci_upper: Option<Vec<Vec<f64>>>,
    pub first_order_p_values: Option<Vec<Vec<f64>>>,
    pub total_effect_ci_lower: Option<Vec<Vec<f64>>>,
    pub total_effect_ci_upper: Option<Vec<Vec<f64>>>,
    pub total_effect_p_values: Option<Vec<Vec<f64>>>,
}
```

---

### Layer 4: egui UI の拡張 🔵

**信頼性**: 🔵 *ユーザヒアリング（エラーバーオーバーレイ選択）・既存実装より*

**ファイル**: `egui-app/src/ui/widgets/importance_chart.rs`

**現在の描画ループ**（変更対象）:
```rust
for (name, score) in &scores {
    ui.horizontal(|ui| {
        // ラベル
        // バー
        ui.label(format!("{score:.3}"));
    });
}
```

**変更後**:
```rust
for (name, score, ci_lower, ci_upper, p_adj, sig_mark) in &rows {
    ui.horizontal(|ui| {
        // ラベル（変更なし）
        let (rect, _) = ui.allocate_exact_size(...);

        // バー描画（変更なし）
        // エラーバーオーバーレイ（追加）
        if let (Some(lo), Some(hi)) = (ci_lower, ci_upper) {
            // I字型エラーバーをrect右端に描画
            let lo_x = rect.min.x + (lo / max_score * bar_max_width) as f32;
            let hi_x = rect.min.x + (hi / max_score * bar_max_width).min(bar_max_width) as f32;
            let cy = rect.center().y;
            let painter = ui.painter();
            // 横棒
            painter.line_segment([pos2(lo_x, cy), pos2(hi_x, cy)], Stroke::new(1.5, CI_COLOR));
            // 縦棒（下限）
            painter.line_segment([pos2(lo_x, cy-4.0), pos2(lo_x, cy+4.0)], Stroke::new(1.5, CI_COLOR));
            // 縦棒（上限）
            painter.line_segment([pos2(hi_x, cy-4.0), pos2(hi_x, cy+4.0)], Stroke::new(1.5, CI_COLOR));
        }

        // p値テキスト + 有意性マーク（追加）
        if let Some(p) = p_adj {
            let text = format!("{:.3} {sig_mark}");
            ui.label(egui::RichText::new(text).color(p_value_color(*p)));
        } else {
            ui.label(format!("{score:.3}"));
        }
    });
}
```

**補助関数（追加）**:
```rust
// Bonferroni補正p値の色分け
fn p_value_color(p_adjusted: f64) -> egui::Color32
// p < 0.05 → 青、p < 0.01 → 緑、p < 0.001 → 濃緑、非有意 → グレー

// 現在のスコアから統計情報を取得
fn get_stat_info(
    result: &SensitivityResult,
    metric: &ImportanceMetric,
    param_idx: usize,
    obj_idx: usize,
    n_params: usize,
) -> (Option<f64>, Option<f64>, Option<f64>)
// returns: (ci_lower, ci_upper, p_adjusted)
```

---

## ディレクトリ構造への影響 🔵

**信頼性**: 🔵 *既存プロジェクト構造より*

```
rust_core/src/
├── core/math/
│   ├── linear_algebra.rs      （既存・変更なし）
│   ├── mod.rs                 ← statistics モジュールを追加
│   └── statistics.rs          ← 新規追加（t分布CDF、正規CDF、Bonferroni）
│
└── sensitivity/
    ├── types.rs               ← 全構造体に統計フィールドを追加
    ├── spearman.rs            ← compute_spearman_with_stats() 追加
    ├── ridge.rs               ← compute_ridge_with_stats()、対角逆行列 追加
    ├── rf_anova.rs            ← 木ごとの raw 重要度記録に変更
    ├── mdi.rs                 ← 同上
    ├── shap.rs                ← 同上
    ├── sobol.rs               ← Jansen分散計算 追加
    ├── analysis/
    │   ├── full.rs            ← 統計フィールドを埋めるよう更新
    │   └── selected.rs        ← 同上
    └── tests.rs               ← 統計計算の単体テスト追加

egui-app/src/
└── ui/widgets/
    └── importance_chart.rs    ← エラーバー・p値表示を追加
```

新規ファイル: `rust_core/src/core/math/statistics.rs` の **1ファイルのみ**。
その他はすべて既存ファイルへの加算的変更。

---

## 非機能要件の実現方法

### パフォーマンス 🟡

**信頼性**: 🟡 *NFR-STAT-001〜003の要件から妥当な推測*

| メトリクス | 追加計算内容 | 推定追加時間 |
|---------|---------|---------|
| Spearman | t統計量 + Fisher z変換 | < 1ms（定数時間） |
| Ridge | p回のガウス消去（O(p³)） | p=30で < 5ms |
| RF-ANOVA/MDI/SHAP | 木ごとの分散計算 | < 10ms（既存ループに追加） |
| Sobol | Jansen分散（N次の和） | N=1024で < 20ms |

**目標**: すべてのメトリクスで +50ms 以内（NFR要件の最大値）

### 数値安定性 🔵

**信頼性**: 🔵 *NFR-STAT-010〜012・数学的制約より*

- `ρ = ±1.0` → t統計量 = ±∞ → p値を 0.0 にクリップ
- `SE = 0` → t統計量 = NaN → p値を `None`（`statistics::significance_mark()` は空文字返却）
- 信頼区間下限 < 0 → クリップして `max(ci_lower, 0.0)`（重要度は非負）
- `Var_Y < ε` → Sobol統計は `None`（既存処理と一貫）

### 外部ライブラリ 🔵

**信頼性**: 🔵 *既存プロジェクト方針・NFR-STAT-020より*

- 外部クレート追加ゼロ（既存の制約と同一）
- t分布・正規分布はすべて pure Rust で実装

---

## 技術的制約

### Ridge の近似バイアス 🔵

**信頼性**: 🔵 *Ridge理論・REQ-STAT-025より*

正則化項 α が β に対してバイアスを導入するため、t検定のp値は近似値。
`RidgeResult.is_approximate = true` を設定し、UIで「～」プレフィックスを表示。
α を小さくするほどバイアスは小さくなるが、現行 α=1.0 を変更しない方針。

### MDI の正規化制約 🟡

**信頼性**: 🟡 *MDI特性・REQ-STAT-034より*

MDI の正規化後重要度は合計=1制約があり、木間で独立でない。
対策: 正規化前の raw 不純度低下量を木ごとに記録し、その分散でt検定を行う。
最終表示は正規化後のスケールに戻して CI を表示する（スケール変換が必要）。

---

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **型定義（Rust）**: [interfaces.rs](interfaces.rs)
- **実装ガイド**: [implementation-guide.md](implementation-guide.md)
- **ヒアリング記録**: [design-interview.md](design-interview.md)
- **要件定義**: [requirements.md](../../spec/sensitivity-analysis-statistical-importance/requirements.md)

---

## 信頼性レベルサマリー

- 🔵 青信号: 15件 (88%)
- 🟡 黄信号: 2件 (12%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
