/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 */

import { describe, test, expect, beforeEach, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import '@testing-library/jest-dom'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

vi.mock('echarts-for-react')

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

vi.mock('deck.gl', () => ({
  DeckGL: vi.fn(({ children }: { children?: unknown }) => (
    <div data-testid="deck-gl">{children as never}</div>
  )),
  ScatterplotLayer: vi.fn().mockImplementation((props: { id: string }) => ({
    id: props.id,
    type: 'ScatterplotLayer',
  })),
  GridLayer: vi.fn().mockImplementation((props: { id: string }) => ({
    id: props.id,
    type: 'GridLayer',
  })),
}))

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

const mockStudySelector = vi.fn()

vi.mock('../../stores/studyStore', () => ({
  useStudyStore: (selector: (s: unknown) => unknown) => mockStudySelector(selector),
}))

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

vi.mock('../../wasm/wasmLoader', () => ({
  WasmLoader: {
    getInstance: vi.fn().mockResolvedValue({
      getTrials: vi.fn().mockReturnValue([]),
      computeHvHistory: vi.fn().mockReturnValue({
        trialIds: new Uint32Array([0, 1, 2]),
        hvValues: new Float64Array([0.5, 0.7, 0.9]),
      }),
    }),
  },
}))

// Mock new stores to prevent module-level side effects (analysisStore/clusterStore
// call useStudyStore.getState() at module initialization time, which fails when
// studyStore is mocked as a plain function without .getState)

const { mockUseAnalysisStoreCC } = vi.hoisted(() => {
  const mockSensitivityResult = {
    spearman: [[0.8], [0.5]],
    ridge: [{ beta: [0.8, 0.5], rSquared: 0.9 }],
    paramNames: ['x', 'y'],
    objectiveNames: ['value'],
    durationMs: 1,
  }
  const mockUseAnalysisStoreCC = vi.fn().mockReturnValue({
    sensitivityResult: mockSensitivityResult,
    isComputingSensitivity: false,
    sensitivityError: null,
    computeSensitivity: vi.fn(),
    computeSensitivitySelected: vi.fn(),
    surrogateModelType: 'ridge' as const,
    surface3dCache: new Map(),
    isComputingSurface: false,
    surface3dError: null,
    setSurrogateModelType: vi.fn(),
    computeSurface3d: vi.fn().mockResolvedValue(undefined),
    clearSurface3dCache: vi.fn(),
  })
  return { mockUseAnalysisStoreCC }
})

vi.mock('../../stores/analysisStore', () => ({
  useAnalysisStore: mockUseAnalysisStoreCC,
}))

vi.mock('../../stores/clusterStore', () => ({
  useClusterStore: vi.fn().mockReturnValue({
    pcaProjections: null,
    clusterLabels: null,
    isRunning: false,
    clusterError: null,
    runClustering: vi.fn(),
    estimateK: vi.fn(),
  }),
}))

// Mock mcdmStore to prevent module-level side effects
// (mcdmStore calls useStudyStore.getState() at initialization time)
vi.mock('../../stores/mcdmStore', () => ({
  useMcdmStore: vi.fn().mockReturnValue({
    topsisResult: null,
    topsisWeights: [],
    isComputing: false,
    topsisError: null,
    topN: 10,
    computeTopsis: vi.fn(),
    setTopsisWeights: vi.fn(),
    setTopN: vi.fn(),
    reset: vi.fn(),
  }),
}))

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

import { useLayoutStore, DEFAULT_FREE_LAYOUT } from '../../stores/layoutStore'
import { FreeLayoutCanvas } from './FreeLayoutCanvas'
import type { Study, TrialData } from '../../types'
import type { GpuBuffer } from '../../wasm/gpuBuffer'

// Documentation.
void DEFAULT_FREE_LAYOUT

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/** Documentation. */
function makeMultiObjectiveStudy(): Study {
  return {
    studyId: 1,
    name: 'test-study',
    directions: ['minimize', 'minimize'],
    completedTrials: 5,
    totalTrials: 5,
    paramNames: ['x', 'y'],
    objectiveNames: ['obj0', 'obj1'],
    userAttrNames: [],
    hasConstraints: false,
  }
}

/** Documentation. */
function makeSingleObjectiveStudy(): Study {
  return {
    studyId: 1,
    name: 'test-study',
    directions: ['minimize'],
    completedTrials: 5,
    totalTrials: 5,
    paramNames: ['x', 'y'],
    objectiveNames: ['value'],
    userAttrNames: [],
    hasConstraints: false,
  }
}

/** Documentation. */
function makeGpuBuffer(): GpuBuffer {
  const n = 5
  const positions = new Float32Array(n * 2).fill(0.5)
  return {
    trialCount: n,
    positions,
    positions3d: new Float32Array(n * 3),
    sizes: new Float32Array(n),
  } as unknown as GpuBuffer
}

/** Documentation. */
function makeTrialData(): TrialData[] {
  return [
    { trialId: 0, params: { x: 1.5, y: 0.5 }, values: [10.0], paretoRank: null },
    { trialId: 1, params: { x: 2.5, y: 1.5 }, values: [5.0], paretoRank: null },
  ]
}

/** Documentation. */
function setStudyStore(
  currentStudy: Study | null,
  gpuBuffer: GpuBuffer | null,
  trialRows: TrialData[] = [],
  loadError: string | null = null,
) {
  mockStudySelector.mockImplementation(
    (
      selector: (s: {
        currentStudy: Study | null
        gpuBuffer: GpuBuffer | null
        trialRows: TrialData[]
        loadError: string | null
      }) => unknown,
    ) => selector({ currentStudy, gpuBuffer, trialRows, loadError }),
  )
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('FreeLayoutCanvas — ChartContent', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    // Documentation.
    useLayoutStore.setState({
      layoutMode: 'D',
      freeModeLayout: null,
      layoutLoadError: null,
      visibleCharts: new Set(),
      panelSizes: { leftPanel: 280, bottomPanel: 200 },
    })
  })

  // ----------------------------------------------------------------
  // Documentation.
  // ----------------------------------------------------------------

  describe('translated test case', () => {
    test('TC-CC-001', () => {
      // Documentation.
      setStudyStore(null, null)
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'pareto-front', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      })
      render(<FreeLayoutCanvas />)
      expect(screen.getByTestId('empty-state')).toHaveTextContent('Please load data')
    })

    test('shows the study load error when the selected log file is empty', () => {
      setStudyStore(null, null, [], 'The selected log file is empty.')
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'pareto-front', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      })

      render(<FreeLayoutCanvas />)

      expect(screen.getByTestId('empty-state')).toHaveTextContent('The selected log file is empty.')
    })

    test('TC-CC-002', () => {
      // Documentation.
      setStudyStore(makeSingleObjectiveStudy(), makeGpuBuffer())
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'unknown-chart' as never, gridRow: [1, 3], gridCol: [1, 3] }],
        },
      })
      render(<FreeLayoutCanvas />)
      expect(screen.getByTestId('empty-state')).toHaveTextContent('This chart is under development')
    })
  })

  // ----------------------------------------------------------------
  // objective-pair-matrix
  // ----------------------------------------------------------------

  describe('objective-pair-matrix', () => {
    test('TC-CC-010', () => {
      // Documentation.
      setStudyStore(makeMultiObjectiveStudy(), makeGpuBuffer())
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'objective-pair-matrix', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      })
      render(<FreeLayoutCanvas />)
      expect(screen.getByTestId('objective-pair-matrix')).toBeInTheDocument()
    })

    test('TC-CC-011', () => {
      // Documentation.
      setStudyStore(makeSingleObjectiveStudy(), makeGpuBuffer())
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'objective-pair-matrix', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      })
      render(<FreeLayoutCanvas />)
      expect(screen.getByTestId('empty-state')).toHaveTextContent(
        'Available for multi-objective studies only',
      )
    })

    test('TC-CC-012', () => {
      // Documentation.
      setStudyStore(makeMultiObjectiveStudy(), null)
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'objective-pair-matrix', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      })
      // Documentation.
      // Documentation.
      expect(() => render(<FreeLayoutCanvas />)).not.toThrow()
    })
  })

  // ----------------------------------------------------------------
  // importance
  // ----------------------------------------------------------------

  describe('importance', () => {
    beforeEach(() => {
      mockUseAnalysisStoreCC.mockReturnValue({
        sensitivityResult: {
          spearman: [[0.8], [0.5]],
          ridge: [{ beta: [0.8, 0.5], rSquared: 0.9 }],
          paramNames: ['x', 'y'],
          objectiveNames: ['value'],
          durationMs: 1,
        },
        isComputingSensitivity: false,
        sensitivityError: null,
        computeSensitivity: vi.fn(),
        computeSensitivitySelected: vi.fn(),
        surrogateModelType: 'ridge' as const,
        surface3dCache: new Map(),
        isComputingSurface: false,
        surface3dError: null,
        setSurrogateModelType: vi.fn(),
        computeSurface3d: vi.fn().mockResolvedValue(undefined),
        clearSurface3dCache: vi.fn(),
      })
    })

    test('TC-CC-020', () => {
      // Documentation. (Now uses ImportanceChart with analysisStore)
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'importance', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      })
      render(<FreeLayoutCanvas />)
      const echartsEl = screen.getByTestId('echarts')
      expect(echartsEl).toBeInTheDocument()
    })

    test('TC-CC-021', () => {
      // Documentation.
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'importance', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      })
      render(<FreeLayoutCanvas />)
      const option = JSON.parse(screen.getByTestId('echarts').dataset.option ?? '{}')
      expect(option.title?.text).toContain('Importance')
    })

    test('TC-CC-022', () => {
      // Documentation. (ImportanceChart shows EmptyState when sensitivityResult has no params)
      mockUseAnalysisStoreCC.mockReturnValue({
        sensitivityResult: {
          spearman: [],
          ridge: [],
          paramNames: [],
          objectiveNames: ['value'],
          durationMs: 1,
        },
        isComputingSensitivity: false,
        sensitivityError: null,
        computeSensitivity: vi.fn(),
        computeSensitivitySelected: vi.fn(),
        surrogateModelType: 'ridge' as const,
        surface3dCache: new Map(),
        isComputingSurface: false,
        surface3dError: null,
        setSurrogateModelType: vi.fn(),
        computeSurface3d: vi.fn().mockResolvedValue(undefined),
        clearSurface3dCache: vi.fn(),
      })
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'importance', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      })
      render(<FreeLayoutCanvas />)
      expect(screen.getByTestId('empty-state')).toBeInTheDocument()
    })

    test('TC-CC-023', () => {
      // Documentation.
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'importance', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      })
      render(<FreeLayoutCanvas />)
      const option = JSON.parse(screen.getByTestId('echarts').dataset.option ?? '{}')
      // Documentation. (2 params in sensitivityResult → 2 bars)
      expect(option.series?.[0]?.data?.length).toBe(2)
    })
  })

  // ----------------------------------------------------------------
  // slice
  // ----------------------------------------------------------------

  describe('slice', () => {
    test('TC-CC-030', () => {
      // Documentation.
      setStudyStore(makeSingleObjectiveStudy(), makeGpuBuffer(), makeTrialData())
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'slice', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      })
      render(<FreeLayoutCanvas />)
      expect(screen.getByTestId('slice-plot')).toBeInTheDocument()
    })

    test('TC-CC-031', () => {
      // Documentation.
      setStudyStore(makeSingleObjectiveStudy(), makeGpuBuffer(), [])
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'slice', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      })
      render(<FreeLayoutCanvas />)
      expect(screen.getByTestId('empty-state')).toBeInTheDocument()
    })
  })

  // ----------------------------------------------------------------
  // contour
  // ----------------------------------------------------------------

  // ----------------------------------------------------------------
  // hypervolume
  // ----------------------------------------------------------------

  describe('hypervolume', () => {
    test('TC-CC-060', async () => {
      // Documentation.
      setStudyStore(makeMultiObjectiveStudy(), makeGpuBuffer())
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'hypervolume', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      })
      render(<FreeLayoutCanvas />)
      await waitFor(() => {
        expect(screen.getByTestId('hypervolume-chart')).toBeInTheDocument()
      })
    })

    test('TC-CC-061', () => {
      // Documentation.
      setStudyStore(makeSingleObjectiveStudy(), makeGpuBuffer())
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'hypervolume', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      })
      render(<FreeLayoutCanvas />)
      expect(screen.getByTestId('empty-state')).toHaveTextContent(
        'Available for multi-objective studies only',
      )
    })

    test('TC-CC-062', async () => {
      // Documentation.
      const { WasmLoader } = await import('../../wasm/wasmLoader')
      vi.mocked(WasmLoader.getInstance).mockResolvedValueOnce({
        getTrials: vi.fn().mockReturnValue([]),
        computeHvHistory: vi.fn().mockRejectedValue(new Error('HV failed')),
        filterByRanges: vi.fn().mockReturnValue(new Uint32Array([])),
        serializeCsv: vi.fn().mockReturnValue(''),
        appendJournalDiff: vi.fn().mockReturnValue({ new_completed: 0, consumed_bytes: 0 }),
        computeReportStats: vi.fn().mockReturnValue('{}'),
        parseJournal: vi.fn().mockReturnValue({ studies: [], durationMs: 0 }),
        selectStudy: vi.fn().mockReturnValue({}),
        computeParetoRanks: vi.fn().mockReturnValue({ ranks: [] }),
        computeHvHistory: vi.fn().mockRejectedValue(new Error('HV failed')),
      } as never)
      setStudyStore(makeMultiObjectiveStudy(), makeGpuBuffer())
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'hypervolume', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      })
      render(<FreeLayoutCanvas />)
      await waitFor(() => {
        expect(screen.getByTestId('empty-state')).toHaveTextContent('HV computation error')
      })
    })
  })

  // ----------------------------------------------------------------
  // topsis-ranking
  // ----------------------------------------------------------------

  describe('topsis-ranking', () => {
    test('TC-1623-01: renders TopsisRankingChart for topsis-ranking chartId', () => {
      // English comment.
      // English comment.
      setStudyStore(makeMultiObjectiveStudy(), makeGpuBuffer())
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'topsis-ranking', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      })
      render(<FreeLayoutCanvas />)
      // English comment.
      expect(screen.getByTestId('free-layout-card-topsis-ranking')).toBeInTheDocument()
    })
  })

  // ----------------------------------------------------------------
  // surface3d
  // ----------------------------------------------------------------

  describe('surface3d', () => {
    test('TC-1629-01: renders SurfacePlot3D card for surface3d chartId', () => {
      // English comment.
      setStudyStore(makeMultiObjectiveStudy(), makeGpuBuffer())
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'surface3d', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      })
      render(<FreeLayoutCanvas />)
      // English comment.
      expect(screen.getByTestId('free-layout-card-surface3d')).toBeInTheDocument()
    })
  })

  describe('contour', () => {
    test('TC-CC-040', () => {
      // Documentation.
      setStudyStore(makeSingleObjectiveStudy(), makeGpuBuffer(), makeTrialData())
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'contour', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      })
      render(<FreeLayoutCanvas />)
      expect(screen.getByTestId('contour-plot')).toBeInTheDocument()
    })

    test('TC-CC-041', () => {
      // Documentation.
      const studyOneParam: Study = { ...makeSingleObjectiveStudy(), paramNames: ['x'] }
      setStudyStore(studyOneParam, makeGpuBuffer(), makeTrialData())
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'contour', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      })
      render(<FreeLayoutCanvas />)
      expect(screen.getByTestId('empty-state')).toHaveTextContent('At least 2 parameters required')
    })

    test('TC-CC-042', () => {
      // Documentation.
      setStudyStore(makeSingleObjectiveStudy(), makeGpuBuffer(), [])
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'contour', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      })
      render(<FreeLayoutCanvas />)
      expect(screen.getByTestId('empty-state')).toBeInTheDocument()
    })
  })
})
