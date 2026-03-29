/**
 * LiveUpdateStore — ライブ更新状態管理 Zustand Store (TASK-1201)
 *
 * 【役割】: ライブ更新の ON/OFF・進捗・履歴・エラーを一元管理する
 * 【設計方針】:
 *   - FsapiPoller を内部で保持し、start/stop を制御
 *   - FSAPI 非対応ブラウザのフォールバック検出
 *   - 更新履歴: 直近 MAX_HISTORY 件を保持
 *   - 連続エラー 3 回で自動停止 + エラートースト用メッセージセット
 *   - Brushing 不干渉: selectionStore は一切操作しない 🟢 REQ-134
 * 🟢 REQ-130〜REQ-135 に準拠
 */

import { create } from 'zustand'
import { FsapiPoller } from '../wasm/fsapiPoller'

// -------------------------------------------------------------------------
// 型定義
// -------------------------------------------------------------------------

/**
 * 【更新履歴レコード】: 直近の更新記録
 */
export interface UpdateRecord {
  /** 更新日時 */
  at: Date
  /** 今回の差分で追加された試行数 */
  newTrials: number
}

/**
 * 【LiveUpdateStore 状態型】
 */
interface LiveUpdateState {
  // --- 状態 ---
  /** ライブ更新が実行中かどうか */
  isLive: boolean
  /** FSAPI がこのブラウザでサポートされているか */
  isSupported: boolean
  /** ポーリング間隔 (ms) */
  pollIntervalMs: number
  /** 最終更新日時 */
  lastUpdateAt: Date | null
  /** 直近 MAX_HISTORY 件の更新履歴 */
  updateHistory: UpdateRecord[]
  /** エラーメッセージ（null=正常）*/
  error: string | null

  // --- アクション ---
  /** ファイル選択してライブ更新を開始する */
  startLive: () => Promise<void>
  /** ライブ更新を停止する */
  stopLive: () => void
  /** ポーリング間隔を変更する */
  setPollInterval: (ms: number) => void
  /** エラーメッセージをクリアする */
  clearError: () => void

  // --- 内部アクション（テスト用に公開）---
  /** 新試行通知を受け取り履歴を更新する（FsapiPoller コールバックから呼ばれる）*/
  _onNewTrials: (newCompleted: number) => void
  /** エラーを受け取り state に反映する */
  _onError: (err: Error) => void
  /** 自動停止通知を受け取る */
  _onAutoStop: () => void
}

// -------------------------------------------------------------------------
// 定数
// -------------------------------------------------------------------------

/** 【更新履歴保持件数】: 直近 N 件のみ保持 */
const MAX_HISTORY = 10

/** 【デフォルトポーリング間隔】: 5 秒 */
const DEFAULT_POLL_INTERVAL_MS = 5000

// -------------------------------------------------------------------------
// Store 実装
// -------------------------------------------------------------------------

/** ポーリングインスタンス（Store 外に保持して React レンダリングの影響を受けない）*/
let _poller: FsapiPoller | null = null

/**
 * 【LiveUpdateStore 作成】
 */
export const useLiveUpdateStore = create<LiveUpdateState>()((set, get) => ({
  // -------------------------------------------------------------------------
  // 初期状態
  // -------------------------------------------------------------------------
  isLive: false,
  isSupported: FsapiPoller.isSupported(),
  pollIntervalMs: DEFAULT_POLL_INTERVAL_MS,
  lastUpdateAt: null,
  updateHistory: [],
  error: null,

  // -------------------------------------------------------------------------
  // アクション実装
  // -------------------------------------------------------------------------

  /**
   * 【ライブ更新開始】: ファイル選択ダイアログを開いてポーリングを開始する
   * FSAPI 非対応時はエラーをセットして終了 🟢 REQ-133
   */
  startLive: async () => {
    const { isSupported, pollIntervalMs } = get()

    if (!isSupported) {
      set({ error: 'ライブ更新は Chrome / Edge のみ対応しています' })
      return
    }

    // 【ポーラー作成】: コールバックを Store アクションに接続
    _poller = new FsapiPoller({
      intervalMs: pollIntervalMs,
      onNewTrials: (n) => get()._onNewTrials(n),
      onError: (err) => get()._onError(err),
      onAutoStop: () => get()._onAutoStop(),
    })

    // 【ファイル選択】: ユーザーが選択キャンセルなら開始しない
    const selected = await _poller.pickFile()
    if (!selected) {
      _poller = null
      return
    }

    _poller.start()
    set({ isLive: true, error: null })
  },

  /**
   * 【ライブ更新停止】: ポーリングを停止して isLive を false にする
   */
  stopLive: () => {
    if (_poller) {
      _poller.stop()
      _poller = null
    }
    set({ isLive: false })
  },

  /**
   * 【ポーリング間隔変更】: 現在のポーラーにも反映する
   */
  setPollInterval: (ms) => {
    set({ pollIntervalMs: ms })
    if (_poller) {
      _poller.setInterval(ms)
    }
  },

  /** 【エラークリア】 */
  clearError: () => set({ error: null }),

  /**
   * 【新試行通知】: 差分で新規 COMPLETE 試行が検出されたとき呼ばれる
   * 🟢 REQ-134: selectionStore は操作しない（Brushing 不干渉）
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
   * 【エラー通知】: ポーリングエラーを Store に反映する
   */
  _onError: (err) => {
    set({ error: err.message })
  },

  /**
   * 【自動停止通知】: 3 回連続エラーで自動停止したとき呼ばれる 🟢 REQ-135
   */
  _onAutoStop: () => {
    _poller = null
    set({
      isLive: false,
      error: '更新に失敗しました（3回連続エラーにより自動停止しました）',
    })
  },
}))
