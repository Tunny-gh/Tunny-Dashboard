import { describe, it, expect, vi, beforeEach } from 'vitest'
import { act } from 'react'

// ---------------------------------------------------------------------------
// Hoisted mocks
// ---------------------------------------------------------------------------

const {
  mockComputeSensitivity,
  mockComputeSensitivitySelected,
  mockComputePdp2d,
  mockGetInstance,
  capturedStudySubscribers,
} = vi.hoisted(() => {
  const mockComputeSensitivity = vi.fn()
  const mockComputeSensitivitySelected = vi.fn()
  const mockComputePdp2d = vi.fn().mockReturnValue({
    param1Name: 'x',
    param2Name: 'y',
    objectiveName: 'obj0',
    grid1: [1.0, 2.0, 3.0],
    grid2: [1.0, 2.0, 3.0],
    values: [
      [0.1, 0.2, 0.3],
      [0.4, 0.5, 0.6],
      [0.7, 0.8, 0.9],
    ],
    rSquared: 0.95,
  })
  const mockGetInstance = vi.fn().mockResolvedValue({
    computeSensitivity: mockComputeSensitivity,
    computeSensitivitySelected: mockComputeSensitivitySelected,
    computePdp2d: mockComputePdp2d,
  })
  const capturedStudySubscribers: Array<(state: { currentStudy: unknown }) => void> = []
  return {
    mockComputeSensitivity,
    mockComputeSensitivitySelected,
    mockComputePdp2d,
    mockGetInstance,
    capturedStudySubscribers,
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

import { useAnalysisStore } from './analysisStore'

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const mockSensitivityResult = {
  spearman: [[0.5, 0.3]],
  ridge: [{ beta: [0.3], rSquared: 0.8 }],
  paramNames: ['x'],
  objectiveNames: ['y'],
  durationMs: 10,
}

function resetStore() {
  useAnalysisStore.setState({
    sensitivityResult: null,
    isComputingSensitivity: false,
    sensitivityError: null,
  })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('analysisStore', () => {
  beforeEach(() => {
    resetStore()
    vi.clearAllMocks()
    mockGetInstance.mockResolvedValue({
      computeSensitivity: mockComputeSensitivity,
      computeSensitivitySelected: mockComputeSensitivitySelected,
      computePdp2d: mockComputePdp2d,
    })
  })

  it('TC-1603-01: initial state is correct', () => {
    const state = useAnalysisStore.getState()
    expect(state.sensitivityResult).toBeNull()
    expect(state.isComputingSensitivity).toBe(false)
    expect(state.sensitivityError).toBeNull()
  })

  it('TC-1603-02: computeSensitivity sets sensitivityResult on success', async () => {
    mockComputeSensitivity.mockReturnValue(mockSensitivityResult)

    await act(async () => {
      await useAnalysisStore.getState().computeSensitivity()
    })

    expect(useAnalysisStore.getState().sensitivityResult).toEqual(mockSensitivityResult)
    expect(useAnalysisStore.getState().isComputingSensitivity).toBe(false)
    expect(useAnalysisStore.getState().sensitivityError).toBeNull()
  })

  it('TC-1603-03: computeSensitivity sets isComputingSensitivity=true during execution', async () => {
    let resolveWasm!: (value: unknown) => void
    mockGetInstance.mockReturnValue(
      new Promise((resolve) => {
        resolveWasm = resolve
      }),
    )

    const promise = useAnalysisStore.getState().computeSensitivity()
    expect(useAnalysisStore.getState().isComputingSensitivity).toBe(true)

    resolveWasm({ computeSensitivity: () => mockSensitivityResult })
    await act(async () => {
      await promise
    })
    expect(useAnalysisStore.getState().isComputingSensitivity).toBe(false)
  })

  it('TC-1603-04: computeSensitivity sets sensitivityError on failure', async () => {
    mockComputeSensitivity.mockImplementation(() => {
      throw new Error('No active study')
    })

    await act(async () => {
      await useAnalysisStore.getState().computeSensitivity()
    })

    expect(useAnalysisStore.getState().sensitivityError).toBe('No active study')
    expect(useAnalysisStore.getState().isComputingSensitivity).toBe(false)
    expect(useAnalysisStore.getState().sensitivityResult).toBeNull()
  })

  it('TC-1603-05: computeSensitivitySelected sets sensitivityResult on success', async () => {
    const indices = new Uint32Array([0, 1, 2])
    mockComputeSensitivitySelected.mockReturnValue(mockSensitivityResult)

    await act(async () => {
      await useAnalysisStore.getState().computeSensitivitySelected(indices)
    })

    expect(useAnalysisStore.getState().sensitivityResult).toEqual(mockSensitivityResult)
    expect(mockComputeSensitivitySelected).toHaveBeenCalledWith(indices)
  })

  it('TC-1603-06: study change resets sensitivityResult and sensitivityError', () => {
    useAnalysisStore.setState({
      sensitivityResult: mockSensitivityResult,
      sensitivityError: 'old error',
      isComputingSensitivity: false,
    })

    // Trigger the study change by calling the captured subscriber
    expect(capturedStudySubscribers.length).toBeGreaterThan(0)
    const subscriber = capturedStudySubscribers[capturedStudySubscribers.length - 1]
    subscriber({ currentStudy: { name: 'study2' } as unknown })

    expect(useAnalysisStore.getState().sensitivityResult).toBeNull()
    expect(useAnalysisStore.getState().sensitivityError).toBeNull()
    expect(useAnalysisStore.getState().isComputingSensitivity).toBe(false)
  })
})

// ---------------------------------------------------------------------------
// surface3d tests (TASK-1627)
// ---------------------------------------------------------------------------

describe('analysisStore - surface3d', () => {
  beforeEach(() => {
    useAnalysisStore.setState({
      surrogateModelType: 'ridge',
      surface3dCache: new Map(),
      isComputingSurface: false,
      surface3dError: null,
    })
    vi.clearAllMocks()
    mockComputePdp2d.mockReturnValue({
      param1Name: 'x',
      param2Name: 'y',
      objectiveName: 'obj0',
      grid1: [1.0, 2.0, 3.0],
      grid2: [1.0, 2.0, 3.0],
      values: [
        [0.1, 0.2, 0.3],
        [0.4, 0.5, 0.6],
        [0.7, 0.8, 0.9],
      ],
      rSquared: 0.95,
    })
    mockGetInstance.mockResolvedValue({
      computeSensitivity: mockComputeSensitivity,
      computeSensitivitySelected: mockComputeSensitivitySelected,
      computePdp2d: mockComputePdp2d,
    })
  })

  // TC-1627-01: setSurrogateModelType
  it('TC-1627-01: setSurrogateModelType updates surrogateModelType', () => {
    // English comment.
    useAnalysisStore.getState().setSurrogateModelType('random_forest')
    expect(useAnalysisStore.getState().surrogateModelType).toBe('random_forest')
  })

  // English comment.
  it('TC-1627-02: computeSurface3d calls WasmLoader on cache miss', async () => {
    // English comment.
    await act(async () => {
      await useAnalysisStore.getState().computeSurface3d('x', 'y', 'obj0', 10)
    })

    expect(mockComputePdp2d).toHaveBeenCalledTimes(1)
    expect(mockComputePdp2d).toHaveBeenCalledWith('x', 'y', 'obj0', 10)

    // English comment.
    const cache = useAnalysisStore.getState().surface3dCache
    expect(cache.size).toBe(1)
  })

  // English comment.
  it('TC-1627-03: computeSurface3d skips WasmLoader on cache hit', async () => {
    // English comment.
    await act(async () => {
      await useAnalysisStore.getState().computeSurface3d('x', 'y', 'obj0', 10)
    })
    mockComputePdp2d.mockClear()

    // English comment.
    await act(async () => {
      await useAnalysisStore.getState().computeSurface3d('x', 'y', 'obj0', 10)
    })

    // English comment.
    expect(mockComputePdp2d).not.toHaveBeenCalled()
  })

  // English comment.
  it('TC-1627-04: computeSurface3d sets surface3dError on failure', async () => {
    // English comment.
    mockComputePdp2d.mockImplementation(() => {
      throw new Error('computation failed')
    })

    await act(async () => {
      await useAnalysisStore.getState().computeSurface3d('x', 'y', 'obj0', 10)
    })

    expect(useAnalysisStore.getState().surface3dError).toBe('computation failed')
    expect(useAnalysisStore.getState().isComputingSurface).toBe(false)
  })

  // TC-1627-05: clearSurface3dCache
  it('TC-1627-05: clearSurface3dCache clears the cache', async () => {
    // English comment.
    await act(async () => {
      await useAnalysisStore.getState().computeSurface3d('x', 'y', 'obj0', 10)
    })
    expect(useAnalysisStore.getState().surface3dCache.size).toBe(1)

    useAnalysisStore.getState().clearSurface3dCache()
    expect(useAnalysisStore.getState().surface3dCache.size).toBe(0)
  })
})
