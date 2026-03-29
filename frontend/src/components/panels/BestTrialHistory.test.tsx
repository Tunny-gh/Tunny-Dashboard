/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 * Documentation.
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest'
import { render, screen, cleanup, fireEvent } from '@testing-library/react'

import { BestTrialHistory } from './BestTrialHistory'
import type { TrialData } from '../charts/OptimizationHistory'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/** Documentation. */
function makeData(): TrialData[] {
  return [
    { trial: 1, value: 100 },
    { trial: 2, value: 80 }, // Documentation.
    { trial: 3, value: 85 }, // Documentation.
    { trial: 4, value: 60 }, // Documentation.
    { trial: 5, value: 70 }, // Documentation.
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
  test('TC-1001-13', () => {
    // Documentation.
    expect(() => render(<BestTrialHistory data={makeData()} direction="minimize" />)).not.toThrow()
  })

  // Documentation.
  test('TC-1001-14', () => {
    // Documentation.
    render(<BestTrialHistory data={makeData()} direction="minimize" />)

    // Documentation.
    // Documentation.
    expect(screen.getByTestId('best-row-1')).toBeInTheDocument()
    expect(screen.getByTestId('best-row-2')).toBeInTheDocument()
    expect(screen.getByTestId('best-row-4')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-1001-15', () => {
    // Documentation.
    render(<BestTrialHistory data={makeData()} direction="minimize" />)

    // Documentation.
    expect(screen.queryByTestId('best-row-3')).not.toBeInTheDocument()
    expect(screen.queryByTestId('best-row-5')).not.toBeInTheDocument()
  })

  // Documentation.
  test('TC-1001-16', () => {
    // Documentation.
    const onRowClick = vi.fn()
    render(<BestTrialHistory data={makeData()} direction="minimize" onRowClick={onRowClick} />)

    // Documentation.
    fireEvent.click(screen.getByTestId('best-row-2'))

    // Documentation.
    expect(onRowClick).toHaveBeenCalledTimes(1)
    expect(onRowClick).toHaveBeenCalledWith({ trial: 2, value: 80 })
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
  test('TC-1001-E01', () => {
    // Documentation.
    render(<BestTrialHistory data={[]} direction="minimize" />)

    // Documentation.
    expect(screen.getByTestId('best-trial-table')).toBeInTheDocument()
  })
})
