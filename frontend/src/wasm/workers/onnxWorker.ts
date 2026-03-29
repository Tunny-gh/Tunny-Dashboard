/**
 * onnxWorker — ONNX Runtime Web WebWorker (TASK-803)
 *
 * 【役割】: ONNX モデルを Worker スレッドで読み込み・推論する
 * 【設計方針】:
 *   - TASK-803 では Ridge 簡易版 PDP を実装（本ファイルはスタブ）
 *   - ONNX Runtime Web が利用可能な場合は高精度 PDP に切り替わる
 *   - 未実装時は 'loaded' で success=false を返し、呼び出し元で Ridge フォールバックする
 * 🟢 REQ-103〜REQ-106 に準拠（ONNX 高精度 PDP — 将来実装）
 *
 * 【注意】: このファイルは jsdom 環境でテスト不可（Worker context 非対応）
 *           OnnxWorkerClient の単体テストは MockWorker で実施する
 */

// -------------------------------------------------------------------------
// 型定義（Worker メッセージプロトコル）
// -------------------------------------------------------------------------

/**
 * 【ONNX モデル読み込みリクエスト】: ArrayBuffer 形式の .onnx バイナリを受け取る
 * 🟢 REQ-103: .onnx ファイル読み込みで高精度PDP を有効化
 */
export interface OnnxLoadRequest {
  type: 'load'
  /** .onnx モデルのバイナリデータ */
  modelData: ArrayBuffer
}

/**
 * 【ONNX 推論リクエスト】: 入力テンソルを受け取り PDP 値を計算する
 * 🟢 REQ-104: ONNX モデルによる高精度 PDP 推論
 */
export interface OnnxInferRequest {
  type: 'infer'
  /** 入力テンソルデータ（フラット Float32Array） */
  inputData: Float32Array
  /** 入力テンソル形状 [batch_size, n_features] */
  inputShape: [number, number]
}

/** 【Worker リクエスト型】: load / infer の Union */
export type OnnxWorkerRequest = OnnxLoadRequest | OnnxInferRequest

/**
 * 【ONNX モデル読み込みレスポンス】: 読み込み成否を返す
 */
export interface OnnxLoadResponse {
  type: 'loaded'
  /** 読み込み成否 */
  success: boolean
  /** エラーメッセージ（success=false のとき設定） */
  error?: string
}

/**
 * 【ONNX 推論レスポンス】: 出力テンソル（PDP 値）を返す
 */
export interface OnnxInferResponse {
  type: 'result'
  /** 出力テンソルデータ（フラット Float32Array） */
  outputData: Float32Array
}

/** 【Worker レスポンス型】: loaded / result の Union */
export type OnnxWorkerResponse = OnnxLoadResponse | OnnxInferResponse

// -------------------------------------------------------------------------
// Worker 実装（スタブ）
// -------------------------------------------------------------------------

/**
 * 【ONNX 利用可能フラグ】: スタブ段階では false
 *
 * 将来的に ONNX Runtime Web を import して true に変更する:
 * import * as ort from 'onnxruntime-web';
 * const ONNX_AVAILABLE = true;
 * 🟡 ONNX Runtime Web の統合は TASK-803 以降に実装予定
 */
const ONNX_AVAILABLE = false

self.onmessage = (event: MessageEvent<OnnxWorkerRequest>): void => {
  const request = event.data

  switch (request.type) {
    case 'load': {
      // 【読み込み処理】: ONNX Runtime 未実装のためエラーレスポンスを返す
      if (!ONNX_AVAILABLE) {
        const response: OnnxLoadResponse = {
          type: 'loaded',
          success: false,
          // 🟡 エラーメッセージは Ridge フォールバックへの案内を含む
          error: 'ONNX Runtime は未実装です。Ridge 簡易版 PDP を使用してください。',
        }
        self.postMessage(response)
        return
      }
      // 【将来実装】: ort.InferenceSession.create(request.modelData) を呼び出す
      break
    }

    case 'infer': {
      // 【推論処理】: スタブとして空の出力テンソルを返す
      // 🟡 将来的に session.run() で実際の推論を行う
      const response: OnnxInferResponse = {
        type: 'result',
        outputData: new Float32Array(0),
      }
      self.postMessage(response)
      break
    }
  }
}
