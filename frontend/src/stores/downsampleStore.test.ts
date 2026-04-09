import { describe, it, expect, vi, beforeEach } from 'vitest'
import { act } from 'react'

// ---------------------------------------------------------------------------
// Hoisted mocks
// ---------------------------------------------------------------------------

const {
  mockDownsampleSmart,
  mockDownsampleForThumbnail,
  mockDownsampleStratifiedByRank,
  mockDownsampleByCluster,
  mockGetInstance,
  capturedStudySubscribers,
  capturedSelectionListeners,
} = vi.hoisted(() => {
  const mockResult = { indices: [0, 1, 2], paretoCount: 1, totalCount: 100, durationMs: 1 }
  const mockDownsampleSmart = vi.fn().mockReturnValue(mockResult)
  const mockDownsampleForThumbnail = vi.fn().mockReturnValue(mockResult)
  const mockDownsampleStratifiedByRank = vi.fn().mockReturnValue(mockResult)
  const mockDownsampleByCluster = vi.fn().mockReturnValue(mockResult)
  const mockGetInstance = vi.fn().mockResolvedValue({
    downsampleSmart: mockDownsampleSmart,
    downsampleForThumbnail: mockDownsampleForThumbnail,
    downsampleStratifiedByRank: mockDownsampleStratifiedByRank,
    downsampleByCluster: mockDownsampleByCluster,
  })
  const capturedStudySubscribers: Array<(state: { currentStudy: unknown }) => void> = []
  const capturedSelectionListeners: Array<(indices: Uint32Array) => void> = []
  return {
    mockDownsampleSmart,
    mockDownsampleForThumbnail,
    mockDownsampleStratifiedByRank,
    mockDownsampleByCluster,
    mockGetInstance,
    capturedStudySubscribers,
    capturedSelectionListeners,
  }
})

vi.mock('../wasm/wasmLoader', () => ({
  WasmLoader: {
    getInstance: mockGetInstance,
    reset: vi.fn(),
  },
}))

vi.mock('./studyStore', () => ({
  useStudyStore: {
    getState: vi.fn().mockReturnValue({ currentStudy: null }),
    subscribe: vi.fn().mockImplementation((cb: (state: { currentStudy: unknown }) => void) => {
      capturedStudySubscribers.push(cb)
      return () => {}
    }),
  },
}))

vi.mock('./selectionStore', () => ({
  useSelectionStore: {
    getState: vi.fn().mockReturnValue({ selectedIndices: new Uint32Array(0) }),
    subscribe: vi.fn().mockImplementation(
      (_selector: unknown, listener: (indices: Uint32Array) => void) => {
        capturedSelectionListeners.push(listener)
        return () => {}
      },
    ),
  },
}))

import { useDownsampleStore } from './downsampleStore'

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function resetStore() {
  useDownsampleStore.setState({
    cache: {},
    isComputing: false,
    error: null,
    lastTotalCount: 0,
  })
}

const ALL_KEYS = ['scatter', 'thumbnail', 'hover', 'pcp', 'data_points', 'cluster'] as const

// ---------------------------------------------------------------------------
// TASK-1661 Tests: basic structure + Study subscription
// ---------------------------------------------------------------------------

