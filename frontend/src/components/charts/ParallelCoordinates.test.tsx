/**
 * ParallelCoordinates テスト (TASK-601)
 *
 * 【テスト対象】: ParallelCoordinates — ECharts parallel座標 30軸コンポーネント
 * 【テスト方針】: echarts-for-react と selectionStore を vi.mock でモック
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest'
import { render, screen, cleanup } from '@testing-library/react'

// -------------------------------------------------------------------------
// echarts-for-react モック
// onEvents を capture して axisareaselected ハンドラをテストから呼べるようにする
// -------------------------------------------------------------------------

const { mockReactEChartsPC, captureOnEvents } = vi.hoisted(() => {
  // 【onEvents capture】: コンポーネントが渡す onEvents を保持する 🟢
  let capturedOnEvents: Record<string, (params: unknown) => void> = {}
  const captureOnEvents = () => capturedOnEvents

  const mockReactEChartsPC = vi.fn(
    ({
      option,
      onEvents,
    }: {
      option: unknown
      onEvents?: Record<string, (p: unknown) => void>
    }) => {
      // 【キャプチャ】: テストから axisareaselected を呼び出せるよう保持する
      if (onEvents) capturedOnEvents = onEvents
      return <div data-testid="echarts-pc" data-option={JSON.stringify(option)} />
    },
  )
  return { mockReactEChartsPC, captureOnEvents }
})

vi.mock('echarts-for-react', () => ({
  default: mockReactEChartsPC,
}))

// -------------------------------------------------------------------------
// selectionStore モック
// vi.hoisted でファクトリより前に変数を確保する（hoisting 問題の回避）
// -------------------------------------------------------------------------

const { mockAddAxisFilter, mockRemoveAxisFilter } = vi.hoisted(() => {
  const mockAddAxisFilter = vi.fn()
  const mockRemoveAxisFilter = vi.fn()
  return { mockAddAxisFilter, mockRemoveAxisFilter }
})

vi.mock('../../stores/selectionStore', () => ({
  useSelectionStore: vi.fn().mockReturnValue({
    addAxisFilter: mockAddAxisFilter,
    removeAxisFilter: mockRemoveAxisFilter,
  }),
}))

import { ParallelCoordinates } from './ParallelCoordinates'
import { useStudyStore } from '../../stores/studyStore'
import type { GpuBuffer } from '../../wasm/gpuBuffer'
import type { Study } from '../../types'

// -------------------------------------------------------------------------
// テストヘルパー
// -------------------------------------------------------------------------

/**
 * 【ヘルパー】: テスト用 GpuBuffer モックを生成する
 */
function makeGpuBuffer(n = 5): GpuBuffer {
  return {
    trialCount: n,
    positions: new Float32Array(n * 2),
    positions3d: new Float32Array(n * 3),
    colors: new Float32Array(n * 4),
    sizes: new Float32Array(n),
    updateAlphas: vi.fn(),
    resetAlphas: vi.fn(),
  } as unknown as GpuBuffer
}

/**
 * 【ヘルパー】: テスト用 Study を生成する（paramNames/objectiveNames 指定可）
 */
function makeStudy(opts: { paramNames?: string[]; objectiveNames?: string[] } = {}): Study {
  return {
    studyId: 1,
    name: 'test-study',
    directions: ['minimize', 'minimize'],
    completedTrials: 5,
    totalTrials: 5,
    paramNames: opts.paramNames ?? ['x1', 'x2', 'x3'],
    objectiveNames: opts.objectiveNames ?? ['obj1', 'obj2'],
    userAttrNames: [],
    hasConstraints: false,
  }
}

// -------------------------------------------------------------------------
// 正常系
// -------------------------------------------------------------------------

