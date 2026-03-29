/**
 * EdfPlot tests
 *
 * Target: EdfPlot — Empirical CDF chart
 * Strategy: mock echarts-for-react with vi.mock; test computeEdf as a pure function
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest'
import { render, screen, cleanup } from '@testing-library/react'

// -------------------------------------------------------------------------
// echarts-for-react mock (uses __mocks__/echarts-for-react.tsx automatically)
// -------------------------------------------------------------------------

vi.mock('echarts-for-react')

import { EdfPlot, computeEdf } from './EdfPlot'

// -------------------------------------------------------------------------
// computeEdf pure function tests
// -------------------------------------------------------------------------

describe('translated test case', () => {
  // TC-EDF-PURE-01: empty array
  test('TC-EDF-PURE-01', () => {
    expect(computeEdf([])).toEqual([])
  })

  // TC-EDF-PURE-02: single element
  test('TC-EDF-PURE-02', () => {
    expect(computeEdf([5])).toEqual([[5, 1.0]])
  })

  // TC-EDF-PURE-03: multiple elements sorted and cumulative probability computed
  test('TC-EDF-PURE-03', () => {
    const result = computeEdf([3, 1, 2])

    expect(result).toHaveLength(3)
    expect(result[0]).toEqual([1, 1 / 3])
    expect(result[1]).toEqual([2, 2 / 3])
    expect(result[2]).toEqual([3, 1.0])
  })

  // TC-EDF-PURE-04: duplicate values still produce monotonically increasing CDF
  test('TC-EDF-PURE-04', () => {
    const result = computeEdf([1, 1, 2])

    expect(result[0][1]).toBeLessThan(result[1][1])
    expect(result[1][1]).toBeLessThan(result[2][1])
    expect(result[2][1]).toBe(1.0)
  })
})

// -------------------------------------------------------------------------
// Happy path
// -------------------------------------------------------------------------

describe('translated test case', () => {
  beforeEach(() => vi.clearAllMocks())
  afterEach(() => cleanup())

  // TC-EDF-01: with data, ECharts is rendered
  test('TC-EDF-01', () => {
    render(<EdfPlot series={[{ name: 'obj1', values: [0.1, 0.5, 0.3] }]} />)

    expect(screen.getByTestId('edf-plot')).toBeInTheDocument()
    expect(screen.getByTestId('echarts')).toBeInTheDocument()
  })

  // TC-EDF-02: series name is passed to legend
  test('TC-EDF-02', () => {
    render(<EdfPlot series={[{ name: 'cost', values: [1, 2, 3] }]} />)

    const option = JSON.parse(screen.getByTestId('echarts').getAttribute('data-option') ?? '{}')
    expect(option.legend.data).toContain('cost')
  })

  // TC-EDF-03: data is passed as a step='end' line series
  test('TC-EDF-03', () => {
    render(<EdfPlot series={[{ name: 'obj', values: [2, 1, 3] }]} />)

    const option = JSON.parse(screen.getByTestId('echarts').getAttribute('data-option') ?? '{}')
    expect(option.series[0].step).toBe('end')
    expect(option.series[0].data[0][0]).toBeLessThanOrEqual(option.series[0].data[1][0])
  })

  // TC-EDF-04: multiple series render as multiple lines
  test('TC-EDF-04', () => {
    render(
      <EdfPlot
        series={[
          { name: 'obj1', values: [1, 2, 3] },
          { name: 'obj2', values: [4, 5, 6] },
        ]}
      />,
    )

    const option = JSON.parse(screen.getByTestId('echarts').getAttribute('data-option') ?? '{}')
    expect(option.series).toHaveLength(2)
  })
})

// -------------------------------------------------------------------------
// Error cases
// -------------------------------------------------------------------------

describe('translated test case', () => {
  beforeEach(() => vi.clearAllMocks())
  afterEach(() => cleanup())

  // TC-EDF-E01: series=[] shows empty state
  test('TC-EDF-E01', () => {
    render(<EdfPlot series={[]} />)

    expect(screen.getByText('No data available')).toBeInTheDocument()
    expect(screen.queryByTestId('echarts')).toBeNull()
  })

  // TC-EDF-E02: all series with empty values shows empty state
  test('TC-EDF-E02', () => {
    render(<EdfPlot series={[{ name: 'obj', values: [] }]} />)

    expect(screen.getByText('No data available')).toBeInTheDocument()
  })
})
