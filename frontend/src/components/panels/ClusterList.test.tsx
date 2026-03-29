/**
 * ClusterList テスト (TASK-902)
 *
 * 【テスト対象】: ClusterList — クラスタ一覧（件数・特徴サマリー・行クリック選択）
 * 【テスト方針】: selectionStore.brushSelect を vi.mock でモック
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest'
import { render, screen, cleanup, fireEvent } from '@testing-library/react'

// -------------------------------------------------------------------------
// selectionStore モック
// -------------------------------------------------------------------------

const { mockBrushSelect } = vi.hoisted(() => {
  const mockBrushSelect = vi.fn()
  return { mockBrushSelect }
})

vi.mock('../../stores/selectionStore', () => ({
  useSelectionStore: vi
    .fn()
    .mockImplementation((selector: (s: { brushSelect: typeof mockBrushSelect }) => unknown) =>
      selector({ brushSelect: mockBrushSelect }),
    ),
}))

import { ClusterList, getClusterColor } from './ClusterList'
import type { ClusterStatData } from './ClusterList'

// -------------------------------------------------------------------------
// テストデータ
// -------------------------------------------------------------------------

/** テスト用クラスタ統計データ */
const TEST_STATS: ClusterStatData[] = [
  {
    clusterId: 0,
    size: 30,
    centroid: [1.234, 5.678],
    stdDev: [0.111, 0.222],
    significantFeatures: [true, false],
  },
  {
    clusterId: 1,
    size: 20,
    centroid: [3.456, 2.345],
    stdDev: [0.333, 0.444],
    significantFeatures: [false, true],
  },
]

/** テスト用特徴名 */
const TEST_FEATURES = ['x1', 'x2']

/** テスト用クラスタ別インデックス */
const TEST_TRIALS_BY_CLUSTER = [
  new Uint32Array([0, 1, 2]), // cluster 0
  new Uint32Array([3, 4]), // cluster 1
]

// -------------------------------------------------------------------------
// 正常系
// -------------------------------------------------------------------------

