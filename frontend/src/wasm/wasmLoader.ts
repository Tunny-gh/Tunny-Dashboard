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
} from './pkg/tunny_core'
import type { ParseJournalResult, ParetoResult, TrialData } from '../types/index'

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
