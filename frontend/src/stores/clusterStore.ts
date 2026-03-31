import { create } from 'zustand'
import { subscribeWithSelector } from 'zustand/middleware'
import { useStudyStore } from './studyStore'
import { WasmLoader } from '../wasm/wasmLoader'
import type { ClusterStatsWasmResult, ElbowWasmResult } from '../wasm/pkg/tunny_core'

export type ClusterSpace = 'param' | 'objective' | 'all'

interface ClusterState {
  pcaProjections: number[][] | null
  clusterLabels: number[] | null
  clusterStats: ClusterStatsWasmResult | null
  elbowResult: ElbowWasmResult | null
  isRunning: boolean
  clusterError: string | null
  clusterSpace: ClusterSpace
  k: number
  runClustering: (space: ClusterSpace, k: number) => Promise<void>
  estimateK: (space: ClusterSpace) => Promise<void>
}

export const useClusterStore = create<ClusterState>()(
  subscribeWithSelector((set) => ({
    pcaProjections: null,
    clusterLabels: null,
    clusterStats: null,
    elbowResult: null,
    isRunning: false,
    clusterError: null,
    clusterSpace: 'param',
    k: 3,

    runClustering: async (space: ClusterSpace, k: number) => {
      set({ isRunning: true, clusterError: null, clusterSpace: space, k })
      try {
        const wasm = await WasmLoader.getInstance()

        const pcaResult = wasm.runPca(2, space)
        set({ pcaProjections: pcaResult.projections })

        const flat = new Float64Array(pcaResult.projections.flatMap((row) => row))
        const kmeansResult = wasm.runKmeans(k, flat, 2)
        set({ clusterLabels: kmeansResult.labels })

        const labelsInt32 = new Int32Array(kmeansResult.labels)
        const statsResult = wasm.computeClusterStats(labelsInt32)
        set({ clusterStats: statsResult, isRunning: false })
      } catch (e) {
        set({
          clusterError: e instanceof Error ? e.message : String(e),
          isRunning: false,
        })
      }
    },

    estimateK: async (space: ClusterSpace) => {
      set({ isRunning: true, clusterError: null })
      try {
        const wasm = await WasmLoader.getInstance()

        const pcaResult = wasm.runPca(2, space)
        const flat = new Float64Array(pcaResult.projections.flatMap((row) => row))
        const elbowResult = wasm.estimateKElbow(flat, 2, 10)
        set({ elbowResult, isRunning: false })
      } catch (e) {
        set({
          clusterError: e instanceof Error ? e.message : String(e),
          isRunning: false,
        })
      }
    },
  })),
)

// Reset cluster state when active study changes
let _prevStudy = useStudyStore.getState().currentStudy
useStudyStore.subscribe((state) => {
  if (state.currentStudy !== _prevStudy) {
    _prevStudy = state.currentStudy
    useClusterStore.setState({
      pcaProjections: null,
      clusterLabels: null,
      clusterStats: null,
      elbowResult: null,
      clusterError: null,
      isRunning: false,
    })
  }
})
