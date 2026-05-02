# Permutation Feature Importance アーキテクチャ設計

**作成日**: 2026-05-02
**関連要件定義**: [requirements.md](../../spec/permutation-feature-importance/requirements.md)
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *要件定義書 REQ-PFI-001〜007 より*

Tunny Dashboard の Importance Chart に新しいメトリクス **Permutation Feature Importance (PFI)** を追加する。
LightGBM RandomForest モデルを使い、各パラメータを **n_repeats=5 回**シャッフルして MSE 増加量の平均値を重要度スコアとして返す。

既存の RF-Anova（シャッフル1回）と比べ、繰り返しシャッフルにより推定の分散を低減した統計的に安定な実装を提供する。

変更は以下の 7 ファイルに限定される：

| ファイル | 変更内容 |
|---|---|
| `rust_core/src/sensitivity/types.rs` | `SensitivityMetric::Permutation` バリアント追加、`PermutationResult` 構造体追加、`SensitivityResult.permutation` フィールド追加 |
| `rust_core/src/sensitivity/permutation.rs` | **新規作成**。`compute_permutation_importances()` 実装 |
| `rust_core/src/sensitivity/mod.rs` | `mod permutation;` + `pub use` エクスポート追加 |
| `rust_core/src/sensitivity/analysis/full.rs` | `SensitivityMetric::Permutation` ディスパッチケース追加 |
| `egui-app/src/state/results.rs` | `PermutationResult` 構造体追加、`SensitivityResult.permutation` フィールド追加 |
| `egui-app/src/ui/widgets/importance_chart.rs` | `ImportanceMetric::Permutation` バリアント追加 |
| `egui-app/src/ui/chart_registry.rs` | `ImportanceMetric::Permutation` ディスパッチケース追加 |

---

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存プロジェクト Rust/egui アーキテクチャより*

- **パターン**: 既存の 2 層コア/UI 分離パターンを維持
  - `rust_core` クレート: 純粋な計算ロジック（no egui 依存）
  - `egui-app` クレート: UI・状態管理・非同期ディスパッチ

```
Layer 2: UI / Rendering (egui-app)
  importance_chart.rs に ImportanceMetric::Permutation を追加
  chart_registry.rs に Permutation ディスパッチを追加
        ↕ AppMessage::SensitivityDone
Layer 1: Core (rust_core)
  sensitivity/permutation.rs を新規作成
  sensitivity/types.rs に PermutationResult を追加
  sensitivity/analysis/full.rs に Permutation ケースを追加
```

---

## アルゴリズム設計 🔵

**信頼性**: 🔵 *ユーザヒアリング（LightGBM, n_repeats=5, 平均MSE増加量）+ 既存 rf_anova.rs パターンより*

### ステップ 1: データ前処理

1. `y` / `x_matrix` の各行から NaN/Inf を含む行をフィルタリング
2. 有効行が 2 未満 → `(vec![0.0; p], 0.0)` を返し終了
3. 有効行が `PFI_MAX_ROWS=2,000` を超える場合、LCG シード=42 でダウンサンプリング
4. Fisher-Yates シャッフル（シード=43）で 80/20 holdout 分割

### ステップ 2: LightGBM RF 学習

| パラメータ | 値 | 根拠 |
|---|---|---|
| `num_iterations` | 100 | 既存 RF-Anova と同一 |
| `max_depth` | 10 | 既存 RF-Anova と同一 |
| `min_data_in_leaf` | 2 | 既存 RF-Anova と同一 |
| `seed` | 42 | 決定論的再現性 |

train セットで `train_lgbm_rf()` を実行し `LgbmBooster` を取得。

### ステップ 3: ベースライン MSE 計算

eval セットで `lgbm_mse(&booster, x_eval, y_eval)` を実行し `baseline_mse` を取得。

### ステップ 4: n_repeats=5 パーミュテーションループ

各特徴量 `feature_idx` に対して：

```
for feature_idx in 0..p:
    delta_mse_sum = 0.0
    for repeat_idx in 0..N_REPEATS (= 5):
        seed = SEED_BASE + feature_idx * N_REPEATS + repeat_idx
        permuted_x_eval = permute_single_column(x_eval, feature_idx, seed)
        permuted_mse    = lgbm_mse(&booster, &permuted_x_eval, y_eval)
        delta_mse_sum  += max(permuted_mse - baseline_mse, 0.0)
    importances[feature_idx] = delta_mse_sum / N_REPEATS
```

- シード計算式: `42 + feature_idx * 5 + repeat_idx`（決定論的かつ特徴量間で独立）

### ステップ 5: 正規化と R² 計算

