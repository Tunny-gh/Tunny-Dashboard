/**
 * 可視化機能有効化 型定義
 *
 * 作成日: 2026-03-29
 * 関連設計: architecture.md
 *
 * 信頼性レベル:
 * - 🔵 青信号: EARS要件定義書・設計文書・既存実装を参考にした確実な型定義
 * - 🟡 黄信号: EARS要件定義書・設計文書・既存実装から妥当な推測による型定義
 * - 🔴 赤信号: EARS要件定義書・設計文書・既存実装にない推測による型定義
 *
 * このファイルは設計参考用。実装時は各ファイルに直接埋め込む。
 * - tunny_core.d.ts   → SensitivityWasmResult, PcaWasmResult, KmeansWasmResult,
 *                        ElbowWasmResult, ClusterStatsWasmResult の追加と関数シグネチャ
 * - analysisStore.ts  → AnalysisState インターフェース
 * - clusterStore.ts   → ClusterState インターフェース
 * - ImportanceChart.tsx → ImportanceChartProps
 * - ClusterScatter.tsx  → ClusterScatterProps
 * - DimReductionScatter.tsx → DimReductionScatterProps
 */

// ============================================================================
// Layer 1-2: WASM Result Types（tunny_core.d.ts に追加する型）
// ============================================================================

/**
 * computeSensitivity() / computeSensitivitySelected() の戻り値
 * 🔵 信頼性: REQ-VE-001-B / REQ-VE-016 / sensitivity.rs SensitivityResult より
 */
export interface SensitivityWasmResult {
  spearman: number[][];                             // 🔵 [nParams][nObjectives]
  ridge: Array<{ beta: number[]; rSquared: number }>;// 🔵 [nObjectives]
  paramNames: string[];                             // 🔵 パラメータ名配列
  objectiveNames: string[];                         // 🔵 目的関数名配列
  durationMs: number;                              // 🔵 計算時間 (ms)
}

/**
 * runPca() の戻り値
 * 🔵 信頼性: REQ-VE-003-B / REQ-VE-016 / clustering.rs PcaResult より
 */
export interface PcaWasmResult {
  projections: number[][];     // 🔵 [N][n_components] — 各試行の主成分座標
  explainedVariance: number[]; // 🔵 [n_components] — 各主成分の寄与率
  featureNames: string[];      // 🔵 使用した特徴量名（param/objective/both に依存）
  durationMs: number;          // 🔵 計算時間 (ms)
}

/**
 * runKmeans() の戻り値
 * 🔵 信頼性: REQ-VE-004-B / REQ-VE-016 / clustering.rs KmeansResult より
 */
export interface KmeansWasmResult {
  labels: number[];      // 🔵 [N] クラスタラベル (0-indexed)
  centroids: number[][]; // 🔵 [k][n_cols] — 各クラスタの重心
  wcss: number;          // 🔵 Within-Cluster Sum of Squares
  durationMs: number;    // 🔵 計算時間 (ms)
}

/**
 * estimateKElbow() の戻り値
 * 🔵 信頼性: REQ-VE-005-B / REQ-VE-016 / clustering.rs ElbowResult より
 */
export interface ElbowWasmResult {
  wcssPerK: number[];   // 🔵 [max_k - 1] — k=2〜max_k の WCSS 値
  recommendedK: number; // 🔵 Elbow 法による推奨 k
  durationMs: number;   // 🔵 計算時間 (ms)
}

/**
 * computeClusterStats() の戻り値
 * 🔵 信頼性: REQ-VE-006-B / REQ-VE-016 / clustering.rs ClusterStat より
 */
export interface ClusterStatsWasmResult {
  stats: Array<{
    clusterId: number;              // 🔵 クラスタ ID (0-indexed)
    size: number;                   // 🔵 クラスタ内試行数
    centroid: Record<string, number>;   // 🔵 パラメータ/目的関数の平均値
    std: Record<string, number>;        // 🔵 パラメータ/目的関数の標準偏差
    significantDiffs: string[];         // 🔵 全体平均から有意に異なる特徴量名
  }>;
  durationMs: number;               // 🔵 計算時間 (ms)
}

