/**
 * onnxWorker — ONNX Runtime Web WebWorker (TASK-803)
 *
 * Documentation.
 * Design:
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 *
 * Documentation.
 * Documentation.
 */

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 */
export interface OnnxLoadRequest {
  type: 'load'
  /** Documentation. */
  modelData: ArrayBuffer
}

/**
 * Documentation.
 * Documentation.
 */
export interface OnnxInferRequest {
  type: 'infer'
  /** Documentation. */
  inputData: Float32Array
  /** Documentation. */
  inputShape: [number, number]
}

/** Documentation. */
export type OnnxWorkerRequest = OnnxLoadRequest | OnnxInferRequest

/**
 * Documentation.
 */
export interface OnnxLoadResponse {
  type: 'loaded'
  /** Documentation. */
  success: boolean
  /** Documentation. */
  error?: string
}

/**
 * Documentation.
 */
export interface OnnxInferResponse {
  type: 'result'
  /** Documentation. */
  outputData: Float32Array
}

/** Documentation. */
export type OnnxWorkerResponse = OnnxLoadResponse | OnnxInferResponse

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 *
 * Documentation.
 * import * as ort from 'onnxruntime-web';
 * const ONNX_AVAILABLE = true;
 * Documentation.
 */
const ONNX_AVAILABLE = false

self.onmessage = (event: MessageEvent<OnnxWorkerRequest>): void => {
  const request = event.data

  switch (request.type) {
    case 'load': {
      // Documentation.
      if (!ONNX_AVAILABLE) {
        const response: OnnxLoadResponse = {
          type: 'loaded',
          success: false,
          // Documentation.
          error: 'ONNX Runtime is not implemented. Please use the Ridge simplified PDP instead.',
        }
        self.postMessage(response)
        return
      }
      // Documentation.
      break
    }

    case 'infer': {
      // Documentation.
      // Documentation.
      const response: OnnxInferResponse = {
        type: 'result',
        outputData: new Float32Array(0),
      }
      self.postMessage(response)
      break
    }
  }
}
