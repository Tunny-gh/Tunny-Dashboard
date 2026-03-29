/**
 * PDPChart tests (TASK-804)
 *
 * Target: PDPChart — Partial Dependence Plot UI component
 * Strategy:
 *   - Mock echarts-for-react with vi.mock (works in jsdom)
 *   - Verify loading state, empty state, R² warning, ONNX warning, and ICE highlighting
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest'
import { render, screen, cleanup, fireEvent } from '@testing-library/react'

// -------------------------------------------------------------------------
// echarts-for-react mock (uses __mocks__/echarts-for-react.tsx automatically)
// -------------------------------------------------------------------------

vi.mock('echarts-for-react')

import { PDPChart, getModelQuality } from './PDPChart'
import type { PdpData1d, PdpData2d } from './PDPChart'

// -------------------------------------------------------------------------
// Test helpers
// -------------------------------------------------------------------------

/** Creates simple 1D PDP data */
function makePdpData1d(overrides?: Partial<PdpData1d>): PdpData1d {
  return {
    paramName: 'x1',
    objectiveName: 'obj0',
    grid: [0.0, 0.25, 0.5, 0.75, 1.0],
    values: [0.1, 0.3, 0.5, 0.7, 0.9],
    rSquared: 0.85,
    ...overrides,
  }
}

/** Creates 1D PDP data with ICE lines */
function makePdpData1dWithIce(highlightCount = 2): PdpData1d {
  return {
    paramName: 'x1',
    objectiveName: 'obj0',
    grid: [0.0, 0.5, 1.0],
    values: [0.1, 0.5, 0.9],
    rSquared: 0.9,
    iceLines: [
      [0.08, 0.48, 0.88], // ICE line 0
      [0.12, 0.52, 0.92], // ICE line 1
      [0.05, 0.45, 0.85], // ICE line 2
    ].slice(0, highlightCount + 1),
  }
}

/** Creates 2D PDP data */
function makePdpData2d(): PdpData2d {
  return {
    param1Name: 'x1',
    param2Name: 'x2',
    objectiveName: 'obj0',
    grid1: [0.0, 0.5, 1.0],
    grid2: [0.0, 0.5, 1.0],
    values: [
      [0.1, 0.3, 0.5],
      [0.3, 0.5, 0.7],
      [0.5, 0.7, 0.9],
    ],
    rSquared: 0.88,
  }
}

// -------------------------------------------------------------------------
// Happy path
// -------------------------------------------------------------------------

describe('translated test case', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    cleanup()
  })

  // TC-804-01: renders without error when data is null
  test('TC-804-01', () => {
    expect(() => render(<PDPChart data1d={null} />)).not.toThrow()
  })

  // TC-804-02: ECharts is shown when data1d is provided
  test('TC-804-02', () => {
    render(<PDPChart data1d={makePdpData1d()} />)

    expect(screen.getByTestId('echarts')).toBeInTheDocument()
  })

  // TC-804-03: ECharts is shown when only data2d is provided
  test('TC-804-03', () => {
    render(<PDPChart data1d={null} data2d={makePdpData2d()} />)

    expect(screen.getByTestId('echarts')).toBeInTheDocument()
  })

  // TC-804-04: model quality panel is shown
  test('TC-804-04', () => {
    render(<PDPChart data1d={makePdpData1d({ rSquared: 0.85 })} />)

    expect(screen.getByTestId('model-quality-panel')).toBeInTheDocument()
    expect(screen.getByTestId('r2-value')).toBeInTheDocument()
    expect(screen.getByTestId('quality-label')).toBeInTheDocument()
  })

  // TC-804-05: R² value is shown in the panel
  test('TC-804-05', () => {
    render(<PDPChart data1d={makePdpData1d({ rSquared: 0.923 })} />)

    expect(screen.getByTestId('r2-value').textContent).toBe('0.923')
  })
})

// -------------------------------------------------------------------------
// Loading state
// -------------------------------------------------------------------------

describe('translated test case', () => {
  afterEach(() => {
    cleanup()
  })

  // TC-804-06: isLoading=true shows the loading indicator
  test('TC-804-06', () => {
    render(<PDPChart data1d={null} isLoading />)

    expect(screen.getByText('Computing PDP...')).toBeInTheDocument()
  })

  // TC-804-07: ECharts is hidden while loading
  test('TC-804-07', () => {
    render(<PDPChart data1d={makePdpData1d()} isLoading />)

    expect(screen.queryByTestId('echarts')).not.toBeInTheDocument()
  })
})

// -------------------------------------------------------------------------
// R² warning badge
// -------------------------------------------------------------------------

