import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, cleanup, fireEvent } from '@testing-library/react'

// ---------------------------------------------------------------------------
// Mocks
// ---------------------------------------------------------------------------

vi.mock('echarts-for-react')

const {
  mockComputeTopsis,
  mockSetTopsisWeights,
  mockSetTopN,
  mockUseMcdmStore,
  mockUseStudyStore,
  mockSetHighlight,
} = vi.hoisted(() => {
  const mockComputeTopsis = vi.fn().mockResolvedValue(undefined)
  const mockSetTopsisWeights = vi.fn()
  const mockSetTopN = vi.fn()
  const mockSetHighlight = vi.fn()
  const mockUseMcdmStore = vi.fn()
  const mockUseStudyStore = vi.fn()
  return {
    mockComputeTopsis,
    mockSetTopsisWeights,
    mockSetTopN,
    mockUseMcdmStore,
    mockUseStudyStore,
    mockSetHighlight,
  }
})

vi.mock('../../stores/mcdmStore', () => ({
  useMcdmStore: mockUseMcdmStore,
}))

vi.mock('../../stores/studyStore', () => ({
  useStudyStore: mockUseStudyStore,
}))

vi.mock('../../stores/selectionStore', () => ({
  useSelectionStore: vi.fn((selector: (s: { setHighlight: typeof mockSetHighlight }) => unknown) =>
    selector({ setHighlight: mockSetHighlight }),
  ),
}))

import { TopsisRankingChart } from './TopsisRankingChart'

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const mockStudy = {
  studyId: 1,
  name: 'test',
  directions: ['minimize', 'minimize'],
  completedTrials: 3,
  totalTrials: 3,
  paramNames: ['x'],
  objectiveNames: ['obj0', 'obj1'],
  userAttrNames: [],
  hasConstraints: false,
}

const mockTopsisResult = {
  scores: [0.8, 0.5, 0.3],
  rankedIndices: [0, 1, 2],
  positiveIdeal: [1.0, 2.0],
  negativeIdeal: [5.0, 6.0],
  durationMs: 1.0,
}

const defaultMcdmState = {
  topsisResult: mockTopsisResult,
  topsisWeights: [0.5, 0.5],
  isComputing: false,
  topsisError: null,
  topN: 10,
  computeTopsis: mockComputeTopsis,
  setTopsisWeights: mockSetTopsisWeights,
  setTopN: mockSetTopN,
  reset: vi.fn(),
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('TopsisRankingChart', () => {
  beforeEach(() => {
    cleanup()
    mockComputeTopsis.mockClear()
    mockSetTopsisWeights.mockClear()
    mockSetTopN.mockClear()
    mockSetHighlight.mockClear()
  })

  // English comment.
  it('TC-1622-01: shows EmptyState when no study selected', () => {
    // English comment.
    mockUseStudyStore.mockImplementation((selector: (s: { currentStudy: null }) => unknown) =>
      selector({ currentStudy: null }),
    )
    mockUseMcdmStore.mockReturnValue(defaultMcdmState)

    render(<TopsisRankingChart />)

    // English comment.
    expect(screen.getByTestId('empty-state')).toBeDefined()
  })

  // English comment.
  it('TC-1622-02: shows EmptyState when only 1 objective', () => {
    // English comment.
    mockUseStudyStore.mockImplementation(
      (selector: (s: { currentStudy: typeof mockStudy }) => unknown) =>
        selector({ currentStudy: { ...mockStudy, objectiveNames: ['obj0'] } }),
    )
    mockUseMcdmStore.mockReturnValue(defaultMcdmState)

    render(<TopsisRankingChart />)
    expect(screen.getByTestId('empty-state')).toBeDefined()
  })

  // English comment.
  it('TC-1622-03: shows loading state when isComputing=true', () => {
    // English comment.
    mockUseStudyStore.mockImplementation(
      (selector: (s: { currentStudy: typeof mockStudy }) => unknown) =>
        selector({ currentStudy: mockStudy }),
    )
    mockUseMcdmStore.mockReturnValue({
      ...defaultMcdmState,
      topsisResult: null,
      isComputing: true,
    })

    render(<TopsisRankingChart />)
    expect(screen.getByText('Computing...')).toBeDefined()
  })

  // English comment.
  it('TC-1622-04: shows EmptyState when topsisError is set', () => {
    // English comment.
    mockUseStudyStore.mockImplementation(
      (selector: (s: { currentStudy: typeof mockStudy }) => unknown) =>
        selector({ currentStudy: mockStudy }),
    )
    mockUseMcdmStore.mockReturnValue({
      ...defaultMcdmState,
      topsisResult: null,
      topsisError: 'computation failed',
    })

    render(<TopsisRankingChart />)
    expect(screen.getByTestId('empty-state')).toBeDefined()
  })

  // English comment.
  it('TC-1622-05: renders weight sliders for each objective', () => {
    // English comment.
    mockUseStudyStore.mockImplementation(
      (selector: (s: { currentStudy: typeof mockStudy }) => unknown) =>
        selector({ currentStudy: mockStudy }),
    )
    mockUseMcdmStore.mockReturnValue(defaultMcdmState)

    render(<TopsisRankingChart />)

    // English comment.
    const sliders = document.querySelectorAll('input[type="range"]')
    expect(sliders.length).toBe(2)
  })

  // English comment.
  it('TC-1622-06: calls setTopsisWeights when slider changes', () => {
    // English comment.
    mockUseStudyStore.mockImplementation(
      (selector: (s: { currentStudy: typeof mockStudy }) => unknown) =>
        selector({ currentStudy: mockStudy }),
    )
    mockUseMcdmStore.mockReturnValue(defaultMcdmState)

    render(<TopsisRankingChart />)

    const sliders = document.querySelectorAll('input[type="range"]')
    fireEvent.change(sliders[0], { target: { value: '0.7' } })

    // English comment.
    expect(mockSetTopsisWeights).toHaveBeenCalled()
  })

  // English comment.
  it('TC-1622-07: renders topN selector', () => {
    // English comment.
    mockUseStudyStore.mockImplementation(
      (selector: (s: { currentStudy: typeof mockStudy }) => unknown) =>
        selector({ currentStudy: mockStudy }),
    )
    mockUseMcdmStore.mockReturnValue(defaultMcdmState)

    render(<TopsisRankingChart />)

    // English comment.
    const select = document.querySelector('select')
    expect(select).not.toBeNull()
  })
})
