# 可視化機能有効化 コンテキストノート

**生成日**: 2026-03-29

## プロジェクト基本情報

- **リポジトリ**: c:\Users\hiroa\Desktop\Tunny-Dashboard
- **技術スタック**: React 19.2.4 + TypeScript 5.9 / Rust WASM (wasm-pack) / ECharts 6.0.0 / Zustand 5.0.5
- **テスト**: Vitest 3.2.4 / Playwright 1.58.2 / React Testing Library

## 実装対象チャートの現状（本スコープ）

| ChartId | FreeLayoutCanvas 状態 | UI コンポーネント | Rust 実装 | WASM Binding |
|---|---|---|---|---|
| `importance` | ダミー実装（全値 1.0） | なし（FreeLayoutCanvas 内インライン） | `sensitivity.rs` 完了 | なし |
| `sensitivity-heatmap` | default フォールバック | `SensitivityHeatmap.tsx` 完成 | `sensitivity.rs` 完了 | なし |
| `cluster-view` | default フォールバック | なし（新規作成必要） | `clustering.rs` 完了 | なし |
| `umap` | default フォールバック | なし（新規作成必要） | なし（PCA で代替） | なし |

## 実装済み Rust 関数シグネチャ

### sensitivity.rs

```rust
pub fn compute_sensitivity() -> Option<SensitivityResult>
pub fn compute_sensitivity_selected(indices: &[u32]) -> Option<SensitivityResult>
```

`SensitivityResult`:
```rust
pub struct SensitivityResult {
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub spearman: Vec<Vec<f64>>,  // [nParams][nObjectives]
    pub ridge: Vec<RidgeResult>,  // [nObjectives]
}
pub struct RidgeResult {
    pub beta: Vec<f64>,   // [nParams]
    pub r_squared: f64,
}
```

### clustering.rs

```rust
pub fn run_pca(n_components: usize, space: PcaSpace) -> Option<PcaResult>
pub fn run_kmeans(k: usize, flat_data: &[f64], n_cols: usize) -> KmeansResult
pub fn estimate_k_elbow(flat_data: &[f64], n_cols: usize, max_k: usize) -> ElbowResult
pub fn compute_cluster_stats(labels: &[usize]) -> Vec<ClusterStat>
```

`PcaSpace`:
```rust
pub enum PcaSpace { Param, Objective, All }
```

## 重要な実装上の注意事項

### ⚠️ computeClusterStats の Int32Array → usize 変換

WASM バインディング `wasm_compute_cluster_stats(labels: js_sys::Int32Array)` は、
`labels` の各値を `usize` に変換してから `compute_cluster_stats` に渡す必要がある。

- **k-means（本フェーズ）**: ラベルは常に `0` 以上の整数 → `as usize` で安全
- **HDBSCAN（REQ-081 の将来拡張）**: ノイズ点に `-1` を使用 → 変換前に負値チェックが必要

本フェーズは k-means のみ対象のため、`i32 as usize` の変換で問題ない。
HDBSCAN 追加時は `labels[i] >= 0` のフィルタリングを WASM バインディング側に追加する必要がある。

### WASM ビルドの再実行が必要

lib.rs に新規 `#[wasm_bindgen]` 関数を追加した後、必ず wasm-pack ビルドを実行すること:

```bash
cd rust_core
wasm-pack build --target web --out-dir ../frontend/src/wasm/pkg
```

ビルド後 `tunny_core.d.ts` が自動更新されるが、型定義の手動追加（REQ-VE-010〜016）は
現プロジェクトのパターンに従い手動で `tunny_core.d.ts` を更新する。

### Zustand ストアのパターン

既存の `studyStore.ts` / `selectionStore.ts` のパターンに従うこと:
- `create<StateType>()(set, get) => ({ ... })`
- WASM 呼び出しは `WasmLoader.getInstance().then(wasm => ...)` 経由
- エラーハンドリングは try/catch で `loadError` / `clusterError` にセット

### analysisStore の Study 変更リセット

`studyStore` の `selectStudy` アクション内で直接リセットするか、
または `analysisStore` が `useStudyStore.subscribe` でリセットするパターンが考えられる。
既存の `selectionStore` の実装を参考に統一したパターンを採用すること。

## 既存テスト件数の確認

本スコープ実装後のフロントエンドテスト基準（現在の件数）:

```
Test Files: 24 passed
Tests: 159 passed (0 failed)
```

新規追加後は `analysisStore.test.ts`・`clusterStore.test.ts`・
各チャートコンポーネントのテストが追加されることで件数が増加する。

## 関連ファイル一覧

**変更対象（既存）**:
- `rust_core/src/lib.rs` — WASM バインディング追加
- `frontend/src/wasm/pkg/tunny_core.d.ts` — 型定義追加
- `frontend/src/wasm/wasmLoader.ts` — WasmLoader メソッド追加
- `frontend/src/components/layout/FreeLayoutCanvas.tsx` — 4 チャート配線
- `frontend/src/components/panels/LeftPanel.tsx` — ClusterPanel → clusterStore 接続

**新規作成（フロントエンド）**:
- `frontend/src/stores/analysisStore.ts` / `.test.ts`
- `frontend/src/stores/clusterStore.ts` / `.test.ts`
- `frontend/src/components/charts/ImportanceChart.tsx` / `.test.tsx`（Importance チャートを独立コンポーネント化）
- `frontend/src/components/charts/ClusterScatter.tsx` / `.test.tsx`（ClusterView 用）
- `frontend/src/components/charts/DimReductionScatter.tsx` / `.test.tsx`（UMAP チャート用）
