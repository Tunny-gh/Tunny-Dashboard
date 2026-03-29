/**
 * SlicePlot テスト
 *
 * 【テスト対象】: SlicePlot — パラメータ vs 目的関数値 散布図
 * 【テスト方針】: echarts-for-react を vi.mock でモック
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest'
import { render, screen, cleanup, fireEvent } from '@testing-library/react'

// -------------------------------------------------------------------------
// echarts-for-react モック（__mocks__/echarts-for-react.tsx を自動使用）
// -------------------------------------------------------------------------

vi.mock('echarts-for-react')

import { SlicePlot } from './SlicePlot'
import type { SliceTrial } from './SlicePlot'

// -------------------------------------------------------------------------
// テストデータ
// -------------------------------------------------------------------------

const SAMPLE_TRIALS: SliceTrial[] = [
  { trialId: 1, params: { x: 0.1, y: 0.5 }, values: [0.3], paretoRank: 1 },
  { trialId: 2, params: { x: 0.4, y: 0.2 }, values: [0.6], paretoRank: 2 },
  { trialId: 3, params: { x: 0.7, y: 0.8 }, values: [0.1], paretoRank: 1 },
]

// -------------------------------------------------------------------------
// 正常系
// -------------------------------------------------------------------------

describe('SlicePlot — 正常系', () => {
  beforeEach(() => vi.clearAllMocks())
  afterEach(() => cleanup())

  // TC-SLICE-01: データありで ECharts が表示される
  test('TC-SLICE-01: データありで slice-plot コンテナと ECharts が表示される', () => {
    // 【テスト目的】: 有効なデータがある場合にチャートが描画されること 🟢
    render(<SlicePlot trials={SAMPLE_TRIALS} paramNames={['x', 'y']} objectiveNames={['obj1']} />)

    // 【確認内容】: コンテナと ECharts が存在する
    expect(screen.getByTestId('slice-plot')).toBeInTheDocument()
    expect(screen.getByTestId('echarts')).toBeInTheDocument()
  })

  // TC-SLICE-02: パラメータ選択ドロップダウンが表示される
  test('TC-SLICE-02: パラメータ選択ドロップダウンが表示される', () => {
    // 【テスト目的】: パラメータ切り替えUIが存在すること 🟢
    render(<SlicePlot trials={SAMPLE_TRIALS} paramNames={['x', 'y']} objectiveNames={['obj1']} />)

    // 【確認内容】: ドロップダウンが存在する
    expect(screen.getByTestId('slice-param-select')).toBeInTheDocument()
  })

  // TC-SLICE-03: パラメータ変更で再描画される
  test('TC-SLICE-03: パラメータを y に変更すると xAxis 名が更新される', () => {
    // 【テスト目的】: パラメータ選択変更後にグラフが更新されること 🟢
    render(<SlicePlot trials={SAMPLE_TRIALS} paramNames={['x', 'y']} objectiveNames={['obj1']} />)

    const select = screen.getByTestId('slice-param-select') as HTMLSelectElement
    fireEvent.change(select, { target: { value: '1' } })

    // 【確認内容】: xAxis 名が選択したパラメータに変わっている
    const option = JSON.parse(screen.getByTestId('echarts').getAttribute('data-option') ?? '{}')
    expect(option.xAxis.name).toBe('y')
  })

  // TC-SLICE-04: 多目的のとき目的関数選択が表示される
  test('TC-SLICE-04: 目的関数が 2 つのとき目的関数選択ドロップダウンが表示される', () => {
    // 【テスト目的】: 多目的の場合に目的関数切り替えUIが表示されること 🟢
    const multiObjTrials = SAMPLE_TRIALS.map((t) => ({ ...t, values: [0.3, 0.5] }))
    render(
      <SlicePlot
        trials={multiObjTrials}
        paramNames={['x', 'y']}
        objectiveNames={['obj1', 'obj2']}
      />,
    )

    // 【確認内容】: 目的関数選択が表示される
    expect(screen.getByTestId('slice-obj-select')).toBeInTheDocument()
  })

  // TC-SLICE-05: 初期表示で xAxis 名が最初のパラメータ名になる
  test('TC-SLICE-05: 初期表示で xAxis 名が paramNames[0] になる', () => {
    // 【テスト目的】: デフォルト選択パラメータが正しく設定されること 🟢
    render(<SlicePlot trials={SAMPLE_TRIALS} paramNames={['x', 'y']} objectiveNames={['obj1']} />)

    const option = JSON.parse(screen.getByTestId('echarts').getAttribute('data-option') ?? '{}')
    // 【確認内容】: xAxis 名が最初のパラメータ
    expect(option.xAxis.name).toBe('x')
  })

  // TC-SLICE-06: yAxis 名が目的関数名になる
  test('TC-SLICE-06: yAxis 名が objectiveNames[0] になる', () => {
    // 【テスト目的】: Y軸ラベルが目的関数名として設定されること 🟢
    render(<SlicePlot trials={SAMPLE_TRIALS} paramNames={['x']} objectiveNames={['cost']} />)

    const option = JSON.parse(screen.getByTestId('echarts').getAttribute('data-option') ?? '{}')
    // 【確認内容】: yAxis 名が目的関数名
    expect(option.yAxis.name).toBe('cost')
  })
})

// -------------------------------------------------------------------------
// 異常系
// -------------------------------------------------------------------------

describe('SlicePlot — 異常系', () => {
  beforeEach(() => vi.clearAllMocks())
  afterEach(() => cleanup())

  // TC-SLICE-E01: trials=[] で空状態UI
  test('TC-SLICE-E01: trials=[] のとき「データがありません」を表示する', () => {
    // 【テスト目的】: データなし時に適切な空状態UIが表示されること 🟢
    render(<SlicePlot trials={[]} paramNames={['x']} objectiveNames={['obj']} />)

    // 【確認内容】: 空状態メッセージ / ECharts は非表示
    expect(screen.getByText('データがありません')).toBeInTheDocument()
    expect(screen.queryByTestId('echarts')).toBeNull()
  })

  // TC-SLICE-E02: paramNames=[] で空状態UI
  test('TC-SLICE-E02: paramNames=[] のとき「データがありません」を表示する', () => {
    // 【テスト目的】: パラメータなし時に適切な空状態UIが表示されること 🟢
    render(<SlicePlot trials={SAMPLE_TRIALS} paramNames={[]} objectiveNames={['obj']} />)

    // 【確認内容】: 空状態メッセージが表示される
    expect(screen.getByText('データがありません')).toBeInTheDocument()
  })
})
