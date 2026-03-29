/**
 * Documentation.
 *
 * Documentation.
 * Design:
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 */

// -------------------------------------------------------------------------
// Constants・Type definitions
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 */
export type ScatterCellSize = 'thumbnail' | 'preview' | 'full'

/**
 * Documentation.
 * 🟢 thumbnail: 80×80px, preview: 300×300px, full: 600×600px
 */
export const CELL_PIXEL_SIZES: Record<ScatterCellSize, number> = {
  thumbnail: 80, // Documentation.
  preview: 300, // Documentation.
  full: 600, // Documentation.
}

/** Documentation. */
export const WORKER_COUNT = 4

/** Documentation. */
export const WORKER_ROW_GROUP = 10

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 */
interface PendingRender {
  resolve: (data: ImageData | null) => void
  reject: (err: Error) => void
}

/**
 * Documentation.
 */
interface WorkerDoneMessage {
  type: 'done'
  row: number
  col: number
  imageData: ImageData | null
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 * Test Coverage: TC-701-01〜05, TC-701-E01〜E02
 */
export class ScatterMatrixEngine {
  /** Documentation. */
  private workers: Worker[]

  /** Documentation. */
  private pending: Map<string, PendingRender>

  /**
   * Documentation.
   * Documentation.
   * Documentation.
   */
  constructor(workerFactory: () => Worker, workerCount: number = WORKER_COUNT) {
    this.pending = new Map()

    // Documentation.
    this.workers = Array.from({ length: workerCount }, (_, i) => {
      const w = workerFactory()

      // Documentation.
      w.onmessage = (e: MessageEvent) => this.handleMessage(e)

      // Documentation.
      w.onerror = (_ev: Event) => this.handleWorkerError(i)

      return w
    })
  }

  // -------------------------------------------------------------------------
  // Documentation.
  // -------------------------------------------------------------------------

  /**
   * Documentation.
   * Documentation.
   */
  private handleMessage(e: MessageEvent): void {
    const { type, row, col, imageData } = e.data as WorkerDoneMessage

    // Documentation.
    if (type !== 'done') return

    const key = this.cellKey(row, col)
    const pending = this.pending.get(key)

    if (pending) {
      // Documentation.
      pending.resolve(imageData ?? null)
      this.pending.delete(key)
    }
  }

  /**
   * Documentation.
   * Documentation.
   */
  private handleWorkerError(workerIdx: number): void {
    // Documentation.
    const keysToReject: string[] = []
    for (const [key] of this.pending) {
      const row = parseInt(key.split('-')[0], 10)
      if (this.workerIndex(row) === workerIdx) {
        keysToReject.push(key)
      }
    }

    // Documentation.
    for (const key of keysToReject) {
      const pending = this.pending.get(key)
      if (pending) {
        pending.reject(new Error('Worker error'))
        this.pending.delete(key)
      }
    }
  }

  /**
   * Documentation.
   * Documentation.
   * Documentation.
   */
  private cellKey(row: number, col: number): string {
    return `${row}-${col}`
  }

  // -------------------------------------------------------------------------
  // Documentation.
  // -------------------------------------------------------------------------

  /**
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   */
  workerIndex(row: number): number {
    return Math.floor(row / WORKER_ROW_GROUP) % this.workers.length
  }

  /**
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   */
  renderCell(
    row: number,
    col: number,
    size: ScatterCellSize = 'thumbnail',
  ): Promise<ImageData | null> {
    const key = this.cellKey(row, col)

    return new Promise((resolve, reject) => {
      // Documentation.
      this.pending.set(key, { resolve, reject })

      // Documentation.
      const wIdx = this.workerIndex(row)
      this.workers[wIdx].postMessage({
        type: 'render',
        row,
        col,
        size: CELL_PIXEL_SIZES[size],
      })
    })
  }

  /**
   * Documentation.
   * Documentation.
   * Documentation.
   */
  dispose(): void {
    // Documentation.
    this.workers.forEach((w) => w.terminate())

    // Documentation.
    this.pending.clear()
  }
}
