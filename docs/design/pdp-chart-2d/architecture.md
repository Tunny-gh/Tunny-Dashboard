# PDP Chart 2D アーキテクチャ設計

**作成日**: 2026-04-15
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *既存コード調査・ユーザヒアリングより*

`pdp_2d.rs` に実装済みの 2D Partial Dependence Plot（ヒートマップ）ウィジェットを、グリッドキャンバスに接続して利用可能にする。また Kriging / Sparse Kriging モデル選択時は不確実性（標準偏差）を第2ヒートマップとして表示する。

現在の問題は4点:
1. `ChartId` enum に `PdpChart2D` バリアントが存在しない → チャートピッカーに表示されない
2. `grid_canvas.rs` の `show_chart()` に対応ケースがない → グリッドに配置できない
3. `PdpResult2d` を生成する非同期計算パスが未実装 → データが `None` のまま
4. `pdp_2d.rs` にモデル種別選択 UI・目的関数選択 UI・"Run" ボタンが未実装

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存 egui-app アーキテクチャ（toolbar.rs, layout.rs, app.rs）より*

- **パターン**: immediate mode widget + AppMessage 非同期計算パターン
- **計算スポーン**: `layout.rs` の `tx` を `show_main_canvas` → `show_grid_canvas` → `show_chart()` に伝播し、"Run" ボタン押下時に `spawn_task` を呼び出す
- **結果受信**: `AppMessage::Pdp2dDone(PdpResult2d)` を新設し、`app.rs::poll_messages()` で `widget_states.pdp_2d` に格納

---

## コンポーネント構成

### 既存リソース（変更なし） 🔵

**信頼性**: 🔵 *既存コード調査より*

| リソース | ファイル | 状態 |
|----------|----------|------|
| `PdpChart2DState` struct | `egui-app/src/ui/widgets/pdp_2d.rs` | ✅ 実装済み（拡張が必要） |
| `PdpChart2DState::show()` | 同上 | ✅ ヒートマップ描画・spinner・空状態実装済み（UI拡張が必要） |
| `WidgetStates::pdp_2d` フィールド | `egui-app/src/ui/widget_states.rs` | ✅ 存在 |
| `PdpResult2d` 型（egui-app版） | `egui-app/src/state/messages.rs` | ✅ 定義済み（フィールド追加が必要） |
| `rust_core::pdp::api::compute_pdp_2d` | `rust_core/src/pdp/api.rs` | ✅ 実装済み |
| `GpModel` struct | `rust_core/src/core/kriging/gaussian_process/model.rs` | ✅ 存在（フィールド追加が必要） |
| `train_gp` | `rust_core/src/core/kriging/gaussian_process/training.rs` | ✅ 存在（L保存が必要） |
| FITC実装 | `rust_core/src/core/kriging/sparse_fitc.rs` | ✅ 存在（variance関数追加が必要） |

### 変更対象ファイル 🔵

**信頼性**: 🔵 *既存コード調査・ユーザヒアリングより*

#### egui-app 側（接続・UI）

| ファイル | 変更内容 | 優先度 |
|----------|----------|--------|
| `egui-app/src/state/layout_state.rs` | `ChartId::PdpChart2D` バリアント追加・`all()`・`label()` 更新 | **必須** |
| `egui-app/src/state/messages.rs` | `AppMessage::Pdp2dDone(PdpResult2d)` バリアント追加 | **必須** |
| `egui-app/src/ui/widgets/pdp_2d.rs` | モデル種別選択 UI 追加・目的関数選択 UI 追加・"Run" ボタン追加・`pending_compute` フィールド追加・不確実性ヒートマップ描画追加 | **必須** |
| `egui-app/src/ui/layout.rs` | `show_main_canvas` に `&tx` を渡す | **必須** |
| `egui-app/src/ui/main_canvas.rs` | `tx` パラメータ追加・`show_grid_canvas` に伝播 | **必須** |
| `egui-app/src/ui/grid_canvas.rs` | `tx` パラメータ追加・`ChartId::PdpChart2D` ケース追加・計算スポーン | **必須** |
| `egui-app/src/app.rs` | `AppMessage::Pdp2dDone` ハンドラ追加 | **必須** |

#### rust_core 側（不確実性計算）

