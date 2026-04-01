import { create } from 'zustand'
import { subscribeWithSelector } from 'zustand/middleware'
import { useStudyStore } from './studyStore'
import { WasmLoader } from '../wasm/wasmLoader'
import type { SensitivityWasmResult } from '../wasm/wasmLoader'

interface AnalysisState {
  sensitivityResult: SensitivityWasmResult | null
  isComputingSensitivity: boolean
  sensitivityError: string | null
  computeSensitivity: () => Promise<void>
  computeSensitivitySelected: (indices: Uint32Array) => Promise<void>
}

export const useAnalysisStore = create<AnalysisState>()(
  subscribeWithSelector((set) => ({
    sensitivityResult: null,
    isComputingSensitivity: false,
    sensitivityError: null,

    computeSensitivity: async () => {
      set({ isComputingSensitivity: true, sensitivityError: null })
      try {
        const wasm = await WasmLoader.getInstance()
        const result = wasm.computeSensitivity()
        set({ sensitivityResult: result, isComputingSensitivity: false })
      } catch (e) {
        set({
          sensitivityError: e instanceof Error ? e.message : String(e),
          isComputingSensitivity: false,
        })
      }
    },

    computeSensitivitySelected: async (indices: Uint32Array) => {
      set({ isComputingSensitivity: true, sensitivityError: null })
      try {
        const wasm = await WasmLoader.getInstance()
        const result = wasm.computeSensitivitySelected(indices)
        set({ sensitivityResult: result, isComputingSensitivity: false })
      } catch (e) {
        set({
          sensitivityError: e instanceof Error ? e.message : String(e),
          isComputingSensitivity: false,
        })
      }
    },
  })),
)

// Reset sensitivity when active study changes
let _prevStudy = useStudyStore.getState().currentStudy
useStudyStore.subscribe((state) => {
  if (state.currentStudy !== _prevStudy) {
    _prevStudy = state.currentStudy
    useAnalysisStore.setState({
      sensitivityResult: null,
      sensitivityError: null,
      isComputingSensitivity: false,
    })
  }
})
