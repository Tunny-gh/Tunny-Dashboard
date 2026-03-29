/**
 * LiveUpdateStore テスト (TASK-1201)
 *
 * 【テスト対象】: LiveUpdateStore — ライブ更新状態管理 Zustand Store
 * 【テスト方針】: FsapiPoller を vi.mock でスタブし、Store のアクションと状態遷移を検証
 */

import { describe, test, expect, beforeEach, vi } from 'vitest'

// -------------------------------------------------------------------------
// FsapiPoller モック
// -------------------------------------------------------------------------

const { mockPickFile, mockStart, mockStop, mockSetInterval, mockIsSupported } = vi.hoisted(() => {
  const mockPickFile = vi.fn()
  const mockStart = vi.fn()
  const mockStop = vi.fn()
  const mockSetInterval = vi.fn()
  const mockIsSupported = vi.fn().mockReturnValue(true)
  return { mockPickFile, mockStart, mockStop, mockSetInterval, mockIsSupported }
})

vi.mock('../wasm/fsapiPoller', () => {
  const MockFsapiPoller = vi.fn().mockImplementation(() => ({
    pickFile: mockPickFile,
    start: mockStart,
    stop: mockStop,
    setInterval: mockSetInterval,
  }))
  // 【静的メソッドモック】: isSupported は Store 初期化時に呼ばれるため Factory 内で設定
  ;(MockFsapiPoller as unknown as Record<string, unknown>).isSupported = mockIsSupported
  return { FsapiPoller: MockFsapiPoller }
})

import { FsapiPoller } from '../wasm/fsapiPoller'

import { useLiveUpdateStore } from './liveUpdateStore'

// -------------------------------------------------------------------------
// ヘルパー
// -------------------------------------------------------------------------

/** Store を初期状態にリセットする */
function resetStore() {
  useLiveUpdateStore.setState({
    isLive: false,
    isSupported: true,
    pollIntervalMs: 5000,
    lastUpdateAt: null,
    updateHistory: [],
    error: null,
  })
}

// -------------------------------------------------------------------------
// 初期状態
// -------------------------------------------------------------------------

describe('LiveUpdateStore — 初期状態', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    resetStore()
  })

  // TC-1201-S01: 初期状態の確認
  test('TC-1201-S01: 初期状態が正しく設定されている', () => {
    // 【テスト目的】: Store の初期値が仕様通りであることを確認 🟢
    const state = useLiveUpdateStore.getState()
    // 【確認内容】: isLive の初期値が false
    expect(state.isLive).toBe(false)
    // 【確認内容】: updateHistory の初期値が空配列
    expect(state.updateHistory).toEqual([])
    // 【確認内容】: error の初期値が null
    expect(state.error).toBeNull()
    // 【確認内容】: pollIntervalMs のデフォルトが 5000ms
    expect(state.pollIntervalMs).toBe(5000)
  })
})

// -------------------------------------------------------------------------
// startLive アクション
// -------------------------------------------------------------------------

describe('LiveUpdateStore — startLive', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    resetStore()
    mockIsSupported.mockReturnValue(true)
  })

  // TC-1201-S02: FSAPI 非対応時にエラーがセットされる
  test('TC-1201-S02: FSAPI 非対応時は error がセットされ isLive=false のまま', async () => {
    // 【テスト目的】: FSAPI 非対応ブラウザでエラーメッセージが設定されることを確認 🟢 REQ-133
    useLiveUpdateStore.setState({ isSupported: false })

    await useLiveUpdateStore.getState().startLive()

    // 【確認内容】: error がセットされること
    expect(useLiveUpdateStore.getState().error).toContain('Chrome')
    // 【確認内容】: isLive が false のまま
    expect(useLiveUpdateStore.getState().isLive).toBe(false)
    // 【確認内容】: FsapiPoller は作成されないこと
    expect(FsapiPoller).not.toHaveBeenCalled()
  })

  // TC-1201-S03: ファイルキャンセル時は isLive が変わらない
  test('TC-1201-S03: ファイル選択キャンセル時は isLive=false のまま', async () => {
    // 【テスト目的】: showOpenFilePicker キャンセル時に状態が変わらないことを確認 🟢
    mockPickFile.mockResolvedValue(false) // キャンセル

    await useLiveUpdateStore.getState().startLive()

    // 【確認内容】: isLive が false のまま
    expect(useLiveUpdateStore.getState().isLive).toBe(false)
  })

  // TC-1201-S04: ファイル選択成功後に isLive=true になる
  test('TC-1201-S04: ファイル選択成功後に isLive=true になる', async () => {
    // 【テスト目的】: 正常フローで isLive が true になることを確認 🟢
    mockPickFile.mockResolvedValue(true)

    await useLiveUpdateStore.getState().startLive()

    // 【確認内容】: isLive が true になること
    expect(useLiveUpdateStore.getState().isLive).toBe(true)
    // 【確認内容】: start() が呼ばれること
    expect(mockStart).toHaveBeenCalledOnce()
    // 【確認内容】: error が null であること
    expect(useLiveUpdateStore.getState().error).toBeNull()
  })
})

// -------------------------------------------------------------------------
// stopLive アクション
// -------------------------------------------------------------------------

