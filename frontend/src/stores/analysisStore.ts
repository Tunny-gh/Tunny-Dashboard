import { create } from 'zustand'
import { subscribeWithSelector } from 'zustand/middleware'
import { useStudyStore } from './studyStore'
import { WasmLoader } from '../wasm/wasmLoader'
import type { SensitivityWasmResult, SobolWasmResult } from '../wasm/wasmLoader'

interface AnalysisState {
  sensitivityResult: SensitivityWasmResult | null
  isComputingSensitivity: boolean
  sensitivityError: string | null
  computeSensitivity: () => Promise<void>
  computeSensitivitySelected: (indices: Uint32Array) => Promise<void>
  sobolResult: SobolWasmResult | null
  isComputingSobol: boolean
  sobolError: string | null
  computeSobol: (nSamples?: number) => Promise<void>
}

export const useAnalysisStore = create<AnalysisState>()(
  subscribeWithSelector((set, get) => ({
    sensitivityResult: null,
    isComputingSensitivity: false,
    sensitivityError: null,
    sobolResult: null,
    isComputingSobol: false,
    sobolError: null,

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

    computeSobol: async (nSamples = 1024) => {
      const { sobolResult } = get()
      if (sobolResult !== null) return
      set({ isComputingSobol: true, sobolError: null })
      try {
        const wasm = await WasmLoader.getInstance()
        const result = wasm.computeSobol(nSamples)
        set({ sobolResult: result, isComputingSobol: false })
      } catch (e) {
        set({
          sobolError: e instanceof Error ? e.message : String(e),
          isComputingSobol: false,
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
      sobolResult: null,
      sobolError: null,
      isComputingSobol: false,
    })
  }
})
