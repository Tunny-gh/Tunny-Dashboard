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
 */

import { WasmLoader } from './wasmLoader'

// -------------------------------------------------------------------------
// Type definitions
// -------------------------------------------------------------------------

/**
 * Documentation.
 */
export interface FsapiPollerConfig {
  /** Documentation. */
  intervalMs?: number
  /** Documentation. */
  onNewTrials: (newCompleted: number) => void
  /** Documentation. */
  onError: (err: Error) => void
  /** Documentation. */
  onAutoStop?: () => void
}

/**
 * Documentation.
 */
export interface PollResult {
  newCompleted: number
  consumedBytes: number
}

// -------------------------------------------------------------------------
// Constants
// -------------------------------------------------------------------------

/** Documentation. */
const DEFAULT_INTERVAL_MS = 5000

/** Documentation. */
const MAX_ERROR_COUNT = 3

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 * Documentation.
 */
export class FsapiPoller {
  private fileHandle: FileSystemFileHandle | null = null
  private byteOffset = 0
  private timerId: ReturnType<typeof setTimeout> | null = null
  private isRunning = false
  private consecutiveErrors = 0
  private readonly config: Required<FsapiPollerConfig>

  constructor(config: FsapiPollerConfig) {
    this.config = {
      intervalMs: config.intervalMs ?? DEFAULT_INTERVAL_MS,
      onNewTrials: config.onNewTrials,
      onError: config.onError,
      onAutoStop: config.onAutoStop ?? (() => {}),
    }
  }

  // -------------------------------------------------------------------------
  // Documentation.
  // -------------------------------------------------------------------------

  /**
   * Documentation.
   * Documentation.
   */
  static isSupported(): boolean {
    return typeof window !== 'undefined' && 'showOpenFilePicker' in window
  }

  // -------------------------------------------------------------------------
  // Documentation.
  // -------------------------------------------------------------------------

  /**
   * Documentation.
   * Documentation.
   */
  async pickFile(): Promise<boolean> {
    if (!FsapiPoller.isSupported()) {
      this.config.onError(new Error('File System Access API is not available in this browser'))
      return false
    }

    try {
      const [handle] = await (
        window as unknown as {
          showOpenFilePicker: (opts?: object) => Promise<FileSystemFileHandle[]>
        }
      ).showOpenFilePicker({
        types: [
          {
            description: 'Optuna Journal',
            accept: { 'application/json': ['.log', '.jsonl'], 'text/plain': ['.log'] },
          },
        ],
      })
      this.fileHandle = handle
      this.byteOffset = 0 // Documentation.
      this.consecutiveErrors = 0
      return true
    } catch (e) {
      // Documentation.
      if (e instanceof Error && e.name === 'AbortError') {
        return false
      }
      this.config.onError(e instanceof Error ? e : new Error(String(e)))
      return false
    }
  }

  // -------------------------------------------------------------------------
  // Documentation.
  // -------------------------------------------------------------------------

  /**
   * Documentation.
   * Documentation.
   */
  start(): void {
    if (this.isRunning || !this.fileHandle) return
    this.isRunning = true
    this.scheduleNext()
  }

  /**
   * Documentation.
   */
  stop(): void {
    this.isRunning = false
    if (this.timerId !== null) {
      clearTimeout(this.timerId)
      this.timerId = null
    }
  }

  /**
   * Documentation.
   */
  setInterval(ms: number): void {
    this.config.intervalMs = ms
  }

  /** Documentation. */
  get running(): boolean {
    return this.isRunning
  }

  /** Documentation. */
  get offset(): number {
    return this.byteOffset
  }

  // -------------------------------------------------------------------------
  // Documentation.
  // -------------------------------------------------------------------------

  /**
   * Documentation.
   */
  private scheduleNext(): void {
    if (!this.isRunning) return
    this.timerId = setTimeout(() => {
      this.poll().finally(() => this.scheduleNext())
    }, this.config.intervalMs)
  }

  /**
   * Documentation.
   *
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   */
  async poll(): Promise<PollResult> {
    if (!this.fileHandle) {
      return { newCompleted: 0, consumedBytes: 0 }
    }

    try {
      const file = await this.fileHandle.getFile()
      const fileSize = file.size

      // Documentation.
      if (fileSize <= this.byteOffset) {
        this.consecutiveErrors = 0
        return { newCompleted: 0, consumedBytes: 0 }
      }

      // Documentation.
      const slice = file.slice(this.byteOffset)
      const buffer = await slice.arrayBuffer()
      const data = new Uint8Array(buffer)

      // Documentation.
      const wasm = await WasmLoader.getInstance()
      const result = wasm.appendJournalDiff(data)

      // Documentation.
      this.byteOffset += result.consumed_bytes
      this.consecutiveErrors = 0

      if (result.new_completed > 0) {
        this.config.onNewTrials(result.new_completed)
      }

      return {
        newCompleted: result.new_completed,
        consumedBytes: result.consumed_bytes,
      }
    } catch (e) {
      this.consecutiveErrors++
      const err = e instanceof Error ? e : new Error(String(e))
      this.config.onError(err)

      // Documentation.
      if (this.consecutiveErrors >= MAX_ERROR_COUNT) {
        this.stop()
        this.config.onAutoStop()
      }

      return { newCompleted: 0, consumedBytes: 0 }
    }
  }
}
