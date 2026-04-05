/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 */

import { describe, test, expect, beforeEach, vi } from 'vitest'
import { render, screen, fireEvent, act } from '@testing-library/react'
import '@testing-library/jest-dom'
import { useLayoutStore } from '../../stores/layoutStore'
import { FreeLayoutCanvas } from './FreeLayoutCanvas'
import type { FreeModeLayout } from '../../types'

// ---------------------------------------------------------------------------
// Mocks for TC-1608 (new chart cases)
// ---------------------------------------------------------------------------

vi.mock('echarts-for-react')

const {
  mockUseAnalysisStore,
  mockUseClusterStore1608,
  mockUseSelectionStore1608,
  mockWasmGetInstance1608,
  mockUseStudyStore1608,
  analysisState,
  clusterState1608,
  studyState1608,
} = vi.hoisted(() => {
  const analysisState = {
    sensitivityResult: null as null,
    isComputingSensitivity: false,
    sensitivityError: null as string | null,
    computeSensitivity: vi.fn(),
    computeSensitivitySelected: vi.fn(),
    sobolResult: null as null,
    isComputingSobol: false,
    sobolError: null as string | null,
    computeSobol: vi.fn(),
    surrogateModelType: 'ridge' as const,
    surface3dCache: new Map(),
    isComputingSurface: false,
    surface3dError: null as string | null,
    setSurrogateModelType: vi.fn(),
    computeSurface3d: vi.fn().mockResolvedValue(undefined),
    clearSurface3dCache: vi.fn(),
  }
  const mockUseAnalysisStore = vi
    .fn()
    .mockImplementation((selector?: (s: typeof analysisState) => unknown) =>
      typeof selector === 'function' ? selector(analysisState) : analysisState,
    )

  const clusterState1608 = {
    pcaProjections: null as number[][] | null,
    clusterLabels: null as number[] | null,
    isRunning: false,
    clusterError: null as string | null,
  }
  const mockUseClusterStore1608 = vi
    .fn()
    .mockImplementation((selector?: (s: typeof clusterState1608) => unknown) =>
      typeof selector === 'function' ? selector(clusterState1608) : clusterState1608,
    )

  const selectionState1608 = { colorMode: 'objective', selectedIndices: [] as number[] }
  const mockUseSelectionStore1608 = vi
    .fn()
    .mockImplementation((selector?: (s: typeof selectionState1608) => unknown) =>
      typeof selector === 'function' ? selector(selectionState1608) : selectionState1608,
    )

  const studyState1608 = {
    currentStudy: null as { studyId: number; name: string; directions: string[]; completedTrials: number; totalTrials: number; paramNames: string[]; objectiveNames: string[]; userAttrNames: string[]; hasConstraints: boolean } | null,
    gpuBuffer: null as null,
    trialRows: [] as never[],
    loadError: null as string | null,
  }
  const mockUseStudyStore1608 = Object.assign(
    vi.fn().mockImplementation((selector?: (s: typeof studyState1608) => unknown) =>
      typeof selector === 'function' ? selector(studyState1608) : studyState1608,
    ),
    {
      getState: vi.fn().mockReturnValue(studyState1608),
      subscribe: vi.fn().mockReturnValue(vi.fn()),
    },
  )

  const mockWasmGetInstance1608 = vi.fn().mockReturnValue(new Promise(() => {}))

  return {
    mockUseAnalysisStore,
    mockUseClusterStore1608,
    mockUseSelectionStore1608,
    mockWasmGetInstance1608,
    mockUseStudyStore1608,
    analysisState,
    clusterState1608,
    studyState1608,
  }
})

vi.mock('../../stores/analysisStore', () => ({ useAnalysisStore: mockUseAnalysisStore }))
vi.mock('../../stores/clusterStore', () => ({ useClusterStore: mockUseClusterStore1608 }))
vi.mock('../../stores/selectionStore', () => ({ useSelectionStore: mockUseSelectionStore1608 }))
vi.mock('../../stores/studyStore', () => ({ useStudyStore: mockUseStudyStore1608 }))
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
vi.mock('../../wasm/wasmLoader', () => ({
  WasmLoader: { getInstance: mockWasmGetInstance1608, reset: vi.fn() },
}))

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

function resetStore() {
  useLayoutStore.setState({
    layoutMode: 'D',
    freeModeLayout: null,
    layoutLoadError: null,
    visibleCharts: new Set(['pareto-front', 'parallel-coords', 'history', 'scatter-matrix']),
    panelSizes: { leftPanel: 280, bottomPanel: 200 },
  })
}