| ファイル | 変更内容 | 優先度 |
|----------|----------|--------|
| `rust_core/src/pdp/types.rs` | `PdpResult2d` に `uncertainties: Option<Vec<Vec<f64>>>` フィールド追加 | **必須** |
| `rust_core/src/core/kriging/gaussian_process/model.rs` | `GpModel` に `l: Vec<Vec<f64>>`・`log_sn: f64` フィールド追加 | **必須** |
| `rust_core/src/core/kriging/gaussian_process/training.rs` | `train_gp` でコレスキー因子 `l` と `log_sn` を `GpModel` に保存 | **必須** |
| `rust_core/src/core/kriging/gaussian_process/inference.rs` | `predict_variance(model, x_test) -> f64` 関数追加 | **必須** |
| `rust_core/src/core/kriging/sparse_fitc.rs` | `SparseFitcModel` 構造体追加・`fitc_train` 関数追加・`fitc_predict_variance` 関数追加 | **必須** |
| `rust_core/src/pdp/kriging_core.rs` | `compute_pdp_2d_kriging_raw` と `compute_pdp_2d_sparse_kriging_raw` で分散グリッドを計算し `uncertainties` に設定 | **必須** |

---

## PdpResult2d 型の不整合解消 🔵

**信頼性**: 🔵 *既存コード調査より判明した問題*

現状 2 箇所で異なる `PdpResult2d` が定義されている:

| 定義箇所 | フィールド構成 |
|----------|----------------|
| `egui-app/src/state/messages.rs` | `x_values`, `y_values`, `z_values`, `param1_name`, `param2_name`, `objective_name` |
| `rust_core/src/pdp/types.rs` | `grid1`, `grid2`, `values`, `r_squared`, `param1_name`, `param2_name`, `objective_name` |

実装時は `egui-app/src/state/messages.rs` の `PdpResult2d` を正とし、`grid_canvas.rs` の変換ロジックで rust_core の返り値を egui-app の型に変換する。`uncertainties: Option<Vec<Vec<f64>>>` は両方に追加する（rust_core 側で計算し、変換時にそのまま移送）。

---

## 計算トリガーアーキテクチャ 🔵

**信頼性**: 🔵 *既存 `spawn_task` パターン（toolbar.rs）より*

`tx: mpsc::SyncSender<AppMessage>` を `layout.rs` から grid_canvas まで伝播する。

```
layout.rs::show_layout(app, ctx)
  let tx = app.sender();
  │
  ▼
main_canvas.rs::show_main_canvas(ui, app_state, layout, widgets, &tx)  ← &tx 追加
  │
  ▼
grid_canvas.rs::show_grid_canvas(ui, app_state, layout, widgets, &tx)  ← &tx 追加
  │
  ▼
grid_canvas.rs::show_chart(ui, app_state, widgets, chart_id, &tx)      ← &tx 追加
  │
  ├── case ChartId::PdpChart2D
  │     widgets.pdp_2d.show(ui, &param_names, &obj_names)
  │     if widgets.pdp_2d.pending_compute.take() == Some(req):
  │       widgets.pdp_2d.computing = true
  │       spawn_task(tx.clone(), || compute_pdp_2d(...) → Pdp2dDone)
  │
  └── (他のケースは &tx を無視)
```

### `pending_compute` フィールドの追加 🔵

**信頼性**: 🔵 *egui immediate mode 既存パターンより*

`PdpChart2DState` に `pending_compute: Option<Pdp2dComputeRequest>` フィールドを追加する。

```rust
pub struct Pdp2dComputeRequest {
    pub param1: String,
    pub param2: String,
    pub objective: String,
    pub n_grid: usize,
    pub model_type: String,
}

pub struct PdpChart2DState {
    pub selected_param1: String,
    pub selected_param2: String,
    pub selected_objective: usize,
    pub selected_model: ModelType,       // ← 追加
    pub result: Option<PdpResult2d>,
    pub computing: bool,
    pub pending_compute: Option<Pdp2dComputeRequest>,  // ← 追加
}
```

`show()` 内で "Run" ボタンが押されたとき `self.pending_compute = Some(req)` をセットし、`show_chart()` がこれを `.take()` してスポーン処理を行う。

---

## `pdp_2d.rs` の追加 UI 🔵

**信頼性**: 🔵 *既存コード調査（`let _ = obj_names` で未使用）・ユーザヒアリングより*

`pdp_chart.rs` の `ModelType` enum と UI パターンを参照して実装する。現在 `obj_names` が `let _ = obj_names` で無視されている。モデル種別選択・目的関数選択 UI・"Run" ボタンを追加する。

```
[Parameter 1: ▼ x1] [Parameter 2: ▼ x2]
[Objective:   ▼ obj1] [Model: ▼ Ridge]
[         Run 2D PDP         ]   ← 追加
```

**注意**: `pdp_chart.rs` の `ModelType` enum は `pdp_2d.rs` でも再利用またはインポートする。  
モデルリスト: `Ridge`・`Kriging`・`Sparse Kriging`（`pdp_chart.rs:ModelType` 参照）。

---

## `AppMessage::Pdp2dDone` 🔵

**信頼性**: 🔵 *既存 `AppMessage::SensitivityDone` / `ClusteringDone` パターンより*

`messages.rs` に追加:

