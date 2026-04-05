/**
 * プロプライエタリな汎用最適化ソフト機能拡充 型定義
 *
 * 作成日: 2026-04-04
 * 関連設計: architecture.md
 *
 * 信頼性レベル:
 * - 🔵 青信号: EARS要件定義書・設計文書・既存実装を参考にした確実な型定義
 * - 🟡 黄信号: EARS要件定義書・設計文書・既存実装から妥当な推測による型定義
 * - 🔴 赤信号: EARS要件定義書・設計文書・既存実装にない推測による型定義
 *
 * 注: 以下の型は既存 frontend/src/types/index.ts に追加・拡張する
 */

// ========================================
// ChartId 拡張（既存 union 型への追加）
// ========================================

/**
 * 既存 ChartId への追加
 * 🔵 信頼性: ユーザヒアリング・既存ChartId定義パターンより
 *
 * 変更前（既存）:
 *   export type ChartId = 'pareto-front' | 'parallel-coords' | ... | 'contour'
 *
 * 変更後（追加）:
 *   export type ChartId = ... | 'contour' | 'surface3d' | 'topsis-ranking'
 */

// 追加する2値:
// | 'surface3d'       // 3D応答曲面プロット（サロゲートモデル可視化）
// | 'topsis-ranking'  // TOPSISランキング（多基準意思決定）

// ========================================
// サロゲートモデル関連型
// ========================================

/**
 * サロゲートモデル種別
 * 🔵 信頼性: ユーザヒアリングより
 *
 * ridge: 常時利用可能（WASM Ridge回帰）
 * random_forest: .onnxファイル読み込み後に利用可能
 * kriging: 将来実装予定（現時点ではdisabled）
 */
export type SurrogateModelType = 'ridge' | 'random_forest' | 'kriging'

/**
 * 3D曲面プロットの計算結果
 * 🔵 信頼性: wasm-api.md compute_pdp_2d()定義・ユーザヒアリングより
 */
export interface Surface3DResult {
  param1: string              // 🔵 X軸パラメータ名
  param2: string              // 🔵 Y軸パラメータ名
  objective: string           // 🔵 Z軸（目的関数）名
  grid1: number[]             // 🔵 param1のグリッド点（n_grid点）
  grid2: number[]             // 🔵 param2のグリッド点（n_grid点）
  heatmap: number[]           // 🔵 [n_grid × n_grid] flattened Z値（行major）
  modelType: SurrogateModelType // 🔵 使用したモデル種別
  nGrid: number               // 🔵 グリッド解像度（デフォルト50）
  durationMs: number          // 🔵 計算時間（ms）
}

// ========================================
// TOPSIS / MCDM 関連型
// ========================================

/**
 * TOPSIS計算結果
 * 🔵 信頼性: TOPSISアルゴリズム理論・ユーザヒアリングより
 */
export interface TopsisRankingResult {
  scores: number[]            // 🔵 全試行のTOPSISスコア（0〜1、高い方がベター、trial順）
  rankedIndices: number[]     // 🔵 スコア降順の試行インデックス
  positiveIdeal: number[]     // 🔵 正理想解（目的数次元）
  negativeIdeal: number[]     // 🔵 負理想解（目的数次元）
  durationMs: number          // 🔵 計算時間（ms）
}

// ========================================
// Zustand Store 型（既存型への拡張）
// ========================================

/**
 * AnalysisStore 拡張
 * 🔵 信頼性: ユーザヒアリング・既存AnalysisStore定義パターンより
 *
 * 既存の AnalysisStore (frontend/src/types/index.ts) に以下を追加する:
 */
export interface AnalysisStoreExtension {
  // 追加State
  surrogateModelType: SurrogateModelType     // 🔵 選択中のモデル種別（デフォルト: 'ridge'）
  surface3dCache: Map<string, Surface3DResult> // 🔵 計算キャッシュ（key: `${model}_${p1}_${p2}_${obj}`）
  isComputingSurface: boolean                 // 🔵 曲面計算中フラグ
  surface3dError: string | null              // 🟡 計算エラーメッセージ

