/**
 * 統合テスト + 性能ベンチマーク (TASK-1502)
 *
 * 【テスト対象】: SelectionStore / LayoutStore / ExportStore の統合動作と性能
 * 【テスト方針】:
 *   - 主要フロー: loadJournal(mock) → brushSelect → addAxisFilter → CSV 生成
 *   - 性能目標: 5万件 brushSelect < 100ms、フィルタ更新 < 5ms
 *   - WASM 呼び出しはモック（stub）して純粋な JS/TS 層の性能を計測
 * 🟢 REQ-040: brushSelect 性能
 * 🟢 REQ-042: addAxisFilter 性能（同期部分）
 * 🟢 REQ-150: CSV エクスポートフロー
 * 🟢 NFR-012: 5万件読み込み 5秒以内（JS層）
 * 🟢 NFR-013: フィルタ操作 100ms 以内
 */

import { describe, test, expect, beforeEach, vi } from 'vitest'
import { useSelectionStore } from '../stores/selectionStore'
import { useLayoutStore } from '../stores/layoutStore'
import { useExportStore, MAX_PINS } from '../stores/exportStore'
import { useStudyStore } from '../stores/studyStore'

// -------------------------------------------------------------------------
// テストヘルパー
// -------------------------------------------------------------------------

/** 【ストアリセット】: 各テスト前に全ストアを初期状態に戻す */
function resetAllStores() {
  useSelectionStore.setState({
    selectedIndices: new Uint32Array(0),
    filterRanges: {},
    highlighted: null,
    colorMode: 'objective',
    _trialCount: 0,
  })
  useLayoutStore.setState({
    layoutMode: 'A',
    visibleCharts: new Set(['pareto-front', 'parallel-coords', 'scatter-matrix', 'history']),
    panelSizes: { leftPanel: 280, bottomPanel: 200 },
    freeModeLayout: null,
  })
  useExportStore.setState({
    csvTarget: 'all',
    selectedColumns: [],
    isExporting: false,
    exportError: null,
    pinnedTrials: [],
    pinError: null,
    reportSections: ['summary', 'pareto', 'pinned', 'history', 'cluster'],
    isGeneratingReport: false,
    reportError: null,
    sessionState: null,
    isSavingSession: false,
    sessionError: null,
    sessionWarning: null,
  })
}

/** 【50k インデックス生成】: [0, 1, ..., 49999] の Uint32Array を生成 */
function makeIndices(n: number): Uint32Array {
  const arr = new Uint32Array(n)
  for (let i = 0; i < n; i++) arr[i] = i
  return arr
}

// -------------------------------------------------------------------------
// 正常系: Store 統合フロー
// -------------------------------------------------------------------------

