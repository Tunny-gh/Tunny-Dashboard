/**
 * 3D Surface Plot サロゲートモデル拡張 型定義差分
 *
 * 作成日: 2026-04-05
 * 関連設計: architecture.md
 * 対象ファイル:
 *   - frontend/src/wasm/wasmLoader.ts     — computePdp2d シグネチャ変更
 *   - frontend/src/types/index.ts         — SurrogateModelType（変更なし・参考）
 *
 * 信頼性レベル:
 * - 🔵 青信号: EARS要件定義書・設計文書・既存実装を参考にした確実な型定義
 * - 🟡 黄信号: EARS要件定義書・設計文書・既存実装から妥当な推測による型定義
 * - 🔴 赤信号: EARS要件定義書・設計文書・既存実装にない推測による型定義
 */

// ============================================================
// wasmLoader.ts — 変更対象
// ============================================================

/**
 * 2D PDP 計算結果（WASM から返却）
 * 🔵 信頼性: 既存 Pdp2dWasmResult（wasmLoader.ts:88-96）より
 *
 * 変更なし — 戻り値構造はモデル種別によらず同一
 */
export interface Pdp2dWasmResult {
  param1Name: string       // 🔵 既存実装より
  param2Name: string       // 🔵 既存実装より
  objectiveName: string    // 🔵 既存実装より
  grid1: number[]          // 🔵 既存実装より — param1 のグリッド点（n_grid 点）
  grid2: number[]          // 🔵 既存実装より — param2 のグリッド点（n_grid 点）
  values: number[][]       // 🔵 既存実装より — values[i][j] = f(grid1[i], grid2[j])
  rSquared: number         // 🔵 既存実装より — モデル適合度（0〜1）
}

/**
 * WasmLoader.computePdp2d — 変更後シグネチャ
 * 🔵 信頼性: ユーザヒアリング Q4（model_type 引数追加）より
 *
 * 変更点: modelType 引数を末尾に追加
 *
 * 変更前（rust_core/src/lib.rs:383）:
 *   computePdp2d(param1Name, param2Name, objectiveName, nGrid)
 *
 * 変更後:
 *   computePdp2d(param1Name, param2Name, objectiveName, nGrid, modelType)
 */
interface WasmLoaderComputePdp2dUpdated {
  computePdp2d: (
    param1Name: string,       // 🔵 既存実装より
    param2Name: string,       // 🔵 既存実装より
    objectiveName: string,    // 🔵 既存実装より
    nGrid: number,            // 🔵 既存実装より
    modelType: SurrogateModelType,  // 🔵 追加 — ユーザヒアリング Q4 より
  ) => Pdp2dWasmResult
}

// ============================================================
// frontend/src/types/index.ts — 参考（変更なし）
// ============================================================

/**
 * サロゲートモデル種別
 * 🔵 信頼性: 既存 types/index.ts:307 より（変更なし）
 *
 * RF/Kriging 実装完了後、UI の disabled フラグが解除される。
 * 型定義自体はすでに 3 値を含んでおり変更不要。
 */
export type SurrogateModelType = 'ridge' | 'random_forest' | 'kriging'
// 🔵 'ridge'        — 既存・実装済み
// 🔵 'random_forest' — 新規実装対象（rf.rs）
// 🔵 'kriging'      — 新規実装対象（kriging.rs）

// ============================================================
// rust_core — 参考（TypeScript 型ではないが設計上重要）
// ============================================================

/**
 * Rust 側の PdpResult2d 構造体（参考）
 * 🔵 信頼性: rust_core/src/pdp.rs:40-57 より
 *
 * serde rename_all = "camelCase" により TypeScript 側は camelCase で受け取る。
 * 変更なし — RF/Kriging も同一構造体を返す。
 *
 * ```rust
 * #[derive(Debug, Clone, serde::Serialize)]
 * #[serde(rename_all = "camelCase")]
 * pub struct PdpResult2d {
 *     pub param1_name: String,      // → param1Name
 *     pub param2_name: String,      // → param2Name
 *     pub objective_name: String,   // → objectiveName
 *     pub grid1: Vec<f64>,
 *     pub grid2: Vec<f64>,
 *     pub values: Vec<Vec<f64>>,
 *     pub r_squared: f64,           // → rSquared
 * }
 * ```
 */

/**
 * compute_pdp_2d 関数シグネチャ — 変更後（Rust）
 * 🔵 信頼性: ユーザヒアリング Q4（model_type 引数追加）より
 *
 * ```rust
 * pub fn compute_pdp_2d(
 *     param1_name: &str,
 *     param2_name: &str,
 *     objective_name: &str,
 *     n_grid: usize,
 *     model_type: &str,   // 追加: "ridge" | "random_forest" | "kriging"
 * ) -> Option<PdpResult2d>
 * ```
 */

/**
 * wasm_compute_pdp_2d WASM エクスポート — 変更後（Rust）
 * 🔵 信頼性: ユーザヒアリング Q4（model_type 引数追加）より
 *
 * ```rust
 * #[wasm_bindgen(js_name = "computePdp2d")]
 * pub fn wasm_compute_pdp_2d(
 *     param1_name: &str,
 *     param2_name: &str,
 *     objective_name: &str,
 *     n_grid: u32,
 *     model_type: &str,   // 追加
 * ) -> Result<JsValue, JsValue>
 * ```
 */

// ============================================================
// analysisStore.ts — 変更サマリー（型定義変更なし）
// ============================================================

/**
 * analysisStore の computeSurface3d — 変更後の呼び出しパターン
 * 🔵 信頼性: ユーザヒアリング Q4 + 既存 analysisStore.ts:104 のバグ修正より
 *
 * 変更前（バグ: surrogateModelType が WASM に渡っていない）:
 *   const result = wasm.computePdp2d(param1, param2, objective, nGrid)
 *
 * 変更後:
 *   const result = wasm.computePdp2d(param1, param2, objective, nGrid, surrogateModelType)
 *
 * 型定義は変更なし（surrogateModelType: SurrogateModelType は既存）
 */

// ============================================================
// SurfacePlot3D.tsx — 変更サマリー（型定義変更なし）
// ============================================================

/**
 * MODEL_OPTIONS — 変更後
 * 🔵 信頼性: ユーザヒアリング・既存 SurfacePlot3D.tsx の MODEL_OPTIONS より
 *
 * 変更前:
 *   { value: 'random_forest', label: 'Random Forest (coming soon)', disabled: true },
 *   { value: 'kriging', label: 'Kriging (coming soon)', disabled: true },
 *
 * 変更後:
 *   { value: 'random_forest', label: 'Random Forest' },
 *   { value: 'kriging', label: 'Kriging' },
 *
 * 型定義は変更なし
 */

// ============================================================
// 信頼性レベルサマリー
// ============================================================
/**
 * - 🔵 青信号: 12件 (100%)
 * - 🟡 黄信号: 0件 (0%)
 * - 🔴 赤信号: 0件 (0%)
 *
 * 品質評価: 高品質
 *
 * 備考:
 * - 新規の TypeScript 型は `SurrogateModelType` の拡張のみ（既存型は変更なし）
 * - 主要な変更は Rust 側（rf.rs, kriging.rs 新規）と wasmLoader.ts のシグネチャ変更
 * - 既存の Pdp2dWasmResult は RF/Kriging でも再利用可能（追加型不要）
 */
