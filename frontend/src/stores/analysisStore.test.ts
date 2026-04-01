import { describe, it, expect, vi, beforeEach } from 'vitest'
import { act } from 'react'

// ---------------------------------------------------------------------------
// Hoisted mocks
// ---------------------------------------------------------------------------

const {
  mockComputeSensitivity,
  mockComputeSensitivitySelected,
  mockGetInstance,
  capturedStudySubscribers,
} = vi.hoisted(() => {
  const mockComputeSensitivity = vi.fn()
  const mockComputeSensitivitySelected = vi.fn()
  const mockGetInstance = vi.fn().mockResolvedValue({
    computeSensitivity: mockComputeSensitivity,
    computeSensitivitySelected: mockComputeSensitivitySelected,
  })
  const capturedStudySubscribers: Array<(state: { currentStudy: unknown }) => void> = []
  return {
    mockComputeSensitivity,
    mockComputeSensitivitySelected,
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
