/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 */

import { describe, test, expect, beforeEach, vi, afterEach } from 'vitest'

// -------------------------------------------------------------------------
// Documentation.
// Documentation.
// -------------------------------------------------------------------------

const { mockFilterByRanges, mockGetInstance } = vi.hoisted(() => {
  const mockFilterByRanges = vi.fn()
  const mockGetInstance = vi.fn().mockResolvedValue({ filterByRanges: mockFilterByRanges })
  return { mockFilterByRanges, mockGetInstance }
})

vi.mock('../wasm/wasmLoader', () => ({
  WasmLoader: {
    getInstance: mockGetInstance,
    reset: vi.fn(),
  },
}))

import { useSelectionStore } from './selectionStore'
import { GpuBuffer } from '../wasm/gpuBuffer'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 */
function makeGpuBufferData(n: number) {
  const positions = new Float32Array(n * 2)
  const positions3d = new Float32Array(n * 3)
  const sizes = new Float32Array(n * 1)
  for (let i = 0; i < n * 2; i++) positions[i] = i * 0.01 + 0.001
  for (let i = 0; i < n * 3; i++) positions3d[i] = i * 0.01 + 0.001
  for (let i = 0; i < n; i++) sizes[i] = i * 0.1 + 1.0
  return {
    positions: positions.buffer,
    positions3d: positions3d.buffer,
    sizes: sizes.buffer,
    trialCount: n,
  }
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 */
function resetStore() {
  useSelectionStore.setState({
    selectedIndices: new Uint32Array(0),
    filterRanges: {},
    highlighted: null,
    colorMode: 'objective',
    _trialCount: 0,
  })
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  beforeEach(() => {
    resetStore()
    vi.clearAllMocks()
    mockFilterByRanges.mockReturnValue(new Uint32Array([0, 2, 4]))
    mockGetInstance.mockResolvedValue({ filterByRanges: mockFilterByRanges })
  })

  // Documentation.
  test('TC-302-01', () => {
    // Documentation.
    const selected = new Uint32Array([1, 3, 5])

    // Documentation.
    useSelectionStore.getState().brushSelect(selected)

    // Documentation.
    expect(Array.from(useSelectionStore.getState().selectedIndices)).toEqual([1, 3, 5])
  })

  // Documentation.
  test('TC-302-02', () => {
    // Documentation.

    // Documentation.
    useSelectionStore.setState({ _trialCount: 5 })

    // Documentation.
    useSelectionStore.getState().clearSelection()

    const state = useSelectionStore.getState()
    // Documentation.
    expect(Array.from(state.selectedIndices)).toEqual([0, 1, 2, 3, 4])
    // Documentation.
    expect(state.filterRanges).toEqual({})
  })

  // Documentation.
  test('TC-302-03', () => {
    // Documentation.

    // Documentation.
    useSelectionStore.getState().addAxisFilter('x', 0.1, 0.9)

    // Documentation.
    expect(useSelectionStore.getState().filterRanges['x']).toEqual({ min: 0.1, max: 0.9 })
  })

  // Documentation.
  test('TC-302-04', async () => {
    // Documentation.
    mockFilterByRanges.mockReturnValue(new Uint32Array([0, 2, 4]))

    // Documentation.
    useSelectionStore.getState().addAxisFilter('x', 0.0, 0.5)

    // Documentation.
    await vi.waitFor(() => {
      expect(mockFilterByRanges).toHaveBeenCalledOnce()
      expect(Array.from(useSelectionStore.getState().selectedIndices)).toEqual([0, 2, 4])
    })
  })

  // Documentation.
  test('TC-302-05', async () => {
    // Documentation.
    useSelectionStore.getState().addAxisFilter('x', 0.0, 1.0)
    await vi.waitFor(() => expect(mockFilterByRanges).toHaveBeenCalledOnce())

    vi.clearAllMocks()
    // Documentation.
    useSelectionStore.getState().removeAxisFilter('x')

    // Documentation.
    expect(useSelectionStore.getState().filterRanges['x']).toBeUndefined()
  })

  // Documentation.
  test('TC-302-06', () => {
    // Documentation.
    useSelectionStore.getState().setHighlight(7)
    expect(useSelectionStore.getState().highlighted).toBe(7)
  })

  // Documentation.
  test('TC-302-07', () => {
    // Documentation.
    useSelectionStore.getState().setColorMode('cluster')
    expect(useSelectionStore.getState().colorMode).toBe('cluster')
  })

  // Documentation.
  test('TC-302-08', () => {
    // Documentation.

    // Documentation.
    const buf = new GpuBuffer(makeGpuBufferData(5))

    // Documentation.
    const unsubscribe = useSelectionStore.subscribe(
      (state) => state.selectedIndices,
      (indices) => buf.updateAlphas(indices),
    )

    // Documentation.
    useSelectionStore.getState().brushSelect(new Uint32Array([2, 4]))

    // Documentation.
    expect(buf.colors[2 * 4 + 3]).toBeCloseTo(1.0)
    expect(buf.colors[4 * 4 + 3]).toBeCloseTo(1.0)
    // Documentation.
    expect(buf.colors[0 * 4 + 3]).toBeCloseTo(0.2)
    expect(buf.colors[1 * 4 + 3]).toBeCloseTo(0.2)

    unsubscribe()
  })
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  beforeEach(() => {
    resetStore()
    vi.clearAllMocks()
  })

  // Documentation.
  test('TC-302-E02', async () => {
    // Documentation.
    mockGetInstance.mockRejectedValueOnce(new Error('WASM not ready'))

    // Documentation.
    expect(() => useSelectionStore.getState().addAxisFilter('y', 0, 1)).not.toThrow()

    // Documentation.
    expect(useSelectionStore.getState().filterRanges['y']).toEqual({ min: 0, max: 1 })

    // Documentation.
    await new Promise((r) => setTimeout(r, 20)) // Documentation.
    expect(useSelectionStore.getState().selectedIndices.length).toBe(0)
  })
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  beforeEach(() => {
    resetStore()
    vi.clearAllMocks()
  })

  // Documentation.
  test('TC-302-B01', () => {
    // Documentation.
    useSelectionStore.setState({ _trialCount: 0 })

    expect(() => useSelectionStore.getState().clearSelection()).not.toThrow()
    expect(useSelectionStore.getState().selectedIndices.length).toBe(0)
  })

  // Documentation.
  test('TC-302-B02', () => {
    // Documentation.
    expect(() => useSelectionStore.getState().removeAxisFilter('nonexistent')).not.toThrow()
    expect(useSelectionStore.getState().filterRanges).toEqual({})
  })
})