describe('downsampleStore - basic structure (TASK-1661)', () => {
  beforeEach(() => {
    resetStore()
    vi.clearAllMocks()
    mockGetInstance.mockResolvedValue({
      downsampleSmart: mockDownsampleSmart,
      downsampleForThumbnail: mockDownsampleForThumbnail,
      downsampleStratifiedByRank: mockDownsampleStratifiedByRank,
      downsampleByCluster: mockDownsampleByCluster,
    })
  })

  it('TC-1661-01: initial state has empty cache and isComputing=false', () => {
    const state = useDownsampleStore.getState()
    expect(state.cache).toEqual({})
    expect(state.isComputing).toBe(false)
    expect(state.error).toBeNull()
    expect(state.lastTotalCount).toBe(0)
  })

  it('TC-1661-02: reset() clears cache back to empty', async () => {
    await act(async () => {
      await useDownsampleStore.getState().recompute()
    })
    expect(Object.keys(useDownsampleStore.getState().cache).length).toBeGreaterThan(0)

    useDownsampleStore.getState().reset()
    expect(useDownsampleStore.getState().cache).toEqual({})
    expect(useDownsampleStore.getState().error).toBeNull()
  })

  it('TC-1661-03: recompute() updates all 6 cache keys', async () => {
    await act(async () => {
      await useDownsampleStore.getState().recompute()
    })

    const cache = useDownsampleStore.getState().cache
    for (const key of ALL_KEYS) {
      expect(cache[key]).toBeInstanceOf(Uint32Array)
    }
    expect(useDownsampleStore.getState().isComputing).toBe(false)
  })

  it('TC-1661-04: recompute() calls correct WASM functions for each strategy', async () => {
    await act(async () => {
      await useDownsampleStore.getState().recompute()
    })

    expect(mockDownsampleSmart).toHaveBeenCalledWith(10_000, true)  // scatter
    expect(mockDownsampleSmart).toHaveBeenCalledWith(5_000, false)  // data_points
    expect(mockDownsampleForThumbnail).toHaveBeenCalledWith(500)    // thumbnail
    expect(mockDownsampleForThumbnail).toHaveBeenCalledWith(3_000)  // hover
    expect(mockDownsampleStratifiedByRank).toHaveBeenCalledWith(5_000, 5)
    expect(mockDownsampleByCluster).toHaveBeenCalledWith(10_000)
  })

  it('TC-1661-05: recompute() sets error on WASM failure', async () => {
    mockDownsampleSmart.mockImplementationOnce(() => {
      throw new Error('WASM error')
    })

    await act(async () => {
      await useDownsampleStore.getState().recompute()
    })

    expect(useDownsampleStore.getState().error).toBeTruthy()
    expect(useDownsampleStore.getState().isComputing).toBe(false)
  })

  it('TC-1661-06: study change triggers recompute and updates all cache keys', async () => {
    expect(capturedStudySubscribers.length).toBeGreaterThan(0)
    const subscriber = capturedStudySubscribers[capturedStudySubscribers.length - 1]

    await act(async () => {
      subscriber({ currentStudy: { name: 'study1' } })
    })

    const cache = useDownsampleStore.getState().cache
    for (const key of ALL_KEYS) {
      expect(cache[key]).toBeInstanceOf(Uint32Array)
    }
  })
})

// ---------------------------------------------------------------------------
// TASK-1663 Tests: getIndices fallback and DOWNSAMPLE_CONFIGS routing
// ---------------------------------------------------------------------------

describe('downsampleStore - getIndices (TASK-1663)', () => {
  beforeEach(() => {
    resetStore()
    vi.clearAllMocks()
  })

  it('TC-1663-01: getIndices returns empty Uint32Array as fallback when cache is empty', () => {
    const result = useDownsampleStore.getState().getIndices('scatter')
    expect(result).toBeInstanceOf(Uint32Array)
  })

  it('TC-1663-02: getIndices returns cached value when cache is populated', async () => {
    mockGetInstance.mockResolvedValue({
      downsampleSmart: vi.fn().mockReturnValue({ indices: [0, 5, 10], paretoCount: 1, totalCount: 100, durationMs: 1 }),
      downsampleForThumbnail: mockDownsampleForThumbnail,
      downsampleStratifiedByRank: mockDownsampleStratifiedByRank,
      downsampleByCluster: mockDownsampleByCluster,
    })

    await act(async () => {
      await useDownsampleStore.getState().recompute()
    })

    const result = useDownsampleStore.getState().getIndices('scatter')
    expect(result).toBeInstanceOf(Uint32Array)
    expect(result.length).toBe(3)
    expect(Array.from(result)).toEqual([0, 5, 10])
  })

  it('TC-1663-03: DOWNSAMPLE_CONFIGS has correct maxPoints for all 6 keys', async () => {
    const { DOWNSAMPLE_CONFIGS } = await import('../types/downsampling')
    expect(DOWNSAMPLE_CONFIGS.scatter.maxPoints).toBe(10_000)
    expect(DOWNSAMPLE_CONFIGS.thumbnail.maxPoints).toBe(500)
    expect(DOWNSAMPLE_CONFIGS.hover.maxPoints).toBe(3_000)
    expect(DOWNSAMPLE_CONFIGS.pcp.maxPoints).toBe(5_000)
    expect(DOWNSAMPLE_CONFIGS.data_points.maxPoints).toBe(5_000)
    expect(DOWNSAMPLE_CONFIGS.cluster.maxPoints).toBe(10_000)
  })
})