```
sum = importances.iter().sum()
if sum < f64::EPSILON:
    importances = vec![0.0; p]
else:
    importances[i] /= sum  (sum = 1.0 に正規化)

r_squared = mse_to_r_squared(baseline_mse, y_eval)
```

---

## コンポーネント構成

### rust_core 側（計算ロジック） 🔵

**信頼性**: 🔵 *既存 sensitivity/ モジュール構造・rf_anova.rs パターンより*

```
rust_core/src/sensitivity/
├── types.rs          ← SensitivityMetric::Permutation 追加
│                        PermutationResult 構造体追加
│                        SensitivityResult.permutation フィールド追加
├── permutation.rs    ← 新規作成
│                        compute_permutation_importances()
│                        permute_single_column()（内部ヘルパー）
│                        normalize()（内部ヘルパー）
├── mod.rs            ← mod permutation; / pub use 追加
└── analysis/
    └── full.rs       ← SensitivityMetric::Permutation ケース追加
```

**`compute_permutation_importances()` シグネチャ**:

```rust
pub fn compute_permutation_importances(
    x_matrix: &[Vec<f64>],
    y: &[f64],
) -> (Vec<f64>, f64)
// Returns: (importances_normalized, r_squared)
// importances.sum() ≈ 1.0 (有効データがある場合)
```

**定数**:

```rust
const N_REPEATS:           usize = 5;
const PFI_MAX_ROWS:        usize = 2_000;
const PFI_SEED_BASE:       u64   = 42;
const PFI_SPLIT_SEED:      u64   = 43;
const PFI_TREES:           usize = 100;
const PFI_MAX_DEPTH:       i32   = 10;
const PFI_MIN_DATA_LEAF:   i32   = 2;
```

**`SensitivityResult` の変更** (`types.rs`):

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

pub struct PermutationResult {                       // ← 新規追加
    pub importances: Vec<Vec<f64>>,  // [param][objective]
    pub r_squared:   Vec<f64>,       // [objective]
}
```

**`full.rs` ディスパッチ追加**:

```rust
SensitivityMetric::Permutation => {
    let x_matrix: Vec<Vec<f64>> = /* RF-Anova と同一の x_matrix 構築 */;
    let y: Vec<f64> = /* RF-Anova と同一の y 構築 */;
    let (imp, r2) = compute_permutation_importances(&x_matrix, &y);
    let permutation = Some(PermutationResult {
        importances: transpose_importances(imp, n_params),
        r_squared: vec![r2],
    });
    SensitivityResult { permutation, ..Default::default_empty() }
}
```

---

### egui-app 側（UI） 🔵

**信頼性**: 🔵 *既存 importance_chart.rs・chart_registry.rs・results.rs パターンより*

```
egui-app/src/
├── state/
│   └── results.rs          ← PermutationResult 追加、SensitivityResult.permutation 追加
└── ui/
    ├── widgets/
    │   └── importance_chart.rs
    │       ← ImportanceMetric::Permutation バリアント追加
    │       ← label() = "Permutation"
    │       ← cache_id() = 7
    │       ← is_sobol() = false
    │       ← compute_sorted_importance() Permutation ケース追加
    │       ← R² 表示 Permutation ケース追加
    │       ← ComboBox Tree-based グループに追加
    └── chart_registry.rs
        ← ImportanceMetric::Permutation → SensitivityMetric::Permutation マッピング追加
        ← SensitivityDone 変換に permutation フィールド追加
```

**`ImportanceMetric` の変更** (`importance_chart.rs`):

```rust
pub enum ImportanceMetric {
    Spearman,
    Ridge,
    RfAnova,
    Mdi,
    SobolFirst,
    SobolTotal,
    Shap,
    Permutation,    // ← 追加 (cache_id = 7)
}
```

**`results.rs` の変更**:

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

#[derive(Debug, Clone)]
pub struct PermutationResult {                       // ← 新規追加
    pub importances: Vec<Vec<f64>>,
    pub r_squared:   Vec<f64>,
}
```

---

## データ型の対応関係 🔵

**信頼性**: 🔵 *既存 chart_registry.rs の rf_anova/shap 変換パターンより*

```
rust_core::sensitivity::permutation.rs
  compute_permutation_importances() → (Vec<f64>, f64)
                ↓ full.rs 変換
  rust_core::sensitivity::SensitivityResult { permutation: Some(PermutationResult {...}) }
                ↓ chart_registry.rs 変換
  AppMessage::SensitivityDone {
    key: (7, obj_idx),
    result: egui_app::state::results::SensitivityResult { permutation: Some(...) }
  }
                ↓ message_handler.rs（既存コード）
  app_state.importance_cache.insert((7, obj_idx), result)
                ↓ importance_chart.rs show()
  compute_sorted_importance(&result, &ImportanceMetric::Permutation, obj_idx)
  → Vec<(String, f64)> → 水平バーチャート描画
```

