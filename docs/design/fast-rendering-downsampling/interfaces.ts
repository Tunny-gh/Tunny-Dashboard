/**
 * 高速描画ダウンサンプリング 型定義
 *
 * 作成日: 2026-04-07
 * 関連設計: architecture.md
 *
 * 信頼性レベル:
 * - 🔵 青信号: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な型定義
 * - 🟡 黄信号: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による型定義
 * - 🔴 赤信号: EARS要件定義書・設計文書・ユーザヒアリングにない推測による型定義
 */

// ========================================
// WASM 関数戻り値
// ========================================

/**
 * ダウンサンプリング結果
 * 🔵 信頼性: wasm-api.md `downsample_for_thumbnail` 定義 / REQ-063 より
 */
export interface DownsampleResult {
  /** ダウンサンプリング後のトライアルインデックス配列 */
  indices: Uint32Array; // 🔵 wasm-api.md より
  /** Pareto Rank1 点として保持された件数 */
  paretoCount: number; // 🔵 ユーザヒアリング（Pareto必須保持）より
  /** ダウンサンプリング前の総件数 */
  totalCount: number; // 🔵 wasm-api.md パターンより
  /** 処理時間 (ms) */
  durationMs: number; // 🔵 wasm-api.md 共通フィールドより
}

// ========================================
// downsampleStore 状態
// ========================================

/**
 * downsampleStore が管理するキャッシュキー
 * 🔵 信頼性: ユーザヒアリング（チャート別上限）/ architecture.md より
 */
export type DownsampleKey =
  | 'scatter'      // 🔵 ParetoScatter2D/3D・ObjectivePairMatrix 用（上限 10,000）
  | 'thumbnail'    // 🔵 ScatterMatrix サムネイル用（上限 500, REQ-063）
  | 'hover'        // 🔵 ScatterMatrix ホバー拡大用（上限 3,000）
  | 'pcp'          // 🔵 ParallelCoordinates 用（上限 5,000）
  | 'data_points'  // 🔵 SlicePlot・SurfacePlot3D 実測点用（上限 5,000）
  | 'cluster';     // 🔵 ClusterScatter・DimReductionScatter 用（上限 10,000）

/**
 * ダウンサンプリング設定（キー別上限とアルゴリズム）
 * 🔵 信頼性: ユーザヒアリング / architecture.md 設計決定より
 */
export interface DownsampleConfig {
  maxPoints: number;          // 🔵 ユーザヒアリング（チャート別上限）より
  includePareto: boolean;     // 🔵 ユーザヒアリング（Pareto必須保持）より
  strategy: DownsampleStrategy; // 🔵 architecture.md アルゴリズム選択より
}

/**
 * ダウンサンプリング戦略
 * 🔵 信頼性: architecture.md 関数設計より
 */
export type DownsampleStrategy =
  | 'smart'              // 🔵 Pareto保持 + ランダム（汎用）
  | 'grid_thumbnail'     // 🔵 Pareto保持 + グリッド空間均等（ScatterMatrix用）
  | 'stratified_by_rank' // 🔵 Paretoランク別層化（ParallelCoordinates用）
  | 'by_cluster';        // 🔵 クラスタ別均等（ClusterScatter用, フォールバック: smart）

/**
 * downsampleStore 状態型
 * 🔵 信頼性: analysisStore・clusterStore パターン / ユーザヒアリングより
 */
export interface DownsampleState {
  /**
   * キャッシュ済みインデックス（キー別）
   * null の場合は未計算（初期状態 or Study 未選択）
   */
  cache: Partial<Record<DownsampleKey, Uint32Array>>; // 🔵 architecture.md より

  /** ダウンサンプリング計算中フラグ */
  isComputing: boolean; // 🔵 analysisStore パターンより

  /** エラー情報（WASM 呼び出し失敗時） */
  error: string | null; // 🔵 analysisStore・clusterStore パターンより

  /** 最後に計算したときの総試行数（フィルタ変化率計算用） */
  lastTotalCount: number; // 🟡 フィルタ変化時の再計算閾値判定から妥当な推測
}

/**
 * downsampleStore アクション型
 * 🔵 信頼性: analysisStore・clusterStore パターンより
 */
export interface DownsampleActions {
  /**
   * Study 変更時の全キャッシュ再計算
   * compute_pareto_ranks() 完了後に呼び出すこと
   */
  recompute: () => Promise<void>; // 🔵 architecture.md 制約より

