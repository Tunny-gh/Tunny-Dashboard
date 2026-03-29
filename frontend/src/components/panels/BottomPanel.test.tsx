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

const { mockSetHighlight } = vi.hoisted(() => {
  const mockSetHighlight = vi.fn()
  return { mockSetHighlight }
})

vi.mock('../../stores/selectionStore', () => ({
  useSelectionStore: vi
    .fn()
    .mockImplementation(
      (
        selector: (s: {
          selectedIndices: Uint32Array
          highlighted: number | null
          setHighlight: typeof mockSetHighlight
        }) => unknown,
      ) =>
        selector({
          selectedIndices: new Uint32Array([0, 1, 2]),
          highlighted: null,
          setHighlight: mockSetHighlight,
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
          directions?: string[]
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
          paramNames: ['x1'],
          objectiveNames: ['obj1'],
        },
        trialRows: [
          { trialId: 0, params: { x1: 1.5 }, values: [0.25], paretoRank: null },
          { trialId: 1, params: { x1: 2.5 }, values: [0.5], paretoRank: null },
          { trialId: 2, params: { x1: 3.5 }, values: [0.75], paretoRank: null },
        ],
      }),
  ),
}))

import { BottomPanel } from './BottomPanel'
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
  test('TC-402-05', () => {
    // Documentation.
    expect(() => render(<BottomPanel />)).not.toThrow()
  })

  // Documentation.
  test('TC-402-06', () => {
    // Documentation.
    render(<BottomPanel />)
    expect(screen.getByText('trial_id')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-402-07', () => {
    // Documentation.
    render(<BottomPanel />)

    // Documentation.
    const firstRow = screen.getByTestId('trial-row-0')
    fireEvent.click(firstRow)

    // Documentation.
    expect(mockSetHighlight).toHaveBeenCalledWith(0)
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
  test('TC-402-E02', () => {
    // Documentation.
    ;(useStudyStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (s: { currentStudy: null }) => unknown) => selector({ currentStudy: null }),
    )

    render(<BottomPanel />)
    expect(screen.getByText('Data not loaded')).toBeInTheDocument()
  })
})
