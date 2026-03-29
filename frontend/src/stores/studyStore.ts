/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 * Documentation.
 */

import { create } from 'zustand'
import type { Study, DataFrameInfo, StudyMode, TrialData } from '../types'
import { WasmLoader } from '../wasm/wasmLoader'
import { GpuBuffer } from '../wasm/gpuBuffer'
import { useSelectionStore } from './selectionStore'
import { useComparisonStore } from './comparisonStore'

// -------------------------------------------------------------------------
// Type definitions
// -------------------------------------------------------------------------

/**
 * Documentation.
 */
interface StudyState {
  // Documentation.
  currentStudy: Study | null
  allStudies: Study[]
  studyMode: StudyMode
  isLoading: boolean
  loadError: string | null

  // --- Actions ---
  loadJournal: (file: File) => Promise<void>
  selectStudy: (studyId: number) => void
  setComparisonStudies: (studyIds: number[]) => void
  getDataFrameInfo: () => DataFrameInfo | null

  // Documentation.
  /** Documentation. */
  gpuBuffer: GpuBuffer | null
  /** Documentation. */
  trialRows: TrialData[]
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

export const useStudyStore = create<StudyState>()((set, get) => ({
  // -------------------------------------------------------------------------
  // Documentation.
  // -------------------------------------------------------------------------
  currentStudy: null,
  allStudies: [],
  studyMode: 'single-objective' as StudyMode,
  isLoading: false,
  loadError: null,
  gpuBuffer: null,
  trialRows: [],

  // -------------------------------------------------------------------------
  // Documentation.
  // -------------------------------------------------------------------------

  /**
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   */
  loadJournal: async (file) => {
    // Documentation.
    set({ isLoading: true, loadError: null })

    try {
      // Documentation.
      const arrayBuffer = await file.arrayBuffer()
      const data = new Uint8Array(arrayBuffer)

      // Documentation.
      const wasm = await WasmLoader.getInstance()
      const result = wasm.parseJournal(data)

      // Documentation.
      set({ allStudies: result.studies, isLoading: false })
      if (result.studies.length > 0) {
        get().selectStudy(result.studies[0].studyId)
      }
    } catch (err) {
      // Documentation.
      set({ isLoading: false, loadError: String(err) })
    }
  },

  /**
   * Documentation.
   * Documentation.
   * Documentation.
   */
  selectStudy: (studyId) => {
    WasmLoader.getInstance()
      .then((wasm) => {
        // Documentation.
        const result = wasm.selectStudy(studyId)
        const gpuBuffer = new GpuBuffer(result)

        // Documentation.
        const study = get().allStudies.find((s) => s.studyId === studyId) ?? null
        // Documentation.
        const studyMode: StudyMode =
          study && study.directions.length > 1 ? 'multi-objective' : 'single-objective'

        // Documentation.
        let trialRows: TrialData[] = []
        try {
          trialRows = wasm.getTrials()
        } catch {
          // Documentation.
        }

        set({ currentStudy: study, gpuBuffer, studyMode, trialRows })

        // Documentation.
        useSelectionStore.getState()._setTrialCount(gpuBuffer.trialCount)
        useSelectionStore.getState().clearSelection()

        // Documentation.
        useComparisonStore.getState().reset()
      })
      .catch(() => {
        // Documentation.
      })
  },

  /**
   * Documentation.
   */
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  setComparisonStudies: (_studyIds) => {
    // Documentation.
  },

  /**
   * Documentation.
   */
  getDataFrameInfo: () => null,
}))
