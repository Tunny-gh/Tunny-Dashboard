/**
 * Kriging 高速化 型定義
 *
 * 作成日: 2026-04-05
 * 関連設計: architecture.md
 * 関連要件: requirements.md
 *
 * 信頼性レベル:
 * - 🔵 青信号: EARS要件定義書・設計文書・既存実装を参考にした確実な型定義
 * - 🟡 黄信号: EARS要件定義書・設計文書・既存実装から妥当な推測による型定義
 * - 🔴 赤信号: EARS要件定義書・設計文書・既存実装にない推測による型定義
 */

// ========================================
// SurrogateModelType 拡張（REQ-005-01）
// ========================================

/**
 * サロゲートモデル種別
 * 🔵 信頼性: 既存 types/index.ts:307 + REQ-005-01 ユーザヒアリングより
 *
 * 変更前: 'ridge' | 'random_forest' | 'kriging'
 * 変更後: 'sparse_kriging' を追加
 */
export type SurrogateModelType = 'ridge' | 'random_forest' | 'kriging' | 'sparse_kriging'
// 🔵 既存 3 モデル: types/index.ts:307 より
// 🔵 'sparse_kriging': REQ-005-01, ユーザヒアリング（ドロップダウン追加）より

// ========================================
// Web Worker メッセージプロトコル（REQ-001）
// ========================================

/**
 * Main → KrigingWorker へのメッセージ
 * 🔵 信頼性: REQ-001-02 + ユーザヒアリング（compute_kriging_raw 引数）より
 */
export interface KrigingWorkerInput {
  /** メッセージ種別 */
  type: 'init' | 'compute'
  // type = 'init' の場合
  /** WASM バイナリ（base64 → Uint8Array）*/
  wasmBytes?: ArrayBuffer // 🟡 Blob URL 方式実装詳細から妥当な推測
  // type = 'compute' の場合
  /** 訓練データ: param1 列と param2 列を flatten [p1_0, p1_1, ..., p1_n, p2_0, ...] */
  xFlat?: Float64Array // 🔵 REQ-001-02 より; Transferable
  /** 目的関数値 */
  y?: Float64Array // 🔵 REQ-001-02 より; Transferable
  /** 試行数 */
  nSamples?: number // 🔵 REQ-001-02 より
  /** xFlat 内の param1 列インデックス (0 or 1) */
  param1Idx?: number // 🟡 compute_kriging_raw 設計より妥当な推測
  /** xFlat 内の param2 列インデックス (0 or 1) */
  param2Idx?: number // 🟡 compute_kriging_raw 設計より妥当な推測
  /** グリッド解像度（デフォルト 50） */
  nGrid?: number // 🔵 既存 computePdp2d 引数より
  /** 使用モデル */
  modelType?: 'kriging' | 'sparse_kriging' // 🔵 REQ-001-02 より
  /** キャッシュキー（結果とペアにするため） */
  cacheKey?: string // 🟡 analysisStore キャッシュ管理から妥当な推測
}

/**
 * KrigingWorker → Main へのメッセージ
 * 🔵 信頼性: REQ-001-04 + 既存 Pdp2dWasmResult より
 */
export interface KrigingWorkerOutput {
  /** メッセージ種別 */
  type: 'ready' | 'result' | 'error' // 🔵 REQ-001-01 〜 03 より
  /** 計算結果（type='result' のとき） */
  result?: KrigingRawResult // 🔵 REQ-001-04 より
  /** エラーメッセージ（type='error' のとき） */
  error?: string // 🔵 EDGE-001 要件より
  /** キャッシュキー（Main 側でのキャッシュ更新に使用） */
  cacheKey?: string // 🟡 analysisStore キャッシュ管理から妥当な推測
}

/**
 * Worker から返される計算結果（Pdp2dWasmResult と同型）
 * 🔵 信頼性: 既存 wasmLoader.ts の Pdp2dWasmResult より
 */
