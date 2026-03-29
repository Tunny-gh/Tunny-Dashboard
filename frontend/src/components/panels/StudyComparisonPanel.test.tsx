/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 */

import { describe, test, expect, beforeEach, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import '@testing-library/jest-dom'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

const mockSetComparisonStudyIds = vi.fn()
const mockSetMode = vi.fn()

const mockComparisonState = {
  comparisonStudyIds: [] as number[],
  mode: 'overlay' as const,
  results: [] as import('../../types').StudyComparisonResult[],
  isComputing: false,
  setComparisonStudyIds: mockSetComparisonStudyIds,
  setMode: mockSetMode,
  computeResults: vi.fn(),
  reset: vi.fn(),
}

vi.mock('../../stores/comparisonStore', () => ({
  useComparisonStore: vi.fn(() => mockComparisonState),
  canComparePareto: (a: import('../../types').Study, b: import('../../types').Study) => {
    return (
      a.directions.length === b.directions.length &&
      a.directions.every((d: string, i: number) => d === b.directions[i])
    )
  },
  COMPARISON_COLORS: ['#ef4444', '#22c55e', '#a855f7', '#f59e0b'],
  MAX_COMPARISON_STUDIES: 4,
}))

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

const mockStudyState = {
  currentStudy: {
    studyId: 1,
    name: 'main-study',
    directions: ['minimize', 'minimize'] as import('../../types').OptimizationDirection[],
    completedTrials: 50,
    totalTrials: 50,
    paramNames: ['x', 'y'],
    objectiveNames: ['obj1', 'obj2'],
    userAttrNames: [],
    hasConstraints: false,
  },
  allStudies: [
    {
      studyId: 1,
      name: 'main-study',
      directions: ['minimize', 'minimize'] as import('../../types').OptimizationDirection[],
      completedTrials: 50,
      totalTrials: 50,
      paramNames: ['x', 'y'],
      objectiveNames: ['obj1', 'obj2'],
      userAttrNames: [],
      hasConstraints: false,
    },
    {
      studyId: 2,
      name: 'study-2',
      directions: ['minimize', 'minimize'] as import('../../types').OptimizationDirection[],
      completedTrials: 30,
      totalTrials: 30,
      paramNames: ['x', 'y'],
      objectiveNames: ['obj1', 'obj2'],
      userAttrNames: [],
      hasConstraints: false,
    },
    {
      studyId: 3,
      name: 'study-3',
      directions: ['minimize', 'minimize'] as import('../../types').OptimizationDirection[],
      completedTrials: 20,
      totalTrials: 20,
      paramNames: ['x', 'y'],
      objectiveNames: ['obj1', 'obj2'],
      userAttrNames: [],
      hasConstraints: false,
    },
    {
      studyId: 4,
      name: 'study-incompatible',
      directions: ['minimize'] as import('../../types').OptimizationDirection[], // Documentation.
      completedTrials: 15,
      totalTrials: 15,
      paramNames: ['x'],
      objectiveNames: ['obj1'],
      userAttrNames: [],
      hasConstraints: false,
    },
  ],
}

vi.mock('../../stores/studyStore', () => ({
  useStudyStore: vi.fn(() => mockStudyState),
}))

import { useStudyStore } from '../../stores/studyStore'

import { StudyComparisonPanel } from './StudyComparisonPanel'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('StudyComparisonPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockComparisonState.comparisonStudyIds = []
    mockComparisonState.mode = 'overlay'
    mockComparisonState.results = []
  })

  // Documentation.
  test('TC-1401-P01', () => {
    // Documentation.
    render(<StudyComparisonPanel />)
    // Documentation.
    expect(screen.getByTestId('comparison-warning-4')).toBeInTheDocument()
    // Documentation.
    expect(screen.queryByTestId('comparison-warning-2')).not.toBeInTheDocument()
  })

  // Documentation.
  test('TC-1401-P02', () => {
    // Documentation.
    mockComparisonState.comparisonStudyIds = [2, 3, 4]
    render(<StudyComparisonPanel />)
    // Documentation.
    expect(screen.getByTestId('comparison-color-badge-2')).toBeInTheDocument()
    expect(screen.getByTestId('comparison-color-badge-3')).toBeInTheDocument()
    expect(screen.getByTestId('comparison-color-badge-4')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-1401-P03', () => {
    // Documentation.
    render(<StudyComparisonPanel />)
    expect(screen.getByTestId('comparison-mode-overlay')).toBeInTheDocument()
    expect(screen.getByTestId('comparison-mode-side-by-side')).toBeInTheDocument()
    expect(screen.getByTestId('comparison-mode-diff')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-1401-P04', () => {
    // Documentation.
    render(<StudyComparisonPanel />)
    // Documentation.
    expect(screen.queryByTestId('comparison-study-checkbox-1')).not.toBeInTheDocument()
    // Documentation.
    expect(screen.getByTestId('comparison-study-checkbox-2')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-1401-P05', () => {
    // Documentation.
    render(<StudyComparisonPanel />)
    fireEvent.click(screen.getByTestId('comparison-study-checkbox-2'))
    // Documentation.
    expect(mockSetComparisonStudyIds).toHaveBeenCalledWith([2])
  })

  // Documentation.
  test('TC-1401-P06', () => {
    // Documentation.
    render(<StudyComparisonPanel />)
    fireEvent.click(screen.getByTestId('comparison-mode-side-by-side'))
    // Documentation.
    expect(mockSetMode).toHaveBeenCalledWith('side-by-side')
  })

  // Documentation.
  test('TC-1401-P07', () => {
    // Documentation.
    ;(useStudyStore as ReturnType<typeof vi.fn>).mockReturnValue({
      ...mockStudyState,
      currentStudy: null,
    })
    render(<StudyComparisonPanel />)
    expect(screen.queryByTestId('study-comparison-panel')).not.toBeInTheDocument()
  })
})
