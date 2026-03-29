import type { ColormapName } from '../colormaps'

/**
 * Tunny Dashboard - TypeScript type definitions
 * Boundary contract with WASM, Zustand stores, and UI component props
 */

// =============================================================================
// Core entities
// =============================================================================

/** Optuna Study metadata */
export interface Study {
  studyId: number
  name: string
  directions: OptimizationDirection[]
  completedTrials: number
  totalTrials: number
  paramNames: string[]
  objectiveNames: string[]
  userAttrNames: string[]
  hasConstraints: boolean
}

export type OptimizationDirection = 'minimize' | 'maximize'

/** Per-trial data returned by the getTrials() WASM method */
export interface TrialData {
  trialId: number
  params: Record<string, number>
  values: number[]
  paretoRank: number | null
}

/** Single trial data (for the bottom table display) */
export interface Trial {
  trialId: number
  state: TrialState
  params: Record<string, number | string>
  values: number[] | null
  paretoRank: number | null
  clusterId: number | null
  isFeasible: boolean | null
  userAttrs: Record<string, number | string>
  artifactIds: string[]
}

export type TrialState = 'COMPLETE' | 'RUNNING' | 'PRUNED' | 'FAIL'

/** Column metadata of the WASM DataFrame (as recognized by the JS side) */
export interface DataFrameInfo {
  rowCount: number
  columnNames: string[]
  paramColumns: string[]
  objectiveColumns: string[]
  userAttrColumns: string[]
  constraintColumns: string[]
  derivedColumns: string[] // is_feasible, constraint_sum, pareto_rank, cluster_id (derived)
}

/** Distribution info used for inverse transformation */
export type Distribution =
  | { type: 'FloatDistribution'; low: number; high: number; log: boolean }
  | { type: 'IntDistribution'; low: number; high: number; step: number; log: boolean }
  | { type: 'CategoricalDistribution'; choices: (string | number | boolean)[] }
  | { type: 'UniformDistribution'; low: number; high: number }

// =============================================================================
// GPU buffers
// =============================================================================

/** GPU buffer set for WebGL rendering */
export interface GpuBuffers {
  positions: Float32Array // x, y coordinates (N × 2)
  positions3d: Float32Array // x, y, z coordinates (N × 3) — for 3D Pareto
  colors: Float32Array // r, g, b, a (N × 4)
  sizes: Float32Array // point sizes (N × 1)
  trialCount: number
}

// =============================================================================
// Zustand Stores
// =============================================================================

/** Core brushing & linking store (spec Section 6) */
export interface SelectionStore {
  // State
  selectedIndices: Uint32Array
  filterRanges: Record<string, Range>
  highlighted: number | null
  colorMode: ColormapName

  // Actions
  brushSelect: (indices: Uint32Array) => void
  addAxisFilter: (axis: string, min: number, max: number) => void
  removeAxisFilter: (axis: string) => void
  clearSelection: () => void
  setHighlight: (index: number | null) => void
  setColorMode: (mode: ColormapName) => void
}

export type { ColormapName }

export type ColorMode = ColormapName

export interface Range {
  min: number
  max: number
}

/** Study management store */
export interface StudyStore {
  // State
  currentStudy: Study | null
  allStudies: Study[]
  studyMode: StudyMode
  isLoading: boolean
  loadError: string | null

  // Actions
  loadJournal: (file: File) => Promise<void>
  selectStudy: (studyId: number) => void
  setComparisonStudies: (studyIds: number[]) => void
  getDataFrameInfo: () => DataFrameInfo | null
}

export type StudyMode = 'single-objective' | 'multi-objective'

/** Layout management store */
export interface LayoutStore {
  // State
  layoutMode: LayoutMode
  visibleCharts: Set<ChartId>
  panelSizes: PanelSizes
  freeModeLayout: FreeModeLayout | null

  // Actions
  setLayoutMode: (mode: LayoutMode) => void
  toggleChart: (chartId: ChartId) => void
  saveLayout: () => LayoutConfig
  loadLayout: (config: LayoutConfig) => void
}

export type LayoutMode = 'A' | 'B' | 'C' | 'D'

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
  // Additional charts equivalent to optuna-dashboard (no Python required)
  | 'slice' // Slice Plot: parameter vs objective value scatter
  | 'edf' // EDF: Empirical Distribution Function
  | 'contour' // Contour Plot: 2-parameter correlation (scatter only; ML interpolation requires Python)
