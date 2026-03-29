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

import type {
  OnnxWorkerRequest,
  OnnxWorkerResponse,
  OnnxLoadResponse,
  OnnxInferResponse,
} from './onnxWorker'

// -------------------------------------------------------------------------
// Type definitions
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 */
export type WorkerFactory = () => Worker

/**
 * Documentation.
 */
export interface OnnxLoadResult {
  /** Documentation. */
  success: boolean
  /** Documentation. */
  error?: string
}

/**
 * Documentation.
 */
export interface OnnxInferResult {
  /** Documentation. */
  outputData: Float32Array
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 *
 * Documentation.
 * ```ts
 * const client = new OnnxWorkerClient(() => new Worker(new URL('./onnxWorker.ts', import.meta.url)));
 * const result = await client.load(modelBuffer);
 * if (!result.success) {
 * Documentation.
 * }
 * ```
 * Documentation.
 */
export class OnnxWorkerClient {
  /** Documentation. */
  private readonly worker: Worker

  /** Documentation. */
  private pendingRequests = new Map<
    number,
    { resolve: (res: OnnxWorkerResponse) => void; reject: (err: Error) => void }
  >()

  /** Documentation. */
  private nextId = 0

  /**
   * Documentation.
   */
  constructor(workerFactory: WorkerFactory) {
    this.worker = workerFactory()
    // Documentation.
    this.worker.onmessage = (event: MessageEvent<OnnxWorkerResponse>) => {
      this.handleMessage(event.data)
    }
    this.worker.onerror = (error) => {
      // Documentation.
      const err = new Error(`OnnxWorker error: ${error.message}`)
      for (const { reject } of this.pendingRequests.values()) {
        reject(err)
      }
      this.pendingRequests.clear()
    }
  }

  /**
   * Documentation.
   *
   * Documentation.
   * Documentation.
   * Documentation.
   */
  async load(modelData: ArrayBuffer): Promise<OnnxLoadResult> {
    const response = await this.sendRequest({ type: 'load', modelData })
    if (response.type !== 'loaded') {
      return { success: false, error: 'Unexpected response type' }
    }
    const loadResponse = response as OnnxLoadResponse
    return { success: loadResponse.success, error: loadResponse.error }
  }

  /**
   * Documentation.
   *
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   */
  async infer(inputData: Float32Array, inputShape: [number, number]): Promise<OnnxInferResult> {
    const response = await this.sendRequest({ type: 'infer', inputData, inputShape })
    if (response.type !== 'result') {
      return { outputData: new Float32Array(0) }
    }
    const inferResponse = response as OnnxInferResponse
    return { outputData: inferResponse.outputData }
  }

  /**
   * Documentation.
   */
  terminate(): void {
    this.worker.terminate()
    this.pendingRequests.clear()
  }

  // -------------------------------------------------------------------------
  // Documentation.
  // -------------------------------------------------------------------------

  /**
   * Documentation.
   *
   * Documentation.
   * Documentation.
   * Documentation.
   */
  private sendRequest(request: OnnxWorkerRequest): Promise<OnnxWorkerResponse> {
    return new Promise((resolve, reject) => {
      const id = this.nextId++
      this.pendingRequests.set(id, { resolve, reject })
      this.worker.postMessage(request)
    })
  }

  /**
   * Documentation.
   *
   * Documentation.
   */
  private handleMessage(response: OnnxWorkerResponse): void {
    // Documentation.
    const firstKey = this.pendingRequests.keys().next().value
    if (firstKey === undefined) {
      return
    }
    const pending = this.pendingRequests.get(firstKey)
    if (pending) {
      this.pendingRequests.delete(firstKey)
      pending.resolve(response)
    }
  }
}
