/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest'
import { ScatterMatrixEngine, CELL_PIXEL_SIZES, WORKER_ROW_GROUP } from './ScatterMatrixEngine'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 */
class MockWorker {
  // Documentation.
  postMessage = vi.fn()
  terminate = vi.fn()

  // Documentation.
  onmessage: ((e: MessageEvent) => void) | null = null
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  onerror: ((e?: any) => void) | null = null

  // Documentation.
  simulateMessage(data: unknown): void {
    this.onmessage?.({ data } as MessageEvent)
  }

  // Documentation.
  simulateError(): void {
    this.onerror?.()
  }
}

/** Documentation. */
function createWorkerFactory(instances: MockWorker[]): () => Worker {
  return () => {
    const w = new MockWorker()
    instances.push(w)
    return w as unknown as Worker
  }
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  let mockWorkers: MockWorker[]
  let engine: ScatterMatrixEngine

  beforeEach(() => {
    // Documentation.
    mockWorkers = []
    engine = new ScatterMatrixEngine(createWorkerFactory(mockWorkers), 4)
  })

  afterEach(() => {
    engine.dispose()
  })

  // Documentation.
  test('TC-701-01', () => {
    // Documentation.
    // Documentation.
    expect(mockWorkers).toHaveLength(4)
  })

  // Documentation.
  test('TC-701-02', () => {
    // Documentation.
    engine.renderCell(0, 1, 'thumbnail')

    // Documentation.
    expect(mockWorkers[0].postMessage).toHaveBeenCalledWith({
      type: 'render',
      row: 0,
      col: 1,
      size: 80,
    })
  })

  // Documentation.
  test('TC-701-03', async () => {
    // Documentation.
    const promise = engine.renderCell(0, 1, 'thumbnail')

    // Documentation.
    mockWorkers[0].simulateMessage({
      type: 'done',
      row: 0,
      col: 1,
      imageData: null,
    })

    // Documentation.
    const result = await promise
    expect(result).toBeNull()
  })

  // Documentation.
  test('TC-701-04', () => {
    // Documentation.
    // Documentation.
    expect(engine.workerIndex(0)).toBe(0) // Documentation.
    expect(engine.workerIndex(9)).toBe(0) // Documentation.
    expect(engine.workerIndex(10)).toBe(1) // Documentation.
    expect(engine.workerIndex(19)).toBe(1) // Documentation.
    expect(engine.workerIndex(20)).toBe(2) // Documentation.
    expect(engine.workerIndex(30)).toBe(3) // Documentation.
  })

  // Documentation.
  test('TC-701-05', () => {
    // Documentation.
    expect(CELL_PIXEL_SIZES.thumbnail).toBe(80) // Documentation.
    expect(CELL_PIXEL_SIZES.preview).toBe(300) // Documentation.
    expect(CELL_PIXEL_SIZES.full).toBe(600) // Documentation.
    // Documentation.
    expect(WORKER_ROW_GROUP).toBe(10) // Documentation.
  })
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  let mockWorkers: MockWorker[]
  let engine: ScatterMatrixEngine

  beforeEach(() => {
    mockWorkers = []
    engine = new ScatterMatrixEngine(createWorkerFactory(mockWorkers), 4)
  })

  afterEach(() => {
    engine.dispose()
  })

  // Documentation.
  test('TC-701-E01', async () => {
    // Documentation.
    // Documentation.
    const promise = engine.renderCell(0, 1)

    // Documentation.
    mockWorkers[0].simulateError()

    // Documentation.
    await expect(promise).rejects.toThrow('Worker error')
  })

  // Documentation.
  test('TC-701-E02', () => {
    // Documentation.
    engine.dispose()

    // Documentation.
    mockWorkers.forEach((w) => {
      expect(w.terminate).toHaveBeenCalledTimes(1)
    })
  })
})