```rust
pub enum AppMessage {
    // ... 既存 ...
    Pdp2dDone(PdpResult2d),  // ← 追加
}
```

`app.rs::poll_messages()` に追加:

```rust
AppMessage::Pdp2dDone(result) => {
    self.widget_states.pdp_2d.result = Some(result);
    self.widget_states.pdp_2d.computing = false;
}
```

---

## 不確実性の可視化（Kriging / Sparse Kriging） 🔵

**信頼性**: 🔵 *ユーザヒアリング（不確実性表示要求）+ 既存 GP 実装調査より*

Kriging と Sparse Kriging は GP ポスタリア分散を計算できる。これを `PdpResult2d::uncertainties: Option<Vec<Vec<f64>>>` として格納し、`pdp_2d.rs` で平均ヒートマップの右隣に標準偏差ヒートマップとして表示する。

### GP（Kriging）分散計算 🔵

**信頼性**: 🔵 *`training.rs` 実装（コレスキー因子 `l` が計算済みだが未保存）より*

`GpModel` 構造体を拡張し、コレスキー因子 `l` と観測ノイズ `log_sn` を保存する:

```rust
pub(crate) struct GpModel {
    pub alpha: Vec<f64>,
    pub x_train: Vec<Vec<f64>>,
    pub log_ls: Vec<f64>,
    pub log_sf: f64,
    pub l: Vec<Vec<f64>>,    // ← 追加: chol(K_XX + σ_n² I)
    pub log_sn: f64,         // ← 追加: 観測ノイズ
}
```

`inference.rs` に `predict_variance` を追加:

```rust
// var(x*) = k(x*,x*) + σ_n² - k(X,x*)^T (K_XX + σ_n²I)^{-1} k(X,x*)
//         = k** - ||L^{-1} k_*||²
// ただし L = chol(K_XX + σ_n² I) は GpModel に保存済み
pub(crate) fn predict_variance(model: &GpModel, x_test: &[f64]) -> f64 {
    let k_star: Vec<f64> = model.x_train.iter()
        .map(|x_i| matern52_ard(x_test, x_i, &model.log_ls, model.log_sf))
        .collect();
    let k_ss = matern52_ard(x_test, x_test, &model.log_ls, model.log_sf)
        + (2.0 * model.log_sn).exp();
    // v = L^{-1} k_star  (前進代入)
    let v = forward_sub(&model.l, &k_star);
    let var = k_ss - v.iter().map(|&vi| vi * vi).sum::<f64>();
    var.max(0.0)
}
```

### FITC（Sparse Kriging）分散計算 🔵

**信頼性**: 🔵 *`sparse_fitc.rs` 実装調査より（L_sigma は `fitc_predict_weights` 内で計算済みだが未返却）*

`sparse_fitc.rs` に学習済みモデル構造体と分散予測関数を追加:

```rust
/// FITC 学習済みモデル（平均・分散予測に再利用可能）
pub(crate) struct SparseFitcModel {
    pub w: Vec<f64>,          // posterior weights: Σ^{-1} t
    pub l_sigma: Vec<f64>,    // chol(Σ) (flat row-major M×M)
    pub z: Vec<f64>,          // inducing points (column-major M×n_dims)
    pub params: Vec<f64>,     // [log_ls..., log_sf, log_sn]
    pub m: usize,
}

/// var(x*) = k(x*,x*) - k(Z,x*)^T Σ^{-1} k(Z,x*)
///         = k** - ||L_sigma^{-1} k(Z,x*)||²
pub(crate) fn fitc_predict_variance(model: &SparseFitcModel, x_test: &[f64]) -> f64 {
    let n_dims = model.params.len() - 2;
    let log_ls = &model.params[..n_dims];
    let log_sf = model.params[n_dims];
    let log_sn = model.params[n_dims + 1];
    // k(x*,x*) + σ_n²
    let k_ss = matern52_ard(x_test, x_test, log_ls, log_sf) + (2.0 * log_sn).exp();
    // k(Z,x*) = [k(z_j, x*)]_{j=1..M}
    let kzs: Vec<f64> = (0..model.m)
        .map(|j| {
            let zj: Vec<f64> = (0..n_dims).map(|d| model.z[d * model.m + j]).collect();
            matern52_ard(&zj, x_test, log_ls, log_sf)
        })
        .collect();
    let v = forward_sub_flat(&model.l_sigma, &kzs, model.m);
    let var = k_ss - v.iter().map(|&vi| vi * vi).sum::<f64>();
    var.max(0.0)
}
```

`fitc_train` 関数（新設）を追加し、`SparseFitcModel` を返すよう `kriging_core.rs` から呼び出す:

```rust
pub(crate) fn fitc_train(
    x: &[f64], z: &[f64], y: &[f64], params: &[f64], n: usize, m: usize,
) -> Option<SparseFitcModel> {
    // fitc_predict_weights の内部ロジックを抽出し、w と l_sigma を両方返す
    let (w, l_sigma) = fitc_train_internal(x, z, y, params, n, m)?;
    Some(SparseFitcModel { w, l_sigma, z: z.to_vec(), params: params.to_vec(), m })
}
```

### `pdp_2d.rs` の不確実性ヒートマップ描画 🔵

**信頼性**: 🔵 *既存 `draw_heatmap` 実装 + ユーザヒアリングより*

`result.uncertainties` が `Some` の場合（Kriging / Sparse Kriging）、平均ヒートマップの右隣に σ（標準偏差）ヒートマップを表示する:

```
[    Mean (予測平均)    ] [    σ (標準偏差)     ]
[カラーバー(viridis)   ] [カラーバー(plasma等) ]
```

実装方針:
- 幅を均等2分割して両ヒートマップを描画
- σ ヒートマップは `uncertainties.map(|v| v.sqrt())` で標準偏差に変換してから `draw_heatmap` と同一ロジックで描画（カラーマップは plasma 等、viridis と区別できるもの）
- Ridge モデルの場合は `uncertainties == None` → 現状どおり平均ヒートマップのみ表示

---

## ディレクトリ構造 🔵

**信頼性**: 🔵 *既存プロジェクト構造より*

```
egui-app/src/
├── state/
│   ├── layout_state.rs   ← ChartId::PdpChart2D 追加
│   └── messages.rs       ← AppMessage::Pdp2dDone 追加
├── ui/
│   ├── layout.rs         ← show_main_canvas に &tx 追加
│   ├── main_canvas.rs    ← tx パラメータ追加・伝播
│   ├── grid_canvas.rs    ← tx 追加・PdpChart2D ケース追加
│   └── widgets/
│       └── pdp_2d.rs     ← pending_compute フィールド + モデル選択UI + Run ボタン + 不確実性ヒートマップ
└── app.rs                ← Pdp2dDone ハンドラ追加

rust_core/src/
├── pdp/
│   ├── types.rs          ← PdpResult2d に uncertainties フィールド追加
│   └── kriging_core.rs   ← 分散グリッド計算・uncertainties 設定
└── core/kriging/
    ├── gaussian_process/
    │   ├── model.rs      ← GpModel に l・log_sn フィールド追加
    │   ├── training.rs   ← l・log_sn を GpModel に保存
    │   └── inference.rs  ← predict_variance 追加
    └── sparse_fitc.rs    ← SparseFitcModel・fitc_train・fitc_predict_variance 追加
```

新規ファイルの作成は不要。

---

## スコープ外 🔵

**信頼性**: 🔵 *ユーザヒアリング（PdpChart2D のみ選択）より*

- **ParetoScatter3D**: 今回スコープ外。wgpu GPU レンダリングが必要。ユーザー希望アーキテクチャは wgpu 直接（将来の設計で対応）。

---

## 非機能要件の実現方法

### パフォーマンス 🟡

**信頼性**: 🟡 *`rust_core::pdp::api::compute_pdp_2d` の既存実装から妥当な推測*

- `compute_pdp_2d` は Ridge / Kriging / Sparse Kriging をサポート。`n_grid = 20` 程度で数百ms以内の想定。
- 不確実性計算（分散グリッド）は平均計算と同一ループ内で実施するため追加コストは O(1) per cell（cholesky は一度だけ計算）。
- 重計算は `std::thread::spawn` + AppMessage チャネルで非同期化し、UI をブロックしない。
- 計算中は `computing = true` → spinner 表示（pdp_2d.rs 実装済み）。

### スタイル 🔵

**信頼性**: 🔵 *既存 egui-app スタイル方針より*

- Tailwind CSS 不使用（ネイティブ Rust アプリ）。egui UI API のみ使用。

### エラー耐性 🔵

**信頼性**: 🔵 *既存 EmptyState パターン・pdp_2d.rs 実装より*

- `compute_pdp_2d` が `None` を返した場合（データ不足等）: `AppMessage::Error` で通知。
- `pdp_2d.result` が `None` の状態では "No 2D PDP data" を表示（実装済み）。
- 分散計算が失敗（GP 数値不安定等）した場合: `uncertainties = None` として結果を返し、平均ヒートマップのみ表示する（エラーとして扱わない）。

---

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **ヒアリング記録**: [design-interview.md](design-interview.md)
- **既存 chart-implementation 設計**: [../chart-implementation/architecture.md](../chart-implementation/architecture.md)
- **既存 egui-migration 設計**: [../egui-migration/architecture.md](../egui-migration/architecture.md)

## 信頼性レベルサマリー

- 🔵 青信号: 14件 (88%)
- 🟡 黄信号: 2件 (12%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