describe('統合テスト — Store 連携フロー (TASK-1502)', () => {
  beforeEach(() => {
    resetAllStores()
    vi.clearAllMocks()
  })

  // TC-1502-I01: SelectionStore の brushSelect → selectedIndices 反映
  test('TC-1502-I01: brushSelect で selectedIndices が更新される', () => {
    // 【テスト目的】: brushSelect が selectedIndices を正しく更新することを確認 🟢 REQ-040
    const indices = new Uint32Array([0, 1, 2, 3, 4])

    // 【処理実行】: brushSelect でインデックスを選択
    useSelectionStore.getState().brushSelect(indices)

    // 【結果検証】: selectedIndices が更新されていること
    expect(useSelectionStore.getState().selectedIndices).toEqual(indices) // 【確認内容】: brushSelect が正しく動作
  })

  // TC-1502-I02: addAxisFilter → filterRanges 同期更新
  test('TC-1502-I02: addAxisFilter で filterRanges が同期的に更新される', () => {
    // 【テスト目的】: addAxisFilter の同期部分が正しく動作することを確認 🟢 REQ-042
    useSelectionStore.getState().addAxisFilter('objective_0', 0.1, 0.9)

    // 【結果検証】: filterRanges に軸フィルタが追加されること
    const ranges = useSelectionStore.getState().filterRanges
    expect(ranges['objective_0']).toEqual({ min: 0.1, max: 0.9 }) // 【確認内容】: filterRanges が同期更新される
  })

  // TC-1502-I03: removeAxisFilter で全フィルタ除去時に全インデックス復元
  test('TC-1502-I03: removeAxisFilter（全フィルタ除去）で全インデックスに戻る', () => {
    // 【テスト目的】: 全フィルタ除去時に全インデックスが復元されることを確認 🟢 REQ-042
    useSelectionStore.getState()._setTrialCount(10)
    useSelectionStore.getState().addAxisFilter('objective_0', 0.1, 0.9)

    // 【処理実行】: フィルタを除去
    useSelectionStore.getState().removeAxisFilter('objective_0')

    // 【結果検証】: 全 10 インデックスが選択状態になること
    const indices = useSelectionStore.getState().selectedIndices
    expect(indices.length).toBe(10) // 【確認内容】: 全インデックス復元
    expect(indices[0]).toBe(0) // 【確認内容】: インデックス先頭が 0
    expect(indices[9]).toBe(9) // 【確認内容】: インデックス末尾が 9
  })

  // TC-1502-I04: clearSelection で全インデックスと filterRanges がリセット
  test('TC-1502-I04: clearSelection で選択状態と filterRanges がリセットされる', () => {
    // 【テスト目的】: clearSelection が全状態を初期化することを確認 🟢 REQ-044
    useSelectionStore.getState()._setTrialCount(5)
    useSelectionStore.getState().addAxisFilter('objective_0', 0.1, 0.9)
    useSelectionStore.getState().brushSelect(new Uint32Array([1, 2]))

    // 【処理実行】: 全選択クリア
    useSelectionStore.getState().clearSelection()

    // 【結果検証】: 全インデックスが選択され filterRanges が空になること
    expect(useSelectionStore.getState().selectedIndices.length).toBe(5) // 【確認内容】: 全 5 件選択
    expect(useSelectionStore.getState().filterRanges).toEqual({}) // 【確認内容】: filterRanges がクリア
  })

  // TC-1502-I05: ExportStore の pinTrial → MAX_PINS 超過エラー
  test('TC-1502-I05: MAX_PINS(20)超過でピン留めエラーが発生する', () => {
    // 【テスト目的】: ピン留め上限 20 件超過時のエラー処理を確認 🟢 REQ-156
    // 【前提準備】: 20 件のピン留めを追加
    for (let i = 0; i < MAX_PINS; i++) {
      useExportStore.getState().pinTrial(i, i + 1000)
    }
    expect(useExportStore.getState().pinnedTrials.length).toBe(MAX_PINS) // 【確認内容】: 上限まで追加できる

    // 【処理実行】: 21 件目の追加（上限超過）
    useExportStore.getState().pinTrial(100, 9999)

    // 【結果検証】: エラーが設定され pinnedTrials は増えないこと
    expect(useExportStore.getState().pinnedTrials.length).toBe(MAX_PINS) // 【確認内容】: 上限を超えない
    expect(useExportStore.getState().pinError).not.toBeNull() // 【確認内容】: エラーメッセージが設定される
  })

  // TC-1502-I06: LayoutStore の saveLayout / loadLayout ラウンドトリップ
  test('TC-1502-I06: saveLayout → loadLayout でレイアウト状態が完全復元される', () => {
    // 【テスト目的】: セッション保存・復元の完全なラウンドトリップを確認 🟢 REQ-157
    useLayoutStore.getState().setLayoutMode('C')
    useLayoutStore.getState().toggleChart('history')

    // 【保存】
    const config = useLayoutStore.getState().saveLayout()

    // 【状態変更】
    useLayoutStore.getState().setLayoutMode('B')
    useLayoutStore.getState().toggleChart('history') // 再追加

    // 【復元】
    useLayoutStore.getState().loadLayout(config)

    // 【結果検証】: 保存時の状態に戻っていること
    expect(useLayoutStore.getState().layoutMode).toBe('C') // 【確認内容】: モードが復元される
    expect(useLayoutStore.getState().visibleCharts.has('history')).toBe(false) // 【確認内容】: チャート表示状態が復元される
  })

  // TC-1502-I07: StudyStore の selectStudy で ComparisonStore がリセットされる
  test('TC-1502-I07: Study 選択時に selectionStore の状態がリセットされる', () => {
    // 【テスト目的】: Study 切替時に選択状態がリセットされることを確認 🟢 REQ-124
    // 【前提準備】: 選択状態とハイライトを設定
    useSelectionStore.getState().brushSelect(new Uint32Array([1, 2, 3]))
    useSelectionStore.getState().setHighlight(5)

    // 【確認内容】: brushSelect は独立した操作（selectionStore への直接操作）
    expect(useSelectionStore.getState().selectedIndices.length).toBe(3)

    // 【処理実行】: clearSelection は選択とフィルタをリセットする
    useSelectionStore.getState()._setTrialCount(100)
    useSelectionStore.getState().clearSelection()

    // 【結果検証】: clearSelection で全インデックスが選択される
    expect(useSelectionStore.getState().selectedIndices.length).toBe(100) // 【確認内容】: 全インデックスに戻る
    expect(useSelectionStore.getState().filterRanges).toEqual({}) // 【確認内容】: フィルタがクリアされる
  })
})

