/**
 * ContourPlot tests
 *
 * Target: ContourPlot — 2-parameter correlation scatter plot (simplified contour plot)
 * Strategy: mock echarts-for-react with vi.mock
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest'
import { render, screen, cleanup, fireEvent } from '@testing-library/react'

// -------------------------------------------------------------------------
// echarts-for-react mock (uses __mocks__/echarts-for-react.tsx automatically)
// -------------------------------------------------------------------------

vi.mock('echarts-for-react')

import { ContourPlot } from './ContourPlot'
import type { ContourTrial } from './ContourPlot'

// -------------------------------------------------------------------------
// Test data
// -------------------------------------------------------------------------

const SAMPLE_TRIALS: ContourTrial[] = [
  { params: { x: 0.1, y: 0.2 }, values: [0.5] },
  { params: { x: 0.5, y: 0.6 }, values: [0.3] },
  { params: { x: 0.9, y: 0.1 }, values: [0.8] },
]

// -------------------------------------------------------------------------
// Happy path
// -------------------------------------------------------------------------

describe('ContourPlot — 正常系', () => {
  beforeEach(() => vi.clearAllMocks())
  afterEach(() => cleanup())

  // TC-CONTOUR-01: with data, ECharts is rendered
  test('TC-CONTOUR-01: データありで contour-plot コンテナと ECharts が表示される', () => {
    render(<ContourPlot trials={SAMPLE_TRIALS} paramNames={['x', 'y']} objectiveNames={['obj']} />)

    expect(screen.getByTestId('contour-plot')).toBeInTheDocument()
    expect(screen.getByTestId('echarts')).toBeInTheDocument()
  })

  // TC-CONTOUR-02: Python warning banner is shown
  test('TC-CONTOUR-02: Python/scikit-learn 注意バナーが表示される', () => {
    render(<ContourPlot trials={SAMPLE_TRIALS} paramNames={['x', 'y']} objectiveNames={['obj']} />)

    expect(screen.getByTestId('contour-note')).toBeInTheDocument()
  })

  // TC-CONTOUR-03: X/Y parameter dropdowns are shown
  test('TC-CONTOUR-03: X / Y パラメータ選択ドロップダウンが表示される', () => {
    render(<ContourPlot trials={SAMPLE_TRIALS} paramNames={['x', 'y']} objectiveNames={['obj']} />)

    expect(screen.getByTestId('contour-x-select')).toBeInTheDocument()
    expect(screen.getByTestId('contour-y-select')).toBeInTheDocument()
  })

  // TC-CONTOUR-04: initial xAxis name is paramNames[0]
  test('TC-CONTOUR-04: 初期表示で xAxis 名が paramNames[0] になる', () => {
    render(<ContourPlot trials={SAMPLE_TRIALS} paramNames={['x', 'y']} objectiveNames={['obj']} />)

    const option = JSON.parse(screen.getByTestId('echarts').getAttribute('data-option') ?? '{}')
    expect(option.xAxis.name).toBe('x')
  })

  // TC-CONTOUR-05: changing X selection updates the axis name
  test('TC-CONTOUR-05: X を 3 番目パラメータに変更後 xAxis 名が更新される', () => {
    render(
      <ContourPlot
        trials={[{ params: { x: 1, y: 2, z: 3 }, values: [0.5] }]}
        paramNames={['x', 'y', 'z']}
        objectiveNames={['obj']}
      />,
    )

    fireEvent.change(screen.getByTestId('contour-x-select'), { target: { value: '2' } })

    const option = JSON.parse(screen.getByTestId('echarts').getAttribute('data-option') ?? '{}')
    expect(option.xAxis.name).toBe('z')
  })

  // TC-CONTOUR-06: objective selector shown for multi-objective
  test('TC-CONTOUR-06: 目的関数が 2 つのとき目的関数選択ドロップダウンが表示される', () => {
    const multiObjTrials = SAMPLE_TRIALS.map((t) => ({ ...t, values: [0.5, 0.3] }))
    render(
      <ContourPlot
        trials={multiObjTrials}
        paramNames={['x', 'y']}
        objectiveNames={['obj1', 'obj2']}
      />,
    )

    expect(screen.getByTestId('contour-obj-select')).toBeInTheDocument()
  })

  // TC-CONTOUR-07: scatter data length matches valid trial count
  test('TC-CONTOUR-07: scatter series の data 点数が有効トライアル数と一致する', () => {
    render(<ContourPlot trials={SAMPLE_TRIALS} paramNames={['x', 'y']} objectiveNames={['obj']} />)

    const option = JSON.parse(screen.getByTestId('echarts').getAttribute('data-option') ?? '{}')
    expect(option.series[0].data).toHaveLength(3)
  })
})

// -------------------------------------------------------------------------
// Error cases
// -------------------------------------------------------------------------

describe('ContourPlot — 異常系', () => {
  beforeEach(() => vi.clearAllMocks())
  afterEach(() => cleanup())

  // TC-CONTOUR-E01: trials=[] shows empty state
  test('TC-CONTOUR-E01: trials=[] のとき空状態UIを表示する', () => {
    render(<ContourPlot trials={[]} paramNames={['x', 'y']} objectiveNames={['obj']} />)

    expect(screen.getByText(/No data available/)).toBeInTheDocument()
    expect(screen.queryByTestId('echarts')).toBeNull()
  })

  // TC-CONTOUR-E02: only one paramName shows empty state
  test('TC-CONTOUR-E02: paramNames が 1 つのとき空状態UIを表示する（2 つ必要）', () => {
    render(<ContourPlot trials={SAMPLE_TRIALS} paramNames={['x']} objectiveNames={['obj']} />)

    expect(screen.getByText(/No data available/)).toBeInTheDocument()
  })
})