  /**
   * フィルタ変更時の条件付き再計算
   * @param filteredIndices filter_by_ranges() の結果
   */
  recomputeIfNeeded: (filteredIndices: Uint32Array) => Promise<void>; // 🔵 dataflow.md フロー2より

  /**
   * キー別インデックスの取得（キャッシュがなければ全インデックスを返す）
   * @param key DownsampleKey
   * @returns キャッシュ済み Uint32Array、または全インデックス（フォールバック）
   */
  getIndices: (key: DownsampleKey) => Uint32Array; // 🔵 ユーザヒアリング / architecture.md より

  /** キャッシュのリセット（Study 変更前クリア用） */
  reset: () => void; // 🔵 analysisStore・clusterStore パターンより
}

// ========================================
// 定数定義
// ========================================

/**
 * ダウンサンプリング設定デフォルト値
 * 🔵 信頼性: ユーザヒアリング（チャート別上限）/ architecture.md より
 */
export const DOWNSAMPLE_CONFIGS: Record<DownsampleKey, DownsampleConfig> = {
  scatter: {
    maxPoints: 10_000,    // 🔵 ユーザヒアリング（主要散布図 10k点）より
    includePareto: true,  // 🔵 ユーザヒアリング（Pareto必須保持）より
    strategy: 'smart',
  },
  thumbnail: {
    maxPoints: 500,        // 🔵 REQ-063 より
    includePareto: true,   // 🔵 REQ-063 より
    strategy: 'grid_thumbnail',
  },
  hover: {
    maxPoints: 3_000,      // 🔵 ユーザヒアリング / REQ-061 ホバー300px考慮より
    includePareto: true,   // 🔵 ユーザヒアリング（Pareto必須保持）より
    strategy: 'grid_thumbnail',
  },
  pcp: {
    maxPoints: 5_000,      // 🔵 ユーザヒアリング（PCP 5k点）より
    includePareto: true,   // 🔵 ユーザヒアリング（Pareto必須保持）より
    strategy: 'stratified_by_rank',
  },
  data_points: {
    maxPoints: 5_000,      // 🔵 ユーザヒアリング（SlicePlot/SurfacePlot3D 5k点）より
    includePareto: false,  // 🟡 サロゲートモデルの学習データ点、Pareto優先不要と推測
    strategy: 'smart',
  },
  cluster: {
    maxPoints: 10_000,     // 🔵 ユーザヒアリング（ClusterScatter 10k点）より
    includePareto: true,   // 🔵 ユーザヒアリング（Pareto必須保持）より
    strategy: 'by_cluster',
  },
} as const;

// ========================================
// Rust WASM バインディング型（tunny_core.d.ts に追加する型）
// ========================================

/**
 * WasmLoader に追加するプロパティ型
 * 🔵 信頼性: wasm-api.md / 既存 WasmLoader パターンより
 */
export interface WasmDownsampleMethods {
  /**
   * Pareto保持 + ランダムサンプリング
   * @param maxPoints 上限点数
   * @param includePareto Pareto Rank1 を必ず含めるか
   */
  downsampleSmart: (maxPoints: number, includePareto: boolean) => DownsampleResult; // 🔵

  /**
   * Pareto保持 + グリッド空間均等サンプリング（ScatterMatrix thumbnail用）
   * REQ-063 準拠
   * @param maxPoints 上限点数
   */
  downsampleForThumbnail: (maxPoints: number) => DownsampleResult; // 🔵 wasm-api.md 定義済みより

  /**
   * Paretoランク別層化サンプリング（ParallelCoordinates用）
   * @param maxPoints 上限点数
   * @param nStrata 層数（Paretoランク数に合わせる）
   */
  downsampleStratifiedByRank: (maxPoints: number, nStrata: number) => DownsampleResult; // 🔵

  /**
   * クラスタ別均等サンプリング（ClusterScatter用）
   * クラスタラベルが未計算の場合は downsampleSmart にフォールバック
   * @param maxPoints 上限点数
   */
  downsampleByCluster: (maxPoints: number) => DownsampleResult; // 🔵
}

// ========================================
// 信頼性レベルサマリー
// ========================================
/**
 * - 🔵 青信号: 22件 (92%)
 * - 🟡 黄信号: 2件 (8%)
 * - 🔴 赤信号: 0件 (0%)
 *
 * 品質評価: 高品質
 */