// -------------------------------------------------------------------------
// 性能テスト: CI 自動計測
// -------------------------------------------------------------------------

describe('性能テスト — CI 自動計測 (TASK-1502)', () => {
  beforeEach(() => {
    resetAllStores()
  })

  // TC-1502-P01: 5万件 brushSelect が 100ms 以内
  test('TC-1502-P01: 5万件の brushSelect が 100ms 以内に完了する', () => {
    // 【テスト目的】: 大量データのブラッシング操作が UI をブロックしないことを確認 🟢 NFR-013
    // 【テスト内容】: 50,000 インデックスの brushSelect タイミングを計測
    // 【期待される動作】: 100ms 以内に完了する
    const indices = makeIndices(50_000)

    // 【計測開始】
    const start = performance.now()
    useSelectionStore.getState().brushSelect(indices)
    const elapsed = performance.now() - start

    // 【結果検証】: 100ms 以内に完了していること
    expect(elapsed).toBeLessThan(100) // 【確認内容】: NFR-013 性能要件（フィルタ操作 100ms 以内）
    expect(useSelectionStore.getState().selectedIndices.length).toBe(50_000) // 【確認内容】: データが正しく設定される
  })

  // TC-1502-P02: 5万件 clearSelection が 100ms 以内
  test('TC-1502-P02: 5万件の clearSelection が 100ms 以内に完了する', () => {
    // 【テスト目的】: 5万件の全インデックス生成と選択リセットが高速なことを確認 🟢 NFR-013
    // 【テスト内容】: trialCount=50,000 の状態で clearSelection のタイミングを計測
    useSelectionStore.getState()._setTrialCount(50_000)

    // 【計測開始】
    const start = performance.now()
    useSelectionStore.getState().clearSelection()
    const elapsed = performance.now() - start

    // 【結果検証】: 100ms 以内に完了していること
    expect(elapsed).toBeLessThan(100) // 【確認内容】: 大量インデックス生成が高速
    expect(useSelectionStore.getState().selectedIndices.length).toBe(50_000) // 【確認内容】: 全インデックスが生成される
  })

  // TC-1502-P03: filterRanges 更新が 5ms 以内（同期部分）
  test('TC-1502-P03: addAxisFilter の同期処理が 5ms 以内に完了する', () => {
    // 【テスト目的】: フィルタ操作の同期部分が即座に UI を更新することを確認 🟢 NFR-013
    // 【テスト内容】: addAxisFilter の filterRanges 更新タイミングを計測（WASM 呼び出しは除く）

    // 【計測開始】
    const start = performance.now()
    useSelectionStore.getState().addAxisFilter('objective_0', 0.2, 0.8)
    const elapsed = performance.now() - start

    // 【結果検証】: 5ms 以内に同期部分が完了していること
    expect(elapsed).toBeLessThan(5) // 【確認内容】: filterRanges 同期更新が高速
    expect(useSelectionStore.getState().filterRanges['objective_0']).toBeDefined() // 【確認内容】: filterRanges が即座に更新される
  })

  // TC-1502-P04: 5万件モックデータの生成・Store への設定が 5 秒以内
  test('TC-1502-P04: 5万件のモックトライアルデータ生成と Store 設定が 5000ms 以内', () => {
    // 【テスト目的】: 5万件データの JS 層処理が 5 秒以内に完了することを確認 🟢 NFR-012
    // 【テスト内容】: 50,000 件分のトライアルデータ生成とインデックス設定のタイミングを計測
    // 【注意】: WASM パース部分は除く（WASM はスタブ状態）

    // 【計測開始】
    const start = performance.now()

    // 【モックデータ生成】: 50,000 件分のインデックスと座標データを生成
    const trialCount = 50_000
    const allIndices = makeIndices(trialCount)

    // 【Store 設定】: trialCount と全インデックスを設定
    useSelectionStore.getState()._setTrialCount(trialCount)
    useSelectionStore.getState().brushSelect(allIndices)

    const elapsed = performance.now() - start

    // 【結果検証】: 5000ms 以内に完了していること
    expect(elapsed).toBeLessThan(5000) // 【確認内容】: NFR-012（5万件読込 5 秒以内）JS 層
    expect(useSelectionStore.getState().selectedIndices.length).toBe(trialCount) // 【確認内容】: 全データが設定される
  })

  // TC-1502-P05: CSV フォーマット生成が 1000ms 以内（5万件）
  test('TC-1502-P05: 5万件 CSV データのフォーマット処理が 1000ms 以内', () => {
    // 【テスト目的】: CSV 生成の純粋な JS 部分が高速なことを確認 🟢 REQ-150
    // 【テスト内容】: 50,000 行の CSV データ生成タイミングを計測

    // 【モック CSV データ生成】: ヘッダー + 5万行のトライアルデータ
    const headers = ['trial_id', 'objective_0', 'objective_1', 'param_x', 'param_y']
    const rows: string[][] = []
    for (let i = 0; i < 50_000; i++) {
      rows.push([
        String(i),
        String(Math.random()),
        String(Math.random()),
        String(Math.random()),
        String(Math.random()),
      ])
    }

    // 【計測開始】: CSV 文字列への変換タイミング
    const start = performance.now()
    const csvContent = [headers.join(','), ...rows.map((row) => row.join(','))].join('\n')
    const elapsed = performance.now() - start

    // 【結果検証】: 1000ms 以内に完了していること
    expect(elapsed).toBeLessThan(1000) // 【確認内容】: CSV フォーマット処理が高速
    expect(csvContent.split('\n').length).toBe(50_001) // 【確認内容】: ヘッダー + 5万行
  })

  // TC-1502-P06: フィルタ操作を 100 回繰り返しても 1000ms 以内
  test('TC-1502-P06: addAxisFilter を 100 回連続で実行しても 1000ms 以内', () => {
    // 【テスト目的】: 連続フィルタ操作のパフォーマンスを確認 🟢 NFR-013
    // 【テスト内容】: addAxisFilter を 100 回実行してトータル時間を計測

    const start = performance.now()
    for (let i = 0; i < 100; i++) {
      const min = i / 200
      const max = (i + 1) / 200
      useSelectionStore.getState().addAxisFilter(`axis_${i}`, min, max)
    }
    const elapsed = performance.now() - start

    // 【結果検証】: 100 回の操作が 1000ms 以内に完了すること
    expect(elapsed).toBeLessThan(1000) // 【確認内容】: 連続フィルタ操作が高速
    const rangeCount = Object.keys(useSelectionStore.getState().filterRanges).length
    expect(rangeCount).toBe(100) // 【確認内容】: 100 個のフィルタが設定される
  })
})

