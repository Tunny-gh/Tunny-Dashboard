/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest'
import { render, screen, cleanup } from '@testing-library/react'

// -------------------------------------------------------------------------
// Documentation.
// Documentation.
// -------------------------------------------------------------------------

const { mockReactEChartsPC, captureOnEvents } = vi.hoisted(() => {
  // Documentation.
  let capturedOnEvents: Record<string, (params: unknown) => void> = {}
  const captureOnEvents = () => capturedOnEvents

  const mockReactEChartsPC = vi.fn(
    ({
      option,
      onEvents,
    }: {
      option: unknown
      onEvents?: Record<string, (p: unknown) => void>
    }) => {
      // Documentation.
      if (onEvents) capturedOnEvents = onEvents
      return <div data-testid="echarts-pc" data-option={JSON.stringify(option)} />
    },
  )
  return { mockReactEChartsPC, captureOnEvents }
})

vi.mock('echarts-for-react', () => ({
  default: mockReactEChartsPC,
}))

// -------------------------------------------------------------------------
// Documentation.
// Documentation.
// -------------------------------------------------------------------------

const { mockAddAxisFilter, mockRemoveAxisFilter } = vi.hoisted(() => {
  const mockAddAxisFilter = vi.fn()
  const mockRemoveAxisFilter = vi.fn()
  return { mockAddAxisFilter, mockRemoveAxisFilter }
})

vi.mock('../../stores/selectionStore', () => ({
  useSelectionStore: vi
    .fn()
    .mockImplementation((selector?: (s: Record<string, unknown>) => unknown) => {
      const state = {
        addAxisFilter: mockAddAxisFilter,
        removeAxisFilter: mockRemoveAxisFilter,
        selectedIndices: new Uint32Array(0),
      }
      return selector ? selector(state) : state
    }),
}))

import { ParallelCoordinates } from './ParallelCoordinates'
import { useStudyStore } from '../../stores/studyStore'
import type { GpuBuffer } from '../../wasm/gpuBuffer'
import type { Study } from '../../types'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 */
function makeGpuBuffer(n = 5): GpuBuffer {
  return {
    trialCount: n,
    positions: new Float32Array(n * 2),
    positions3d: new Float32Array(n * 3),
    colors: new Float32Array(n * 4),
    sizes: new Float32Array(n),
    updateAlphas: vi.fn(),
    resetAlphas: vi.fn(),
  } as unknown as GpuBuffer
}

/**
 * Documentation.
 */
function makeStudy(opts: { paramNames?: string[]; objectiveNames?: string[] } = {}): Study {
  return {
    studyId: 1,
    name: 'test-study',
    directions: ['minimize', 'minimize'],
    completedTrials: 5,
    totalTrials: 5,
    paramNames: opts.paramNames ?? ['x1', 'x2', 'x3'],
    objectiveNames: opts.objectiveNames ?? ['obj1', 'obj2'],
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
    useStudyStore.setState({ trialRows: [] })
  })

  afterEach(() => {
    cleanup()
  })

  // Documentation.
  test('TC-601-01', () => {
    // Documentation.
    expect(() => render(<ParallelCoordinates gpuBuffer={null} currentStudy={null} />)).not.toThrow()
  })

  // Documentation.
  test('TC-601-02', () => {
    // Documentation.
    render(<ParallelCoordinates gpuBuffer={makeGpuBuffer()} currentStudy={makeStudy()} />)
    expect(screen.getByTestId('echarts-pc')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-601-03', () => {
    // Documentation.
    const study = makeStudy({ paramNames: ['x1', 'x2'], objectiveNames: ['obj1'] })

    // Documentation.
    render(<ParallelCoordinates gpuBuffer={makeGpuBuffer()} currentStudy={study} />)

    // Documentation.
    const el = screen.getByTestId('echarts-pc')
    const option = JSON.parse(el.getAttribute('data-option') ?? '{}') as {
      parallelAxis: Array<{ name: string }>
    }

    // Documentation.
    const axisNames = option.parallelAxis.map((a) => a.name)
    expect(axisNames).toContain('x1')
    expect(axisNames).toContain('x2')
    expect(axisNames).toContain('obj1')
  })

  // Documentation.
  test('TC-601-04', () => {
    // Documentation.
    const study = makeStudy({ paramNames: ['x1', 'x2'], objectiveNames: [] })

    // Documentation.
    render(<ParallelCoordinates gpuBuffer={makeGpuBuffer()} currentStudy={study} />)

    // Documentation.
    // axisIndex=0 → 'x1', intervals=[[0.2, 0.8]]
    const onEvents = captureOnEvents()
    onEvents['axisareaselected']({
      axesInfo: [{ axisIndex: 0, intervals: [[0.2, 0.8]] }],
    })

    // Documentation.
    expect(mockAddAxisFilter).toHaveBeenCalledWith('x1', 0.2, 0.8)
  })

  // Documentation.
  test('TC-601-05', () => {
    const study = makeStudy({ paramNames: ['x1', 'x2'], objectiveNames: ['obj1'] })
    useStudyStore.setState({
      trialRows: [
        {
          trialId: 1,
          params: { x1: 10, x2: 20 },
          values: [0.5],
          paretoRank: 0,
        },
        {
          trialId: 2,
          params: { x1: 11, x2: 21 },
          values: [0.6],
          paretoRank: 1,
        },
      ],
    })

    render(<ParallelCoordinates gpuBuffer={makeGpuBuffer(2)} currentStudy={study} />)

    const el = screen.getByTestId('echarts-pc')
    const option = JSON.parse(el.getAttribute('data-option') ?? '{}') as {
      series: Array<{ data: number[][] }>
    }

    expect(option.series[0]?.data).toEqual([
      [10, 20, 0.5],
      [11, 21, 0.6],
    ])
  })
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    useStudyStore.setState({ trialRows: [] })
  })

  afterEach(() => {
    cleanup()
  })

  // Documentation.
  test('TC-601-E01', () => {
    // Documentation.
    render(<ParallelCoordinates gpuBuffer={null} currentStudy={null} />)
    expect(screen.getByText('Data not loaded')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-601-E02', () => {
    // Documentation.
    render(<ParallelCoordinates gpuBuffer={makeGpuBuffer()} currentStudy={null} />)
    expect(screen.getByText('Data not loaded')).toBeInTheDocument()
  })
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    useStudyStore.setState({ trialRows: [] })
  })

  afterEach(() => {
    cleanup()
  })

  // Documentation.
  test('TC-601-B01', () => {
    // Documentation.
    const paramNames = Array.from({ length: 30 }, (_, i) => `x${i + 1}`)
    const objectiveNames = ['obj1', 'obj2', 'obj3', 'obj4']
    const study = makeStudy({ paramNames, objectiveNames })

    // Documentation.
    expect(() =>
      render(<ParallelCoordinates gpuBuffer={makeGpuBuffer(50)} currentStudy={study} />),
    ).not.toThrow()

    // Documentation.
    const el = screen.getByTestId('echarts-pc')
    const option = JSON.parse(el.getAttribute('data-option') ?? '{}') as {
      parallelAxis: unknown[]
    }
    expect(option.parallelAxis).toHaveLength(34)
  })
})