// ============================================================================
// Layer 4a: analysisStore State
// ============================================================================

/**
 * analysisStore Zustand 状態インターフェース
 * 🔵 信頼性: REQ-VE-031〜034 / 既存 studyStore / selectionStore パターンより
 */
export interface AnalysisState {
  // ---- State ----

  /** 感度分析結果。未計算または Study 変更後は null */
  sensitivityResult: SensitivityWasmResult | null;  // 🔵 REQ-VE-031

  /** 感度計算中フラグ */
  isComputingSensitivity: boolean;                   // 🔵 REQ-VE-031

  /** 感度計算エラーメッセージ。正常時は null */
  sensitivityError: string | null;                  // 🔵 REQ-VE-031

  // ---- Actions ----

  /**
   * 全試行の感度を計算する。
   * 計算中は isComputingSensitivity = true。
   * WasmLoader.getInstance() 経由で computeSensitivity() を呼び出す。
   */
  computeSensitivity: () => Promise<void>;          // 🔵 REQ-VE-032

  /**
   * 選択サブセットの感度を再計算する（Brushing 後）。
   * @param indices 選択済み試行インデックス (Uint32Array)
   */
  computeSensitivitySelected: (indices: Uint32Array) => Promise<void>; // 🔵 REQ-VE-033
}

// ============================================================================
// Layer 4b: clusterStore State
// ============================================================================

/** clusterStore で使用する PCA 空間の選択肢 */
export type ClusterSpace = 'param' | 'objective' | 'all'; // 🔵 REQ-VE-041 / PcaSpace enum

/**
 * clusterStore Zustand 状態インターフェース
 * 🔵 信頼性: REQ-VE-041〜045 / 既存 ClusterStore 型定義 (types/index.ts) より
 *
 * 注: types/index.ts の ClusterStore は旧設計。本インターフェースが本フェーズの確定版。
 */
export interface ClusterState {
  // ---- State ----

  /** PCA 2D 投影座標。[N][2] 形式。未計算時は null */
  pcaProjections: number[][] | null;              // 🔵 REQ-VE-041

  /** k-means クラスタラベル。[N] 形式 (0-indexed)。未実行時は null */
  clusterLabels: number[] | null;                // 🔵 REQ-VE-041

  /** クラスタ統計量。未実行時は null */
  clusterStats: ClusterStatsWasmResult | null;  // 🔵 REQ-VE-041

  /** Elbow 法結果。未実行時は null */
  elbowResult: ElbowWasmResult | null;           // 🔵 REQ-VE-041

  /** クラスタリング/PCA 実行中フラグ */
  isRunning: boolean;                            // 🔵 REQ-VE-041

  /** エラーメッセージ。正常時は null */
  clusterError: string | null;                  // 🔵 REQ-VE-041

  /** 最後に使用した PCA 空間 */
  clusterSpace: ClusterSpace;                   // 🔵 REQ-VE-041

  /** 最後に使用したクラスタ数 */
  k: number;                                    // 🔵 REQ-VE-041

  // ---- Actions ----

  /**
   * PCA → k-means → cluster stats を順次実行する。
   * @param space PCA の対象空間 ('param' | 'objective' | 'all')
   * @param k クラスタ数
   */
  runClustering: (space: ClusterSpace, k: number) => Promise<void>; // 🔵 REQ-VE-042

  /**
   * Elbow 法で最適 k を推定する。
   * @param space PCA の対象空間
   */
  estimateK: (space: ClusterSpace) => Promise<void>;                // 🔵 REQ-VE-043
}

// ============================================================================
// Layer 5: Chart Component Props
// ============================================================================

/**
 * ImportanceChart コンポーネント Props
 * 🔵 信頼性: REQ-VE-061〜065 / 設計ヒアリング・SensitivityHeatmap パターンより
 */
export interface ImportanceChartProps {
  /** 感度分析結果。null の場合はローディングまたは EmptyState を表示 */
  data: SensitivityWasmResult | null; // 🔵 REQ-VE-064

