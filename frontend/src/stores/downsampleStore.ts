import { create } from 'zustand'
import { subscribeWithSelector } from 'zustand/middleware'
import { useStudyStore } from './studyStore'
import { useSelectionStore } from './selectionStore'
import { WasmLoader } from '../wasm/wasmLoader'
import type { DownsampleResult } from '../wasm/wasmLoader'
import { DOWNSAMPLE_CONFIGS } from '../types/downsampling'
import type { DownsampleKey, DownsampleState, DownsampleActions } from '../types/downsampling'

// Stable empty array — returning `new Uint32Array()` inline would create a fresh
// object on every selector call, causing Object.is to return false and triggering
// an infinite useSyncExternalStore re-render loop.
const EMPTY_INDICES = new Uint32Array(0)

function callDownsampleWasm(
  wasm: Awaited<ReturnType<typeof WasmLoader.getInstance>>,
  key: DownsampleKey,
): DownsampleResult {
  const config = DOWNSAMPLE_CONFIGS[key]
  switch (config.strategy) {
    case 'smart':
      return wasm.downsampleSmart(config.maxPoints, config.includePareto)
    case 'grid_thumbnail':
      return wasm.downsampleForThumbnail(config.maxPoints)
    case 'stratified_by_rank':
      return wasm.downsampleStratifiedByRank(config.maxPoints, config.nStrata ?? 5)
    case 'by_cluster':
      return wasm.downsampleByCluster(config.maxPoints)
  }
}

export const useDownsampleStore = create<DownsampleState & DownsampleActions>()(
  subscribeWithSelector((set, get) => ({
    cache: {},
    isComputing: false,
    error: null,
    lastTotalCount: 0,

    reset: () => set({ cache: {}, error: null }),

    recompute: async () => {
      set({ isComputing: true, error: null })
      try {
        const wasm = await WasmLoader.getInstance()
        const results: Partial<Record<DownsampleKey, Uint32Array>> = {}
        const keys = Object.keys(DOWNSAMPLE_CONFIGS) as DownsampleKey[]
        for (const key of keys) {
          const result = callDownsampleWasm(wasm, key)
          results[key] = new Uint32Array(result.indices)
        }
        const totalCount = results.scatter?.length ?? 0
        set({ cache: results, isComputing: false, lastTotalCount: totalCount })
      } catch (e) {
        set({ error: e instanceof Error ? e.message : String(e), isComputing: false })
      }
    },

    recomputeIfNeeded: async (filteredIndices: Uint32Array) => {
      const { lastTotalCount, recompute } = get()
      const newCount = filteredIndices.length
      const changeRatio = Math.abs(newCount - lastTotalCount) / Math.max(lastTotalCount, 1)
      if (changeRatio >= 0.2) {
        set({ lastTotalCount: newCount })
        await recompute()
      }
    },

    getIndices: (key: DownsampleKey) => {
      return get().cache[key] ?? EMPTY_INDICES
    },
  })),
)

// Trigger recompute when the active study changes
let _prevStudy = useStudyStore.getState().currentStudy
useStudyStore.subscribe((state) => {
  if (state.currentStudy !== _prevStudy) {
    _prevStudy = state.currentStudy
    useDownsampleStore.getState().recompute()
  }
})

// Trigger conditional recompute when filter results change
useSelectionStore.subscribe(
  (state) => state.selectedIndices,
  (selectedIndices) => {
    useDownsampleStore.getState().recomputeIfNeeded(selectedIndices)
  },
)
