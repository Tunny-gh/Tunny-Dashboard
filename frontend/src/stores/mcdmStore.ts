import { create } from 'zustand'
import { subscribeWithSelector } from 'zustand/middleware'
import { useStudyStore } from './studyStore'
import { WasmLoader } from '../wasm/wasmLoader'
import type { TopsisRankingResult } from '../types'

interface McdmState {
  // State
  topsisWeights: number[]
  topsisResult: TopsisRankingResult | null
  isComputing: boolean
  topN: number
  topsisError: string | null

  // Actions
  setTopsisWeights: (weights: number[]) => void
  computeTopsis: () => Promise<void>
  setTopN: (n: number) => void
  reset: () => void
}

export const useMcdmStore = create<McdmState>()(
  subscribeWithSelector((set, get) => ({
    topsisWeights: [],
    topsisResult: null,
    isComputing: false,
    topN: 10,
    topsisError: null,

    setTopsisWeights: (weights: number[]) => {
      // Normalize weights to sum=1.0
      const sum = weights.reduce((a, b) => a + b, 0)
      const normalized = sum > 0 ? weights.map((w) => w / sum) : weights
      set({ topsisWeights: normalized })
    },

    computeTopsis: async () => {
      const study = useStudyStore.getState().currentStudy
      if (!study || study.objectiveNames.length === 0) return

      const nObjectives = study.objectiveNames.length
      const { topsisWeights } = get()

      // Use equal weights if not set or dimension mismatch
      const weights =
        topsisWeights.length === nObjectives
          ? topsisWeights
          : study.objectiveNames.map(() => 1 / nObjectives)

      set({ isComputing: true, topsisError: null })
      try {
        const wasm = await WasmLoader.getInstance()

        // Get objective values from WASM getTrials()
        const trials = wasm.getTrials()
        const nTrials = trials.length
        if (nTrials === 0) {
          set({ isComputing: false })
          return
        }

        // Build flat [N×M] Float64Array (row-major: trial0_obj0, trial0_obj1, ...)
        const flatValues = new Float64Array(nTrials * nObjectives)
        for (let i = 0; i < nTrials; i++) {
          const vals = trials[i].values
          for (let j = 0; j < nObjectives; j++) {
            flatValues[i * nObjectives + j] = vals[j] ?? NaN
          }
        }

        const isMinimize = study.directions.map((d) => d === 'minimize')
        const result = wasm.computeTopsis(
          flatValues,
          nTrials,
          nObjectives,
          Float64Array.from(weights),
          isMinimize,
        )

        set({
          topsisResult: {
            scores: Array.from(result.scores as number[]),
            rankedIndices: Array.from(result.rankedIndices as number[]),
            positiveIdeal: Array.from(result.positiveIdeal as number[]),
            negativeIdeal: Array.from(result.negativeIdeal as number[]),
            durationMs: result.durationMs,
          },
          isComputing: false,
        })
      } catch (e) {
        set({
          topsisError: e instanceof Error ? e.message : String(e),
          isComputing: false,
        })
      }
    },

    setTopN: (n: number) => set({ topN: n }),

    reset: () => set({ topsisResult: null, topsisWeights: [], topsisError: null }),
  })),
)

// Reset MCDM state when active study changes (same pattern as clusterStore.ts)
let _prevStudy = useStudyStore.getState().currentStudy
useStudyStore.subscribe((state) => {
  if (state.currentStudy !== _prevStudy) {
    _prevStudy = state.currentStudy
    useMcdmStore.getState().reset()
  }
})
