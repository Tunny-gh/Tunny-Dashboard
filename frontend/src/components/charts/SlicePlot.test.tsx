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

vi.mock('echarts-for-react')

const { mockGetIndicesSP } = vi.hoisted(() => {
  const mockGetIndicesSP = vi.fn().mockReturnValue(new Uint32Array(0))
  return { mockGetIndicesSP }
})

vi.mock('../../stores/downsampleStore', () => ({
  useDownsampleStore: vi.fn((selector: (s: { getIndices: typeof mockGetIndicesSP }) => unknown) =>
    selector({ getIndices: mockGetIndicesSP }),
  ),
}))

import { SlicePlot } from './SlicePlot'
import type { SliceTrial } from './SlicePlot'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

const SAMPLE_TRIALS: SliceTrial[] = [
  { trialId: 1, params: { x: 0.1, y: 0.5 }, values: [0.3], paretoRank: 1 },
  { trialId: 2, params: { x: 0.4, y: 0.2 }, values: [0.6], paretoRank: 2 },
  { trialId: 3, params: { x: 0.7, y: 0.8 }, values: [0.1], paretoRank: 1 },
]

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  beforeEach(() => vi.clearAllMocks())
  afterEach(() => cleanup())

  // Documentation.
  test('TC-SLICE-01', () => {
    // Documentation.
    render(<SlicePlot trials={SAMPLE_TRIALS} paramNames={['x', 'y']} objectiveNames={['obj1']} />)

    // Documentation.
    expect(screen.getByTestId('slice-plot')).toBeInTheDocument()
    expect(screen.getByTestId('echarts')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-SLICE-02', () => {
    // Documentation.
    render(<SlicePlot trials={SAMPLE_TRIALS} paramNames={['x', 'y']} objectiveNames={['obj1']} />)

    // Documentation.
    expect(screen.getByTestId('slice-param-select')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-SLICE-03', () => {
    // Documentation.
    render(<SlicePlot trials={SAMPLE_TRIALS} paramNames={['x', 'y']} objectiveNames={['obj1']} />)

    const select = screen.getByTestId('slice-param-select') as HTMLSelectElement
    fireEvent.change(select, { target: { value: '1' } })

    // Documentation.
    const option = JSON.parse(screen.getByTestId('echarts').getAttribute('data-option') ?? '{}')
    expect(option.xAxis.name).toBe('y')
  })

  // Documentation.
  test('TC-SLICE-04', () => {
    // Documentation.
    const multiObjTrials = SAMPLE_TRIALS.map((t) => ({ ...t, values: [0.3, 0.5] }))
    render(
      <SlicePlot
        trials={multiObjTrials}
        paramNames={['x', 'y']}
        objectiveNames={['obj1', 'obj2']}
      />,
    )

    // Documentation.
    expect(screen.getByTestId('slice-obj-select')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-SLICE-05', () => {
    // Documentation.
    render(<SlicePlot trials={SAMPLE_TRIALS} paramNames={['x', 'y']} objectiveNames={['obj1']} />)

    const option = JSON.parse(screen.getByTestId('echarts').getAttribute('data-option') ?? '{}')
    // Documentation.
    expect(option.xAxis.name).toBe('x')
  })

  // Documentation.
  test('TC-SLICE-06', () => {
    // Documentation.
    render(<SlicePlot trials={SAMPLE_TRIALS} paramNames={['x']} objectiveNames={['cost']} />)

    const option = JSON.parse(screen.getByTestId('echarts').getAttribute('data-option') ?? '{}')
    // Documentation.
    expect(option.yAxis.name).toBe('cost')
  })
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  beforeEach(() => vi.clearAllMocks())
  afterEach(() => cleanup())

  // Documentation.
  test('TC-SLICE-E01', () => {
    // Documentation.
    render(<SlicePlot trials={[]} paramNames={['x']} objectiveNames={['obj']} />)

    // Documentation.
    expect(screen.getByText('No data available')).toBeInTheDocument()
    expect(screen.queryByTestId('echarts')).toBeNull()
  })

  // Documentation.
  test('TC-SLICE-E02', () => {
    // Documentation.
    render(<SlicePlot trials={SAMPLE_TRIALS} paramNames={[]} objectiveNames={['obj']} />)

    // Documentation.
    expect(screen.getByText('No data available')).toBeInTheDocument()
  })
})

// -------------------------------------------------------------------------
// TASK-1669: data_points index integration
// -------------------------------------------------------------------------

describe('SlicePlot - data_points index integration (TASK-1669)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockGetIndicesSP.mockReturnValue(new Uint32Array(0))
  })

  afterEach(() => {
    cleanup()
  })

  test('TC-1669-01: getIndices called with "data_points"', () => {
    render(<SlicePlot trials={SAMPLE_TRIALS} paramNames={['x']} objectiveNames={['obj1']} />)
    expect(mockGetIndicesSP).toHaveBeenCalledWith('data_points')
  })

  test('TC-1669-02: data_points indices filter trials', () => {
    // Only show trial at index 1 (x=0.4, value=0.6)
    mockGetIndicesSP.mockImplementation((key: string) => {
      if (key === 'data_points') return new Uint32Array([1])
      return new Uint32Array(0)
    })

    render(<SlicePlot trials={SAMPLE_TRIALS} paramNames={['x']} objectiveNames={['obj1']} />)

    const option = JSON.parse(screen.getByTestId('echarts').getAttribute('data-option') ?? '{}')
    const allData = option.series.flatMap((s: { data: unknown[] }) => s.data ?? [])
    expect(allData).toHaveLength(1)
    expect(allData[0][0]).toBeCloseTo(0.4)
  })
})
