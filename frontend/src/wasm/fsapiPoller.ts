/**
 * FsapiPoller — File System Access API を使ったジャーナル差分ポーリング (TASK-1201)
 *
 * 【役割】: Journal ファイルをバイト差分で読み込み、WASM の append_journal_diff() に渡す
 * 【設計方針】:
 *   - File System Access API 対応チェック → 非対応時はフォールバックを通知
 *   - バイトオフセット管理: 前回位置から差分のみ読み込む（全体再読み込みなし）
 *   - ポーリング間隔: 1〜30 秒設定可（デフォルト 5 秒）
 *   - 連続 3 回エラーで自動停止 🟢 REQ-135
 * 🟢 REQ-130〜REQ-135 に準拠
 */

import { WasmLoader } from './wasmLoader'

// -------------------------------------------------------------------------
// 型定義
// -------------------------------------------------------------------------

/**
 * 【ポーリング設定】: FsapiPoller の動作パラメータ
 */
export interface FsapiPollerConfig {
  /** ポーリング間隔 (ms): デフォルト 5000ms 🟢 */
  intervalMs?: number
  /** 新規 COMPLETE 試行が検出されたときのコールバック */
  onNewTrials: (newCompleted: number) => void
  /** エラー発生時のコールバック */
  onError: (err: Error) => void
  /** 自動停止したときのコールバック（3 回連続エラー）*/
  onAutoStop?: () => void
}

/**
 * 【差分読み込み結果】
 */
export interface PollResult {
  newCompleted: number
  consumedBytes: number
}

// -------------------------------------------------------------------------
// 定数
// -------------------------------------------------------------------------

/** 【デフォルトポーリング間隔】: 5 秒 */
const DEFAULT_INTERVAL_MS = 5000

/** 【自動停止エラー閾値】: 3 回連続エラーで停止 🟢 REQ-135 */
const MAX_ERROR_COUNT = 3

// -------------------------------------------------------------------------
// FsapiPoller クラス
// -------------------------------------------------------------------------

/**
 * 【機能概要】: File System Access API を使ったジャーナル差分ポーリングクラス
 * 【フォールバック】: FSAPI 非対応時は isSupported()=false を返す
 * 🟢 REQ-130〜REQ-135 に準拠
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
  // 静的メソッド
  // -------------------------------------------------------------------------

  /**
   * 【FSAPI 対応チェック】: File System Access API がブラウザで使用可能か判定する
   * Firefox / Safari では false になる 🟢 REQ-133
   */
  static isSupported(): boolean {
    return typeof window !== 'undefined' && 'showOpenFilePicker' in window
  }

  // -------------------------------------------------------------------------
  // ファイル選択
  // -------------------------------------------------------------------------

  /**
   * 【ファイル選択】: showOpenFilePicker でユーザーに Journal ファイルを選択させる
   * 成功時 true、キャンセルまたはエラー時 false を返す 🟢 REQ-130
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
      this.byteOffset = 0 // 【オフセットリセット】: 新ファイル選択時は先頭から
      this.consecutiveErrors = 0
      return true
    } catch (e) {
      // ユーザーキャンセルは AbortError → エラー通知しない
      if (e instanceof Error && e.name === 'AbortError') {
        return false
      }
      this.config.onError(e instanceof Error ? e : new Error(String(e)))
      return false
    }
  }

  // -------------------------------------------------------------------------
  // ポーリング制御
  // -------------------------------------------------------------------------

  /**
   * 【ポーリング開始】: 定期的に差分を読み込む
   * ファイルが選択されていない場合は何もしない
   */
  start(): void {
    if (this.isRunning || !this.fileHandle) return
    this.isRunning = true
    this.scheduleNext()
  }

  /**
   * 【ポーリング停止】: タイマーをクリアして停止
   */
  stop(): void {
    this.isRunning = false
    if (this.timerId !== null) {
      clearTimeout(this.timerId)
      this.timerId = null
    }
  }

  /**
   * 【ポーリング間隔変更】: 次回スケジュールから有効
   */
  setInterval(ms: number): void {
    this.config.intervalMs = ms
  }

  /** 現在のポーリング状態 */
  get running(): boolean {
    return this.isRunning
  }

  /** 現在のバイトオフセット（テスト・モニタリング用）*/
  get offset(): number {
    return this.byteOffset
  }

  // -------------------------------------------------------------------------
  // 内部実装
  // -------------------------------------------------------------------------

  /**
   * 【次回スケジュール】: 指定間隔後に poll() を呼ぶ
   */
  private scheduleNext(): void {
    if (!this.isRunning) return
    this.timerId = setTimeout(() => {
      this.poll().finally(() => this.scheduleNext())
    }, this.config.intervalMs)
  }

  /**
   * 【差分読み込み】: ファイルから byteOffset 以降を読み込み WASM に渡す
   *
   * 【処理フロー】:
   *   1. ファイルサイズを確認 → 変化がなければスキップ
   *   2. byteOffset からスライスを読み込む
   *   3. WASM append_journal_diff() に渡す
   *   4. consumed_bytes だけ byteOffset を更新
   *   5. エラー時は consecutiveErrors をインクリメント → 3 回で自動停止 🟢 REQ-135
   */
  async poll(): Promise<PollResult> {
    if (!this.fileHandle) {
      return { newCompleted: 0, consumedBytes: 0 }
    }

    try {
      const file = await this.fileHandle.getFile()
      const fileSize = file.size

      // 【差分なし】: ファイルサイズが変わっていなければスキップ
      if (fileSize <= this.byteOffset) {
        this.consecutiveErrors = 0
        return { newCompleted: 0, consumedBytes: 0 }
      }

      // 【差分読み込み】: byteOffset から末尾まで
      const slice = file.slice(this.byteOffset)
      const buffer = await slice.arrayBuffer()
      const data = new Uint8Array(buffer)

      // 【WASM 呼び出し】: 差分データを渡して新規 COMPLETE 試行数を取得
      const wasm = await WasmLoader.getInstance()
      const result = wasm.appendJournalDiff(data)

      // 【オフセット更新】: consumed_bytes だけ進める
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

      // 【自動停止】: 3 回連続エラーで停止 🟢 REQ-135
      if (this.consecutiveErrors >= MAX_ERROR_COUNT) {
        this.stop()
        this.config.onAutoStop()
      }

      return { newCompleted: 0, consumedBytes: 0 }
    }
  }
}