// Not yet implemented (requires data extension):
//   'timeline'            — needs Trial.datetime_start/datetime_complete
//                           recorded in Optuna Journal but WASM parser extension required
//   'intermediate-values' — needs Trial.intermediate_values
//                           PRUNED trial intermediate values are in the Journal but not yet parsed by WASM

export interface PanelSizes {
  leftPanel: number
  bottomPanel: number
}

export interface FreeModeLayout {
  cells: Array<{
    /** Unique cell identifier. Generated via crypto.randomUUID() when added from the catalog; equals chartId in default layouts. */
    cellId: string
    chartId: ChartId
    gridRow: [number, number]
    gridCol: [number, number]
  }>
}

export interface LayoutConfig {
  mode: LayoutMode
  visibleCharts: ChartId[]
  panelSizes: PanelSizes
  freeModeLayout: FreeModeLayout | null
}

/** Clustering store */
export interface ClusterStore {
  // State
  clusterConfig: ClusterConfig
  clusterLabels: Int32Array | null // cluster ID per trial (-1 = noise)
  clusterStats: ClusterStats[] | null
  elbowData: ElbowData | null
  isRunning: boolean
  progress: number // 0–1

  // Actions
  setClusterConfig: (config: Partial<ClusterConfig>) => void
  runClustering: () => Promise<void>
  selectCluster: (clusterId: number | null) => void
}

export interface ClusterConfig {
  space: 'objective' | 'variable' | 'combined'
  algorithm: 'kmeans' | 'hdbscan'
  dimensionReduction: 'pca' | 'umap' | 'none'
  reducedDims: number
  kAuto: boolean
  k: number
  kEstimateMethod: 'elbow' | 'silhouette'
  targetSamples: 'all' | 'pareto' | 'selected'
}

export interface ClusterStats {
  clusterId: number
  size: number
  centroid: Record<string, number>
  std: Record<string, number>
  significantDiffs: string[] // variables with large deviation from the overall mean
}

export interface ElbowData {
  ks: number[]
  wcss: number[]
  recommendedK: number
}

/** Analysis (sensitivity / PDP) store */
export interface AnalysisStore {
  // State
  sensitivityResult: SensitivityResult | null
  sensitivityMetric: SensitivityMetric
  pdpCache: Map<string, PDPResult> // key = `${paramName}_${objectiveName}`
  modelQuality: Record<string, ModelQuality>
  isComputingSensitivity: boolean
  isComputingMic: boolean

  // Actions
  computeSensitivity: () => Promise<void>
  computePDP: (paramName: string, objectiveName: string) => Promise<PDPResult>
  loadOnnxModel: (file: File, objectiveName: string) => Promise<void>
  loadShapValues: (file: File) => Promise<void>
  setSensitivityMetric: (metric: SensitivityMetric) => void
}

export type SensitivityMetric = 'spearman' | 'beta' | 'mic' | 'rf_importance' | 'shap'

export interface SensitivityResult {
  metric: SensitivityMetric
  /** matrix[paramIdx][objectiveIdx] = coefficient value */
  matrix: number[][]
  paramNames: string[]
  objectiveNames: string[]
  r2: number[] | null // R² per objective (Layer 1-C)
}

export interface PDPResult {
  paramName: string
  objectiveName: string
  gridPoints: number[] // x-axis: variable value grid (50 points)
  pdpValues: number[] // PDP curve
  confidenceLow: number[] // 95% CI lower bound
  confidenceHigh: number[] // 95% CI upper bound
  iceLines: number[][] | null // ICE individual trajectories (up to 100)
  rugValues: number[] // actual sample values (for rug display)
  modelType: 'ridge' | 'random_forest'
}

export interface ModelQuality {
  objectiveName: string
  r2: number
  rmse: number
  quality: 'good' | 'warning' | 'poor'
}

/** Export & session store */
export interface ExportStore {
  // State
  pinnedTrials: PinnedTrial[]
  sessionState: SessionState | null

  // Actions
  pinTrial: (trialId: number, note?: string) => void
  unpinTrial: (trialId: number) => void
  exportCsv: (config: CsvExportConfig) => Promise<void>
  exportPng: (chartId: ChartId) => Promise<void>
  exportHtml: (chartId: ChartId) => Promise<void>
  generateReport: (config: ReportConfig) => Promise<void>
  saveSession: () => Promise<void>
  loadSession: (file: File) => Promise<void>
}

