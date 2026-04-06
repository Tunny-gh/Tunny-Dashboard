# Kriging 高速化 コンテキストノート

**作成日**: 2026-04-05
**プロジェクト**: Tunny Dashboard

---

## 技術スタック

| 層 | 技術 |
| --- | --- |
| WASM コア | Rust（wasm-pack, wasm-bindgen） |
| WASM バンドル | viteSingleFile（単一 HTML 出力） |
| フロントエンド | React 18 + TypeScript + Vite |
| 状態管理 | Zustand |
| チャート | echarts-for-react + echarts-gl |
| テスト | Vitest（フロントエンド）/ cargo test（Rust） |

---

## 関連実装ファイル

| ファイル | 役割 |
| --- | --- |
| `rust_core/src/kriging.rs` | GP 実装: cholesky, log_ml, L-BFGS, predict |
| `rust_core/src/pdp.rs` | compute_pdp_2d_kriging() ディスパッチ |
| `rust_core/src/lib.rs` | WASM バインディング（wasm_compute_pdp_2d） |
| `frontend/src/wasm/wasmLoader.ts` | TypeScript WASM ラッパー |
| `frontend/src/stores/analysisStore.ts` | キャッシュ・状態管理・WASM 呼び出し |
| `frontend/src/components/charts/SurfacePlot3D.tsx` | UI（モデル選択・スピナー表示） |
| `frontend/src/types/index.ts` | SurrogateModelType 型定義 |

---

## 現状の性能ボトルネック

### L-BFGS 1イテレーションの計算量

| 処理 | 呼び出し元 | 計算量 |
| --- | --- | --- |
| neg_lml → build_kernel_matrix + cholesky | optimize_hyperparams | O(N³) |
| log_ml_gradient → build_kernel_matrix + cholesky + K^{-1} | optimize_hyperparams | O(N³) × 2 |
| 線探索での neg_lml（最大20回） | armijo_line_search | O(N³) × 最大20 |
| 次イテレーション開始時の log_ml_gradient | optimize_hyperparams | O(N³) × 2 |

最大 100 イテレーション × 最大 ~6×O(N³) = **最悪 600×O(10⁹) ops (N=1000)**

### 現状パラメータ

```rust
subsample_n = 1000   // N > 1000 のとき 1000 点に削減
n_iter = 100         // L-BFGS 最大イテレーション
m_history = 5        // L-BFGS 履歴サイズ
```

---

## 開発ルール

- 外部クレートなし（純 Rust 実装）
- viteSingleFile: WASM は base64 埋め込み（worker への transfer に特別対応が必要）
- Tailwind CSS 禁止（インラインスタイルを使用）
- 既存テストを破壊しない
- WASM リビルド: `wasm-pack build --target web --out-dir ../frontend/src/wasm/pkg`

---

## WASM 全局状態の仕組み

```rust
// lib.rs
static GLOBAL_STATE: OnceLock<Mutex<GlobalState>> = OnceLock::new();
```

Worker スレッドで Kriging を実行するには:
1. Worker 内で独立した WASM モジュールインスタンスを初期化
2. 計算に必要なデータ（x_flat, y, param 情報）を postMessage で転送
3. Worker 内の WASM で直接計算（グローバル状態を使わない新しいエントリポイント）

---

## Sparse GP (FITC) の概要

M 個の誘導点 Z を使った近似GP:
```
Q(x_i, x_j) = k(x_i, Z) · K_ZZ^{-1} · k(Z, x_j)
K_FITC = Q + diag(K - Q)
```

計算量: O(N×M²)（M=50 なら N=5000 でも N×2500 ≈ 12.5M ops）
vs 標準 Kriging: O(N³) = O(1000³) ≈ 10⁹ ops（サブサンプル後）

誘導点の選択: K-means クラスタリングまたは均等グリッドサンプリング
