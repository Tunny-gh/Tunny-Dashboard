/**
 * Tunny Dashboard - TypeScript型定義
 * WASM側との境界契約・Zustand Store・UIコンポーネントのprops定義
 */

// =============================================================================
// コアエンティティ
// =============================================================================

/** Optuna Study のメタ情報 */
export interface Study {
  studyId: number;
  name: string;
  directions: OptimizationDirection[];
  completedTrials: number;
  totalTrials: number;
  paramNames: string[];
  objectiveNames: string[];
  userAttrNames: string[];
  hasConstraints: boolean;
}

export type OptimizationDirection = 'minimize' | 'maximize';

/** getTrials() WASM メソッドが返す per-trial データ */
export interface TrialData {
  trialId: number;
  params: Record<string, number>;
  values: number[];
  paretoRank: number | null;
}

/** 1試行分のデータ（Bottom Table表示用） */
export interface Trial {
  trialId: number;
  state: TrialState;
  params: Record<string, number | string>;
  values: number[] | null;
  paretoRank: number | null;
  clusterId: number | null;
  isFeasible: boolean | null;
  userAttrs: Record<string, number | string>;
  artifactIds: string[];
}

export type TrialState = 'COMPLETE' | 'RUNNING' | 'PRUNED' | 'FAIL';

/** WASM DataFrame の列情報（JS側が認識するメタデータ） */
export interface DataFrameInfo {
  rowCount: number;
  columnNames: string[];
  paramColumns: string[];
  objectiveColumns: string[];
  userAttrColumns: string[];
  constraintColumns: string[];
  derivedColumns: string[]; // is_feasible, constraint_sum, pareto_rank, cluster_id
}

/** 配布の逆変換に使用する分布情報 */
export type Distribution =
  | { type: 'FloatDistribution'; low: number; high: number; log: boolean }
  | { type: 'IntDistribution'; low: number; high: number; step: number; log: boolean }
  | { type: 'CategoricalDistribution'; choices: (string | number | boolean)[] }
  | { type: 'UniformDistribution'; low: number; high: number };

// =============================================================================
// GPU バッファ
// =============================================================================

/** WebGL描画用のGPUバッファ群 */
export interface GpuBuffers {
  positions: Float32Array;  // x, y座標 (N × 2)
  positions3d: Float32Array; // x, y, z座標 (N × 3)（3D Pareto用）
  colors: Float32Array;     // r, g, b, a (N × 4)
  sizes: Float32Array;      // 点サイズ (N × 1)
  trialCount: number;
}

// =============================================================================
// Zustand Stores
// =============================================================================

/** Brushing & Linking 中核Store（spec Section 6より） */
export interface SelectionStore {
  // 状態
  selectedIndices: Uint32Array;
  filterRanges: Record<string, Range>;
  highlighted: number | null;
  colorMode: ColorMode;

  // アクション
  brushSelect: (indices: Uint32Array) => void;
  addAxisFilter: (axis: string, min: number, max: number) => void;
  removeAxisFilter: (axis: string) => void;
  clearSelection: () => void;
  setHighlight: (index: number | null) => void;
  setColorMode: (mode: ColorMode) => void;
}

export type ColorMode = 'objective' | 'cluster' | 'rank' | 'generation';

export interface Range {
  min: number;
  max: number;
}

/** Study管理Store */
export interface StudyStore {
  // 状態
  currentStudy: Study | null;
  allStudies: Study[];
  studyMode: StudyMode;
  isLoading: boolean;
  loadError: string | null;

  // アクション
  loadJournal: (file: File) => Promise<void>;
  selectStudy: (studyId: number) => void;
  setComparisonStudies: (studyIds: number[]) => void;
  getDataFrameInfo: () => DataFrameInfo | null;
}

export type StudyMode = 'single-objective' | 'multi-objective';

/** レイアウト管理Store */
export interface LayoutStore {
  // 状態
  layoutMode: LayoutMode;
  visibleCharts: Set<ChartId>;
  panelSizes: PanelSizes;
  freeModeLayout: FreeModeLayout | null;

  // アクション
  setLayoutMode: (mode: LayoutMode) => void;
  toggleChart: (chartId: ChartId) => void;
  saveLayout: () => LayoutConfig;
  loadLayout: (config: LayoutConfig) => void;
}

export type LayoutMode = 'A' | 'B' | 'C' | 'D';

export type ChartId =
  | 'pareto-front'
  | 'parallel-coords'
  | 'scatter-matrix'
  | 'importance'
  | 'history'
  | 'hypervolume'
  | 'objective-pair-matrix'
  | 'pdp'
  | 'sensitivity-heatmap'
  | 'cluster-view'
  | 'umap'
  // 🟢 optuna-dashboard 相当の追加チャート（Python 不要）
  | 'slice'    // Slice Plot: パラメータ vs 目的関数値 scatter
  | 'edf'      // EDF: 経験累積分布関数 (Empirical Distribution Function)
  | 'contour'; // Contour Plot: 2パラメータ相関（実点散布のみ、ML 補間は Python 必須）