---

## LightGBM リンク構成 🔵

**信頼性**: 🔵 *rust_core/build.rs・lgbm_sys.rs より（既存 rf_anova.rs と完全共有）*

- `libs/lib_lightgbm.dll` (Windows) / `libs/lib_lightgbm.dylib` (macOS) をワークスペースルートに配置
- `rust_core/build.rs` が `rustc-link-lib=dylib=lib_lightgbm` を設定
- `lgbm_sys.rs` が FFI バインディングを提供
- 新規コードは既存関数（`train_lgbm_rf`, `lgbm_mse`, `mse_to_r_squared`）のみを使用
- **追加の DLL 依存なし**

---

## 非機能要件の実現方法

### パフォーマンス 🟡

**信頼性**: 🟡 *NFR-PFI-001・既存 rf_anova.rs パフォーマンス実績から n_repeats=5 倍考慮*

| 条件 | 推定時間 | 根拠 |
|---|---|---|
| n=100, p=5 | ~500ms | RF-Anova ~100ms × n_repeats=5 |
| n=1000, p=20 | ~2,500ms | RF-Anova ~500ms × n_repeats=5 |
| n=2000, p=20 (MAX) | ~5,000ms | 上限目標値（NFR-PFI-001） |

- バックグラウンドスレッドで `spawn_task()` 実行 → UI スレッドをブロックしない（NFR-PFI-002）

### キャッシュ戦略 🔵

**信頼性**: 🔵 *NFR-PFI-003/004 + 既存 importance_cache パターンより*

- キャッシュキー: `(cache_id=7, obj_idx)` → `app_state.importance_cache` に格納
- Study 変更時: `app_state.clear()` により自動破棄（既存動作）
- 同一 (metric, obj_idx) の再実行: `already_cached` チェックにより no-op

### 決定論性 🔵

**信頼性**: 🔵 *NFR-PFI-005 + 既存 LCG パターンより*

- 全乱数は LCG（Linear Congruential Generator）で固定シード
- シード計算式 `42 + feature_idx * 5 + repeat_idx` で特徴量・繰り返し間でシードが衝突しない
- 同一データセットで常に同一結果を保証

---

## 後方互換性 🔵

**信頼性**: 🔵 *既存メトリクスの独立性より*

- `SensitivityResult.permutation` はデフォルト `None` のため、既存メトリクスの動作に影響しない
- `ImportanceMetric` の既存バリアント（Spearman/Ridge/RfAnova/Mdi/Shap/SobolFirst/SobolTotal）の cache_id・label・動作を変更しない
- 既存テストをすべてパスすること

---

## ディレクトリ構造（変更対象のみ） 🔵

**信頼性**: 🔵 *既存プロジェクト構造より*

```
rust_core/src/sensitivity/
  types.rs          ← PermutationResult, SensitivityMetric::Permutation, SensitivityResult.permutation
  permutation.rs    ← 新規作成
  mod.rs            ← mod permutation; + pub use
  analysis/
    full.rs         ← Permutation ケース追加

egui-app/src/
  state/results.rs                      ← PermutationResult, SensitivityResult.permutation
  ui/widgets/importance_chart.rs        ← ImportanceMetric::Permutation
  ui/chart_registry.rs                  ← ディスパッチ追加
```

---

## 技術的制約

### 新規 LightGBM 依存なし 🔵

**信頼性**: 🔵 *ユーザヒアリング（LightGBM 選択）より*

- 既存の `rust_core::core::lgbm` モジュールの関数のみを使用
- 新規クレート依存不要

### SensitivityResult のデフォルト値 🔵

**信頼性**: 🔵 *既存 full.rs の SensitivityResult 構築パターンより*

- 各メトリクスのケースで `SensitivityResult` を構築する際、関係のないフィールドは `None` または空ベクタ
- `permutation: None` がデフォルト初期値

---

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **実装ガイド**: [implementation-guide.md](implementation-guide.md)
- **設計ヒアリング記録**: [design-interview.md](design-interview.md)
- **要件定義**: [requirements.md](../../spec/permutation-feature-importance/requirements.md)
- **参考（RF-Anova 同等実装）**: `rust_core/src/sensitivity/rf_anova.rs`

---

## 信頼性レベルサマリー

- 🔵 青信号: 18件 (90%)
- 🟡 黄信号: 2件 (10%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
