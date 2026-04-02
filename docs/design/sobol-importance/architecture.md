# Sobol感度指数による重要度分析 アーキテクチャ設計

**作成日**: 2026-04-01
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *ユーザヒアリングより*

Optuna最適化結果のパラメータ重要度を分散ベースの感度指数（Sobol指数）で定量化する機能を、
既存の `SensitivityMetric` 体系に統合する。

- **一次 Sobol 指数 S_i**: パラメータ i 単独で説明できる出力分散の割合
- **全効果 Sobol 指数 ST_i**: パラメータ i とそのすべての交互作用を含む分散の割合
- `ST_i - S_i > 0` の場合、交互作用の寄与を示す

Optuna のデータは均一サンプリングではないため、**二次Ridgeサロゲート + Saltelli サンプリング**方式を採用する。

---

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存 architecture.md・ユーザヒアリングより*

既存の4層クライアントサイドアーキテクチャに準拠する。新機能は以下の各層に影響する。

```
Layer 4: UI / Rendering
  ImportanceChart.tsx に 'sobol_first' / 'sobol_total' のケースを追加
        ↕
Layer 3: State Management (Zustand)
  analysisStore.ts に sobolResult: SobolWasmResult | null と computeSobol() を追加
  SensitivityMetric 型に 'sobol_first' | 'sobol_total' を追加
        ↕
Layer 2: JS Bridge
  wasmLoader.ts に computeSobol() メソッドと SobolWasmResult 型を追加
        ↕
Layer 1: WASM Core (Rust)
  sensitivity.rs に SobolResult 構造体・SobolSurrogate 構造体・compute_sobol() を追加
  lib.rs に wasm_compute_sobol() バインディングを追加
```

---

## アルゴリズム設計 🔵

**信頼性**: 🔵 *ユーザヒアリング（サロゲート法・二次Ridge・N=1024選択）より*

### Step 1: 二次Ridgeサロゲートの構築

既存の学習データから二次Ridgeサロゲートを構築する。

**特徴量の構成** (p パラメータの場合):

| 種別 | 特徴量 | 数 |
|------|--------|-----|
| 線形項 | x₁, x₂, …, xₚ | p |
| 二乗項 | x₁², x₂², …, xₚ² | p |
| 交差項 | x₁x₂, x₁x₃, …, x_{p-1}xₚ | p(p-1)/2 |
| **合計** | | **p(p+3)/2** |

p=30 の場合: 30 + 30 + 435 = **495 特徴量**

**サロゲート構築手順**:
1. DataFrameからパラメータ行列 X とターゲット行列 Y を取得
2. 各パラメータを Z スコア標準化: x_std = (x − μ) / σ、(μ, σ) を保存
3. 標準化後の入力から二次特徴量を構築
4. 二次特徴量を再標準化: quad_feat_std = (quad_feat − μ_q) / σ_q、(μ_q, σ_q) を保存
5. Ridge回帰 (α=1.0) を各目的関数に対して適合

### Step 2: Saltelli サンプリング

**PRNG**: LCG64 (外部クレートなしの純 Rust 実装)

```
state_{n+1} = state_n × 6364136223846793005 + 1442695040888963407  (mod 2⁶⁴)
u = (state >> 11) / 2⁵³  →  u ∈ [0, 1)
z = u × 6.0 − 3.0        →  z ∈ [−3, 3]  (標準化空間の近似)
```

**行列生成** (N=1024, p パラメータ):
- 行列 A: N × p のランダム行列 (各要素 U(−3, 3))
- 行列 B: N × p の独立なランダム行列
- 行列 AB_i: A の第 i 列を B の第 i 列で置き換えた行列 (i = 0, …, p−1)

合計 (p+2) 個の行列、各行列 N 行を評価。

### Step 3: サロゲート評価

各行列の全行に対して二次特徴量を構築し、保存した (μ_q, σ_q) で標準化後、
Ridge モデルで評価する。

