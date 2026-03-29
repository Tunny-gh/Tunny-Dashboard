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
import type { ScatterMatrixEngine } from '../../wasm/workers/ScatterMatrixEngine'
import type { Study } from '../../types'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/** Documentation. */
function makeMockEngine(): ScatterMatrixEngine {
  return {
    renderCell: vi.fn().mockResolvedValue(null),
    workerIndex: vi.fn().mockReturnValue(0),
    dispose: vi.fn(),
  } as unknown as ScatterMatrixEngine
}

/** Documentation. */
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
    expect(() => render(<ScatterMatrix engine={null} currentStudy={makeStudy()} />)).not.toThrow()
  })

  // Documentation.
  test('TC-702-02', () => {
    // Documentation.
    render(<ScatterMatrix engine={null} currentStudy={makeStudy()} />)

    // Documentation.
    expect(screen.getByTestId('mode-btn-mode1')).toBeInTheDocument()
    expect(screen.getByTestId('mode-btn-mode2')).toBeInTheDocument()
    expect(screen.getByTestId('mode-btn-mode3')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-702-03', () => {
    // Documentation.
    render(<ScatterMatrix engine={null} currentStudy={makeStudy()} />)

    // Documentation.
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
    render(<ScatterMatrix engine={null} currentStudy={makeStudy()} />)

    // Documentation.
    const sortSelect = screen.getByTestId('sort-select')
    expect(sortSelect).toBeInTheDocument()
    expect(sortSelect).toHaveValue('alphabetical')
  })

  // Documentation.
  test('TC-702-05', () => {
    // Documentation.
    render(<ScatterMatrix engine={null} currentStudy={makeStudy()} />)

    // Documentation.
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
    render(<ScatterMatrix engine={null} currentStudy={null} />)

    // Documentation.
    expect(screen.getByText('Data not loaded')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-702-E02', () => {
    // Documentation.
    const engine = makeMockEngine()
    render(<ScatterMatrix engine={engine} currentStudy={makeStudy()} />)

    // Documentation.
    expect(screen.getByTestId('scatter-grid')).toBeInTheDocument()
  })
})
