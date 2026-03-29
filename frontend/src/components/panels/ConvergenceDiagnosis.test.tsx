/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 * Documentation.
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest'
import { render, screen, cleanup } from '@testing-library/react'

import { ConvergenceDiagnosis, diagnoseConvergence } from './ConvergenceDiagnosis'
import type { TrialData } from '../charts/OptimizationHistory'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/** Documentation. */
function makeConvergedData(total: number): TrialData[] {
  return Array.from({ length: total }, (_, i) => ({
    trial: i + 1,
    // Documentation.
    value: i < total * 0.8 ? 100 - i * 1.0 : 100 - total * 0.8,
  }))
}

/** Documentation. */
function makeConvergingData(total: number): TrialData[] {
  return Array.from({ length: total }, (_, i) => ({
    trial: i + 1,
    value: 100 - i * 0.3,
  }))
}

/** Documentation. */
function makeInsufficientData(count: number): TrialData[] {
  return Array.from({ length: count }, (_, i) => ({
    trial: i + 1,
    value: 100 - i,
  }))
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
  test('TC-1001-07', () => {
    // Documentation.
    render(<ConvergenceDiagnosis data={[]} direction="minimize" />)

    // Documentation.
    expect(screen.getByText('Insufficient (not enough trials)')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-1001-08', () => {
    // Documentation.
    render(<ConvergenceDiagnosis data={makeConvergedData(30)} direction="minimize" />)

    // Documentation.
    expect(screen.getByTestId('badge-converged')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-1001-09', () => {
    // Documentation.
    render(<ConvergenceDiagnosis data={makeConvergingData(20)} direction="minimize" />)

    // Documentation.
    expect(screen.getByTestId('badge-converging')).toBeInTheDocument()
  })
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  // Documentation.
  test('TC-1001-10', () => {
    // Documentation.
    const result = diagnoseConvergence(makeInsufficientData(5), 'minimize')
    expect(result).toBe('insufficient') // Documentation.
  })

  // Documentation.
  test('TC-1001-11', () => {
    // Documentation.
    const result = diagnoseConvergence(makeConvergedData(30), 'minimize')
    expect(result).toBe('converged') // Documentation.
  })

  // Documentation.
  test('TC-1001-12', () => {
    // Documentation.
    const result = diagnoseConvergence(makeConvergingData(20), 'minimize')
    expect(result).not.toBe('converged') // Documentation.
    expect(result).not.toBe('insufficient') // Documentation.
  })
})
