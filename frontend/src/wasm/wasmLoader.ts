/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 */

import initWasm, {
  parseJournal as wasmParseJournal,
  selectStudy as wasmSelectStudy,
  getTrials as wasmGetTrials,
  filterByRanges as wasmFilterByRanges,
  serializeCsv as wasmSerializeCsv,
  computeHvHistory as wasmComputeHvHistory,
  appendJournalDiff as wasmAppendJournalDiff,
  computeReportStats as wasmComputeReportStats,
  computeSensitivity as wasmComputeSensitivity,
  computeSensitivitySelected as wasmComputeSensitivitySelected,
  runPca as wasmRunPca,
  runKmeans as wasmRunKmeans,
  estimateKElbow as wasmEstimateKElbow,
  computeClusterStats as wasmComputeClusterStats,
  computeSobol as wasmComputeSobol,
  computeTopsis as wasmComputeTopsis,
  computePdp2d as wasmComputePdp2d,
} from './pkg/tunny_core'
import type { ParseJournalResult, ParetoResult, TrialData } from '../types/index'

export interface SensitivityWasmResult {
  paramNames: string[]
  objectiveNames: string[]
  spearman: number[][]
  ridge: Array<{ beta: number[]; rSquared: number }>
  durationMs?: number
}

export interface PcaWasmResult {
  projections: number[][]
  explainedVariance?: number[]
  featureNames?: string[]
  durationMs?: number
}

export interface KmeansWasmResult {
  labels: number[]
  centroids?: number[][]
  wcss?: number
  durationMs?: number
}

export interface ElbowWasmResult {
  wcssPerK: number[]
  recommendedK: number
  durationMs?: number
}

export interface SobolWasmResult {
  paramNames: string[]
  objectiveNames: string[]
  firstOrder: number[][]
  totalEffect: number[][]
  nSamples: number
  durationMs?: number
}

export interface ClusterStatsWasmResult {
  stats: Array<{
    clusterId: number
    size: number
    centroid: Record<string, number>
    std: Record<string, number>
    significantDiffs: string[]
  }>
  durationMs?: number
}

export interface TopsisWasmResult {
  scores: number[]
  rankedIndices: number[]
  positiveIdeal: number[]
  negativeIdeal: number[]
  durationMs: number
}

export interface Pdp2dWasmResult {
  param1Name: string
  param2Name: string
  objectiveName: string
  grid1: number[]
  grid2: number[]
  values: number[][]
  rSquared: number
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 */
export interface SelectStudyResult {
  positions: ArrayBuffer
  positions3d: ArrayBuffer
  sizes: ArrayBuffer
  trialCount: number
}

/**
 * Documentation.
 * Documentation.
 */
export interface HvHistoryResult {
  trialIds: Uint32Array
  hvValues: Float64Array
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 */
export class WasmLoader {
  /**
   * Documentation.
   * Documentation.
   */
  private static _promise: Promise<WasmLoader> | null = null

  // -------------------------------------------------------------------------
  // Documentation.
  // -------------------------------------------------------------------------

  /**
   * Documentation.
   * Documentation.
   */
  parseJournal!: (data: Uint8Array) => ParseJournalResult

  /**
   * Documentation.
   * Documentation.
   */
  selectStudy!: (studyId: number) => SelectStudyResult

  /**
   * Documentation.
   * Documentation.
   */
  filterByRanges!: (rangesJson: string) => Uint32Array

  /**
   * Documentation.
   * Documentation.
   */
  computeParetoRanks!: (isMinimize: boolean[]) => ParetoResult

  /**
   * Documentation.
   * Documentation.
   */
  computeHvHistory!: (isMinimize: boolean[]) => HvHistoryResult

  /**
   * Documentation.
   * Documentation.
   */
  serializeCsv!: (indices: number[], columnsJson: string) => string

  /**
   * Documentation.
   * Documentation.
   */
  appendJournalDiff!: (data: Uint8Array) => { new_completed: number; consumed_bytes: number }

  /**
   * Documentation.
   * Documentation.
   */
  computeReportStats!: () => string

  /**
   * Documentation.
   * Documentation.
   */
  getTrials!: () => TrialData[]

