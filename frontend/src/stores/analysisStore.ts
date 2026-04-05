import { create } from 'zustand'
import { subscribeWithSelector } from 'zustand/middleware'
import { useStudyStore } from './studyStore'
import { WasmLoader } from '../wasm/wasmLoader'
import type { SensitivityWasmResult, SobolWasmResult, Pdp2dWasmResult } from '../wasm/wasmLoader'
import type { SurrogateModelType } from '../types'

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
  // 3D surface plot state
  surrogateModelType: SurrogateModelType
  surface3dCache: Map<string, Pdp2dWasmResult>
  isComputingSurface: boolean
  surface3dError: string | null
  setSurrogateModelType: (type: SurrogateModelType) => void
  computeSurface3d: (
    param1: string,
    param2: string,
    objective: string,
    nGrid?: number,
  ) => Promise<void>
  clearSurface3dCache: () => void
}

export const useAnalysisStore = create<AnalysisState>()(
  subscribeWithSelector((set, get) => ({
    sensitivityResult: null,
    isComputingSensitivity: false,
    sensitivityError: null,
    sobolResult: null,
    isComputingSobol: false,
    sobolError: null,
    surrogateModelType: 'ridge' as SurrogateModelType,
    surface3dCache: new Map<string, Pdp2dWasmResult>(),
    isComputingSurface: false,
    surface3dError: null,

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

    setSurrogateModelType: (type: SurrogateModelType) => {
      set({ surrogateModelType: type })
    },

    computeSurface3d: async (param1: string, param2: string, objective: string, nGrid = 50) => {
      const { surrogateModelType, surface3dCache } = get()
      const cacheKey = `${surrogateModelType}_${param1}_${param2}_${objective}_${nGrid}`

      // Cache hit: skip WASM call
      if (surface3dCache.has(cacheKey)) return

      set({ isComputingSurface: true, surface3dError: null })
      try {
        const wasm = await WasmLoader.getInstance()
        const result = wasm.computePdp2d(param1, param2, objective, nGrid)
        const newCache = new Map(surface3dCache)
        newCache.set(cacheKey, result)
        set({ surface3dCache: newCache, isComputingSurface: false })
      } catch (e) {
        set({
          surface3dError: e instanceof Error ? e.message : 'Surface3D computation failed',
          isComputingSurface: false,
        })
      }
    },

    clearSurface3dCache: () => {
      set({ surface3dCache: new Map(), surface3dError: null })
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
      surface3dCache: new Map(),
      surface3dError: null,
    })
  }
})
