/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 */

import { create } from 'zustand'
import { subscribeWithSelector } from 'zustand/middleware'
import type { ColorMode, Range } from '../types'
import { DEFAULT_COLORMAP } from '../colormaps'
import { WasmLoader } from '../wasm/wasmLoader'

// -------------------------------------------------------------------------
// Type definitions
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 */
interface SelectionState {
  // Documentation.
  /** Documentation. */
  selectedIndices: Uint32Array
  /** Documentation. */
  filterRanges: Record<string, Range>
  /** Documentation. */
  highlighted: number | null
  /** Documentation. */
  colorMode: ColorMode

  // Documentation.
  brushSelect: (indices: Uint32Array) => void
  addAxisFilter: (axis: string, min: number, max: number) => void
  removeAxisFilter: (axis: string) => void
  clearSelection: () => void
  setHighlight: (index: number | null) => void
  setColorMode: (mode: ColorMode) => void

  // Documentation.
  /** Documentation. */
  _trialCount: number
  /** Documentation. */
  _setTrialCount: (n: number) => void
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 *   `useSelectionStore.subscribe((s) => s.selectedIndices, (indices) => gpuBuf.updateAlphas(indices))`
 * Documentation.
 * Documentation.
 */
export const useSelectionStore = create<SelectionState>()(
  subscribeWithSelector((set, get) => ({
    // -------------------------------------------------------------------------
    // Documentation.
    // -------------------------------------------------------------------------
    selectedIndices: new Uint32Array(0),
    filterRanges: {},
    highlighted: null,
    colorMode: DEFAULT_COLORMAP as ColorMode,
    _trialCount: 0,

    // -------------------------------------------------------------------------
    // Documentation.
    // -------------------------------------------------------------------------

    /**
     * Documentation.
     * Documentation.
     */
    brushSelect: (indices) => {
      set({ selectedIndices: indices })
    },

    /**
     * Documentation.
     * Documentation.
     * Documentation.
     * Documentation.
     * Documentation.
     */
    addAxisFilter: (axis, min, max) => {
      // Documentation.
      const newRanges: Record<string, Range> = {
        ...get().filterRanges,
        [axis]: { min, max },
      }
      set({ filterRanges: newRanges })

      // Documentation.
      WasmLoader.getInstance()
        .then((wasm) => {
          const indices = wasm.filterByRanges(JSON.stringify(newRanges))
          set({ selectedIndices: indices })
        })
        .catch(() => {
          // Documentation.
        })
    },

    /**
     * Documentation.
     * Documentation.
     * Documentation.
     */
    removeAxisFilter: (axis) => {
      // Documentation.
      const current = get().filterRanges
      // eslint-disable-next-line @typescript-eslint/no-unused-vars
      const { [axis]: _removed, ...newRanges } = current
      set({ filterRanges: newRanges as Record<string, Range> })

      if (Object.keys(newRanges).length === 0) {
        // Documentation.
        const n = get()._trialCount
        set({ selectedIndices: _makeAllIndices(n) })
        return
      }

      // Documentation.
      WasmLoader.getInstance()
        .then((wasm) => {
          const indices = wasm.filterByRanges(JSON.stringify(newRanges))
          set({ selectedIndices: indices })
        })
        .catch(() => {
          // Documentation.
        })
    },

    /**
     * Documentation.
     * Documentation.
     */
    clearSelection: () => {
      const n = get()._trialCount
      set({
        selectedIndices: _makeAllIndices(n),
        filterRanges: {},
      })
    },

    /** Documentation. */
    setHighlight: (index) => set({ highlighted: index }),

    /** Documentation. */
    setColorMode: (mode) => set({ colorMode: mode }),

    /** Documentation. */
    _setTrialCount: (n) => set({ _trialCount: n }),
  })),
)

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 */
function _makeAllIndices(n: number): Uint32Array {
  const arr = new Uint32Array(n)
  for (let i = 0; i < n; i++) arr[i] = i
  return arr
}