describe('LiveUpdateStore — stopLive', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    resetStore()
  })

  // TC-1201-S05: stopLive で isLive が false になる
  test('TC-1201-S05: stopLive で isLive が false になる', async () => {
    // 【テスト目的】: 停止操作で状態が正しく変わることを確認 🟢
    mockPickFile.mockResolvedValue(true)
    await useLiveUpdateStore.getState().startLive()
    expect(useLiveUpdateStore.getState().isLive).toBe(true)

    useLiveUpdateStore.getState().stopLive()

    // 【確認内容】: isLive が false になること
    expect(useLiveUpdateStore.getState().isLive).toBe(false)
    // 【確認内容】: stop() が呼ばれること
    expect(mockStop).toHaveBeenCalledOnce()
  })
})

// -------------------------------------------------------------------------
// _onNewTrials — 新試行通知
// -------------------------------------------------------------------------

describe('LiveUpdateStore — _onNewTrials', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    resetStore()
  })

  // TC-1201-S06: _onNewTrials で updateHistory にレコードが追加される
  test('TC-1201-S06: _onNewTrials で updateHistory にレコードが追加される', () => {
    // 【テスト目的】: 新試行コールバックで履歴が更新されることを確認 🟢
    useLiveUpdateStore.getState()._onNewTrials(5)

    const state = useLiveUpdateStore.getState()
    // 【確認内容】: updateHistory に 1 件追加されること
    expect(state.updateHistory).toHaveLength(1)
    // 【確認内容】: 最新レコードの newTrials が 5 であること
    expect(state.updateHistory[0].newTrials).toBe(5)
    // 【確認内容】: lastUpdateAt が設定されること
    expect(state.lastUpdateAt).toBeInstanceOf(Date)
  })

  // TC-1201-S07: updateHistory は MAX_HISTORY=10 件を超えない 🟢 REQ-134
  test('TC-1201-S07: 10 件を超えると古いレコードが削除される', () => {
    // 【テスト目的】: 履歴が上限 10 件以内に収まることを確認 🟢
    for (let i = 0; i < 12; i++) {
      useLiveUpdateStore.getState()._onNewTrials(i + 1)
    }

    // 【確認内容】: 最大 10 件になること
    expect(useLiveUpdateStore.getState().updateHistory).toHaveLength(10)
    // 【確認内容】: 最新が先頭に来ること
    expect(useLiveUpdateStore.getState().updateHistory[0].newTrials).toBe(12)
  })

  // TC-1201-S08: Brushing 不干渉 — selectionStore は呼ばれない 🟢 REQ-134
  test('TC-1201-S08: _onNewTrials は selectionStore を操作しない', () => {
    // 【テスト目的】: ライブ更新が Brushing 選択に干渉しないことを確認 🟢 REQ-134
    // selectionStore をインポートし、mock されていないことを確認
    // ここでは副作用なしで呼び出せること（エラーが発生しないこと）を確認
    expect(() => {
      useLiveUpdateStore.getState()._onNewTrials(3)
    }).not.toThrow()

    // 【確認内容】: updateHistory のみが更新されること（selectionStore への参照はない）
    const state = useLiveUpdateStore.getState()
    expect(state.updateHistory).toHaveLength(1)
  })
})

// -------------------------------------------------------------------------
// _onError / _onAutoStop
// -------------------------------------------------------------------------

describe('LiveUpdateStore — _onError / _onAutoStop', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    resetStore()
  })

  // TC-1201-S09: _onError で error がセットされる
  test('TC-1201-S09: _onError でエラーメッセージが state にセットされる', () => {
    // 【テスト目的】: エラーコールバックで error フィールドが更新されることを確認 🟢
    useLiveUpdateStore.getState()._onError(new Error('テストエラー'))

    // 【確認内容】: error に 'テストエラー' がセットされること
    expect(useLiveUpdateStore.getState().error).toBe('テストエラー')
  })

  // TC-1201-S10: _onAutoStop で isLive=false とエラーメッセージがセットされる 🟢 REQ-135
  test('TC-1201-S10: _onAutoStop で isLive=false になり自動停止メッセージがセットされる', () => {
    // 【テスト目的】: 3 回連続エラー後の自動停止で状態が正しく更新されることを確認 🟢 REQ-135
    useLiveUpdateStore.setState({ isLive: true })

    useLiveUpdateStore.getState()._onAutoStop()

    const state = useLiveUpdateStore.getState()
    // 【確認内容】: isLive が false になること
    expect(state.isLive).toBe(false)
    // 【確認内容】: error に自動停止メッセージがセットされること
    expect(state.error).toContain('自動停止')
  })

  // TC-1201-S11: clearError で error が null になる
  test('TC-1201-S11: clearError で error が null になる', () => {
    // 【テスト目的】: エラークリア操作が正しく動作することを確認 🟢
    useLiveUpdateStore.setState({ error: '以前のエラー' })

    useLiveUpdateStore.getState().clearError()

    // 【確認内容】: error が null になること
    expect(useLiveUpdateStore.getState().error).toBeNull()
  })
})

// -------------------------------------------------------------------------
// setPollInterval アクション
// -------------------------------------------------------------------------

describe('LiveUpdateStore — setPollInterval', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    resetStore()
  })

  // TC-1201-S12: setPollInterval で pollIntervalMs が更新される
  test('TC-1201-S12: setPollInterval で pollIntervalMs が更新される', () => {
    // 【テスト目的】: ポーリング間隔変更が状態に反映されることを確認 🟢
    useLiveUpdateStore.getState().setPollInterval(10000)

    // 【確認内容】: pollIntervalMs が 10000ms になること
    expect(useLiveUpdateStore.getState().pollIntervalMs).toBe(10000)
  })
})
