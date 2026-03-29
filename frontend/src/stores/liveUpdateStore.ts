/**
 * Documentation.
 *
 * Documentation.
 * Design:
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 */

import { create } from 'zustand'
import { FsapiPoller } from '../wasm/fsapiPoller'

// -------------------------------------------------------------------------
// Type definitions
// -------------------------------------------------------------------------

/**
 * Documentation.
 */
export interface UpdateRecord {
  /** Documentation. */
  at: Date
  /** Documentation. */
  newTrials: number
}

/**
 * Documentation.
 */
interface LiveUpdateState {
  // --- State ---
  /** Documentation. */
  isLive: boolean
  /** Documentation. */
  isSupported: boolean
  /** Documentation. */
  pollIntervalMs: number
  /** Documentation. */
  lastUpdateAt: Date | null
  /** Documentation. */
  updateHistory: UpdateRecord[]
  /** Documentation. */
  error: string | null

  // --- Actions ---
  /** Documentation. */
  startLive: () => Promise<void>
  /** Documentation. */
  stopLive: () => void
  /** Documentation. */
  setPollInterval: (ms: number) => void
  /** Documentation. */
  clearError: () => void

  // Documentation.
  /** Documentation. */
  _onNewTrials: (newCompleted: number) => void
  /** Documentation. */
  _onError: (err: Error) => void
  /** Documentation. */
  _onAutoStop: () => void
}

// -------------------------------------------------------------------------
// Constants
// -------------------------------------------------------------------------

/** Documentation. */
const MAX_HISTORY = 10

/** Documentation. */
const DEFAULT_POLL_INTERVAL_MS = 5000

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/** Documentation. */
let _poller: FsapiPoller | null = null

/**
 * Documentation.
 */
export const useLiveUpdateStore = create<LiveUpdateState>()((set, get) => ({
  // -------------------------------------------------------------------------
  // Documentation.
  // -------------------------------------------------------------------------
  isLive: false,
  isSupported: FsapiPoller.isSupported(),
  pollIntervalMs: DEFAULT_POLL_INTERVAL_MS,
  lastUpdateAt: null,
  updateHistory: [],
  error: null,

  // -------------------------------------------------------------------------
  // Documentation.
  // -------------------------------------------------------------------------

  /**
   * Documentation.
   * Documentation.
   */
  startLive: async () => {
    const { isSupported, pollIntervalMs } = get()

    if (!isSupported) {
      set({ error: 'Live update is only supported on Chrome / Edge' })
      return
    }

    // Documentation.
    _poller = new FsapiPoller({
      intervalMs: pollIntervalMs,
      onNewTrials: (n) => get()._onNewTrials(n),
      onError: (err) => get()._onError(err),
      onAutoStop: () => get()._onAutoStop(),
    })

    // Documentation.
    const selected = await _poller.pickFile()
    if (!selected) {
      _poller = null
      return
    }

    _poller.start()
    set({ isLive: true, error: null })
  },

  /**
   * Documentation.
   */
  stopLive: () => {
    if (_poller) {
      _poller.stop()
      _poller = null
    }
    set({ isLive: false })
  },

  /**
   * Documentation.
   */
  setPollInterval: (ms) => {
    set({ pollIntervalMs: ms })
    if (_poller) {
      _poller.setInterval(ms)
    }
  },

  /** Documentation. */
  clearError: () => set({ error: null }),

  /**
   * Documentation.
   * Documentation.
   */
  _onNewTrials: (newCompleted) => {
    const now = new Date()
    const record: UpdateRecord = { at: now, newTrials: newCompleted }

    set((s) => ({
      lastUpdateAt: now,
      updateHistory: [record, ...s.updateHistory].slice(0, MAX_HISTORY),
    }))
  },

  /**
   * Documentation.
   */
  _onError: (err) => {
    set({ error: err.message })
  },

  /**
   * Documentation.
   */
  _onAutoStop: () => {
    _poller = null
    set({
      isLive: false,
      error: 'Update failed (auto-stopped after 3 consecutive errors)',
    })
  },
}))