評価回数: N × (p+2) × (二次特徴量数) = 1024 × 32 × 495 ≈ **16M 演算** (p=30 の場合)

### Step 4: Jansen 推定量によるSobol指数計算

目的関数 k、パラメータ i について:

```
Var_Y_k  = mean(f_A_k²) − mean(f_A_k)²

一次 Sobol 指数 (Saltelli 2010, formula A):
  S_i_k  = mean(f_B_k × (f_AB_i_k − f_A_k)) / Var_Y_k

全効果 Sobol 指数 (Jansen 1999):
  ST_i_k = mean((f_A_k − f_AB_i_k)²) / (2 × Var_Y_k)

出力値を [0, 1] にクリップ
```

---

## コンポーネント構成

### Layer 1: WASM Core の変更 🔵

**信頼性**: 🔵 *既存 sensitivity.rs の設計パターン・ユーザヒアリングより*

**追加する型・関数** (`rust_core/src/sensitivity.rs`):

```rust
pub struct SobolResult {
    pub param_names:    Vec<String>,
    pub objective_names: Vec<String>,
    pub first_order:    Vec<Vec<f64>>,  // [param_idx][obj_idx]
    pub total_effect:   Vec<Vec<f64>>,  // [param_idx][obj_idx]
    pub n_samples:      usize,
}

struct SobolSurrogate {
    n_params:         usize,
    param_means:      Vec<f64>,        // 線形特徴量の標準化パラメータ
    param_stds:       Vec<f64>,
    quad_feat_means:  Vec<f64>,        // 二次特徴量の標準化パラメータ
    quad_feat_stds:   Vec<f64>,
    betas:            Vec<Vec<f64>>,   // [obj_idx][quad_feat_idx]
    intercepts:       Vec<f64>,        // [obj_idx]  (目的関数の平均値)
}

fn build_quad_features(x_std: &[f64]) -> Vec<f64>
fn lcg_rand(state: &mut u64) -> f64
fn build_sobol_surrogate(/* df */) -> Option<SobolSurrogate>
pub fn compute_sobol(n_samples: usize) -> Option<SobolResult>
```

**追加するWASMバインディング** (`rust_core/src/lib.rs`):

```rust
#[wasm_bindgen(js_name = "computeSobol")]
pub fn wasm_compute_sobol(n_samples: u32) -> Result<JsValue, JsValue>
```

出力JSON:
```json
{
  "paramNames":    ["x1", "x2", ...],
  "objectiveNames": ["obj0", ...],
  "firstOrder":   [[S_0_0, S_0_1, ...], [S_1_0, ...], ...],
  "totalEffect":  [[ST_0_0, ...], ...],
  "nSamples":     1024,
  "durationMs":   150.0
}
```

### Layer 2: JS Bridge の変更 🔵

**信頼性**: 🔵 *既存 wasmLoader.ts の実装コード・ユーザヒアリングより*

**追加箇所** (`frontend/src/wasm/wasmLoader.ts`):
- import に `computeSobol as wasmComputeSobol` を追加
- `SobolWasmResult` インターフェースを追加（`SensitivityWasmResult` の直後）
- `WasmLoader` クラスに `computeSobol!: (nSamples: number) => SobolWasmResult` フィールドを追加
- `_initialize()` に `loader.computeSobol = ...` のバインドを追加

### Layer 3: State Management の変更 🔵

**信頼性**: 🔵 *既存 analysisStore.ts の実装コード・ユーザヒアリングより*

**変更箇所** (`frontend/src/stores/analysisStore.ts`):
- `AnalysisState` に `sobolResult`, `isComputingSobol`, `sobolError`, `computeSobol` を追加
- `create()` 初期値に `sobolResult: null, isComputingSobol: false, sobolError: null` を追加
- `computeSobol()` アクションを追加（`computeSensitivity()` と同じパターン）
- Study変更時のリセット処理に `sobolResult`, `isComputingSobol`, `sobolError` を追加

