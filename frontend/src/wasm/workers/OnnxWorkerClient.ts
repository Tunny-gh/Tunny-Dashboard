/**
 * OnnxWorkerClient — ONNX Worker 通信クライアント (TASK-803)
 *
 * 【役割】: onnxWorker との非同期メッセージ通信を Promise API でラップする
 * 【設計方針】:
 *   - workerFactory 注入パターンでテスト容易性を確保（ScatterMatrixEngine と同方針）
 *   - load() が success=false の場合は呼び出し元で Ridge フォールバックする
 *   - Promise ベース API で Worker の非同期性を隠蔽する
 * 🟢 REQ-103〜REQ-106 に準拠
 */

import type {
  OnnxWorkerRequest,
  OnnxWorkerResponse,
  OnnxLoadResponse,
  OnnxInferResponse,
} from './onnxWorker'

// -------------------------------------------------------------------------
// 型定義
// -------------------------------------------------------------------------

/**
 * 【Worker ファクトリ型】: テスト時にモック Worker を注入するための型
 * 🟢 依存性注入パターン: 実コードでは new Worker(...) を使用
 */
export type WorkerFactory = () => Worker

/**
 * 【ONNX 読み込み結果型】: load() の戻り値
 */
export interface OnnxLoadResult {
  /** 読み込み成否 */
  success: boolean
  /** エラーメッセージ（success=false のとき設定） */
  error?: string
}

/**
 * 【ONNX 推論結果型】: infer() の戻り値
 */
export interface OnnxInferResult {
  /** 出力テンソルデータ */
  outputData: Float32Array
}

// -------------------------------------------------------------------------
// OnnxWorkerClient クラス
// -------------------------------------------------------------------------

/**
 * 【クラス概要】: ONNX Worker への通信を Promise API でラップするクライアント
 *
 * 【使用例】:
 * ```ts
 * const client = new OnnxWorkerClient(() => new Worker(new URL('./onnxWorker.ts', import.meta.url)));
 * const result = await client.load(modelBuffer);
 * if (!result.success) {
 *   // Ridge フォールバック
 * }
 * ```
 * 🟢 REQ-103: .onnx 読み込み成否に応じてフォールバック可能な設計
 */
export class OnnxWorkerClient {
  /** 【Worker インスタンス】: workerFactory から生成 */
  private readonly worker: Worker

  /** 【保留中リクエスト】: メッセージ ID → resolve/reject マップ */
  private pendingRequests = new Map<
    number,
    { resolve: (res: OnnxWorkerResponse) => void; reject: (err: Error) => void }
  >()

  /** 【メッセージ ID カウンタ】: リクエストと応答を対応付けるための連番 */
  private nextId = 0

  /**
   * @param workerFactory Worker インスタンスを生成するファクトリ関数
   */
  constructor(workerFactory: WorkerFactory) {
    this.worker = workerFactory()
    // 【メッセージ受信ハンドラ登録】: Worker からの応答を対応する Promise に届ける
    this.worker.onmessage = (event: MessageEvent<OnnxWorkerResponse>) => {
      this.handleMessage(event.data)
    }
    this.worker.onerror = (error) => {
      // 【エラーハンドリング】: Worker エラー時は全保留リクエストを reject する
      const err = new Error(`OnnxWorker エラー: ${error.message}`)
      for (const { reject } of this.pendingRequests.values()) {
        reject(err)
      }
      this.pendingRequests.clear()
    }
  }

  /**
   * 【ONNX モデル読み込み】: ArrayBuffer 形式の .onnx を Worker に送信する
   *
   * 【設計】: success=false が返った場合は呼び出し元で Ridge フォールバックを使用する 🟢
   * @param modelData .onnx ファイルの ArrayBuffer
   * @returns 読み込み結果（success フラグ・エラーメッセージ）
   */
  async load(modelData: ArrayBuffer): Promise<OnnxLoadResult> {
    const response = await this.sendRequest({ type: 'load', modelData })
    if (response.type !== 'loaded') {
      return { success: false, error: '想定外のレスポンスタイプです' }
    }
    const loadResponse = response as OnnxLoadResponse
    return { success: loadResponse.success, error: loadResponse.error }
  }

  /**
   * 【ONNX 推論実行】: 入力テンソルを Worker に送り推論結果を受け取る
   *
   * 【設計】: 入力形状 [batch_size, n_features] で受け取り、出力を Float32Array で返す 🟢
   * @param inputData 入力テンソルデータ（フラット Float32Array）
   * @param inputShape 入力形状 [batch_size, n_features]
   * @returns 推論結果（出力テンソルデータ）
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
   * 【Worker 終了】: Worker スレッドを終了する
   */
  terminate(): void {
    this.worker.terminate()
    this.pendingRequests.clear()
  }

  // -------------------------------------------------------------------------
  // 内部メソッド
  // -------------------------------------------------------------------------

  /**
   * 【リクエスト送信】: Promise ベースで Worker にメッセージを送る
   *
   * 【設計】: ID ベースの応答マッチングを使用して非同期を管理する 🟢
   * 【注意】: 現状の onnxWorker スタブは ID を使わず即時応答するため
   *           一度に1リクエストのみ保留することを前提とする（簡易版）
   */
  private sendRequest(request: OnnxWorkerRequest): Promise<OnnxWorkerResponse> {
    return new Promise((resolve, reject) => {
      const id = this.nextId++
      this.pendingRequests.set(id, { resolve, reject })
      this.worker.postMessage(request)
    })
  }

  /**
   * 【メッセージ処理】: Worker からの応答を最古の保留リクエストに届ける
   *
   * 【設計】: スタブ段階では FIFO で最初の保留リクエストに応答する 🟡
   */
  private handleMessage(response: OnnxWorkerResponse): void {
    // 最古の保留リクエストを取り出す（FIFO）
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