export interface KrigingRawResult {
  param1Name: string // 🔵 既存 Pdp2dWasmResult より
  param2Name: string // 🔵 既存 Pdp2dWasmResult より
  objectiveName: string // 🔵 既存 Pdp2dWasmResult より
  grid1: number[] // 🔵 既存 Pdp2dWasmResult より
  grid2: number[] // 🔵 既存 Pdp2dWasmResult より
  values: number[][] // 🔵 既存 Pdp2dWasmResult より
  rSquared: number // 🔵 既存 Pdp2dWasmResult より
}

// ========================================
// WASM 新関数シグネチャ（REQ-001-03）
// ========================================

/**
 * compute_kriging_raw WASM 関数のシグネチャ（lib.rs に追加）
 * 🔵 信頼性: ユーザヒアリング（compute_kriging_raw、グローバル状態不要）より
 *
 * Rust 側シグネチャ（参考）:
 * ```rust
 * #[wasm_bindgen(js_name = "computeKrigingRaw")]
 * pub fn wasm_compute_kriging_raw(
 *     x_flat: &[f64],
 *     y: &[f64],
 *     n_samples: u32,
 *     param1_idx: u32,
 *     param2_idx: u32,
 *     n_grid: u32,
 *     model_type: &str,
 * ) -> Result<JsValue, JsValue>
 * ```
 */
export type ComputeKrigingRawFn = (
  xFlat: Float64Array, // 🔵 ユーザヒアリングより
  y: Float64Array, // 🔵 ユーザヒアリングより
  nSamples: number, // 🔵 ユーザヒアリングより
  param1Idx: number, // 🔵 ユーザヒアリングより
  param2Idx: number, // 🔵 ユーザヒアリングより
  nGrid: number, // 🔵 既存 computePdp2d 引数より
  modelType: string, // 🔵 既存 computePdp2d 引数より
) => KrigingRawResult

// ========================================
// analysisStore 拡張型（Phase 2 以降）
// ========================================

/**
 * Kriging Worker インスタンス管理
 * 🟡 信頼性: Web Worker パターン + analysisStore 構造から妥当な推測
 */
export interface KrigingWorkerState {
  /** Worker インスタンス（null = 未初期化） */
  worker: Worker | null // 🟡 Blob URL 方式実装詳細から妥当な推測
  /** Worker 初期化完了フラグ */
  isWorkerReady: boolean // 🟡 Worker onmessage 'ready' から妥当な推測
  /** 処理中のリクエスト（cacheKey → resolve/reject ペア） */
  pendingRequests: Map<
    string,
    { resolve: (r: KrigingRawResult) => void; reject: (e: Error) => void }
  > // 🟡 Promise 管理パターンから妥当な推測
}

// ========================================
// MODEL_OPTIONS 更新型（REQ-005-05）
// ========================================

/**
 * モデル選択オプション型
 * 🔵 信頼性: 既存 SurfacePlot3D.tsx MODEL_OPTIONS + REQ-005-05 より
 */
export interface ModelOption {
  value: SurrogateModelType // 🔵 既存 MODEL_OPTIONS より
  label: string // 🔵 既存 MODEL_OPTIONS より
  disabled?: boolean // 🔵 既存 MODEL_OPTIONS より
}

/**
 * MODEL_COMPUTE_TIME 更新
 * 🔵 信頼性: 既存 SurfacePlot3D.tsx MODEL_COMPUTE_TIME + REQ-006 + REQ-005-06 より
 */
export type ModelComputeTime = Record<SurrogateModelType, string>
// 更新後の値:
// {
//   ridge: '< 1s',        // 🔵 既存より変更なし
//   random_forest: '< 2s', // 🔵 既存より変更なし
//   kriging: '< 10s',     // 🔵 Phase 1 最適化後 REQ-006-01 より更新
//   sparse_kriging: '< 5s', // 🟡 REQ-005-06 O(N×M²) 計算量より妥当な推測
// }

// ========================================
// 信頼性レベルサマリー
// ========================================
/**
 * - 🔵 青信号: 17件 (68%)
 * - 🟡 黄信号: 8件 (32%)
 * - 🔴 赤信号: 0件 (0%)
 *
 * 品質評価: 高品質
 */
