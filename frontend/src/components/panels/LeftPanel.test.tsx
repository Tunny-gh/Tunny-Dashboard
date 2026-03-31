/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest'
import { render, screen, cleanup, fireEvent } from '@testing-library/react'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

const { mockAddAxisFilter, mockSetColorMode, mockRemoveAxisFilter, mockRunClustering, mockUseClusterStore } = vi.hoisted(() => {
  const mockAddAxisFilter = vi.fn()
  const mockSetColorMode = vi.fn()
  const mockRemoveAxisFilter = vi.fn()
  const mockRunClustering = vi.fn()
  const mockUseClusterStore = vi.fn().mockReturnValue({
    runClustering: mockRunClustering,
    isRunning: false,
    elbowResult: null,
    clusterError: null,
  })
  return { mockAddAxisFilter, mockSetColorMode, mockRemoveAxisFilter, mockRunClustering, mockUseClusterStore }
})

vi.mock('../../stores/clusterStore', () => ({
  useClusterStore: mockUseClusterStore,
}))

vi.mock('../../stores/selectionStore', () => ({
  useSelectionStore: vi
    .fn()
    .mockImplementation(
      (
        selector: (s: {
          selectedIndices: Uint32Array
          colorMode: string
          addAxisFilter: typeof mockAddAxisFilter
          removeAxisFilter: typeof mockRemoveAxisFilter
          setColorMode: typeof mockSetColorMode
        }) => unknown,
      ) =>
        selector({
          selectedIndices: new Uint32Array([0, 1, 2]),
          colorMode: 'Viridis',
          addAxisFilter: mockAddAxisFilter,
          removeAxisFilter: mockRemoveAxisFilter,
          setColorMode: mockSetColorMode,
        }),
    ),
}))

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

vi.mock('../../stores/studyStore', () => ({
  useStudyStore: vi.fn().mockImplementation(
    (
      selector: (s: {
        currentStudy: {
          paramNames: string[]
          objectiveNames: string[]
          completedTrials: number
        } | null
        trialRows: Array<{
          trialId: number
          params: Record<string, number>
          values: number[]
          paretoRank: number | null
        }>
      }) => unknown,
    ) =>
      selector({
        currentStudy: {
          paramNames: ['x1', 'x2'],
          objectiveNames: ['obj1'],
          completedTrials: 10,
        },
        trialRows: [
          { trialId: 0, params: { x1: 0.0, x2: -5.0 }, values: [1.0], paretoRank: null },
          { trialId: 1, params: { x1: 10.0, x2: 5.0 }, values: [2.0], paretoRank: null },
          { trialId: 2, params: { x1: 5.0, x2: 0.0 }, values: [3.0], paretoRank: null },
        ],
      }),
  ),
}))

import { LeftPanel } from './LeftPanel'
import { useStudyStore } from '../../stores/studyStore'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    cleanup()
  })

  // Documentation.
  test('TC-402-01', () => {
    // Documentation.
    expect(() => render(<LeftPanel />)).not.toThrow()
  })

  // Documentation.
  test('TC-402-02', () => {
    // Documentation.
    render(<LeftPanel />)

    // Documentation.
    expect(screen.getByTestId('selected-count')).toHaveTextContent('3')
  })

  // Documentation.
  test('TC-402-03', async () => {
    // Documentation.
    vi.useFakeTimers()
    render(<LeftPanel />)

    // Documentation.
    const slider = screen.getByTestId('slider-hi-x1')
    fireEvent.change(slider, { target: { value: '5' } })

    // Documentation.
    vi.advanceTimersByTime(200)
    expect(mockAddAxisFilter).toHaveBeenCalled()
    vi.useRealTimers()
  })

  // Documentation.
  test('TC-402-04', () => {
    // Documentation.
    render(<LeftPanel />)

    // Documentation.
    const select = screen.getByTestId('colormap-select')
    fireEvent.change(select, { target: { value: 'Plasma' } })

    // Documentation.
    expect(mockSetColorMode).toHaveBeenCalledWith('Plasma')
  })
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  afterEach(() => {
    cleanup()
  })

  // Documentation.
  test('TC-402-E01', () => {
    // Documentation.
    ;(useStudyStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (s: { currentStudy: null; trialRows: [] }) => unknown) =>
        selector({ currentStudy: null, trialRows: [] }),
    )

    render(<LeftPanel />)
    expect(screen.getByText('Data not loaded')).toBeInTheDocument()
  })
})

// ---------------------------------------------------------------------------
// TC-1609: LeftPanel → clusterStore wiring
// ---------------------------------------------------------------------------

describe('TC-1609: LeftPanel → clusterStore wiring', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    // Restore studyStore mock with non-null currentStudy (TC-402-E01 may have overridden it)
    ;(useStudyStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (s: {
        currentStudy: { paramNames: string[]; objectiveNames: string[]; completedTrials: number } | null
        trialRows: Array<{ trialId: number; params: Record<string, number>; values: number[]; paretoRank: number | null }>
      }) => unknown) =>
        selector({
          currentStudy: { paramNames: ['x1', 'x2'], objectiveNames: ['obj1'], completedTrials: 10 },
          trialRows: [
            { trialId: 0, params: { x1: 0.0, x2: -5.0 }, values: [1.0], paretoRank: null },
            { trialId: 1, params: { x1: 10.0, x2: 5.0 }, values: [2.0], paretoRank: null },
          ],
        }),
    )
    mockUseClusterStore.mockReturnValue({
      runClustering: mockRunClustering,
      isRunning: false,
      elbowResult: null,
      clusterError: null,
    })
  })

  afterEach(() => {
    cleanup()
  })

  test('TC-1609-01: onRunClustering calls clusterStore.runClustering', () => {
    render(<LeftPanel />)

    const runBtn = screen.getByTestId('run-clustering-btn')
    fireEvent.click(runBtn)

    expect(mockRunClustering).toHaveBeenCalledWith('param', 4)
  })

  test('TC-1609-02: clusterStore.isRunning=true disables run button', () => {
    mockUseClusterStore.mockReturnValue({
      runClustering: mockRunClustering,
      isRunning: true,
      elbowResult: null,
      clusterError: null,
    })

    render(<LeftPanel />)

    const runBtn = screen.getByTestId('run-clustering-btn')
    expect(runBtn).toBeDisabled()
  })
})