// 🔴 未実装（データ拡張が必要）:
//   'timeline'            — Trial.datetime_start/datetime_complete が必要
//                           Optuna Journal には記録されているが WASM パーサの拡張が必要
//   'intermediate-values' — Trial.intermediate_values が必要
//                           PRUNED 試行の途中値は Journal に含まれるが WASM パーサ未対応

export interface PanelSizes {
  leftPanel: number;
  bottomPanel: number;
}

export interface FreeModeLayout {
  cells: Array<{
    /** セルの一意識別子。カタログからの追加時は crypto.randomUUID() で生成。デフォルトレイアウトは chartId と同一。 */
    cellId: string;
    chartId: ChartId;
    gridRow: [number, number];
    gridCol: [number, number];
  }>;
}

export interface LayoutConfig {
  mode: LayoutMode;
  visibleCharts: ChartId[];
  panelSizes: PanelSizes;
  freeModeLayout: FreeModeLayout | null;
}

/** クラスタリングStore */
export interface ClusterStore {
  // 状態
  clusterConfig: ClusterConfig;
  clusterLabels: Int32Array | null;       // 各trialのクラスタID (-1 = noise)
  clusterStats: ClusterStats[] | null;
  elbowData: ElbowData | null;
  isRunning: boolean;
  progress: number; // 0〜1

  // アクション
  setClusterConfig: (config: Partial<ClusterConfig>) => void;
  runClustering: () => Promise<void>;
  selectCluster: (clusterId: number | null) => void;
}

export interface ClusterConfig {
  space: 'objective' | 'variable' | 'combined';
  algorithm: 'kmeans' | 'hdbscan';
  dimensionReduction: 'pca' | 'umap' | 'none';
  reducedDims: number;
  kAuto: boolean;
  k: number;
  kEstimateMethod: 'elbow' | 'silhouette';
  targetSamples: 'all' | 'pareto' | 'selected';
}

export interface ClusterStats {
  clusterId: number;
  size: number;
  centroid: Record<string, number>;
  std: Record<string, number>;
  significantDiffs: string[]; // 全体平均との差異が大きい変数名
}

export interface ElbowData {
  ks: number[];
  wcss: number[];
  recommendedK: number;
}

/** 分析（感度・PDP）Store */
export interface AnalysisStore {
  // 状態
  sensitivityResult: SensitivityResult | null;
  sensitivityMetric: SensitivityMetric;
  pdpCache: Map<string, PDPResult>; // key = `${paramName}_${objectiveName}`
  modelQuality: Record<string, ModelQuality>;
  isComputingSensitivity: boolean;
  isComputingMic: boolean;

  // アクション
  computeSensitivity: () => Promise<void>;
  computePDP: (paramName: string, objectiveName: string) => Promise<PDPResult>;
  loadOnnxModel: (file: File, objectiveName: string) => Promise<void>;
  loadShapValues: (file: File) => Promise<void>;
  setSensitivityMetric: (metric: SensitivityMetric) => void;
}

export type SensitivityMetric = 'spearman' | 'beta' | 'mic' | 'rf_importance' | 'shap';

export interface SensitivityResult {
  metric: SensitivityMetric;
  /** matrix[paramIdx][objectiveIdx] = 係数値 */
  matrix: number[][];
  paramNames: string[];
  objectiveNames: string[];
  r2: number[] | null; // 目的関数ごとのR²（Layer 1-C）
}

export interface PDPResult {
  paramName: string;
  objectiveName: string;
  gridPoints: number[];       // x軸: 変数値グリッド（50点）
  pdpValues: number[];         // PDP曲線
  confidenceLow: number[];     // 95%CI下限
  confidenceHigh: number[];    // 95%CI上限
  iceLines: number[][] | null; // ICE個別軌跡（最大100本）
  rugValues: number[];         // 実サンプル値（rug表示用）
  modelType: 'ridge' | 'random_forest';
}

export interface ModelQuality {
  objectiveName: string;
  r2: number;
  rmse: number;
  quality: 'good' | 'warning' | 'poor';
}

/** エクスポート・セッションStore */
export interface ExportStore {
  // 状態
  pinnedTrials: PinnedTrial[];
  sessionState: SessionState | null;

  // アクション
  pinTrial: (trialId: number, note?: string) => void;
  unpinTrial: (trialId: number) => void;
  exportCsv: (config: CsvExportConfig) => Promise<void>;
  exportPng: (chartId: ChartId) => Promise<void>;
  exportHtml: (chartId: ChartId) => Promise<void>;
  generateReport: (config: ReportConfig) => Promise<void>;
  saveSession: () => Promise<void>;
  loadSession: (file: File) => Promise<void>;
}

