/**
 * SelectionStore テスト (TASK-302)
 *
 * 【テスト対象】: useSelectionStore — Brushing & Linking の中核 Zustand Store
 * 【テスト方針】: WasmLoader を vi.mock でモック、Zustand 実 API を使用
 */

import { describe, test, expect, beforeEach, vi, afterEach } from 'vitest'

// -------------------------------------------------------------------------
// WasmLoader モック
// vi.hoisted でファクトリより前に変数を確保する（hoisting 問題の回避）
// -------------------------------------------------------------------------

const { mockFilterByRanges, mockGetInstance } = vi.hoisted(() => {
  const mockFilterByRanges = vi.fn()
  const mockGetInstance = vi.fn().mockResolvedValue({ filterByRanges: mockFilterByRanges })
  return { mockFilterByRanges, mockGetInstance }
})

vi.mock('../wasm/wasmLoader', () => ({
  WasmLoader: {
    getInstance: mockGetInstance,
    reset: vi.fn(),
  },
}))

import { useSelectionStore } from './selectionStore'
import { GpuBuffer } from '../wasm/gpuBuffer'

// -------------------------------------------------------------------------
// テストヘルパー（GpuBuffer 用）
// -------------------------------------------------------------------------

/**
 * 【ヘルパー】: N 点分の GpuBuffer 初期化データを生成する
 */
function makeGpuBufferData(n: number) {
  const positions = new Float32Array(n * 2)
  const positions3d = new Float32Array(n * 3)
  const sizes = new Float32Array(n * 1)
  for (let i = 0; i < n * 2; i++) positions[i] = i * 0.01 + 0.001
  for (let i = 0; i < n * 3; i++) positions3d[i] = i * 0.01 + 0.001
  for (let i = 0; i < n; i++) sizes[i] = i * 0.1 + 1.0
  return {
    positions: positions.buffer,
    positions3d: positions3d.buffer,
    sizes: sizes.buffer,
    trialCount: n,
  }
}

// -------------------------------------------------------------------------
// テストヘルパー
// -------------------------------------------------------------------------

/**
 * 【ヘルパー】: 各テスト前に Zustand store を初期状態にリセットする
 */
function resetStore() {
  useSelectionStore.setState({
    selectedIndices: new Uint32Array(0),
    filterRanges: {},
    highlighted: null,
    colorMode: 'objective',
    _trialCount: 0,
  })
}

// -------------------------------------------------------------------------
// 正常系
// -------------------------------------------------------------------------