// ---------------------------------------------------------------------------
// TASK-1662 Tests: filter change detection with ±20% threshold
// ---------------------------------------------------------------------------

describe('downsampleStore - recomputeIfNeeded (TASK-1662)', () => {
  beforeEach(() => {
    resetStore()
    vi.clearAllMocks()
    mockGetInstance.mockResolvedValue({
      downsampleSmart: mockDownsampleSmart,
      downsampleForThumbnail: mockDownsampleForThumbnail,
      downsampleStratifiedByRank: mockDownsampleStratifiedByRank,
      downsampleByCluster: mockDownsampleByCluster,
    })
  })

  it('TC-1662-01: small change (5%) does not trigger recompute', async () => {
    useDownsampleStore.setState({ lastTotalCount: 1000 })

    await act(async () => {
      // 1000 → 1050 = +5% change (below 20% threshold)
      await useDownsampleStore.getState().recomputeIfNeeded(new Uint32Array(1050))
    })

    expect(mockDownsampleSmart).not.toHaveBeenCalled()
  })

  it('TC-1662-02: large change (40%) triggers recompute', async () => {
    useDownsampleStore.setState({ lastTotalCount: 1000 })

    await act(async () => {
      // 1000 → 600 = -40% change (exceeds 20% threshold)
      await useDownsampleStore.getState().recomputeIfNeeded(new Uint32Array(600))
    })

    expect(mockDownsampleSmart).toHaveBeenCalled()
  })

  it('TC-1662-03: exactly 20% change triggers recompute', async () => {
    useDownsampleStore.setState({ lastTotalCount: 1000 })

    await act(async () => {
      // 1000 → 800 = exactly -20% (at threshold → triggers)
      await useDownsampleStore.getState().recomputeIfNeeded(new Uint32Array(800))
    })

    expect(mockDownsampleSmart).toHaveBeenCalled()
  })

  it('TC-1662-04: just below 20% change does not trigger recompute', async () => {
    useDownsampleStore.setState({ lastTotalCount: 1000 })

    await act(async () => {
      // 1000 → 819 ≈ -18.1% (below 20% threshold)
      await useDownsampleStore.getState().recomputeIfNeeded(new Uint32Array(819))
    })

    expect(mockDownsampleSmart).not.toHaveBeenCalled()
  })

  it('TC-1662-05: lastTotalCount=0 triggers recompute for any non-empty input', async () => {
    useDownsampleStore.setState({ lastTotalCount: 0 })

    await act(async () => {
      await useDownsampleStore.getState().recomputeIfNeeded(new Uint32Array(100))
    })

    expect(mockDownsampleSmart).toHaveBeenCalled()
  })

  it('TC-1662-06: selectionStore subscription triggers recomputeIfNeeded', async () => {
    useDownsampleStore.setState({ lastTotalCount: 1000 })
    expect(capturedSelectionListeners.length).toBeGreaterThan(0)
    const listener = capturedSelectionListeners[capturedSelectionListeners.length - 1]

    await act(async () => {
      // -40% change → should trigger recompute
      listener(new Uint32Array(600))
    })

    expect(mockDownsampleSmart).toHaveBeenCalled()
  })
})