export interface PinnedTrial {
  trialId: number;
  note: string;
  pinnedAt: number; // timestamp
}

export interface CsvExportConfig {
  target: 'all' | 'selected' | 'pareto' | { clusterId: number };
  columns: {
    trialId: boolean;
    params: boolean;
    objectives: boolean;
    paretoRank: boolean;
    clusterId: boolean;
    trialState: boolean;
  };
}

export interface ReportConfig {
  studyName: string;
  author: string;
  comment: string;
  sections: ReportSection[];
  resolution: 'standard' | 'high';
  theme: 'light' | 'dark';
  format: 'html' | 'pdf' | 'markdown';
}

export type ReportSection =
  | 'convergence'
  | 'pareto'
  | 'sensitivity'
  | 'clustering'
  | 'pinned-trials'
  | 'scatter-matrix';

/** ライブ更新Store */
export interface LiveUpdateStore {
  // 状態
  isLive: boolean;
  pollInterval: number; // 秒
  lastUpdated: number | null; // timestamp
  newTrialCount: number;
  recentUpdates: Array<{ count: number; timestamp: number }>;

  // アクション
  toggleLive: () => Promise<void>;
  setPollInterval: (seconds: number) => void;
}

// =============================================================================
// WASM境界の型（wasm-bindgen経由で受け取る値）
// =============================================================================

/** WASM parse_journal() の戻り値 */
export interface ParseJournalResult {
  studies: Study[];
  durationMs: number;
}

/** WASM filter_by_ranges() の戻り値 */
export interface FilterResult {
  selectedIndices: Uint32Array;
  selectedCount: number;
  durationMs: number;
}

/** WASM compute_pareto_ranks() の戻り値 */
export interface ParetoResult {
  ranks: Uint32Array;        // 各trialのParetoランク (0 = 非Pareto対象外)
  paretoIndices: Uint32Array; // Rank1のtrial_idのインデックス
  hypervolume: number | null;
  durationMs: number;
}

/** WASM run_kmeans() の戻り値 */
export interface KmeansResult {
  labels: Int32Array;         // 各trialのクラスタラベル
  centroids: number[][];      // [k][dims]
  wcss: number;
  silhouetteScore: number | null;
  durationMs: number;
}

/** WASM compute_spearman() の戻り値 */
export interface SpearmanResult {
  matrix: Float64Array; // flattenedの [nParams × nObjectives]
  paramNames: string[];
  objectiveNames: string[];
  durationMs: number;
}

/** WASM compute_ridge() の戻り値 */
export interface RidgeResult {
  betaMatrix: Float64Array; // [nParams × nObjectives]
  r2Values: Float64Array;   // [nObjectives]
  durationMs: number;
}

/** WASM compute_pdp() の戻り値 */
export interface PdpRawResult {
  gridPoints: Float64Array;
  pdpValues: Float64Array;
  confidenceLow: Float64Array;
  confidenceHigh: Float64Array;
  iceLines: Float64Array | null; // [nIce × nGrid] flattened
  durationMs: number;
}

// =============================================================================
// アーティファクト
// =============================================================================

export interface ArtifactMeta {
  artifactId: string;
  filename: string;
  mimetype: string;
  trialId: number;
}

export type ArtifactType = 'image' | 'csv' | 'text' | 'json' | 'audio' | 'video' | 'other';

/** アーティファクトビューア表示用 */
export interface ArtifactViewItem {
  meta: ArtifactMeta;
  type: ArtifactType;
  url: string | null; // ObjectURL（読み込み後に設定）
  trial: Trial;
}

// =============================================================================
// 複数Study比較
// =============================================================================

export type ComparisonMode = 'overlay' | 'side-by-side' | 'diff';

export interface ComparisonConfig {
  mainStudyId: number;
  comparisonStudyIds: number[];
  mode: ComparisonMode;
}

export interface StudyComparisonResult {
  mainStudyId: number;
  comparisonStudyId: number;
  canComparePareto: boolean; // 目的数・方向が一致するか
  warningMessage: string | null;
  paretoDominanceRatio: {
    mainDominatesComparison: number; // %
    nonDominated: number;            // %
    comparisonDominatesMain: number; // %
  } | null;
}

// =============================================================================
// セッション保存形式
// =============================================================================

/** 分析セッションの永続化形式（JSON） */
export interface SessionState {
  version: string;
  journalPath: string;
  selectedStudyId: number;
  filterRanges: Record<string, Range>;
  selectedIndices: number[];  // Uint32Arrayをシリアライズ
  colorMode: ColorMode;
  clusterConfig: ClusterConfig | null;
  layoutMode: LayoutMode;
  visibleCharts: ChartId[];
  pinnedTrials: PinnedTrial[];
  freeModeLayout: FreeModeLayout | null;
  savedAt: string; // ISO 8601
}
