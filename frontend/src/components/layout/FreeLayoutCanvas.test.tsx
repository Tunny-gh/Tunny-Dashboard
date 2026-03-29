/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 */

import { describe, test, expect, beforeEach, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import '@testing-library/jest-dom'
import { useLayoutStore } from '../../stores/layoutStore'
import { FreeLayoutCanvas } from './FreeLayoutCanvas'
import type { FreeModeLayout } from '../../types'

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
