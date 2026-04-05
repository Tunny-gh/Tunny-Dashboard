import { describe, it, expect, vi, beforeEach } from 'vitest'

// ---------------------------------------------------------------------------
// Hoisted mocks
// ---------------------------------------------------------------------------

const { mockComputeTopsis, mockGetTrials, mockGetInstance, capturedStudySubscribers } = vi.hoisted(
  () => {
    const mockComputeTopsis = vi.fn().mockReturnValue({
      scores: [0.8, 0.3],
      rankedIndices: [0, 1],
      positiveIdeal: [1.0, 2.0],
      negativeIdeal: [5.0, 6.0],
      durationMs: 1.5,
    })
    const mockGetTrials = vi.fn().mockReturnValue([
      { trialId: 0, params: {}, values: [1.0, 2.0], paretoRank: null },
      { trialId: 1, params: {}, values: [5.0, 6.0], paretoRank: null },
    ])
    const mockGetInstance = vi.fn().mockResolvedValue({
      computeTopsis: mockComputeTopsis,
      getTrials: mockGetTrials,
    })
    const capturedStudySubscribers: Array<(state: { currentStudy: unknown }) => void> = []
    return { mockComputeTopsis, mockGetTrials, mockGetInstance, capturedStudySubscribers }
  },
)

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

import { useMcdmStore } from './mcdmStore'

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const mockStudy = {
  studyId: 1,
  name: 'test',
  directions: ['minimize', 'minimize'] as const,
  completedTrials: 2,
  totalTrials: 2,
  paramNames: ['x'],
  objectiveNames: ['obj0', 'obj1'],
  userAttrNames: [],
  hasConstraints: false,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('mcdmStore', () => {
  beforeEach(() => {
    // English comment.
    useMcdmStore.getState().reset()
    mockComputeTopsis.mockClear()
    mockGetTrials.mockClear()
    mockGetInstance.mockClear()
  })

  // English comment.
  it('TC-1621-01: setTopsisWeights normalizes weights to sum=1.0', () => {
    // English comment.
    // English comment.
    useMcdmStore.getState().setTopsisWeights([2, 3])

    // English comment.
    const weights = useMcdmStore.getState().topsisWeights
    expect(weights).toHaveLength(2)
    expect(weights[0]).toBeCloseTo(0.4)
    expect(weights[1]).toBeCloseTo(0.6)
  })

  // English comment.
  it('TC-1621-02: setTopsisWeights with equal values normalizes correctly', () => {
    // English comment.
    useMcdmStore.getState().setTopsisWeights([1, 1, 1])
    const weights = useMcdmStore.getState().topsisWeights
    expect(weights[0]).toBeCloseTo(1 / 3)
    expect(weights[1]).toBeCloseTo(1 / 3)
    expect(weights[2]).toBeCloseTo(1 / 3)
  })

  // English comment.
  it('TC-1621-03: reset clears topsisResult and topsisWeights', () => {
    // English comment.
    useMcdmStore.setState({
      topsisResult: {
        scores: [0.8, 0.3],
        rankedIndices: [0, 1],
        positiveIdeal: [1.0],
        negativeIdeal: [5.0],
        durationMs: 1.0,
      },
      topsisWeights: [0.5, 0.5],
      topsisError: 'some error',
    })

    useMcdmStore.getState().reset()

    // English comment.
    const state = useMcdmStore.getState()
    expect(state.topsisResult).toBeNull()
    expect(state.topsisWeights).toEqual([])
    expect(state.topsisError).toBeNull()
  })

  // English comment.
  it('TC-1621-04: setTopN updates topN value', () => {
    // English comment.
    useMcdmStore.getState().setTopN(20)
    expect(useMcdmStore.getState().topN).toBe(20)
  })

  // English comment.
  it('TC-1621-05: computeTopsis returns early when no study selected', async () => {
    // English comment.
    const { useStudyStore } = await import('./studyStore')
    vi.mocked(useStudyStore.getState).mockReturnValue({ currentStudy: null } as ReturnType<
      typeof useStudyStore.getState
    >)

    await useMcdmStore.getState().computeTopsis()

    // English comment.
    expect(mockGetInstance).not.toHaveBeenCalled()
    expect(useMcdmStore.getState().isComputing).toBe(false)
  })

  // English comment.
  it('TC-1621-06: computeTopsis calls WasmLoader and sets topsisResult', async () => {
    // English comment.
    const { useStudyStore } = await import('./studyStore')
    vi.mocked(useStudyStore.getState).mockReturnValue({
      currentStudy: mockStudy,
    } as ReturnType<typeof useStudyStore.getState>)

    await useMcdmStore.getState().computeTopsis()

    // English comment.
    expect(mockComputeTopsis).toHaveBeenCalledTimes(1)

    // English comment.
    const result = useMcdmStore.getState().topsisResult
    expect(result).not.toBeNull()
    expect(result?.scores).toHaveLength(2)
    expect(result?.rankedIndices).toHaveLength(2)
    expect(useMcdmStore.getState().isComputing).toBe(false)
  })

  // English comment.
  it('TC-1621-07: resets when study changes', () => {
    // English comment.
    useMcdmStore.setState({
      topsisResult: {
        scores: [0.8],
        rankedIndices: [0],
        positiveIdeal: [1.0],
        negativeIdeal: [5.0],
        durationMs: 1.0,
      },
    })

    // English comment.
    capturedStudySubscribers.forEach((cb) => cb({ currentStudy: { ...mockStudy, studyId: 999 } }))

    // English comment.
    expect(useMcdmStore.getState().topsisResult).toBeNull()
  })
})
