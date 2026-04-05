import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, cleanup, fireEvent } from '@testing-library/react'

// ---------------------------------------------------------------------------
// Hoisted mocks
// ---------------------------------------------------------------------------

const {
  mockComputeSurface3d,
  mockSetSurrogateModelType,
  mockClearSurface3dCache,
  mockAnalysisState,
} = vi.hoisted(() => {
  const mockComputeSurface3d = vi.fn().mockResolvedValue(undefined)
  const mockSetSurrogateModelType = vi.fn()
  const mockClearSurface3dCache = vi.fn()
  const mockAnalysisState = {
    surrogateModelType: 'ridge' as const,
    surface3dCache: new Map(),
    isComputingSurface: false,
    surface3dError: null,
    setSurrogateModelType: mockSetSurrogateModelType,
    computeSurface3d: mockComputeSurface3d,
    clearSurface3dCache: mockClearSurface3dCache,
  }
  return {
    mockComputeSurface3d,
    mockSetSurrogateModelType,
    mockClearSurface3dCache,
    mockAnalysisState,
  }
})

vi.mock('../../stores/analysisStore', () => ({
  useAnalysisStore: vi.fn(() => mockAnalysisState),
}))

vi.mock('../../stores/studyStore', () => ({
  useStudyStore: vi.fn().mockReturnValue(null),
}))

// Mock echarts-for-react (replaces deck.gl after refactor)
vi.mock('echarts-for-react')

import { SurfacePlot3D } from './SurfacePlot3D'
import { useStudyStore } from '../../stores/studyStore'
import type { Study } from '../../types'

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeStudy(paramNames = ['x1', 'x2', 'x3'], objectiveNames = ['obj0']): Study {
  return {
    studyId: 1,
    name: 'test-study',
    directions: ['minimize'],
    completedTrials: 10,
    totalTrials: 10,
    paramNames,
    objectiveNames,
    userAttrNames: [],
    hasConstraints: false,
  }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('SurfacePlot3D', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockAnalysisState.surrogateModelType = 'ridge'
    mockAnalysisState.surface3dCache = new Map()
    mockAnalysisState.isComputingSurface = false
    mockAnalysisState.surface3dError = null
    mockComputeSurface3d.mockResolvedValue(undefined)
    ;(useStudyStore as ReturnType<typeof vi.fn>).mockReturnValue(null)
  })

  afterEach(() => {
    cleanup()
  })

  // TC-1628-01: No study → EmptyState
  it('TC-1628-01: shows EmptyState when no study is loaded', () => {
    ;(useStudyStore as ReturnType<typeof vi.fn>).mockReturnValue(null)
    render(<SurfacePlot3D />)
    expect(screen.getByTestId('empty-state')).toBeInTheDocument()
  })

  // TC-1628-02: Too few parameters → EmptyState
  it('TC-1628-02: shows EmptyState when fewer than 2 parameters', () => {
    ;(useStudyStore as ReturnType<typeof vi.fn>).mockReturnValue(makeStudy(['x1']))
    render(<SurfacePlot3D />)
    expect(screen.getByTestId('empty-state')).toBeInTheDocument()
  })

  // TC-1628-03: isComputingSurface → loading overlay with spinner text
  it('TC-1628-03: shows loading state when isComputingSurface is true', () => {
    ;(useStudyStore as ReturnType<typeof vi.fn>).mockReturnValue(makeStudy())
    mockAnalysisState.isComputingSurface = true
    render(<SurfacePlot3D />)
    // Spinner overlay replaces the EmptyState; text still contains "Computing"
    expect(screen.getByText(/Computing/)).toBeInTheDocument()
  })

  // TC-1628-04: surface3dError → EmptyState with error message
  it('TC-1628-04: shows error EmptyState when surface3dError is set', () => {
    ;(useStudyStore as ReturnType<typeof vi.fn>).mockReturnValue(makeStudy())
    mockAnalysisState.surface3dError = 'computation failed'
    render(<SurfacePlot3D />)
    expect(screen.getByTestId('empty-state')).toBeInTheDocument()
    expect(screen.getByText(/computation failed/)).toBeInTheDocument()
  })

  // TC-1628-05: Valid study shows dropdowns
  it('TC-1628-05: renders param and objective dropdowns with valid study', () => {
    ;(useStudyStore as ReturnType<typeof vi.fn>).mockReturnValue(makeStudy())
    render(<SurfacePlot3D />)
    const selects = screen.getAllByRole('combobox')
    expect(selects.length).toBeGreaterThanOrEqual(3)
  })

  // TC-1628-06: Model type dropdown triggers setSurrogateModelType
  it('TC-1628-06: model type dropdown calls setSurrogateModelType', () => {
    ;(useStudyStore as ReturnType<typeof vi.fn>).mockReturnValue(makeStudy())
    render(<SurfacePlot3D />)
    const selects = screen.getAllByRole('combobox')
    // Model selector is the last dropdown
    const modelSelect = selects[selects.length - 1]
    fireEvent.change(modelSelect, { target: { value: 'random_forest' } })
    expect(mockSetSurrogateModelType).toHaveBeenCalledWith('random_forest')
  })

  // TC-1628-07: Cache hit renders chart container (ECharts heatmap)
  it('TC-1628-07: renders chart when surface3dCache has data', () => {
    const study = makeStudy()
    ;(useStudyStore as ReturnType<typeof vi.fn>).mockReturnValue(study)
    const cacheKey = `ridge_${study.paramNames[0]}_${study.paramNames[1]}_${study.objectiveNames[0]}_50`
    mockAnalysisState.surface3dCache = new Map([
      [
        cacheKey,
        {
          param1Name: study.paramNames[0],
          param2Name: study.paramNames[1],
          objectiveName: study.objectiveNames[0],
          grid1: [0, 0.5, 1.0],
          grid2: [0, 0.5, 1.0],
          values: [
            [0.1, 0.2, 0.3],
            [0.4, 0.5, 0.6],
            [0.7, 0.8, 0.9],
          ],
          rSquared: 0.95,
        },
      ],
    ])
    render(<SurfacePlot3D />)
    // ECharts is mocked; verify the surface-plot-3d container is rendered
    expect(screen.getByTestId('surface-plot-3d')).toBeInTheDocument()
  })
})