### Layer 4: UI の変更 🔵

**信頼性**: 🔵 *既存 ImportanceChart.tsx の実装コード・ユーザヒアリングより*

**変更箇所** (`frontend/src/components/charts/ImportanceChart.tsx`):
- `type ImportanceMetric` に `'sobol_first' | 'sobol_total'` を追加
- `useAnalysisStore()` の分割代入に `sobolResult`, `isComputingSobol`, `sobolError`, `computeSobol` を追加
- `useEffect` を更新: Sobol メトリクス選択時は `computeSobol()` をトリガー
- score 計算の `if` 分岐に `sobol_first` / `sobol_total` ケースを追加
- `<select>` に `<option value="sobol_first">` と `<option value="sobol_total">` を追加
- `metricLabel` を Sobol メトリクスに対応させる
- 詳細実装は [implementation-guide.md](implementation-guide.md) を参照

---

## ディレクトリ構造への影響 🔵

**信頼性**: 🔵 *既存プロジェクト構造より*

新規ファイルは不要。既存ファイルへの追加変更のみ。

```
rust_core/src/
  sensitivity.rs          ← SobolResult, SobolSurrogate, compute_sobol() を追加
  lib.rs                  ← wasm_compute_sobol() バインディングを追加

frontend/src/wasm/
  wasmLoader.ts           ← SobolWasmResult 型, computeSobol() メソッドを追加

frontend/src/stores/
  analysisStore.ts        ← sobolResult, isComputingSobol, computeSobol() を追加

frontend/src/components/charts/
  ImportanceChart.tsx     ← sobol_first / sobol_total の score 計算・UIを追加
```

---

## 非機能要件との対応

### パフォーマンス 🔵

**信頼性**: 🔵 *ユーザヒアリング (N=1024) + 既存 wasm-api.md の性能目標より*

| 条件 | 推定時間 | 根拠 |
|------|----------|------|
| p=10, N=1024 | ~100ms | 評価回数 1024×22×(55特徴量) |
| p=30, N=1024 | ~500ms | 評価回数 1024×62×(495特徴量) |
| p=30, N=2048 | ~1000ms | 倍スケール |

**目標**: N=1024, p=30 で **2000ms 以内**

サロゲートは線形モデルのため、評価コストは非常に低い (行列積のみ)。

### セキュリティ 🔵

**信頼性**: 🔵 *既存設計の方針より*

- ブラウザ完結型。データは外部に送信しない
- パニックなし: 空データ・p=0 等の境界値で `None` を返す
- NaN値の伝播を抑止: 分散が極小の場合は 0.0 を返す

### 互換性 🔵

**信頼性**: 🔵 *既存アーキテクチャより*

- `SensitivityMetric` への追加は既存の `'spearman'` | `'beta'` 等の動作に影響しない
- `computeSobol()` は `computeSensitivity()` とは独立したエントリポイント

---

## 技術的制約 🔵

**信頼性**: 🔵 *既存 WASM ビルド環境・Cargo.toml より*

- 外部クレート追加不可: WASM バイナリサイズ増加を避けるため、PRNG は LCG64 を自前実装
- p(p+3)/2 の特徴量数: p=30 → 495、p=50 → 1325。メモリは N × 1325 × 8 byte = ~11MB で許容範囲内
- カテゴリカルパラメータ: 既存の ordinal encoding と同様に数値として扱う

---

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.ts](interfaces.ts)
- **ヒアリング記録**: [design-interview.md](design-interview.md)
- **既存感度分析設計**: [../tunny-dashboard/wasm-api.md](../tunny-dashboard/wasm-api.md)
- **既存型定義**: [../tunny-dashboard/interfaces.ts](../tunny-dashboard/interfaces.ts)

---

## 信頼性レベルサマリー

- 🔵 青信号: 16件 (84%)
- 🟡 黄信号: 3件 (16%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
