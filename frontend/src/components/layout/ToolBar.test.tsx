/**
 * ToolBar tests (TASK-401 / TASK-002)
 *
 * Tests the ToolBar file-loading, study-selection, and layout-switching UI.
 * studyStore/layoutStore are mocked with vi.mock; LayoutTabBar is also mocked.
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest'
import { render, screen, cleanup, fireEvent } from '@testing-library/react'

// -------------------------------------------------------------------------
// studyStore / layoutStore mocks
// -------------------------------------------------------------------------

const { mockLoadJournalTB, mockStartLive, mockStopLive } = vi.hoisted(() => {
  const mockLoadJournalTB = vi.fn().mockResolvedValue(undefined)
  const mockStartLive = vi.fn()
  const mockStopLive = vi.fn()
  return { mockLoadJournalTB, mockStartLive, mockStopLive }
})

vi.mock('../../stores/studyStore', () => ({
  useStudyStore: vi
    .fn()
    .mockImplementation(
      (selector: (s: { loadJournal: typeof mockLoadJournalTB; isLoading: boolean }) => unknown) =>
        selector({ loadJournal: mockLoadJournalTB, isLoading: false }),
    ),
}))

vi.mock('../../stores/liveUpdateStore', () => ({
  useLiveUpdateStore: vi
    .fn()
    .mockImplementation(
      (
        selector: (s: {
          isLive: boolean
          isSupported: boolean
          startLive: typeof mockStartLive
          stopLive: typeof mockStopLive
        }) => unknown,
      ) =>
        selector({
          isLive: false,
          isSupported: true,
          startLive: mockStartLive,
          stopLive: mockStopLive,
        }),
    ),
}))

// Mock LayoutTabBar to isolate ToolBar unit tests
vi.mock('./LayoutTabBar', () => ({ LayoutTabBar: () => <div data-testid="layout-tab-bar" /> }))

import { ToolBar } from './ToolBar'
import { useLiveUpdateStore } from '../../stores/liveUpdateStore'

// -------------------------------------------------------------------------
// Happy path
// -------------------------------------------------------------------------

describe('ToolBar — happy path', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    cleanup()
  })

  // TC-401-03: ToolBar renders without errors
  test('TC-401-03: ToolBar renders without throwing', () => {
    expect(() => render(<ToolBar />)).not.toThrow()
  })

  // TC-401-04: file input change calls loadJournal
  test('TC-401-04: changing the file input calls studyStore.loadJournal', () => {
    render(<ToolBar />)

    const file = new File(['content'], 'journal.log', { type: 'text/plain' })

    const fileInput = screen.getByTestId('file-input')
    Object.defineProperty(fileInput, 'files', { value: [file] })
    fireEvent.change(fileInput)

    expect(mockLoadJournalTB).toHaveBeenCalledOnce()
    expect(mockLoadJournalTB).toHaveBeenCalledWith(file)
  })

  // TC-401-07: LayoutTabBar is present inside ToolBar (TASK-002)
  test('TC-401-07: data-testid="layout-tab-bar" exists inside ToolBar', () => {
    // REQ-001 — LayoutTabBar is integrated into ToolBar
    render(<ToolBar />)
    expect(screen.getByTestId('layout-tab-bar')).toBeInTheDocument()
  })

  // TC-401-08: legacy layout buttons are absent (TASK-002)
  test('TC-401-08: layout-btn-A through layout-btn-D are not in the DOM', () => {
    // REQ-405 — legacy buttons have been removed
    render(<ToolBar />)
    expect(screen.queryByTestId('layout-btn-A')).not.toBeInTheDocument()
    expect(screen.queryByTestId('layout-btn-B')).not.toBeInTheDocument()
    expect(screen.queryByTestId('layout-btn-C')).not.toBeInTheDocument()
    expect(screen.queryByTestId('layout-btn-D')).not.toBeInTheDocument()
  })

  // TC-401-10: live-update button is present
  test('TC-401-10: live-update-btn exists in the DOM', () => {
    // REQ-104-G — live-update button is shown in the ToolBar
    render(<ToolBar />)
    expect(screen.getByTestId('live-update-btn')).toBeInTheDocument()
  })

  // TC-401-11: when isLive=false, clicking the button calls startLive
  test('TC-401-11: when isLive=false, clicking the button calls startLive', () => {
    // REQ-104-K — click with isLive=false calls startLive()
    render(<ToolBar />)
    fireEvent.click(screen.getByTestId('live-update-btn'))
    expect(mockStartLive).toHaveBeenCalledOnce()
    expect(mockStopLive).not.toHaveBeenCalled()
  })

  // TC-401-12: when isLive=true, clicking the button calls stopLive
  test('TC-401-12: when isLive=true, clicking the button calls stopLive', () => {
    // REQ-104-J — click with isLive=true calls stopLive()
    vi.mocked(useLiveUpdateStore).mockImplementation((selector) =>
      selector({
        isLive: true,
        isSupported: true,
        startLive: mockStartLive,
        stopLive: mockStopLive,
      }),
    )
    render(<ToolBar />)
    fireEvent.click(screen.getByTestId('live-update-btn'))
    expect(mockStopLive).toHaveBeenCalledOnce()
    expect(mockStartLive).not.toHaveBeenCalled()
  })

  // TC-401-13: when isSupported=false, the button is disabled
  test('TC-401-13: when isSupported=false, the live-update button is disabled', () => {
    // REQ-104-I — button is disabled when the feature is unsupported
    vi.mocked(useLiveUpdateStore).mockImplementation((selector) =>
      selector({
        isLive: false,
        isSupported: false,
        startLive: mockStartLive,
        stopLive: mockStopLive,
      }),
    )
    render(<ToolBar />)
    expect(screen.getByTestId('live-update-btn')).toBeDisabled()
  })
})