  computeSensitivity!: () => SensitivityWasmResult
  computeSensitivitySelected!: (indices: Uint32Array) => SensitivityWasmResult
  runPca!: (nComponents: number, space: string) => PcaWasmResult
  runKmeans!: (k: number, data: Float64Array, nCols: number) => KmeansWasmResult
  estimateKElbow!: (data: Float64Array, nCols: number, maxK: number) => ElbowWasmResult
  computeClusterStats!: (labels: Int32Array) => ClusterStatsWasmResult
  computeSobol!: (nSamples: number) => SobolWasmResult
  computeTopsis!: (
    values: Float64Array,
    nTrials: number,
    nObjectives: number,
    weights: Float64Array,
    isMinimize: boolean[],
  ) => TopsisWasmResult
  computePdp2d!: (
    param1Name: string,
    param2Name: string,
    objectiveName: string,
    nGrid: number,
  ) => Pdp2dWasmResult

  /**
   * Documentation.
   */
  private constructor() {}

  // -------------------------------------------------------------------------
  // Documentation.
  // -------------------------------------------------------------------------

  /**
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   */
  static getInstance(): Promise<WasmLoader> {
    // Documentation.
    if (WasmLoader._promise === null) {
      // Documentation.
      // Documentation.
      WasmLoader._promise = WasmLoader._initialize()
    }
    return WasmLoader._promise
  }

  /**
   * Documentation.
   * Documentation.
   */
  private static async _initialize(): Promise<WasmLoader> {
    // Documentation.
    await initWasm()

    const loader = new WasmLoader()

    // Documentation.
    // Documentation.
    loader.parseJournal = (data: Uint8Array) => wasmParseJournal(data) as ParseJournalResult
    loader.selectStudy = (studyId: number) => wasmSelectStudy(studyId) as SelectStudyResult
    loader.filterByRanges = (rangesJson: string) => wasmFilterByRanges(rangesJson) as Uint32Array
    loader.computeParetoRanks = _notImplemented('computeParetoRanks')
    loader.computeHvHistory = (isMinimize: boolean[]) =>
      wasmComputeHvHistory(isMinimize) as HvHistoryResult
    loader.serializeCsv = (indices: number[], columnsJson: string) =>
      wasmSerializeCsv(indices, columnsJson) as string
    loader.appendJournalDiff = (data: Uint8Array) =>
      wasmAppendJournalDiff(data) as { new_completed: number; consumed_bytes: number }
    loader.computeReportStats = () => wasmComputeReportStats() as string
    loader.getTrials = () => wasmGetTrials() as TrialData[]
    loader.computeSensitivity = () => wasmComputeSensitivity() as SensitivityWasmResult
    loader.computeSensitivitySelected = (indices) =>
      wasmComputeSensitivitySelected(indices) as SensitivityWasmResult
    loader.runPca = (nComponents, space) => wasmRunPca(nComponents, space) as PcaWasmResult
    loader.runKmeans = (k, data, nCols) => wasmRunKmeans(k, data, nCols) as KmeansWasmResult
    loader.estimateKElbow = (data, nCols, maxK) =>
      wasmEstimateKElbow(data, nCols, maxK) as ElbowWasmResult
    loader.computeClusterStats = (labels) =>
      wasmComputeClusterStats(labels) as ClusterStatsWasmResult
    loader.computeSobol = (nSamples: number) => wasmComputeSobol(nSamples) as SobolWasmResult
    loader.computeTopsis = (values, nTrials, nObjectives, weights, isMinimize) =>
      wasmComputeTopsis(values, nTrials, nObjectives, weights, isMinimize) as TopsisWasmResult
    loader.computePdp2d = (param1Name, param2Name, objectiveName, nGrid) =>
      wasmComputePdp2d(param1Name, param2Name, objectiveName, nGrid) as Pdp2dWasmResult

    return loader
  }

  // -------------------------------------------------------------------------
  // Documentation.
  // -------------------------------------------------------------------------

  /**
   * Documentation.
   * Documentation.
   * Documentation.
   */
  static reset(): void {
    WasmLoader._promise = null
  }
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
function _notImplemented(name: string): (...args: any[]) => never {
  return () => {
    throw new Error(`WasmLoader.${name} will be available after TASK-103 implementation`)
  }
}
