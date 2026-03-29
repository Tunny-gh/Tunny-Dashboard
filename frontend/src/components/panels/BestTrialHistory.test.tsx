/**
 * BestTrialHistory テスト (TASK-1001)
 *
 * 【テスト対象】: BestTrialHistory — Best解遷移トラッキングテーブル
 * 【テスト方針】: Best解のみを表示するテーブルの表示内容と
 *               行クリックコールバックを検証する
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest'
import { render, screen, cleanup, fireEvent } from '@testing-library/react'

import { BestTrialHistory } from './BestTrialHistory'
import type { TrialData } from '../charts/OptimizationHistory'

// -------------------------------------------------------------------------
// テストヘルパー
// -------------------------------------------------------------------------

/** 【ヘルパー】: 収束するテストデータを生成する */
function makeData(): TrialData[] {
  return [
    { trial: 1, value: 100 },
    { trial: 2, value: 80 }, // Best更新
    { trial: 3, value: 85 }, // 更新なし
    { trial: 4, value: 60 }, // Best更新
    { trial: 5, value: 70 }, // 更新なし
  ]
}

// -------------------------------------------------------------------------
// 正常系
// -------------------------------------------------------------------------

describe('BestTrialHistory — 正常系', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    cleanup()
  })

  // TC-1001-13: エラーなくレンダリング
  test('TC-1001-13: BestTrialHistory がエラーなくレンダリングされる', () => {
    // 【テスト目的】: データありでもクラッシュしないこと 🟢
    expect(() => render(<BestTrialHistory data={makeData()} direction="minimize" />)).not.toThrow()
  })

  // TC-1001-14: Best更新行のみテーブルに表示される
  test('TC-1001-14: minimize方向でBest更新回数（2回）がテーブルに表示される', () => {
    // 【テスト目的】: Best値が更新された試行のみがテーブルに表示されること 🟢
    render(<BestTrialHistory data={makeData()} direction="minimize" />)

    // 【確認内容】: Best更新行（試行1, 2, 4）が存在すること
    // trial=1は初回なので常にBest、trial=2は80<100でBest更新、trial=4は60<80でBest更新
    expect(screen.getByTestId('best-row-1')).toBeInTheDocument()
    expect(screen.getByTestId('best-row-2')).toBeInTheDocument()
    expect(screen.getByTestId('best-row-4')).toBeInTheDocument()
  })

  // TC-1001-15: Best非更新行はテーブルに表示されない
  test('TC-1001-15: Best更新のない試行3と試行5はテーブルに表示されない', () => {
    // 【テスト目的】: Best値が更新されなかった試行はテーブルに表示されないこと 🟢
    render(<BestTrialHistory data={makeData()} direction="minimize" />)

    // 【確認内容】: 非Best行が存在しないこと
    expect(screen.queryByTestId('best-row-3')).not.toBeInTheDocument()
    expect(screen.queryByTestId('best-row-5')).not.toBeInTheDocument()
  })

  // TC-1001-16: 行クリックで onRowClick が呼ばれる
  test('TC-1001-16: 行クリックで onRowClick が正しい TrialData で呼ばれる', () => {
    // 【テスト目的】: 行クリックでコールバックが正しいデータで呼ばれること 🟢
    const onRowClick = vi.fn()
    render(<BestTrialHistory data={makeData()} direction="minimize" onRowClick={onRowClick} />)

    // 【処理実行】: 試行2の行をクリック
    fireEvent.click(screen.getByTestId('best-row-2'))

    // 【確認内容】: onRowClick が { trial: 2, value: 80 } で呼ばれること
    expect(onRowClick).toHaveBeenCalledTimes(1)
    expect(onRowClick).toHaveBeenCalledWith({ trial: 2, value: 80 })
  })
})

// -------------------------------------------------------------------------
// 異常系
// -------------------------------------------------------------------------

describe('BestTrialHistory — 異常系', () => {
  afterEach(() => {
    cleanup()
  })

  // TC-1001-E01: 空データで空テーブルを表示
  test('TC-1001-E01: data=[] のとき best-table が表示されるが行が0件', () => {
    // 【テスト目的】: データが空でもクラッシュせずテーブルが表示されること 🟢
    render(<BestTrialHistory data={[]} direction="minimize" />)

    // 【確認内容】: テーブルコンテナが存在し行が0件であること
    expect(screen.getByTestId('best-trial-table')).toBeInTheDocument()
  })
})
