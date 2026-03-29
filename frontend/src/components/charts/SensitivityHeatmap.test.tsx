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

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

vi.mock('echarts-for-react')

import { SensitivityHeatmap } from './SensitivityHeatmap'
import type { SensitivityData } from './SensitivityHeatmap'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/** Documentation. */
function makeSensitivityData(): SensitivityData {
  return {
    paramNames: ['x1', 'x2'],
    objectiveNames: ['obj0', 'obj1'],
    // spearman[param_idx][obj_idx]
    spearman: [
      [0.8, -0.3], // Documentation.
      [0.1, 0.9], // Documentation.
    ],
    ridge: [
      { beta: [0.7, 0.1], rSquared: 0.85 }, // obj0
      { beta: [-0.2, 0.8], rSquared: 0.92 }, // obj1
    ],
  }
}

/** Documentation. */
function makeLowCorrelationData(): SensitivityData {
  return {
    paramNames: ['x1', 'x2'],
    objectiveNames: ['obj0'],
    spearman: [
      [0.1], // Documentation.
      [0.05], // Documentation.
    ],
    ridge: [{ beta: [0.05, 0.03], rSquared: 0.02 }],
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
  test('TC-802-01', () => {
    // Documentation.
    expect(() =>
      render(<SensitivityHeatmap data={null} metric="spearman" threshold={0} />),
    ).not.toThrow()
  })

  // Documentation.
  test('TC-802-02', () => {
    // Documentation.
    render(<SensitivityHeatmap data={makeSensitivityData()} metric="spearman" threshold={0} />)

    // Documentation.
    expect(screen.getByTestId('echarts')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-802-03', () => {
    // Documentation.
    render(<SensitivityHeatmap data={makeSensitivityData()} metric="spearman" threshold={0} />)

    // Documentation.
    expect(screen.getByTestId('metric-btn-spearman')).toBeInTheDocument()
    expect(screen.getByTestId('metric-btn-beta')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-802-04', () => {
    // Documentation.
    render(<SensitivityHeatmap data={makeSensitivityData()} metric="spearman" threshold={0} />)

    // Documentation.
    expect(screen.getByTestId('metric-btn-spearman')).toHaveAttribute('aria-pressed', 'true')
    expect(screen.getByTestId('metric-btn-beta')).toHaveAttribute('aria-pressed', 'false')
  })

  // Documentation.
  test('TC-802-05', () => {
    // Documentation.
    render(<SensitivityHeatmap data={makeSensitivityData()} metric="spearman" threshold={0.3} />)

    // Documentation.
    const slider = screen.getByTestId('threshold-slider')
    expect(slider).toBeInTheDocument()
    expect(slider).toHaveValue('0.3')
  })

  // Documentation.
  test('TC-802-06', () => {
    // Documentation.
    const onThresholdChange = vi.fn()
    render(
      <SensitivityHeatmap
        data={makeSensitivityData()}
        metric="spearman"
        threshold={0}
        onThresholdChange={onThresholdChange}
      />,
    )

    // Documentation.
    fireEvent.change(screen.getByTestId('threshold-slider'), {
      target: { value: '0.5' },
    })

    // Documentation.
    expect(onThresholdChange).toHaveBeenCalledWith(0.5)
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
  test('TC-802-07', () => {
    // Documentation.
    render(<SensitivityHeatmap data={null} metric="spearman" threshold={0} isLoading />)

    // Documentation.
    expect(screen.getByText('Computing (WASM)...')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-802-08', () => {
    // Documentation.
    render(<SensitivityHeatmap data={null} metric="spearman" threshold={0} isLoading />)

    // Documentation.
    expect(screen.queryByTestId('echarts')).not.toBeInTheDocument()
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
  test('TC-802-E01', () => {
    // Documentation.
    render(<SensitivityHeatmap data={null} metric="spearman" threshold={0} isLoading={false} />)

    // Documentation.
    expect(screen.getByText('Data not loaded')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-802-E02', () => {
    // Documentation.
    render(
      <SensitivityHeatmap
        data={makeLowCorrelationData()}
        metric="spearman"
        threshold={0.9} // Documentation.
      />,
    )

    // Documentation.
    expect(screen.getByTestId('echarts')).toBeInTheDocument()
  })
})
