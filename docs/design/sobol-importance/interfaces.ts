/**
 * Sobol感度指数による重要度分析 型定義
 *
 * 作成日: 2026-04-01
 * 関連設計: architecture.md, implementation-guide.md
 *
 * 信頼性レベル:
 * - 🔵 青信号: 既存コード・ユーザヒアリングを参考にした確実な型定義
 * - 🟡 黄信号: 既存設計・ユーザヒアリングから妥当な推測による型定義
 * - 🔴 赤信号: ヒアリング情報にない推測による型定義
 *
 * 実装対象ファイル（すべて追加変更のみ、新規ファイル不要）:
 *   1. frontend/src/wasm/wasmLoader.ts
 *   2. frontend/src/stores/analysisStore.ts
 *   3. frontend/src/components/charts/ImportanceChart.tsx
 */

// ========================================
// 1. wasmLoader.ts に追加する型
// ========================================

/**
 * WASM computeSobol() の戻り値
 * 🔵 信頼性: ユーザヒアリング + 既存 SensitivityWasmResult のパターンより
 *
 * 追加先: frontend/src/wasm/wasmLoader.ts
 */
export interface SobolWasmResult {
  /** パラメータ名 (SensitivityWasmResult.paramNames と同順) */
  paramNames: string[]                    // 🔵 既存 SensitivityWasmResult に準拠

  /** 目的関数名 */
  objectiveNames: string[]               // 🔵 既存 SensitivityWasmResult に準拠

  /**
   * 一次 Sobol 指数 S_i [paramIdx][objIdx]
   * 値域 [0, 1]: パラメータ i 単独が出力分散に占める割合
   */
  firstOrder: number[][]                 // 🔵 ユーザヒアリングより

  /**
   * 全効果 Sobol 指数 ST_i [paramIdx][objIdx]
   * 値域 [0, 1]: パラメータ i とその全交互作用を含む分散の割合
   * 常に ST_i >= S_i
   */
  totalEffect: number[][]                // 🔵 ユーザヒアリングより

  /** Saltelli サンプリングに使用した N 値 (デフォルト 1024) */
  nSamples: number                       // 🔵 ユーザヒアリング (N=1024) より

  /** 計算時間 (ms) */
  durationMs?: number                    // 🔵 既存パターン (SensitivityWasmResult) に準拠
}

// ========================================
// 2. analysisStore.ts の AnalysisState 拡張
// ========================================

/**
 * AnalysisState への追加フィールド
 * 🔵 信頼性: 既存 analysisStore.ts の実装パターン・ユーザヒアリングより
 *
 * 既存の AnalysisState:
 * ```typescript
 * interface AnalysisState {
 *   sensitivityResult: SensitivityWasmResult | null
 *   isComputingSensitivity: boolean
 *   sensitivityError: string | null
 *   computeSensitivity: () => Promise<void>
 *   computeSensitivitySelected: (indices: Uint32Array) => Promise<void>
 * }
 * ```
 *
 * 追加後の AnalysisState:
 * ```typescript
 * interface AnalysisState {
 *   // --- 既存 ---
 *   sensitivityResult: SensitivityWasmResult | null
 *   isComputingSensitivity: boolean
 *   sensitivityError: string | null
 *   computeSensitivity: () => Promise<void>
 *   computeSensitivitySelected: (indices: Uint32Array) => Promise<void>
 *   // --- 追加 ---
 *   sobolResult: SobolWasmResult | null
 *   isComputingSobol: boolean
 *   sobolError: string | null
 *   computeSobol: (nSamples?: number) => Promise<void>
 * }
 * ```
 */
export interface AnalysisStateAddition {
  sobolResult: SobolWasmResult | null    // 🔵 既存 sensitivityResult のパターンに準拠
  isComputingSobol: boolean              // 🔵 既存 isComputingSensitivity のパターンに準拠
  sobolError: string | null             // 🔵 既存 sensitivityError のパターンに準拠
  computeSobol: (nSamples?: number) => Promise<void>  // 🔵 ユーザヒアリングより
}

// ========================================
// 3. ImportanceChart.tsx の型変更
// ========================================

/**
 * ImportanceMetric 型の拡張
 * 🔵 信頼性: 既存 ImportanceChart.tsx の ImportanceMetric 型・ユーザヒアリングより
 *
 * 既存: type ImportanceMetric = 'spearman' | 'beta'
 * 変更後:
 */
export type ImportanceMetric = 'spearman' | 'beta' | 'sobol_first' | 'sobol_total'
// 🔵 'sobol_first', 'sobol_total' をユーザヒアリングより追加

/**
 * ImportanceChart 内部で使用する重要度スコア計算の結果型
 * 🔵 信頼性: 既存 ImportanceChart.tsx のローカル型に準拠
 */
export interface ImportanceItem {
  name: string   // パラメータ名
  score: number  // 0〜1 の重要度スコア
}

// ========================================
// 信頼性レベルサマリー
// ========================================
/**
 * - 🔵 青信号: 10件 (100%)
 * - 🟡 黄信号: 0件 (0%)
 * - 🔴 赤信号: 0件 (0%)
 *
 * 品質評価: ✅ 高品質
 */