describe('ParallelCoordinates — 正常系', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    useStudyStore.setState({ trialRows: [] })
  })

  afterEach(() => {
    cleanup()
  })

  // TC-601-01: null データでもエラーなくレンダリング
  test('TC-601-01: gpuBuffer=null, currentStudy=null でもエラーなくレンダリングされる', () => {
    // 【テスト目的】: null データで安全にレンダリングできること 🟢
    expect(() => render(<ParallelCoordinates gpuBuffer={null} currentStudy={null} />)).not.toThrow()
  })

  // TC-601-02: データありで ECharts コンテナ表示
  test('TC-601-02: gpuBuffer と currentStudy があると ECharts コンテナが表示される', () => {
    // 【テスト目的】: データありの場合に ECharts ラッパー要素が表示されること 🟢
    render(<ParallelCoordinates gpuBuffer={makeGpuBuffer()} currentStudy={makeStudy()} />)
    expect(screen.getByTestId('echarts-pc')).toBeInTheDocument()
  })

  // TC-601-03: ECharts option が paramNames + objectiveNames の軸を含む
  test('TC-601-03: ECharts option の parallelAxis に paramNames と objectiveNames が含まれる', () => {
    // 【テスト目的】: 全変数・目的関数が軸として定義されること 🟢
    const study = makeStudy({ paramNames: ['x1', 'x2'], objectiveNames: ['obj1'] })

    // 【処理実行】
    render(<ParallelCoordinates gpuBuffer={makeGpuBuffer()} currentStudy={study} />)

    // 【確認内容】: ECharts option が設定されている
    const el = screen.getByTestId('echarts-pc')
    const option = JSON.parse(el.getAttribute('data-option') ?? '{}') as {
      parallelAxis: Array<{ name: string }>
    }

    // 【確認内容】: parallelAxis に x1, x2, obj1 が含まれている
    const axisNames = option.parallelAxis.map((a) => a.name)
    expect(axisNames).toContain('x1')
    expect(axisNames).toContain('x2')
    expect(axisNames).toContain('obj1')
  })

  // TC-601-04: axisareaselected イベントで addAxisFilter が呼ばれる
  test('TC-601-04: axisareaselected イベントで selectionStore.addAxisFilter が呼ばれる', () => {
    // 【テスト目的】: 軸ブラシ操作が addAxisFilter に正しく連携されること 🟢
    const study = makeStudy({ paramNames: ['x1', 'x2'], objectiveNames: [] })

    // 【処理実行】: コンポーネントをレンダリングして onEvents をキャプチャ
    render(<ParallelCoordinates gpuBuffer={makeGpuBuffer()} currentStudy={study} />)

    // 【処理実行】: axisareaselected イベントをシミュレート
    // axisIndex=0 → 'x1', intervals=[[0.2, 0.8]]
    const onEvents = captureOnEvents()
    onEvents['axisareaselected']({
      axesInfo: [{ axisIndex: 0, intervals: [[0.2, 0.8]] }],
    })

    // 【確認内容】: addAxisFilter が 'x1', 0.2, 0.8 で呼ばれた
    expect(mockAddAxisFilter).toHaveBeenCalledWith('x1', 0.2, 0.8)
  })

  // TC-601-05: trialRows の実データが parallel series に反映される
  test('TC-601-05: trialRows の params/values を parallel series data に反映する', () => {
    const study = makeStudy({ paramNames: ['x1', 'x2'], objectiveNames: ['obj1'] })
    useStudyStore.setState({
      trialRows: [
        {
          trialId: 1,
          params: { x1: 10, x2: 20 },
          values: [0.5],
          paretoRank: 0,
        },
        {
          trialId: 2,
          params: { x1: 11, x2: 21 },
          values: [0.6],
          paretoRank: 1,
        },
      ],
    })

    render(<ParallelCoordinates gpuBuffer={makeGpuBuffer(2)} currentStudy={study} />)

    const el = screen.getByTestId('echarts-pc')
    const option = JSON.parse(el.getAttribute('data-option') ?? '{}') as {
      series: Array<{ data: number[][] }>
    }

    expect(option.series[0]?.data).toEqual([
      [10, 20, 0.5],
      [11, 21, 0.6],
    ])
  })
})

// -------------------------------------------------------------------------
// 異常系
// -------------------------------------------------------------------------

describe('ParallelCoordinates — 異常系', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    useStudyStore.setState({ trialRows: [] })
  })

  afterEach(() => {
    cleanup()
  })

  // TC-601-E01: gpuBuffer=null で空状態UI表示
  test('TC-601-E01: gpuBuffer=null のとき「データが読み込まれていません」を表示する', () => {
    // 【テスト目的】: データなし時に適切な空状態UIが表示されること 🟢
    render(<ParallelCoordinates gpuBuffer={null} currentStudy={null} />)
    expect(screen.getByText('Data not loaded')).toBeInTheDocument()
  })

  // TC-601-E02: currentStudy=null で空状態UI表示
  test('TC-601-E02: currentStudy=null のとき「データが読み込まれていません」を表示する', () => {
    // 【テスト目的】: Study なし時に適切な空状態UIが表示されること 🟢
    render(<ParallelCoordinates gpuBuffer={makeGpuBuffer()} currentStudy={null} />)
    expect(screen.getByText('Data not loaded')).toBeInTheDocument()
  })
})

// -------------------------------------------------------------------------
// 境界値
// -------------------------------------------------------------------------

describe('ParallelCoordinates — 境界値', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    useStudyStore.setState({ trialRows: [] })
  })

  afterEach(() => {
    cleanup()
  })

  // TC-601-B01: 軸数 34（30変数+4目的）でも crash しない
  test('TC-601-B01: 30変数+4目的（合計34軸）でも crash しない', () => {
    // 【テスト目的】: 最大軸数でも安全に動作すること 🟢
    const paramNames = Array.from({ length: 30 }, (_, i) => `x${i + 1}`)
    const objectiveNames = ['obj1', 'obj2', 'obj3', 'obj4']
    const study = makeStudy({ paramNames, objectiveNames })

    // 【処理実行】: クラッシュしないこと
    expect(() =>
      render(<ParallelCoordinates gpuBuffer={makeGpuBuffer(50)} currentStudy={study} />),
    ).not.toThrow()

    // 【確認内容】: 34軸が定義されている
    const el = screen.getByTestId('echarts-pc')
    const option = JSON.parse(el.getAttribute('data-option') ?? '{}') as {
      parallelAxis: unknown[]
    }
    expect(option.parallelAxis).toHaveLength(34)
  })
})