describe('translated test case', () => {
  afterEach(() => {
    cleanup()
  })

  // TC-804-08: R² < 0.8 shows the warning badge
  test('TC-804-08', () => {
    render(<PDPChart data1d={makePdpData1d({ rSquared: 0.65 })} />)

    expect(screen.getByTestId('r2-warning-badge')).toBeInTheDocument()
    expect(screen.getByText(/Caution: PDP interpretation may be unreliable/)).toBeInTheDocument()
  })

  // TC-804-09: warning badge includes the R² value
  test('TC-804-09', () => {
    render(<PDPChart data1d={makePdpData1d({ rSquared: 0.72 })} />)

    const badge = screen.getByTestId('r2-warning-badge')
    expect(badge.textContent).toContain('0.72')
  })

  // TC-804-10: R² >= 0.8 does not show the warning badge
  test('TC-804-10', () => {
    render(<PDPChart data1d={makePdpData1d({ rSquared: 0.85 })} />)

    expect(screen.queryByTestId('r2-warning-badge')).not.toBeInTheDocument()
  })
})

// -------------------------------------------------------------------------
// Linear approximation warning banner
// -------------------------------------------------------------------------

describe('translated test case', () => {
  afterEach(() => {
    cleanup()
  })

  // TC-804-11: useOnnx=false shows the warning banner
  test('TC-804-11', () => {
    render(<PDPChart data1d={makePdpData1d()} useOnnx={false} />)

    expect(screen.getByTestId('linear-approx-banner')).toBeInTheDocument()
    expect(screen.getByText(/Displaying linear approximation/)).toBeInTheDocument()
  })

  // TC-804-12: useOnnx=true hides the warning banner
  test('TC-804-12', () => {
    render(<PDPChart data1d={makePdpData1d()} useOnnx={true} />)

    expect(screen.queryByTestId('linear-approx-banner')).not.toBeInTheDocument()
  })

  // TC-804-13: ONNX load button shown when onOnnxRequest is provided
  test('TC-804-13', () => {
    const onOnnxRequest = vi.fn()
    render(<PDPChart data1d={makePdpData1d()} useOnnx={false} onOnnxRequest={onOnnxRequest} />)

    expect(screen.getByTestId('onnx-request-btn')).toBeInTheDocument()
  })

  // TC-804-14: clicking the ONNX button fires the callback
  test('TC-804-14', () => {
    const onOnnxRequest = vi.fn()
    render(<PDPChart data1d={makePdpData1d()} useOnnx={false} onOnnxRequest={onOnnxRequest} />)

    fireEvent.click(screen.getByTestId('onnx-request-btn'))

    expect(onOnnxRequest).toHaveBeenCalledOnce()
  })
})

// -------------------------------------------------------------------------
// ICE line highlight integration
// -------------------------------------------------------------------------

describe('translated test case', () => {
  afterEach(() => {
    cleanup()
  })

  // TC-804-15: passing highlightedIndices does not crash
  test('TC-804-15', () => {
    expect(() =>
      render(<PDPChart data1d={makePdpData1dWithIce(2)} highlightedIndices={[0, 1]} />),
    ).not.toThrow()
  })

  // TC-804-16: ECharts is still shown after highlightedIndices changes
  test('TC-804-16', () => {
    const { rerender } = render(
      <PDPChart data1d={makePdpData1dWithIce(2)} highlightedIndices={[]} />,
    )

    // Simulate a Brushing update
    rerender(<PDPChart data1d={makePdpData1dWithIce(2)} highlightedIndices={[0, 1]} />)

    expect(screen.getByTestId('echarts')).toBeInTheDocument()
  })

  // TC-804-17: no crash when iceLines is omitted
  test('TC-804-17', () => {
    const data = makePdpData1d() // no iceLines
    expect(() => render(<PDPChart data1d={data} highlightedIndices={[0, 1, 2]} />)).not.toThrow()
  })
})

// -------------------------------------------------------------------------
// Error cases
// -------------------------------------------------------------------------

describe('translated test case', () => {
  afterEach(() => {
    cleanup()
  })

  // TC-804-E01: no data and not loading shows empty state
  test('TC-804-E01', () => {
    render(<PDPChart data1d={null} isLoading={false} />)

    expect(screen.getByText('Data not loaded')).toBeInTheDocument()
  })

  // TC-804-E02: rSquared=0 shows "not recommended" quality label
  test('TC-804-E02', () => {
    render(<PDPChart data1d={makePdpData1d({ rSquared: 0.0 })} />)

    expect(screen.getByTestId('quality-label').textContent).toContain('Not Recommended')
  })
})

// -------------------------------------------------------------------------
// getModelQuality pure function tests
// -------------------------------------------------------------------------

describe('translated test case', () => {
  // Documentation.
  test('TC-804-Q01', () => {
    expect(getModelQuality(0.8)).toBe('Good')
    expect(getModelQuality(0.95)).toBe('Good')
    expect(getModelQuality(1.0)).toBe('Good')
  })

  // Documentation.
  test('TC-804-Q02', () => {
    expect(getModelQuality(0.5)).toBe('Caution')
    expect(getModelQuality(0.65)).toBe('Caution')
    expect(getModelQuality(0.79)).toBe('Caution')
  })

  // Documentation.
  test('TC-804-Q03', () => {
    expect(getModelQuality(0.0)).toBe('Not Recommended')
    expect(getModelQuality(0.3)).toBe('Not Recommended')
    expect(getModelQuality(0.49)).toBe('Not Recommended')
  })
})