export interface PinnedTrial {
  trialId: number
  note: string
  pinnedAt: number // Unix timestamp
}

export interface CsvExportConfig {
  target: 'all' | 'selected' | 'pareto' | { clusterId: number }
  columns: {
    trialId: boolean
    params: boolean
    objectives: boolean
    paretoRank: boolean
    clusterId: boolean
    trialState: boolean
  }
}

export interface ReportConfig {
  studyName: string
  author: string
  comment: string
  sections: ReportSection[]
  resolution: 'standard' | 'high'
  theme: 'light' | 'dark'
  format: 'html' | 'pdf' | 'markdown'
}

export type ReportSection =
  | 'convergence'
  | 'pareto'
  | 'sensitivity'
  | 'clustering'
  | 'pinned-trials'
  | 'scatter-matrix'

/** Live update store */
export interface LiveUpdateStore {
  // State
  isLive: boolean
  pollInterval: number // seconds
  lastUpdated: number | null // Unix timestamp
  newTrialCount: number
  recentUpdates: Array<{ count: number; timestamp: number }>

  // Actions
  toggleLive: () => Promise<void>
  setPollInterval: (seconds: number) => void
}

// =============================================================================
// WASM boundary types (values received via wasm-bindgen)
// =============================================================================

/** Return value of WASM parse_journal() */
export interface ParseJournalResult {
  studies: Study[]
  durationMs: number
}

/** Return value of WASM filter_by_ranges() */
export interface FilterResult {
  selectedIndices: Uint32Array
  selectedCount: number
  durationMs: number
}

/** Return value of WASM compute_pareto_ranks() */
export interface ParetoResult {
  ranks: Uint32Array // Pareto rank per trial (0 = not on Pareto front)
  paretoIndices: Uint32Array // indices of rank-1 trial IDs
  hypervolume: number | null
  durationMs: number
}

/** Return value of WASM run_kmeans() */
export interface KmeansResult {
  labels: Int32Array // cluster label per trial
  centroids: number[][] // [k][dims]
  wcss: number
  silhouetteScore: number | null
  durationMs: number
}

/** Return value of WASM compute_spearman() */
export interface SpearmanResult {
  matrix: Float64Array // flattened [nParams × nObjectives]
  paramNames: string[]
  objectiveNames: string[]
  durationMs: number
}

/** Return value of WASM compute_ridge() */
export interface RidgeResult {
  betaMatrix: Float64Array // [nParams × nObjectives]
  r2Values: Float64Array // [nObjectives]
  durationMs: number
}

/** Return value of WASM compute_pdp() */
export interface PdpRawResult {
  gridPoints: Float64Array
  pdpValues: Float64Array
  confidenceLow: Float64Array
  confidenceHigh: Float64Array
  iceLines: Float64Array | null // [nIce × nGrid] flattened
  durationMs: number
}

// =============================================================================
// Artifacts
// =============================================================================

export interface ArtifactMeta {
  artifactId: string
  filename: string
  mimetype: string
  trialId: number
}

export type ArtifactType = 'image' | 'csv' | 'text' | 'json' | 'audio' | 'video' | 'other'

/** Data for the artifact viewer display */
export interface ArtifactViewItem {
  meta: ArtifactMeta
  type: ArtifactType
  url: string | null // ObjectURL (set after loading)
  trial: Trial
}

// =============================================================================
// Multi-study comparison
// =============================================================================

export type ComparisonMode = 'overlay' | 'side-by-side' | 'diff'

export interface ComparisonConfig {
  mainStudyId: number
  comparisonStudyIds: number[]
  mode: ComparisonMode
}

export interface StudyComparisonResult {
  mainStudyId: number
  comparisonStudyId: number
  canComparePareto: boolean // true if objective count and directions match
  warningMessage: string | null
  paretoDominanceRatio: {
    mainDominatesComparison: number // %
    nonDominated: number // %
    comparisonDominatesMain: number // %
  } | null
}

// =============================================================================
// Documentation.
// =============================================================================

/** Documentation. */
export interface SessionState {
  version: string
  journalPath: string
  selectedStudyId: number
  filterRanges: Record<string, Range>
  selectedIndices: number[] // Documentation.
  colorMode: ColorMode
  clusterConfig: ClusterConfig | null
  layoutMode: LayoutMode
  visibleCharts: ChartId[]
  pinnedTrials: PinnedTrial[]
  freeModeLayout: FreeModeLayout | null
  savedAt: string // ISO 8601
}