  /** ローディング中フラグ */
  isLoading?: boolean;               // 🔵 REQ-VE-064

  /** エラーメッセージ */
  error?: string | null;             // 🔵 REQ-VE-065

  /** 選択中の重要度指標 */
  metric: ImportanceMetric;          // 🔵 REQ-VE-061

  /** 指標変更コールバック */
  onMetricChange: (metric: ImportanceMetric) => void; // 🔵 REQ-VE-061
}

/** Importance チャートで選択可能な重要度指標 */
export type ImportanceMetric = 'spearman' | 'beta'; // 🔵 REQ-VE-061 / ユーザーヒアリング Q3

/**
 * ClusterScatter コンポーネント Props
 * 🔵 信頼性: REQ-VE-072〜077 / 設計ヒアリングより
 */
export interface ClusterScatterProps {
  /** PCA 2D 投影座標 [N][2]。null の場合は EmptyState */
  projections: number[][] | null; // 🔵 REQ-VE-072

  /** クラスタラベル [N]。null の場合は単色プロット */
  labels: number[] | null;       // 🔵 REQ-VE-075

  /** クラスタリング実行中フラグ */
  isRunning?: boolean;           // 🔵 REQ-VE-074

  /** エラーメッセージ */
  error?: string | null;         // 🔵 REQ-VE-077
}

/**
 * DimReductionScatter コンポーネント Props
 * 🔵 信頼性: REQ-VE-083〜087 / 設計ヒアリングより
 */
export interface DimReductionScatterProps {
  /** PCA 2D 投影座標 [N][2]。null の場合は EmptyState または自動取得 */
  projections: number[][] | null; // 🔵 REQ-VE-083

  /** selectionStore から渡す現在のカラーモード（types/index.ts の ColorMode） */
  colorMode: ColorMode;           // 🔵 REQ-VE-084

  /** 選択中の試行インデックス（色付け用） */
  selectedIndices?: Uint32Array;  // 🟡 selectionStore との連携

  /** ローディング中フラグ */
  isLoading?: boolean;           // 🔵 REQ-VE-086
}

// ============================================================================
// tunny_core.d.ts に追加すべき関数シグネチャ（参考）
// ============================================================================

/**
 * 以下を tunny_core.d.ts に追加する:
 *
 * // 🔵 REQ-VE-010
 * export function computeSensitivity(): SensitivityWasmResult;
 *
 * // 🔵 REQ-VE-011
 * export function computeSensitivitySelected(indices: Uint32Array): SensitivityWasmResult;
 *
 * // 🔵 REQ-VE-012
 * export function runPca(n_components: number, space: string): PcaWasmResult;
 *
 * // 🔵 REQ-VE-013
 * export function runKmeans(k: number, data: Float64Array, n_cols: number): KmeansWasmResult;
 *
 * // 🔵 REQ-VE-014
 * export function estimateKElbow(data: Float64Array, n_cols: number, max_k: number): ElbowWasmResult;
 *
 * // 🔵 REQ-VE-015
 * export function computeClusterStats(labels: Int32Array): ClusterStatsWasmResult;
 *
 * また InitOutput にも以下を追加:
 *   readonly computeSensitivity: () => [number, number, number];
 *   readonly computeSensitivitySelected: (a: number, b: number) => [number, number, number];
 *   readonly runPca: (a: number, b: number, c: number) => [number, number, number];
 *   readonly runKmeans: (a: number, b: number, c: number, d: number) => [number, number, number];
 *   readonly estimateKElbow: (a: number, b: number, c: number, d: number) => [number, number, number];
 *   readonly computeClusterStats: (a: number, b: number) => [number, number, number];
 */

// ============================================================================
// 信頼性レベルサマリー
// ============================================================================
/**
 * - 🔵 青信号: 34件 (97%)
 * - 🟡 黄信号: 1件 (3%)
 * - 🔴 赤信号: 0件 (0%)
 *
 * 品質評価: 高品質
 */