describe('ClusterList — 正常系', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    cleanup()
  })

  // TC-902-09: clusterStats あり時にクラスタ一覧が表示される
  test('TC-902-09: clusterStats あり時にクラスタバッジが表示される', () => {
    // 【テスト目的】: クラスタ統計データがあれば一覧が表示されることを確認 🟢
    render(
      <ClusterList
        clusterStats={TEST_STATS}
        featureNames={TEST_FEATURES}
        trialsByCluster={TEST_TRIALS_BY_CLUSTER}
      />,
    )
    // 【確認内容】: C0, C1 バッジが表示されること
    expect(screen.getByTestId('cluster-badge-0')).toBeInTheDocument()
    expect(screen.getByTestId('cluster-badge-1')).toBeInTheDocument()
  })

  // TC-902-10: クラスタ行クリックで brushSelect が呼ばれる
  test('TC-902-10: クラスタ行クリックで selectionStore.brushSelect が呼ばれる', () => {
    // 【テスト目的】: クラスタ選択操作が brushSelect に連携されることを確認 🟢
    render(
      <ClusterList
        clusterStats={TEST_STATS}
        featureNames={TEST_FEATURES}
        trialsByCluster={TEST_TRIALS_BY_CLUSTER}
      />,
    )

    // 【処理実行】: クラスタ 0 の行をクリック
    fireEvent.click(screen.getByTestId('cluster-row-0'))

    // 【確認内容】: brushSelect が呼ばれ、cluster 0 のインデックスが渡されること
    expect(mockBrushSelect).toHaveBeenCalledOnce()
    const arg = mockBrushSelect.mock.calls[0][0] as Uint32Array
    expect(Array.from(arg)).toEqual([0, 1, 2])
  })

  // TC-902-11: クラスタ行クリック後に選択状態のスタイルが適用される
  test('TC-902-11: クラスタ行クリック後に行の background が選択色になる', () => {
    // 【テスト目的】: クリックした行が視覚的に選択状態を示すことを確認 🟢
    render(
      <ClusterList
        clusterStats={TEST_STATS}
        featureNames={TEST_FEATURES}
        trialsByCluster={TEST_TRIALS_BY_CLUSTER}
      />,
    )
    const row = screen.getByTestId('cluster-row-0')

    // 【処理実行】: クリック前は選択色でないことを確認
    expect(row.style.background).not.toBe('rgb(239, 246, 255)')

    // 【処理実行】: クリックで選択
    fireEvent.click(row)

    // 【確認内容】: background が選択色 #eff6ff (= rgb(239, 246, 255)) になること
    expect(row.style.background).toBe('rgb(239, 246, 255)')
  })

  // TC-902-12: 有意差★マークが表示される
  test('TC-902-12: significantFeatures=true の特徴に★マークが表示される', () => {
    // 【テスト目的】: Welch's t 検定で有意差ありの特徴に★が付くことを確認 🟢
    render(
      <ClusterList
        clusterStats={TEST_STATS}
        featureNames={TEST_FEATURES}
        trialsByCluster={TEST_TRIALS_BY_CLUSTER}
      />,
    )
    // 【確認内容】: cluster 0 / feature 0 に★が表示されること（significantFeatures[0]=true）
    expect(screen.getByTestId('sig-0-0')).toBeInTheDocument()
    // 【確認内容】: cluster 0 / feature 1 には★がないこと（significantFeatures[1]=false）
    expect(screen.queryByTestId('sig-0-1')).toBeNull()
  })

  // TC-902-13: centroid ± std が表示される
  test('TC-902-13: centroid ± std のテキストが各セルに表示される', () => {
    // 【テスト目的】: 統計値が正しくフォーマットされて表示されることを確認 🟢
    render(
      <ClusterList
        clusterStats={TEST_STATS}
        featureNames={TEST_FEATURES}
        trialsByCluster={TEST_TRIALS_BY_CLUSTER}
      />,
    )
    // 【確認内容】: cluster 0, feature 0 の centroid=1.234, std=0.111 が表示されること
    const statCell = screen.getByTestId('stat-0-0')
    expect(statCell).toHaveTextContent('1.234')
    expect(statCell).toHaveTextContent('0.111')
  })

  // TC-902-14: Ctrl+クリックで複数クラスタが選択され brushSelect に OR インデックスが渡る
  test('TC-902-14: Ctrl+クリックで複数クラスタを選択すると全インデックスが brushSelect に渡される', () => {
    // 【テスト目的】: Ctrl+クリックによる複数クラスタ OR 選択が機能することを確認 🟢
    render(
      <ClusterList
        clusterStats={TEST_STATS}
        featureNames={TEST_FEATURES}
        trialsByCluster={TEST_TRIALS_BY_CLUSTER}
      />,
    )

    // 【処理実行】: クラスタ 0 を通常クリック
    fireEvent.click(screen.getByTestId('cluster-row-0'))
    // 【処理実行】: クラスタ 1 を Ctrl+クリック
    fireEvent.click(screen.getByTestId('cluster-row-1'), { ctrlKey: true })

    // 【確認内容】: 2 回目の brushSelect 呼び出しで cluster 0 + cluster 1 の全インデックス
    const lastArg = mockBrushSelect.mock.calls[1][0] as Uint32Array
    expect(Array.from(lastArg)).toEqual([0, 1, 2, 3, 4])
  })

  // TC-902-15: getClusterColor が正しい色を返す
  test('TC-902-15: getClusterColor(0) は "#4f46e5" を返す', () => {
    // 【テスト目的】: クラスタ識別色が正しく返ることを確認（純粋関数）🟢
    expect(getClusterColor(0)).toBe('#4f46e5')
    // 【確認内容】: 8 色以上は循環すること
    expect(getClusterColor(8)).toBe(getClusterColor(0))
  })
})

// -------------------------------------------------------------------------
// 異常系
// -------------------------------------------------------------------------

describe('ClusterList — 異常系', () => {
  afterEach(() => {
    cleanup()
  })

  // TC-902-E01: clusterStats が空のとき「クラスタリングが実行されていません」を表示
  test('TC-902-E01: clusterStats=[] のとき「クラスタリングが実行されていません」が表示される', () => {
    // 【テスト目的】: クラスタリング未実行時に適切な空状態UIが表示されることを確認 🟢
    render(<ClusterList clusterStats={[]} featureNames={[]} trialsByCluster={[]} />)
    // 【確認内容】: 「クラスタリングが実行されていません」が表示されること
    expect(screen.getByText('Clustering has not been run yet')).toBeInTheDocument()
  })
})
