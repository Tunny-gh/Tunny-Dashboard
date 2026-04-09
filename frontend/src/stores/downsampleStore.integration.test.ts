/**
 * Integration tests for downsampleStore
 * TASK-1672: Study切り替え・フィルタ変更フロー確認
 *
 * Tests the end-to-end flow:
 *   Flow 1: Study change → recompute → all 6 cache keys updated
 *   Flow 2: Filter change (±20% threshold) → conditional recompute
 *   Flow 3: WASM error → error set, getIndices fallback
 */

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
  const mockResult = { indices: [0, 1, 2, 3, 4], paretoCount: 2, totalCount: 1000, durationMs: 1 }
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

const ALL_KEYS = ['scatter', 'thumbnail', 'hover', 'pcp', 'data_points', 'cluster'] as const

function resetStore() {
  useDownsampleStore.setState({
    cache: {},
    isComputing: false,
    error: null,
    lastTotalCount: 0,
  })
}

// ---------------------------------------------------------------------------
// Flow 1: Study change → recompute → all 6 cache keys updated
// ---------------------------------------------------------------------------

describe('Integration: Study change flow (Flow 1)', () => {
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

  it('TC-1672-01: recompute populates all 6 cache keys', async () => {
    await act(async () => {
      await useDownsampleStore.getState().recompute()
    })

    const cache = useDownsampleStore.getState().cache
    for (const key of ALL_KEYS) {
      expect(cache[key]).toBeInstanceOf(Uint32Array)
      expect(cache[key]!.length).toBeGreaterThan(0)
    }
    expect(useDownsampleStore.getState().isComputing).toBe(false)
  })

  it('TC-1672-02: study subscriber triggers recompute and updates all cache keys', async () => {
    expect(capturedStudySubscribers.length).toBeGreaterThan(0)
    const subscriber = capturedStudySubscribers[capturedStudySubscribers.length - 1]

    await act(async () => {
      subscriber({ currentStudy: { name: 'new-study', studyId: 2 } })
    })

    const cache = useDownsampleStore.getState().cache
    for (const key of ALL_KEYS) {
      expect(cache[key]).toBeInstanceOf(Uint32Array)
    }
  })
})

// ---------------------------------------------------------------------------
// Flow 2: Filter change with ±20% threshold
// ---------------------------------------------------------------------------

describe('Integration: Filter change flow (Flow 2)', () => {
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

  it('TC-1672-03: filter change exceeding 20% triggers recompute', async () => {
    useDownsampleStore.setState({ lastTotalCount: 10_000 })
    expect(capturedSelectionListeners.length).toBeGreaterThan(0)
    const listener = capturedSelectionListeners[capturedSelectionListeners.length - 1]

    await act(async () => {
      // 10000 → 6000 = -40% → exceeds threshold → should recompute
      listener(new Uint32Array(6_000))
    })

    expect(mockDownsampleSmart).toHaveBeenCalled()
    const cache = useDownsampleStore.getState().cache
    for (const key of ALL_KEYS) {
      expect(cache[key]).toBeInstanceOf(Uint32Array)
    }
  })

  it('TC-1672-04: filter change within 20% does not trigger recompute', async () => {
    // Pre-populate cache
    await act(async () => {
      await useDownsampleStore.getState().recompute()
    })
    vi.clearAllMocks()

    useDownsampleStore.setState({ lastTotalCount: 10_000 })
    const listener = capturedSelectionListeners[capturedSelectionListeners.length - 1]

    await act(async () => {
      // 10000 → 9500 = -5% → below threshold → should NOT recompute
      listener(new Uint32Array(9_500))
    })

    expect(mockDownsampleSmart).not.toHaveBeenCalled()
  })
})

// ---------------------------------------------------------------------------
// Flow 3: WASM error → fallback
// ---------------------------------------------------------------------------

describe('Integration: WASM error fallback (Flow 3)', () => {
  beforeEach(() => {
    resetStore()
    vi.clearAllMocks()
  })

  it('TC-1672-05: WASM error sets error state and getIndices returns Uint32Array', async () => {
    mockDownsampleSmart.mockImplementationOnce(() => {
      throw new Error('WASM initialization failed')
    })
    mockGetInstance.mockResolvedValue({
      downsampleSmart: mockDownsampleSmart,
      downsampleForThumbnail: mockDownsampleForThumbnail,
      downsampleStratifiedByRank: mockDownsampleStratifiedByRank,
      downsampleByCluster: mockDownsampleByCluster,
    })

    await act(async () => {
      await useDownsampleStore.getState().recompute()
    })

    // Error should be set
    expect(useDownsampleStore.getState().error).toBeTruthy()
    expect(useDownsampleStore.getState().isComputing).toBe(false)

    // getIndices should return a Uint32Array (not throw)
    for (const key of ALL_KEYS) {
      const result = useDownsampleStore.getState().getIndices(key)
      expect(result).toBeInstanceOf(Uint32Array)
    }
  })

  it('TC-1672-06: after error, reset() clears the error', async () => {
    mockDownsampleSmart.mockImplementationOnce(() => {
      throw new Error('WASM error')
    })
    mockGetInstance.mockResolvedValue({
      downsampleSmart: mockDownsampleSmart,
      downsampleForThumbnail: mockDownsampleForThumbnail,
      downsampleStratifiedByRank: mockDownsampleStratifiedByRank,
      downsampleByCluster: mockDownsampleByCluster,
    })

    await act(async () => {
      await useDownsampleStore.getState().recompute()
    })

    expect(useDownsampleStore.getState().error).toBeTruthy()

    useDownsampleStore.getState().reset()

    expect(useDownsampleStore.getState().error).toBeNull()
    expect(useDownsampleStore.getState().cache).toEqual({})
  })
})
