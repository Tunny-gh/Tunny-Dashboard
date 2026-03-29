/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest'
import { render, screen, cleanup, fireEvent } from '@testing-library/react'

import { ScatterMatrix } from './ScatterMatrix'
import type { Study, TrialData } from '../../types'

// -------------------------------------------------------------------------
// Test helpers
// -------------------------------------------------------------------------

function makeStudy(): Study {
  return {
    studyId: 1,
    name: 'test-study',
    directions: ['minimize', 'minimize'],
    completedTrials: 5,
    totalTrials: 5,
    paramNames: ['x1', 'x2'],
    objectiveNames: ['f1', 'f2'],
    userAttrNames: [],
    hasConstraints: false,
  }
}

function makeTrialRows(): TrialData[] {
  return [
    { trialId: 0, params: { x1: 1, x2: 2 }, values: [10, 20], paretoRank: null },
    { trialId: 1, params: { x1: 3, x2: 4 }, values: [30, 40], paretoRank: null },
    { trialId: 2, params: { x1: 5, x2: 6 }, values: [50, 60], paretoRank: null },
  ]
}

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
  test('TC-702-01', () => {
    // Documentation.
    expect(() => render(<ScatterMatrix trialRows={makeTrialRows()} currentStudy={makeStudy()} />)).not.toThrow()
  })

  // Documentation.
  test('TC-702-02', () => {
    // Documentation.
    render(<ScatterMatrix trialRows={makeTrialRows()} currentStudy={makeStudy()} />)

    expect(screen.getByTestId('mode-btn-mode1')).toBeInTheDocument()
    expect(screen.getByTestId('mode-btn-mode2')).toBeInTheDocument()
    expect(screen.getByTestId('mode-btn-mode3')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-702-03', () => {
    // Documentation.
    render(<ScatterMatrix trialRows={makeTrialRows()} currentStudy={makeStudy()} />)

    expect(screen.getByTestId('mode-btn-mode1')).toHaveAttribute('aria-pressed', 'true')
    expect(screen.getByTestId('mode-btn-mode2')).toHaveAttribute('aria-pressed', 'false')

    // Documentation.
    fireEvent.click(screen.getByTestId('mode-btn-mode2'))

    // Documentation.
    expect(screen.getByTestId('mode-btn-mode2')).toHaveAttribute('aria-pressed', 'true')
    expect(screen.getByTestId('mode-btn-mode1')).toHaveAttribute('aria-pressed', 'false')
  })

  // Documentation.
  test('TC-702-04', () => {
    // Documentation.
    render(<ScatterMatrix trialRows={makeTrialRows()} currentStudy={makeStudy()} />)

    const sortSelect = screen.getByTestId('sort-select')
    expect(sortSelect).toBeInTheDocument()
    expect(sortSelect).toHaveValue('alphabetical')
  })

  // Documentation.
  test('TC-702-05', () => {
    // Documentation.
    render(<ScatterMatrix trialRows={makeTrialRows()} currentStudy={makeStudy()} />)

    const sortSelect = screen.getByTestId('sort-select')
    fireEvent.change(sortSelect, { target: { value: 'correlation' } })

    // Documentation.
    expect(sortSelect).toHaveValue('correlation')
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
  test('TC-702-E01', () => {
    // Documentation.
    render(<ScatterMatrix trialRows={[]} currentStudy={null} />)

    expect(screen.getByText('Data not loaded')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-702-E02', () => {
    // Documentation.
    render(<ScatterMatrix trialRows={makeTrialRows()} currentStudy={makeStudy()} />)

    expect(screen.getByTestId('scatter-grid')).toBeInTheDocument()
  })
})
