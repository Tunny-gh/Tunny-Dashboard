/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 */

import { describe, test, expect, beforeEach, vi } from 'vitest'
import { render, screen, cleanup, fireEvent } from '@testing-library/react'
import '@testing-library/jest-dom'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

const { mockFreeModeLayout } = vi.hoisted(() => {
  const mockFreeModeLayout = {
    cells: [] as Array<{
      cellId: string
      chartId: string
      gridRow: [number, number]
      gridCol: [number, number]
    }>,
  }
  return { mockFreeModeLayout }
})

vi.mock('../../stores/layoutStore', () => ({
  useLayoutStore: vi
    .fn()
    .mockImplementation(
      (selector: (s: { freeModeLayout: typeof mockFreeModeLayout | null }) => unknown) =>
        selector({ freeModeLayout: mockFreeModeLayout }),
    ),
}))

import { ChartCatalogPanel, CHART_CATALOG } from './ChartCatalogPanel'
import { useLayoutStore } from '../../stores/layoutStore'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('ChartCatalogPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockFreeModeLayout.cells = []
    vi.mocked(useLayoutStore).mockImplementation((selector) =>
      selector({ freeModeLayout: mockFreeModeLayout }),
    )
    cleanup()
  })

  // Documentation.
  test('TC-CC-P01', () => {
    // Documentation.
    render(<ChartCatalogPanel />)
    expect(screen.getByTestId('catalog-toggle-btn')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-CC-P02', () => {
    // Documentation.
    render(<ChartCatalogPanel />)
    expect(screen.getByTestId('catalog-list')).not.toBeVisible()
  })

  // Documentation.
  test('TC-CC-P03', () => {
    // Documentation.
    render(<ChartCatalogPanel />)
    fireEvent.click(screen.getByTestId('catalog-toggle-btn'))
    expect(screen.getByTestId('catalog-list')).toBeVisible()
  })

  // Documentation.
  test('TC-CC-P04', () => {
    // Documentation.
    render(<ChartCatalogPanel />)
    fireEvent.click(screen.getByTestId('catalog-toggle-btn'))
    expect(CHART_CATALOG).toHaveLength(14)
    CHART_CATALOG.forEach(({ chartId }) => {
      expect(screen.getByTestId(`catalog-item-${chartId}`)).toBeInTheDocument()
    })
  })

  // Documentation.
  test('TC-CC-P05', () => {
    // Documentation.
    mockFreeModeLayout.cells = [
      { cellId: 'cell-1', chartId: 'slice', gridRow: [1, 3], gridCol: [1, 3] },
      { cellId: 'cell-2', chartId: 'slice', gridRow: [3, 5], gridCol: [1, 3] },
    ]
    vi.mocked(useLayoutStore).mockImplementation((selector) =>
      selector({ freeModeLayout: mockFreeModeLayout }),
    )
    render(<ChartCatalogPanel />)
    fireEvent.click(screen.getByTestId('catalog-toggle-btn'))
    expect(screen.getByTestId('catalog-item-slice')).toHaveAttribute('data-count', '2')
  })

  // Documentation.
  test('TC-CC-P06', () => {
    // Documentation.
    render(<ChartCatalogPanel />)
    fireEvent.click(screen.getByTestId('catalog-toggle-btn'))
    const sliceItem = screen.getByTestId('catalog-item-slice')
    expect(sliceItem).toHaveAttribute('data-count', '0')
    // Instance count text should not be shown when count is 0
    expect(sliceItem.textContent).not.toMatch(/\(\d+\)/)
  })

  // Documentation.
  test('TC-CC-P07', () => {
    // Documentation.
    vi.mocked(useLayoutStore).mockImplementation((selector) => selector({ freeModeLayout: null }))
    render(<ChartCatalogPanel />)
    fireEvent.click(screen.getByTestId('catalog-toggle-btn'))
    CHART_CATALOG.forEach(({ chartId }) => {
      expect(screen.getByTestId(`catalog-item-${chartId}`)).toHaveAttribute('data-count', '0')
    })
  })
})
