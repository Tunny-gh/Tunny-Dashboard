/**
 * LayoutTabBar tests (TASK-001)
 *
 * Tests the LayoutTabBar tab-based layout switcher.
 * layoutStore is mocked with vi.mock.
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest'
import { render, screen, cleanup, fireEvent } from '@testing-library/react'

// -------------------------------------------------------------------------
// layoutStore mock
// -------------------------------------------------------------------------

const { mockSetLayoutMode, mockSetFreeModeLayout } = vi.hoisted(() => ({
  mockSetLayoutMode: vi.fn(),
  mockSetFreeModeLayout: vi.fn(),
}))

vi.mock('../../stores/layoutStore', () => ({
  useLayoutStore: vi
    .fn()
    .mockImplementation(
      (
        selector: (s: {
          layoutMode: string
          setLayoutMode: typeof mockSetLayoutMode
          setFreeModeLayout: typeof mockSetFreeModeLayout
        }) => unknown,
      ) =>
        selector({
          layoutMode: 'A',
          setLayoutMode: mockSetLayoutMode,
          setFreeModeLayout: mockSetFreeModeLayout,
        }),
    ),
}))

import { LayoutTabBar } from './LayoutTabBar'
import { useLayoutStore } from '../../stores/layoutStore'

// -------------------------------------------------------------------------
// Happy path
// -------------------------------------------------------------------------

describe('LayoutTabBar — happy path', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(useLayoutStore).mockImplementation((selector) =>
      selector({
        layoutMode: 'A',
        setLayoutMode: mockSetLayoutMode,
        setFreeModeLayout: mockSetFreeModeLayout,
      }),
    )
  })

  afterEach(() => {
    cleanup()
  })

  // TC-LT-01: layout-tab-bar container is present in the DOM
  test('TC-LT-01: data-testid="layout-tab-bar" exists', () => {
    render(<LayoutTabBar />)
    expect(screen.getByTestId('layout-tab-bar')).toBeInTheDocument()
  })

  // TC-LT-02: all four tabs (A–D) are rendered
  test('TC-LT-02: layout-tab-A through layout-tab-D all exist', () => {
    render(<LayoutTabBar />)
    expect(screen.getByTestId('layout-tab-A')).toBeInTheDocument()
    expect(screen.getByTestId('layout-tab-B')).toBeInTheDocument()
    expect(screen.getByTestId('layout-tab-C')).toBeInTheDocument()
    expect(screen.getByTestId('layout-tab-D')).toBeInTheDocument()
  })

  // TC-LT-03: each tab shows its descriptive label
  test('TC-LT-03', () => {
    // REQ-002 — descriptive labels instead of opaque A/B/C/D
    render(<LayoutTabBar />)
    expect(screen.getByTestId('layout-tab-A')).toHaveTextContent('Quad')
    expect(screen.getByTestId('layout-tab-B')).toHaveTextContent('Left Main')
    expect(screen.getByTestId('layout-tab-C')).toHaveTextContent('Vertical')
    expect(screen.getByTestId('layout-tab-D')).toHaveTextContent('Free')
  })

  // TC-LT-04: when layoutMode=B, layout-tab-B has aria-selected="true"
  test('TC-LT-04: when layoutMode=B, layout-tab-B aria-selected is "true" and others are "false"', () => {
    // REQ-105 — active tab is visually distinguished
    vi.mocked(useLayoutStore).mockImplementation((selector) =>
      selector({
        layoutMode: 'B',
        setLayoutMode: mockSetLayoutMode,
        setFreeModeLayout: mockSetFreeModeLayout,
      }),
    )
    render(<LayoutTabBar />)
    expect(screen.getByTestId('layout-tab-B')).toHaveAttribute('aria-selected', 'true')
    expect(screen.getByTestId('layout-tab-A')).toHaveAttribute('aria-selected', 'false')
    expect(screen.getByTestId('layout-tab-C')).toHaveAttribute('aria-selected', 'false')
    expect(screen.getByTestId('layout-tab-D')).toHaveAttribute('aria-selected', 'false')
  })

  // TC-LT-05: clicking a preset tab calls both setLayoutMode and setFreeModeLayout
  test('TC-LT-05: clicking layout-tab-A calls setLayoutMode("A") and setFreeModeLayout', () => {
    // REQ-101 — tab click switches mode and applies layout simultaneously
    // Precondition: layoutMode = 'B' so tab A is inactive and clickable
    vi.mocked(useLayoutStore).mockImplementation((selector) =>
      selector({
        layoutMode: 'B',
        setLayoutMode: mockSetLayoutMode,
        setFreeModeLayout: mockSetFreeModeLayout,
      }),
    )
    render(<LayoutTabBar />)
    fireEvent.click(screen.getByTestId('layout-tab-A'))
    expect(mockSetLayoutMode).toHaveBeenCalledOnce()
    expect(mockSetLayoutMode).toHaveBeenCalledWith('A')
    expect(mockSetFreeModeLayout).toHaveBeenCalledOnce()
  })

  // TC-LT-06: clicking the free tab calls setLayoutMode only
  test('TC-LT-06: clicking layout-tab-D calls setLayoutMode("D") and does not call setFreeModeLayout', () => {
    // REQ-104 — switching to free mode must not modify freeModeLayout
    // Precondition: layoutMode = 'A' so tab D is inactive
    render(<LayoutTabBar />)
    fireEvent.click(screen.getByTestId('layout-tab-D'))
    expect(mockSetLayoutMode).toHaveBeenCalledOnce()
    expect(mockSetLayoutMode).toHaveBeenCalledWith('D')
    expect(mockSetFreeModeLayout).not.toHaveBeenCalled()
  })

  // TC-LT-07: re-clicking the active tab calls nothing (idempotent)
  test('TC-LT-07: clicking the active tab calls neither setLayoutMode nor setFreeModeLayout', () => {
    // REQ-106 — re-clicking the active tab is a no-op
    // Precondition: layoutMode = 'A' (A is active)
    render(<LayoutTabBar />)
    fireEvent.click(screen.getByTestId('layout-tab-A'))
    expect(mockSetLayoutMode).not.toHaveBeenCalled()
    expect(mockSetFreeModeLayout).not.toHaveBeenCalled()
  })
})