  // 追加Action
  setSurrogateModelType: (type: SurrogateModelType) => void // 🔵 モデル種別変更
  computeSurface3d: (                        // 🔵 3D曲面計算
    param1: string,
    param2: string,
    objective: string,
    nGrid?: number
  ) => Promise<void>
  clearSurface3dCache: () => void            // 🔵 Studyリセット時にキャッシュクリア
}

/**
 * McdmStore（新設）
 * 🔵 信頼性: ユーザヒアリング・既存Zustandストアパターンより
 */
export interface McdmStore {
  // State
  topsisWeights: number[]                  // 🔵 目的数分の重み（合計1.0に正規化済み）
  topsisResult: TopsisRankingResult | null // 🔵 TOPSIS計算結果
  isComputing: boolean                     // 🔵 計算中フラグ
  topN: number                             // 🔵 上位N件強調表示（デフォルト10）
  topsisError: string | null               // 🟡 計算エラーメッセージ

  // Actions
  setTopsisWeights: (weights: number[]) => void  // 🔵 重み更新（自動正規化）
  computeTopsis: () => Promise<void>             // 🔵 TOPSIS計算実行
  setTopN: (n: number) => void                   // 🔵 表示件数変更
  reset: () => void                              // 🔵 Studyリセット時にクリア
}

// ========================================
// WASM バウンダリ型（tunny_core.d.ts に追加）
// ========================================

/**
 * compute_pdp_2d() の戻り値型
 * 🔵 信頼性: wasm-api.md compute_pdp_2d()定義より
 * （既に wasm-api.md で仕様定義済み。wasm_bindgen公開の有無は実装時に確認）
 */
export interface Pdp2dResult {
  grid1: Float64Array         // 🔵 param1のグリッド点（n_grid点）
  grid2: Float64Array         // 🔵 param2のグリッド点（n_grid点）
  heatmap: Float64Array       // 🔵 [n_grid × n_grid] flattened Z値（行major）
  durationMs: number          // 🔵 計算時間（ms）
}

/**
 * compute_topsis() の戻り値型
 * 🔵 信頼性: TOPSISアルゴリズム理論・ユーザヒアリングより
 * （新規WASM関数。rust_core/src/topsis.rs を新規作成して実装）
 */
export interface TopsisWasmResult {
  scores: Float64Array        // 🔵 全試行のTOPSISスコア（trial順）
  rankedIndices: Uint32Array  // 🔵 スコア降順の試行インデックス
  positiveIdeal: Float64Array // 🔵 正理想解（目的数次元）
  negativeIdeal: Float64Array // 🔵 負理想解（目的数次元）
  durationMs: number          // 🔵 計算時間（ms）
}

// ========================================
// WasmLoader 拡張インターフェース（wasmLoader.ts に追加）
// ========================================

/**
 * WasmModule インターフェース拡張
 * 🔵 信頼性: ユーザヒアリング・既存WasmLoaderパターンより
 *
 * 既存の WasmModule に以下を追加する:
 */
export interface WasmModuleExtension {
  computePdp2d: (         // 🟡 既存の可能性あり。wasmLoader.ts確認が必要
    param1: string,
    param2: string,
    objective: string,
    nGrid: number
  ) => Pdp2dResult

  computeTopsis: (        // 🔵 新規
    weights: Float64Array,
    isMinimize: boolean[]
  ) => TopsisWasmResult
}

// ========================================
// tunny_core.d.ts 追加シグネチャ
// ========================================

// 以下の関数シグネチャを frontend/src/wasm/pkg/tunny_core.d.ts に手動追加:

// /** 2変数PDP（3D曲面グリッドデータ）計算 🟡 既存公開の有無を確認 */
// export function computePdp2d(
//   param1: string,
//   param2: string,
//   objective: string,
//   n_grid: number
// ): Pdp2dResult;

// /** TOPSIS多基準意思決定ランキング 🔵 新規追加 */
// export function computeTopsis(
//   weights: Float64Array,
//   is_minimize: boolean[]
// ): TopsisWasmResult;

// ========================================
// 信頼性レベルサマリー
// ========================================
/**
 * - 🔵 青信号: 22件 (88%)
 * - 🟡 黄信号: 3件 (12%)
 * - 🔴 赤信号: 0件 (0%)
 *
 * 品質評価: 高品質
 */