const SAMPLE_LAYOUT: FreeModeLayout = {
  cells: [
    { cellId: 'pareto-front', chartId: 'pareto-front', gridRow: [1, 3], gridCol: [1, 3] },
    { cellId: 'parallel-coords', chartId: 'parallel-coords', gridRow: [1, 3], gridCol: [3, 5] },
    { cellId: 'history', chartId: 'history', gridRow: [3, 5], gridCol: [1, 3] },
    { cellId: 'scatter-matrix', chartId: 'scatter-matrix', gridRow: [3, 5], gridCol: [3, 5] },
  ],
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('FreeLayoutCanvas', () => {
  beforeEach(() => {
    resetStore()
    vi.clearAllMocks()
  })

  // Documentation.
  test('TC-1501-F01', () => {
    // Documentation.
    useLayoutStore.setState({ freeModeLayout: SAMPLE_LAYOUT })
    render(<FreeLayoutCanvas />)
    // Documentation.
    expect(screen.getByTestId('free-layout-card-pareto-front')).toBeInTheDocument()
    expect(screen.getByTestId('free-layout-card-parallel-coords')).toBeInTheDocument()
    expect(screen.getByTestId('free-layout-card-history')).toBeInTheDocument()
    expect(screen.getByTestId('free-layout-card-scatter-matrix')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-1501-F02', () => {
    // Documentation.
    render(<FreeLayoutCanvas />)
    // Documentation.
    expect(screen.getByTestId('free-layout-card-pareto-front')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-1501-F03', () => {
    // Documentation.
    useLayoutStore.setState({ freeModeLayout: SAMPLE_LAYOUT })
    render(<FreeLayoutCanvas />)

    const dragHandle = screen.getByTestId('free-layout-drag-handle-pareto-front')
    const dropZone = screen.getByTestId('free-layout-dropzone-3-3')

    // Documentation.
    fireEvent.dragStart(dragHandle)
    // Documentation.
    fireEvent.dragOver(dropZone)
    fireEvent.drop(dropZone)

    // Documentation.
    const updatedCell = useLayoutStore
      .getState()
      .freeModeLayout?.cells.find((c) => c.chartId === 'pareto-front')
    expect(updatedCell?.gridRow[0]).toBe(3)
    expect(updatedCell?.gridCol[0]).toBe(3)
  })

  // Documentation.
  test('TC-1501-F08', () => {
    // Documentation.
    useLayoutStore.setState({ freeModeLayout: SAMPLE_LAYOUT })
    render(<FreeLayoutCanvas />)
    expect(screen.getByTestId('chart-close-btn-pareto-front')).toBeInTheDocument()
    expect(screen.getByTestId('chart-close-btn-history')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-1501-F09', () => {
    // Documentation.
    useLayoutStore.setState({ freeModeLayout: SAMPLE_LAYOUT })
    render(<FreeLayoutCanvas />)

    fireEvent.click(screen.getByTestId('chart-close-btn-pareto-front'))

    const cells = useLayoutStore.getState().freeModeLayout?.cells
    expect(cells?.find((c) => c.cellId === 'pareto-front')).toBeUndefined()
  })

  // Documentation.
  test('TC-1501-F10', () => {
    // Documentation.
    useLayoutStore.setState({ freeModeLayout: { cells: [] }, layoutMode: 'D' })
    render(<FreeLayoutCanvas />)

    const dropZone = screen.getByTestId('free-layout-dropzone-1-1')
    fireEvent.dragOver(dropZone)
    fireEvent.drop(dropZone, {
      dataTransfer: { getData: () => JSON.stringify({ type: 'add-chart', chartId: 'edf' }) },
    })

    const cells = useLayoutStore.getState().freeModeLayout?.cells
    expect(cells?.length).toBe(1)
    expect(cells?.[0].chartId).toBe('edf')
  })

  // Documentation.
  test('TC-1501-F11', () => {
    // Documentation.
    useLayoutStore.setState({ freeModeLayout: { cells: [] }, layoutMode: 'D' })
    render(<FreeLayoutCanvas />)

    const dropZone1 = screen.getByTestId('free-layout-dropzone-1-1')
    const dropZone2 = screen.getByTestId('free-layout-dropzone-3-1')
    const payload = JSON.stringify({ type: 'add-chart', chartId: 'slice' })

    fireEvent.drop(dropZone1, { dataTransfer: { getData: () => payload } })
    fireEvent.drop(dropZone2, { dataTransfer: { getData: () => payload } })

    const cells = useLayoutStore.getState().freeModeLayout?.cells
    expect(cells?.length).toBe(2)
    expect(cells?.[0].chartId).toBe('slice')
    expect(cells?.[1].chartId).toBe('slice')
    expect(cells?.[0].cellId).not.toBe(cells?.[1].cellId)
  })

  // Documentation.
  test('TC-1501-F04', () => {
    // Documentation.
    useLayoutStore.setState({ freeModeLayout: SAMPLE_LAYOUT })
    render(<FreeLayoutCanvas />)

    const saveBtn = screen.getByTestId('save-free-layout-btn')
    fireEvent.click(saveBtn)

    // Documentation.
    expect(screen.getByTestId('layout-saved-toast')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-1501-F05', () => {
    // Documentation.
    useLayoutStore.setState({ layoutLoadError: 'Failed to load layout' })
    render(<FreeLayoutCanvas />)
    // Documentation.
    expect(screen.getByTestId('layout-error-msg')).toBeInTheDocument()
    expect(screen.getByTestId('layout-error-msg')).toHaveTextContent('Failed to load layout')
  })

  // Documentation.
  test('TC-1501-F06', () => {
    // Documentation.
    render(<FreeLayoutCanvas />)
    expect(screen.queryByTestId('free-layout-preset-A')).not.toBeInTheDocument()
    expect(screen.queryByTestId('free-layout-preset-B')).not.toBeInTheDocument()
    expect(screen.queryByTestId('free-layout-preset-C')).not.toBeInTheDocument()
  })

  // Documentation.
  test('TC-1501-F07', () => {
    // Documentation.
    render(<FreeLayoutCanvas />)
    expect(screen.getByTestId('save-free-layout-btn')).toBeInTheDocument()
  })
})

// ---------------------------------------------------------------------------
// TC-1608: new chart cases wired in FreeLayoutCanvas
// ---------------------------------------------------------------------------

describe('TC-1608: FreeLayoutCanvas new chart cases', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    analysisState.sensitivityResult = null
    analysisState.isComputingSensitivity = false
    analysisState.sensitivityError = null
    analysisState.surface3dCache = new Map()
    analysisState.isComputingSurface = false
    analysisState.surface3dError = null
    clusterState1608.pcaProjections = null
    clusterState1608.isRunning = false
    clusterState1608.clusterError = null
    clusterState1608.clusterLabels = null
    // studyStore: set a valid study so wrappers don't short-circuit to EmptyState
    studyState1608.currentStudy = {
      studyId: 1,
      name: 'test',
      directions: ['minimize'],
      completedTrials: 5,
      totalTrials: 5,
      paramNames: ['x', 'y'],
      objectiveNames: ['value'],
      userAttrNames: [],
      hasConstraints: false,
    }
    mockUseStudyStore1608.getState.mockReturnValue(studyState1608)
    mockUseAnalysisStore.mockImplementation(
      (selector?: (s: typeof analysisState) => unknown) =>
        typeof selector === 'function' ? selector(analysisState) : analysisState,
    )
    mockUseStudyStore1608.mockImplementation(
      (selector?: (s: typeof studyState1608) => unknown) =>
        typeof selector === 'function' ? selector(studyState1608) : studyState1608,
    )
    useLayoutStore.setState({
      layoutMode: 'D',
      freeModeLayout: null,
      layoutLoadError: null,
      visibleCharts: new Set(),
      panelSizes: { leftPanel: 280, bottomPanel: 200 },
    })
  })

  test('TC-1608-01: sensitivity-heatmap renders SensitivityHeatmap', async () => {
    useLayoutStore.setState({
      freeModeLayout: {
        cells: [
          {
            cellId: 'heatmap-cell',
            chartId: 'sensitivity-heatmap',
            gridRow: [1, 3],
            gridCol: [1, 3],
          },
        ],
      },
    })

    await act(async () => {
      render(<FreeLayoutCanvas />)
    })

    expect(screen.getByTestId('sensitivity-heatmap')).toBeDefined()
  })

  test('TC-1608-02: importance renders ImportanceChart (Loading state)', async () => {
    analysisState.isComputingSensitivity = true
    useLayoutStore.setState({
      freeModeLayout: {
        cells: [
          { cellId: 'importance-cell', chartId: 'importance', gridRow: [1, 3], gridCol: [1, 3] },
        ],
      },
    })

    await act(async () => {
      render(<FreeLayoutCanvas />)
    })

    expect(screen.getByText('Loading...')).toBeDefined()
  })

  test('TC-1608-03: cluster-view renders ClusterScatter (EmptyState)', async () => {
    useLayoutStore.setState({
      freeModeLayout: {
        cells: [
          { cellId: 'cluster-cell', chartId: 'cluster-view', gridRow: [1, 3], gridCol: [1, 3] },
        ],
      },
    })

    await act(async () => {
      render(<FreeLayoutCanvas />)
    })

    expect(screen.getByText(/Run clustering/)).toBeDefined()
  })

  test('TC-1608-04: umap renders DimReductionScatter (Loading state)', async () => {
    clusterState1608.isRunning = true
    useLayoutStore.setState({
      freeModeLayout: {
        cells: [{ cellId: 'umap-cell', chartId: 'umap', gridRow: [1, 3], gridCol: [1, 3] }],
      },
    })

    await act(async () => {
      render(<FreeLayoutCanvas />)
    })

    expect(screen.getByText('Loading...')).toBeDefined()
  })
})
