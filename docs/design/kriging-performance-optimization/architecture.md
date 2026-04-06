# Kriging 高速化 アーキテクチャ設計

**作成日**: 2026-04-05
**関連要件定義**: [requirements.md](../../spec/kriging-performance-optimization/requirements.md)
**ヒアリング記録**: [design-interview.md](design-interview.md)
**前フェーズ アーキテクチャ**: [../surface-plot-surrogate-models/architecture.md](../surface-plot-surrogate-models/architecture.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *要件定義書 + ユーザヒアリングより*

現行 Kriging 実装（L-BFGS × 100 イテレーション × O(N³)）の主スレッドブロック問題と
計算時間超過問題（N=1000 で推定 10〜30 秒）を以下 3 つのアプローチで解決する:

1. **Phase 1**: Rust アルゴリズム最適化（LML/勾配統合・イテレーション削減・サブサンプル縮小）
2. **Phase 2**: Web Worker オフロード（WASM を Worker スレッドで実行、主スレッドブロック解消）
3. **Phase 3**: Sparse Kriging モデル追加（FITC 近似、O(N×M²)、`sparse_kriging` ドロップダウン追加）

---

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存 5 層アーキテクチャ + ユーザヒアリング（Blob URL 方式）より*

- **パターン**: 既存レイヤードアーキテクチャ + Web Worker 非同期オフロード
- **選択理由**: viteSingleFile 制約（単一 HTML、WASM base64 埋め込み）に対応するため
  Worker への WASM 転送に Blob URL 方式を採用

---

## Phase 1: アルゴリズム最適化（Rust コア） 🔵

**信頼性**: 🔵 *kriging.rs ボトルネック分析 + ユーザヒアリングより*

### 1a: LML + 勾配統合計算（REQ-002）

現状: L-BFGS 1 イテレーション = `neg_lml` + `log_ml_gradient` × 2 = 最大 ~4×O(N³)

**変更内容**: `log_ml_with_gradient()` 関数を追加

```rust
/// LML 値と勾配を 1 回の Cholesky/K^{-1} 計算で返す
pub(crate) fn log_ml_with_gradient(
    x: &[Vec<f64>],
    y: &[f64],
    params: &[f64],
) -> (f64, Vec<f64>)
```

`optimize_hyperparams` のメインループを変更:

```rust
// Before: neg_lml 呼び出し + log_ml_gradient 呼び出しが別々
let f_x = neg_lml(&params);
let grad_neg = log_ml_gradient(x, y, &params).iter().map(|g| -g).collect();

// After: 1 回の計算で両方取得
let (f_x, grad_neg) = {
    let (lml, grad) = log_ml_with_gradient(x, y, &params);
    (-lml, grad.iter().map(|g| -g).collect())
};
```

**速度改善**: 1 イテレーション ~2× → max_iter=50 と合わせて合計 ~4× 高速化

### 1b: L-BFGS 削減 + 早期停止（REQ-003）

```rust
// 変更前
pub(crate) fn optimize_hyperparams(x, y, n_iter: usize = 100, m_history: usize = 5) -> Vec<f64>

// 変更後
pub(crate) fn optimize_hyperparams(x, y, n_iter: usize = 50, m_history: usize = 5) -> Vec<f64>
// + 早期停止: 直近 5 イテレーションで |ΔLML| < 1e-3 のとき break
```

### 1c: サブサンプル削減（REQ-004）

```rust
// pdp.rs: compute_pdp_2d_kriging 内
let model = kriging::train_gp(x_2d.clone(), y.to_vec(), 500, 42)?;
//                                                       ^^^
//                                              1000 → 500 に変更
```

**速度改善**: N=5000 で O(1000³) → O(500³) = 8× 高速化

---

## Phase 2: Web Worker オフロード 🔵

**信頼性**: 🔵 *ユーザヒアリング（確実に実装したい、Blob URL 方式）より*

### 2a: 新 WASM 関数 `compute_kriging_raw`（REQ-001-03）

グローバル状態（`OnceLock<Mutex<GlobalState>>`）を使わず、
データを直接 Float64Array で受け取る新しい WASM エントリポイント。

```rust
// lib.rs に追加
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "computeKrigingRaw")]
pub fn wasm_compute_kriging_raw(
    x_flat: &[f64],  // n_samples × n_features (flatten row-major)
    y: &[f64],       // n_samples
    n_samples: u32,
    param1_idx: u32,
    param2_idx: u32,
    n_grid: u32,
    model_type: &str,  // "kriging" | "sparse_kriging"
) -> Result<JsValue, JsValue>
```

**注意**: Worker で WASM を初期化する際は `initWasm()` を再度呼ぶ必要がある（独立したインスタンス）。

### 2b: Kriging Worker（REQ-001-01）

**ファイル**: `frontend/src/wasm/krigingWorker.ts`

```typescript
// Blob URL 方式: Worker スクリプトを文字列→Blob→URLとして生成
// viteSingleFile は WASM を base64 data: URI として埋め込む
// Worker 側は window.__WASM_BASE64__ から base64 を取得して initWasm() に渡す
```

**Worker メッセージプロトコル**:

```typescript
// Main → Worker（postMessage）
interface KrigingWorkerInput {
  type: 'compute'
  xFlat: Float64Array      // transferable
  y: Float64Array          // transferable
  nSamples: number
  param1Idx: number
  param2Idx: number
  nGrid: number
  modelType: 'kriging' | 'sparse_kriging'
}

// Worker → Main（postMessage）
interface KrigingWorkerOutput {
  type: 'result' | 'error'
  result?: Pdp2dWasmResult
  error?: string
}
```

### 2c: analysisStore 変更（REQ-001）

```typescript
// computeSurface3d の実行フロー変更
// Before: メインスレッドで await wasm.computePdp2d(...) （setTimeout(0) 後にブロック）
// After:  Worker に postMessage → onmessage で結果を受け取り store 更新

computeSurface3d: async (param1, param2, objective, nGrid) => {
  set({ isComputingSurface: true, surface3dError: null })

  if (surrogateModelType === 'kriging' || surrogateModelType === 'sparse_kriging') {
    // Worker 経由
    const { xFlat, y, nSamples, p1Idx, p2Idx } = extractData(param1, param2, objective)
    worker.postMessage({ type: 'compute', xFlat, y, nSamples, p1Idx, p2Idx, nGrid, modelType }, [
      xFlat.buffer, y.buffer  // Transferable: ゼロコピー転送
    ])
    // → onmessage コールバックで surface3dCache 更新 + isComputingSurface = false
  } else {
    // Ridge / RF: 従来どおりメインスレッド（高速なので OK）
    const result = wasm.computePdp2d(...)
    ...
  }
}
```

### 2d: viteSingleFile 対応（REQ-001-06）

viteSingleFile は全 JS/WASM を単一 HTML に base64 で埋め込む。
Worker を Blob URL で起動するには:

```typescript
// vite.config.ts に追加設定 (worker inline support)
// Worker ソースを文字列テンプレートとしてバンドルし、
// 実行時に Blob URL を生成して Worker を起動する

function createKrigingWorker(wasmBase64: string): Worker {
  const workerSrc = `
    // Worker のソースコード（文字列として埋め込み）
    ...
  `
  const blob = new Blob([workerSrc], { type: 'application/javascript' })
  return new Worker(URL.createObjectURL(blob))
}
```

---

## Phase 3: Sparse Kriging（FITC 近似） 🔵

**信頼性**: 🔵 *ユーザヒアリング（ドロップダウン追加・K-means誘導点）より*

### 3a: Rust 実装（`kriging.rs` 追加）

**FITC 近似の計算式**:
```
Q(x_i, x_j) = k(x_i, Z) · K_ZZ^{-1} · k(Z, x_j)
K_FITC = Q + diag(K_exact - Q) + σ_n² I
```

**計算量**: O(N×M²) (M=50, N=5000 → 5000×2500 = 12.5M ops vs 10⁹)

```rust
/// Sparse GP with FITC approximation
pub(crate) struct SparseGpModel {
    /// Inducing points (M × 2)
    pub inducing_z: Vec<Vec<f64>>,
    /// K_ZZ^{-1} (M × M, computed via Cholesky)
    pub k_zz_inv: Vec<Vec<f64>>,
    /// alpha = K_FITC^{-1} y (N, but expressed via M inducing points)
    pub alpha: Vec<f64>,
    /// Same kernel hyperparams as GpModel
    pub log_ls: Vec<f64>,
    pub log_sf: f64,
    pub log_sn: f64,
}
```

### 3b: K-means 誘導点選択（ユーザヒアリング）

```rust
/// K-meansクラスタリングでM個の誘導点を選択（external crate不使用）
fn select_inducing_points_kmeans(
    x: &[Vec<f64>],
    m: usize,
    seed: u64,
) -> Vec<Vec<f64>>
```

シンプルな Lloyd's アルゴリズム（最大 50 イテレーション）を純 Rust で実装。

### 3c: UI 変更（REQ-005-05）

```typescript
// frontend/src/types/index.ts
export type SurrogateModelType = 'ridge' | 'random_forest' | 'kriging' | 'sparse_kriging'

// frontend/src/components/charts/SurfacePlot3D.tsx
const MODEL_OPTIONS = [
  { value: 'ridge', label: 'Ridge Regression' },
  { value: 'random_forest', label: 'Random Forest' },
  { value: 'kriging', label: 'Kriging' },
  { value: 'sparse_kriging', label: 'Sparse Kriging' },  // 追加
]

const MODEL_COMPUTE_TIME: Record<SurrogateModelType, string> = {
  ridge: '< 1s',
  random_forest: '< 2s',
  kriging: '< 10s',   // 8×高速化後に更新
  sparse_kriging: '< 5s',  // 追加
}
```

---

## 変更ファイル一覧

**Phase 1 (Rust 最適化):**

| ファイル | 変更内容 |
|---|---|
| `rust_core/src/kriging.rs` | `log_ml_with_gradient()` 追加、`optimize_hyperparams()` max_iter=50・早期停止追加 |
| `rust_core/src/pdp.rs` | `compute_pdp_2d_kriging()` の `train_gp(…, 500, …)` に変更 |

**Phase 2 (Web Worker):**

| ファイル | 変更内容 |
|---|---|
| `rust_core/src/lib.rs` | `wasm_compute_kriging_raw()` WASM バインディング追加 |
| `frontend/src/wasm/krigingWorker.ts` | 新規: Worker スクリプト（WASM 初期化 + 計算） |
| `frontend/src/wasm/wasmLoader.ts` | Worker 生成ユーティリティ追加 |
| `frontend/src/stores/analysisStore.ts` | `computeSurface3d` を Worker 経由に変更 |

**Phase 3 (Sparse Kriging):**

| ファイル | 変更内容 |
|---|---|
| `rust_core/src/kriging.rs` | `SparseGpModel`, `select_inducing_points_kmeans()`, FITC 実装追加 |
| `rust_core/src/pdp.rs` | `compute_pdp_2d_sparse_kriging()` 追加、dispatch 追加 |
| `rust_core/src/lib.rs` | `"sparse_kriging"` dispatch 追加 |
| `frontend/src/types/index.ts` | `SurrogateModelType` に `'sparse_kriging'` 追加 |
| `frontend/src/components/charts/SurfacePlot3D.tsx` | MODEL_OPTIONS・MODEL_COMPUTE_TIME 更新 |

---

## 技術的制約と注意事項

### viteSingleFile + Worker 制約 🟡

**信頼性**: 🟡 *viteSingleFile 仕様 + Blob URL パターン調査より*

- viteSingleFile は Worker インポート構文（`new Worker(new URL(...))`）を直接サポートしない
- **解決策**: Worker ソースを文字列リテラルとしてバンドル → 実行時 Blob URL 生成
- Worker 内での WASM 初期化: `initWasm({ module: wasmModule })` を Worker 内でも呼ぶ
- WASM モジュールインスタンスの共有: SharedArrayBuffer なしで独立インスタンス

### Transferable の活用 🔵

**信頼性**: 🔵 *Web Worker Transferable API より*

```typescript
worker.postMessage(data, [data.xFlat.buffer, data.y.buffer])
// Transferable でゼロコピー転送（N=5000 でも ~400KB 程度）
```

### N < M のフォールバック 🔵

**信頼性**: 🔵 *EDGE-003 要件より*

```rust
// pdp.rs の compute_pdp_2d_sparse_kriging 冒頭
if n < M_INDUCING_POINTS {
    return compute_pdp_2d_kriging(...);  // 標準 Kriging にフォールバック
}
```

### Worker 初期化エラーフォールバック 🟡

**信頼性**: 🟡 *EDGE-001 要件から妥当な推測*

Worker 初期化失敗時は `surface3dError` にエラーを設定し、
ユーザーに再試行を促す（Main Thread フォールバックは複雑度が高いため実装スコープ外）。

---

## システム構成図（Phase 2 完了後） 🔵

**信頼性**: 🔵 *要件定義・ユーザヒアリングより*

```
SurfacePlot3D.tsx
  │ (Ridge/RF) 変更なし
  │ (Kriging/Sparse Kriging) Worker 経由
  ↓
analysisStore.computeSurface3d()
  │ 判定: ridge|rf → 従来どおり WASM 呼び出し
  │ 判定: kriging|sparse_kriging → Worker postMessage
  ↓
KrigingWorker（Web Worker スレッド）
  │ 1. initWasm(wasmModule)  ← Worker 独自 WASM インスタンス
  │ 2. compute_kriging_raw(xFlat, y, ...)
  │ 3. postMessage(result)
  ↓
analysisStore.onmessage()
  │ surface3dCache.set(cacheKey, result)
  │ isComputingSurface = false
  ↓
SurfacePlot3D.tsx → echarts-gl 表示
```

---

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.ts](interfaces.ts)
- **設計ヒアリング**: [design-interview.md](design-interview.md)
- **前フェーズ アーキテクチャ**: [../surface-plot-surrogate-models/architecture.md](../surface-plot-surrogate-models/architecture.md)
- **Kriging 理論**: `theory/kriging.md`

## 信頼性レベルサマリー

- 🔵 青信号: 14件 (70%)
- 🟡 黄信号: 6件 (30%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質
