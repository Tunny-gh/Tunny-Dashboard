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

const { mockLoadJournal } = vi.hoisted(() => {
  const mockLoadJournal = vi.fn().mockResolvedValue(undefined)
  return { mockLoadJournal }
})

vi.mock('../../stores/studyStore', () => ({
  useStudyStore: vi
    .fn()
    .mockImplementation(
      (selector: (s: { loadJournal: typeof mockLoadJournal; isLoading: boolean }) => unknown) =>
        selector({ loadJournal: mockLoadJournal, isLoading: false }),
    ),
}))

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

const { mockSetLayoutMode } = vi.hoisted(() => {
  const mockSetLayoutMode = vi.fn()
  return { mockSetLayoutMode }
})

vi.mock('../../stores/layoutStore', () => ({
  useLayoutStore: vi
    .fn()
    .mockImplementation(
      (selector: (s: { layoutMode: string; setLayoutMode: typeof mockSetLayoutMode }) => unknown) =>
        selector({ layoutMode: 'A', setLayoutMode: mockSetLayoutMode }),
    ),
}))

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

vi.mock('./ToolBar', () => ({ ToolBar: () => <div data-testid="toolbar-mock" /> }))
vi.mock('../panels/LeftPanel', () => ({ LeftPanel: () => <div data-testid="left-panel-mock" /> }))
vi.mock('../panels/BottomPanel', () => ({
  BottomPanel: () => <div data-testid="bottom-panel-mock" />,
}))
vi.mock('./FreeLayoutCanvas', () => ({ FreeLayoutCanvas: () => <div data-testid="canvas-mock" /> }))
vi.mock('./ChartCatalogPanel', () => ({
  ChartCatalogPanel: () => <div data-testid="catalog-toggle-btn" />,
}))

import { AppShell } from './AppShell'
import { useStudyStore } from '../../stores/studyStore'

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
  test('TC-401-01', () => {
    // Documentation.
    expect(() => render(<AppShell />)).not.toThrow()
  })

  test('TC-401-01', () => {
    // Documentation.
    render(<AppShell />)
    const shell = screen.getByTestId('app-shell')
    expect(shell).toHaveAttribute('data-layout', 'A')
  })

  // Documentation.
  test('TC-401-08', () => {
    // Documentation.
    render(<AppShell />)
    expect(screen.getByTestId('catalog-toggle-btn')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-401-09', () => {
    // Documentation.
    render(<AppShell />)
    expect(screen.getByTestId('toolbar-mock')).toBeInTheDocument()
    expect(screen.getByTestId('left-panel-mock')).toBeInTheDocument()
    expect(screen.getByTestId('canvas-mock')).toBeInTheDocument()
    expect(screen.getByTestId('bottom-panel-mock')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-401-02', () => {
    // Documentation.
    render(<AppShell />)
    const shell = screen.getByTestId('app-shell')

    // Documentation.
    const file = new File(['content'], 'journal.log', { type: 'text/plain' })
    const dataTransfer = { files: [file] }

    // Documentation.
    fireEvent.dragOver(shell, { preventDefault: vi.fn() })
    fireEvent.drop(shell, { dataTransfer })

    // Documentation.
    expect(mockLoadJournal).toHaveBeenCalledOnce()
    expect(mockLoadJournal).toHaveBeenCalledWith(file)
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
  test('TC-401-E01', () => {
    // Documentation.

    // Documentation.
    // Documentation.
    ;(useStudyStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (s: { loadJournal: typeof mockLoadJournal; isLoading: boolean }) => unknown) =>
        selector({ loadJournal: mockLoadJournal, isLoading: true }),
    )

    // Documentation.
    render(<AppShell />)

    // Documentation.
    expect(screen.getByTestId('loading-indicator')).toBeInTheDocument()
  })
})