describe('SelectionStore — 正常系', () => {
  beforeEach(() => {
    resetStore()
    vi.clearAllMocks()
    mockFilterByRanges.mockReturnValue(new Uint32Array([0, 2, 4]))
    mockGetInstance.mockResolvedValue({ filterByRanges: mockFilterByRanges })
  })

  // TC-302-01: brushSelect が selectedIndices を更新する
  test('TC-302-01: brushSelect が selectedIndices を [1,3,5] に更新する', () => {
    // 【テスト目的】: brushSelect アクションが状態を同期更新すること 🟢
    const selected = new Uint32Array([1, 3, 5])

    // 【処理実行】: brushSelect を呼び出す
    useSelectionStore.getState().brushSelect(selected)

    // 【確認内容】: selectedIndices が更新されている
    expect(Array.from(useSelectionStore.getState().selectedIndices)).toEqual([1, 3, 5])
  })

  // TC-302-02: clearSelection が全インデックスを返す
  test('TC-302-02: _trialCount=5 の状態で clearSelection が [0,1,2,3,4] を返す', () => {
    // 【テスト目的】: clearSelection が trialCount に基づいた全インデックスを生成すること 🟢

    // 【前提条件設定】: trialCount を 5 に初期化
    useSelectionStore.setState({ _trialCount: 5 })

    // 【処理実行】: clearSelection を呼び出す
    useSelectionStore.getState().clearSelection()

    const state = useSelectionStore.getState()
    // 【確認内容】: selectedIndices が全インデックス
    expect(Array.from(state.selectedIndices)).toEqual([0, 1, 2, 3, 4])
    // 【確認内容】: filterRanges が空
    expect(state.filterRanges).toEqual({})
  })

  // TC-302-03: addAxisFilter が filterRanges を同期更新する
  test('TC-302-03: addAxisFilter("x", 0.1, 0.9) が filterRanges を同期で更新する', () => {
    // 【テスト目的】: filterRanges の更新が同期的であること（WASM 呼び出し前に確認可能）🟢

    // 【処理実行】
    useSelectionStore.getState().addAxisFilter('x', 0.1, 0.9)

    // 【確認内容】: filterRanges が即座に更新されている
    expect(useSelectionStore.getState().filterRanges['x']).toEqual({ min: 0.1, max: 0.9 })
  })

  // TC-302-04: addAxisFilter が WASM filterByRanges を呼び出して selectedIndices を更新する
  test('TC-302-04: addAxisFilter 後に filterByRanges が呼ばれ selectedIndices が更新される', async () => {
    // 【テスト目的】: addAxisFilter の非同期 WASM 呼び出しが selectedIndices を更新すること 🟢
    mockFilterByRanges.mockReturnValue(new Uint32Array([0, 2, 4]))

    // 【処理実行】: addAxisFilter を呼び出す
    useSelectionStore.getState().addAxisFilter('x', 0.0, 0.5)

    // 【確認内容】: WASM filterByRanges が呼ばれ結果で selectedIndices が更新される
    await vi.waitFor(() => {
      expect(mockFilterByRanges).toHaveBeenCalledOnce()
      expect(Array.from(useSelectionStore.getState().selectedIndices)).toEqual([0, 2, 4])
    })
  })

  // TC-302-05: removeAxisFilter が filterRanges からキーを除去する
  test('TC-302-05: removeAxisFilter("x") が filterRanges から x を除去する', async () => {
    // 【テスト目的】: removeAxisFilter がフィルタを正しく除去すること 🟢
    useSelectionStore.getState().addAxisFilter('x', 0.0, 1.0)
    await vi.waitFor(() => expect(mockFilterByRanges).toHaveBeenCalledOnce())

    vi.clearAllMocks()
    // 【処理実行】
    useSelectionStore.getState().removeAxisFilter('x')

    // 【確認内容】: filterRanges から x が消えている
    expect(useSelectionStore.getState().filterRanges['x']).toBeUndefined()
  })

  // TC-302-06: setHighlight が highlighted を更新する
  test('TC-302-06: setHighlight(7) が highlighted を 7 に更新する', () => {
    // 【テスト目的】: setHighlight アクションが正しく動作すること 🟢
    useSelectionStore.getState().setHighlight(7)
    expect(useSelectionStore.getState().highlighted).toBe(7)
  })

  // TC-302-07: setColorMode が colorMode を更新する
  test('TC-302-07: setColorMode("cluster") が colorMode を更新する', () => {
    // 【テスト目的】: setColorMode アクションが正しく動作すること 🟢
    useSelectionStore.getState().setColorMode('cluster')
    expect(useSelectionStore.getState().colorMode).toBe('cluster')
  })

  // TC-302-08: subscribe で GpuBuffer が自動更新される
  test('TC-302-08: subscribe で brushSelect 後に GpuBuffer のアルファが更新される', () => {
    // 【テスト目的】: Zustand subscribe を使った React サイクル外 GPU 更新の動作確認 🟢

    // 【テストデータ準備】: N=5 の GpuBuffer を作成
    const buf = new GpuBuffer(makeGpuBufferData(5))

    // 【購読設定】: selectionStore 変更時に GpuBuffer.updateAlphas を呼ぶ
    const unsubscribe = useSelectionStore.subscribe(
      (state) => state.selectedIndices,
      (indices) => buf.updateAlphas(indices),
    )

    // 【処理実行】: idx=2,4 を選択
    useSelectionStore.getState().brushSelect(new Uint32Array([2, 4]))

    // 【確認内容】: 選択点の alpha = 1.0
    expect(buf.colors[2 * 4 + 3]).toBeCloseTo(1.0)
    expect(buf.colors[4 * 4 + 3]).toBeCloseTo(1.0)
    // 【確認内容】: 非選択点の alpha = 0.2
    expect(buf.colors[0 * 4 + 3]).toBeCloseTo(0.2)
    expect(buf.colors[1 * 4 + 3]).toBeCloseTo(0.2)

    unsubscribe()
  })
})

// -------------------------------------------------------------------------
// 異常系
// -------------------------------------------------------------------------

describe('SelectionStore — 異常系', () => {
  beforeEach(() => {
    resetStore()
    vi.clearAllMocks()
  })

  // TC-302-E02: addAxisFilter WASM 未初期化でもクラッシュしない
  test('TC-302-E02: WasmLoader.getInstance が reject しても addAxisFilter はクラッシュしない', async () => {
    // 【テスト目的】: WASM 未初期化時にフィルタ操作がクラッシュしないこと 🟢
    mockGetInstance.mockRejectedValueOnce(new Error('WASM not ready'))

    // 【処理実行】: クラッシュしないこと
    expect(() => useSelectionStore.getState().addAxisFilter('y', 0, 1)).not.toThrow()

    // 【確認内容】: filterRanges は同期で更新されている
    expect(useSelectionStore.getState().filterRanges['y']).toEqual({ min: 0, max: 1 })

    // 【確認内容】: selectedIndices は変更されない（WASM 呼び出し失敗のため）
    await new Promise((r) => setTimeout(r, 20)) // Promise settle 待機
    expect(useSelectionStore.getState().selectedIndices.length).toBe(0)
  })
})

// -------------------------------------------------------------------------
// 境界値
// -------------------------------------------------------------------------

describe('SelectionStore — 境界値', () => {
  beforeEach(() => {
    resetStore()
    vi.clearAllMocks()
  })

  // TC-302-B01: clearSelection で _trialCount=0 でも crash しない
  test('TC-302-B01: _trialCount=0 の状態で clearSelection しても crash しない', () => {
    // 【テスト目的】: 空 DataFrame でも clearSelection が安全に動作すること 🟢
    useSelectionStore.setState({ _trialCount: 0 })

    expect(() => useSelectionStore.getState().clearSelection()).not.toThrow()
    expect(useSelectionStore.getState().selectedIndices.length).toBe(0)
  })

  // TC-302-B02: removeAxisFilter 存在しないキーでも crash しない
  test('TC-302-B02: filterRanges が空の状態で removeAxisFilter("nonexistent") してもクラッシュしない', () => {
    // 【テスト目的】: 存在しないフィルタキーの削除が安全に処理されること 🟢
    expect(() => useSelectionStore.getState().removeAxisFilter('nonexistent')).not.toThrow()
    expect(useSelectionStore.getState().filterRanges).toEqual({})
  })
})
