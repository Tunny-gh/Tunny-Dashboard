/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 */

import { describe, test, expect, beforeEach, vi } from 'vitest'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

const { mockPickFile, mockStart, mockStop, mockSetInterval, mockIsSupported } = vi.hoisted(() => {
  const mockPickFile = vi.fn()
  const mockStart = vi.fn()
  const mockStop = vi.fn()
  const mockSetInterval = vi.fn()
  const mockIsSupported = vi.fn().mockReturnValue(true)
  return { mockPickFile, mockStart, mockStop, mockSetInterval, mockIsSupported }
})

vi.mock('../wasm/fsapiPoller', () => {
  const MockFsapiPoller = vi.fn().mockImplementation(() => ({
    pickFile: mockPickFile,
    start: mockStart,
    stop: mockStop,
    setInterval: mockSetInterval,
  }))
  // Documentation.
  ;(MockFsapiPoller as unknown as Record<string, unknown>).isSupported = mockIsSupported
  return { FsapiPoller: MockFsapiPoller }
})

import { FsapiPoller } from '../wasm/fsapiPoller'

import { useLiveUpdateStore } from './liveUpdateStore'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/** Documentation. */
function resetStore() {
  useLiveUpdateStore.setState({
    isLive: false,
    isSupported: true,
    pollIntervalMs: 5000,
    lastUpdateAt: null,
    updateHistory: [],
    error: null,
  })
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    resetStore()
  })

  // Documentation.
  test('TC-1201-S01', () => {
    // Documentation.
    const state = useLiveUpdateStore.getState()
    // Documentation.
    expect(state.isLive).toBe(false)
    // Documentation.
    expect(state.updateHistory).toEqual([])
    // Documentation.
    expect(state.error).toBeNull()
    // Documentation.
    expect(state.pollIntervalMs).toBe(5000)
  })
})

// -------------------------------------------------------------------------
// startLive Actions
// -------------------------------------------------------------------------

describe('LiveUpdateStore — startLive', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    resetStore()
    mockIsSupported.mockReturnValue(true)
  })

  // Documentation.
  test('TC-1201-S02', async () => {
    // Documentation.
    useLiveUpdateStore.setState({ isSupported: false })

    await useLiveUpdateStore.getState().startLive()

    // Documentation.
    expect(useLiveUpdateStore.getState().error).toContain('Chrome')
    // Documentation.
    expect(useLiveUpdateStore.getState().isLive).toBe(false)
    // Documentation.
    expect(FsapiPoller).not.toHaveBeenCalled()
  })

  // Documentation.
  test('TC-1201-S03', async () => {
    // Documentation.
    mockPickFile.mockResolvedValue(false) // Documentation.

    await useLiveUpdateStore.getState().startLive()

    // Documentation.
    expect(useLiveUpdateStore.getState().isLive).toBe(false)
  })

  // Documentation.
  test('TC-1201-S04', async () => {
    // Documentation.
    mockPickFile.mockResolvedValue(true)

    await useLiveUpdateStore.getState().startLive()

    // Documentation.
    expect(useLiveUpdateStore.getState().isLive).toBe(true)
    // Documentation.
    expect(mockStart).toHaveBeenCalledOnce()
    // Documentation.
    expect(useLiveUpdateStore.getState().error).toBeNull()
  })
})

// -------------------------------------------------------------------------
// stopLive Actions
// -------------------------------------------------------------------------

describe('LiveUpdateStore — stopLive', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    resetStore()
  })

  // Documentation.
  test('TC-1201-S05', async () => {
    // Documentation.
    mockPickFile.mockResolvedValue(true)
    await useLiveUpdateStore.getState().startLive()
    expect(useLiveUpdateStore.getState().isLive).toBe(true)

    useLiveUpdateStore.getState().stopLive()

    // Documentation.
    expect(useLiveUpdateStore.getState().isLive).toBe(false)
    // Documentation.
    expect(mockStop).toHaveBeenCalledOnce()
  })
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('LiveUpdateStore — _onNewTrials', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    resetStore()
  })

  // Documentation.
  test('TC-1201-S06', () => {
    // Documentation.
    useLiveUpdateStore.getState()._onNewTrials(5)

    const state = useLiveUpdateStore.getState()
    // Documentation.
    expect(state.updateHistory).toHaveLength(1)
    // Documentation.
    expect(state.updateHistory[0].newTrials).toBe(5)
    // Documentation.
    expect(state.lastUpdateAt).toBeInstanceOf(Date)
  })

  // Documentation.
  test('TC-1201-S07', () => {
    // Documentation.
    for (let i = 0; i < 12; i++) {
      useLiveUpdateStore.getState()._onNewTrials(i + 1)
    }

    // Documentation.
    expect(useLiveUpdateStore.getState().updateHistory).toHaveLength(10)
    // Documentation.
    expect(useLiveUpdateStore.getState().updateHistory[0].newTrials).toBe(12)
  })

  // Documentation.
  test('TC-1201-S08', () => {
    // Documentation.
    // Documentation.
    // Documentation.
    expect(() => {
      useLiveUpdateStore.getState()._onNewTrials(3)
    }).not.toThrow()

    // Documentation.
    const state = useLiveUpdateStore.getState()
    expect(state.updateHistory).toHaveLength(1)
  })
})

// -------------------------------------------------------------------------
// _onError / _onAutoStop
// -------------------------------------------------------------------------

describe('LiveUpdateStore — _onError / _onAutoStop', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    resetStore()
  })

  // Documentation.
  test('TC-1201-S09', () => {
    // Documentation.
    useLiveUpdateStore.getState()._onError(new Error('translated'))

    // Documentation.
    expect(useLiveUpdateStore.getState().error).toBe('translated')
  })

  // Documentation.
  test('TC-1201-S10', () => {
    // Documentation.
    useLiveUpdateStore.setState({ isLive: true })

    useLiveUpdateStore.getState()._onAutoStop()

    const state = useLiveUpdateStore.getState()
    // Documentation.
    expect(state.isLive).toBe(false)
    // Documentation.
    expect(state.error).toContain('auto-stopped')
  })

  // Documentation.
  test('TC-1201-S11', () => {
    // Documentation.
    useLiveUpdateStore.setState({ error: 'translated' })

    useLiveUpdateStore.getState().clearError()

    // Documentation.
    expect(useLiveUpdateStore.getState().error).toBeNull()
  })
})

// -------------------------------------------------------------------------
// setPollInterval Actions
// -------------------------------------------------------------------------

describe('LiveUpdateStore — setPollInterval', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    resetStore()
  })

  // Documentation.
  test('TC-1201-S12', () => {
    // Documentation.
    useLiveUpdateStore.getState().setPollInterval(10000)

    // Documentation.
    expect(useLiveUpdateStore.getState().pollIntervalMs).toBe(10000)
  })
})