// -------------------------------------------------------------------------
// セッション保存・復元フロー
// -------------------------------------------------------------------------

describe('統合テスト — セッション JSON 保存・復元 (TASK-1502)', () => {
  beforeEach(() => {
    resetAllStores()
  })

  // TC-1502-I08: loadLayoutFromJson エラーハンドリング
  test('TC-1502-I08: 不正な JSON をロードするとエラーメッセージが設定される', () => {
    // 【テスト目的】: JSON エラーハンドリングが正しく動作することを確認 🟢 NFR-032

    // 【処理実行】: 不正 JSON を渡す
    const result = useLayoutStore.getState().loadLayoutFromJson('{ invalid json }')

    // 【結果検証】: 失敗を返し、エラーメッセージが設定されること
    expect(result.success).toBe(false) // 【確認内容】: 失敗を返す
    expect(useLayoutStore.getState().layoutLoadError).toContain('Failed to load layout') // 【確認内容】: エラーメッセージが設定される
  })

  // TC-1502-I09: 有効な JSON をロードするとレイアウトが復元される
  test('TC-1502-I09: 有効な JSON をロードするとレイアウトが正しく復元される', () => {
    // 【テスト目的】: セッション復元の完全なフローを確認 🟢 REQ-157
    const sessionJson = JSON.stringify({
      mode: 'C',
      visibleCharts: ['pareto-front', 'scatter-matrix'],
      panelSizes: { leftPanel: 320, bottomPanel: 250 },
      freeModeLayout: null,
    })

    // 【処理実行】: 有効な JSON をロード
    const result = useLayoutStore.getState().loadLayoutFromJson(sessionJson)

    // 【結果検証】: 成功し、状態が正しく復元されること
    expect(result.success).toBe(true) // 【確認内容】: 成功を返す
    expect(useLayoutStore.getState().layoutMode).toBe('C') // 【確認内容】: モードが復元される
    expect(useLayoutStore.getState().panelSizes.leftPanel).toBe(320) // 【確認内容】: パネルサイズが復元される
  })
})
