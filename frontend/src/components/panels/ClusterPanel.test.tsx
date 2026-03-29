/**
 * ClusterPanel テスト (TASK-902)
 *
 * 【テスト対象】: ClusterPanel — クラスタリング設定・実行・Elbow 表示パネル
 * 【テスト方針】: Props ベースのコンポーネントのためストアモック不要
 *               ECharts は vi.mock で差し替え
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest'
import { render, screen, cleanup, fireEvent } from '@testing-library/react'

// -------------------------------------------------------------------------
// ECharts モック（__mocks__/echarts-for-react.tsx を自動使用）
// -------------------------------------------------------------------------

vi.mock('echarts-for-react')

import { ClusterPanel } from './ClusterPanel'
import type { ClusterPanelProps } from './ClusterPanel'

// -------------------------------------------------------------------------
// テストヘルパー
// -------------------------------------------------------------------------

/** デフォルト Props — onRunClustering はモック関数 */
function makeProps(overrides: Partial<ClusterPanelProps> = {}): ClusterPanelProps {
  return {
    onRunClustering: vi.fn(),
    isRunning: false,
    progress: 0,
    elbowResult: null,
    error: null,
    ...overrides,
  }
}

// -------------------------------------------------------------------------
// 正常系
// -------------------------------------------------------------------------

describe('ClusterPanel — 正常系', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    cleanup()
  })

  // TC-902-01: 実行ボタンクリックで onRunClustering が呼ばれる
  test('TC-902-01: 実行ボタンクリックで onRunClustering が正しい引数で呼ばれる', () => {
    // 【テスト目的】: 実行ボタンが onRunClustering コールバックを呼ぶことを確認 🟢
    const props = makeProps()
    render(<ClusterPanel {...props} />)

    // 【処理実行】: 実行ボタンをクリック
    const btn = screen.getByTestId('run-clustering-btn')
    fireEvent.click(btn)

    // 【確認内容】: デフォルト (space='param', k=4) で呼ばれること
    expect(props.onRunClustering).toHaveBeenCalledWith('param', 4)
  })

  // TC-902-02: isRunning=true でプログレスコンテナが表示される
  test('TC-902-02: isRunning=true でプログレスバーが表示される', () => {
    // 【テスト目的】: 計算中フラグでプログレスUIが現れることを確認 🟢
    render(<ClusterPanel {...makeProps({ isRunning: true, progress: 0 })} />)
    // 【確認内容】: プログレスコンテナが DOM に存在する
    expect(screen.getByTestId('progress-container')).toBeInTheDocument()
  })

  // TC-902-03: progress=68 で「計算中...68%」が表示される
  test('TC-902-03: progress=68 のとき「計算中...68%」テキストが表示される', () => {
    // 【テスト目的】: プログレステキストに正確な数値が表示されることを確認 🟢
    render(<ClusterPanel {...makeProps({ isRunning: true, progress: 68 })} />)
    // 【確認内容】: 「計算中...68%」を含むテキストが表示されること
    expect(screen.getByTestId('progress-text')).toHaveTextContent('Computing...68%')
  })

  // TC-902-05: elbowResult あり時に推薦 k が表示される
  test('TC-902-05: elbowResult あり時に推薦 k テキストが表示される', () => {
    // 【テスト目的】: Elbow 結果があれば推薦 k を強調表示することを確認 🟢
    render(
      <ClusterPanel
        {...makeProps({
          elbowResult: { wcssPerK: [100, 50, 30, 20], recommendedK: 4 },
        })}
      />,
    )
    // 【確認内容】: 「推薦 k = 4」テキストが表示されること
    const el = screen.getByTestId('elbow-recommended')
    expect(el).toHaveTextContent('4')
  })

  // TC-902-06: isRunning=false でプログレスバーは非表示
  test('TC-902-06: isRunning=false のときプログレスバーは非表示', () => {
    // 【テスト目的】: 計算中でなければプログレスUIが表示されないことを確認 🟢
    render(<ClusterPanel {...makeProps({ isRunning: false })} />)
    // 【確認内容】: progress-container が DOM に存在しないこと
    expect(screen.queryByTestId('progress-container')).toBeNull()
  })

  // TC-902-07: space=Objective に変更できる
  test('TC-902-07: 目的関数ラジオボタン選択後に onRunClustering が "objective" で呼ばれる', () => {
    // 【テスト目的】: 空間選択ラジオが動作しコールバックに反映されることを確認 🟢
    const props = makeProps()
    render(<ClusterPanel {...props} />)

    // 【処理実行】: 目的関数ラジオを選択してから実行
    fireEvent.click(screen.getByTestId('space-objective'))
    fireEvent.click(screen.getByTestId('run-clustering-btn'))

    // 【確認内容】: 'objective' が第1引数で渡されること
    expect(props.onRunClustering).toHaveBeenCalledWith('objective', expect.any(Number))
  })

  // TC-902-08: elbowResult=null のとき Elbow チャートは非表示
  test('TC-902-08: elbowResult=null のとき elbow-recommended が非表示', () => {
    // 【テスト目的】: Elbow 結果がない状態では推薦 k UI が表示されないことを確認 🟢
    render(<ClusterPanel {...makeProps({ elbowResult: null })} />)
    // 【確認内容】: elbow-recommended が DOM に存在しないこと
    expect(screen.queryByTestId('elbow-recommended')).toBeNull()
  })
})

// -------------------------------------------------------------------------
// ローディング
// -------------------------------------------------------------------------

describe('ClusterPanel — ローディング', () => {
  afterEach(() => {
    cleanup()
  })

  // TC-902-L01: isRunning=true のとき実行ボタンが disabled になる
  test('TC-902-L01: isRunning=true のとき実行ボタンが disabled', () => {
    // 【テスト目的】: 計算中は重複実行を防ぐため実行ボタンを非活性にすることを確認 🟢
    render(<ClusterPanel {...makeProps({ isRunning: true })} />)
    const btn = screen.getByTestId('run-clustering-btn')
    // 【確認内容】: disabled 属性が付いていること
    expect(btn).toBeDisabled()
  })
})

// -------------------------------------------------------------------------
// 異常系
// -------------------------------------------------------------------------

describe('ClusterPanel — 異常系', () => {
  afterEach(() => {
    cleanup()
  })

  // TC-902-E01: k=1 指定時は「k=2以上を指定してください」警告を表示
  test('TC-902-E01: k=1 で実行ボタンをクリックすると k-error 警告が表示される', () => {
    // 【テスト目的】: k=1 の不正値が検出されクライアントバリデーションが動くことを確認 🟢
    const props = makeProps()
    render(<ClusterPanel {...props} />)

    // 【k=1 に設定】: input に 1 を入力
    const kInput = screen.getByTestId('k-input')
    fireEvent.change(kInput, { target: { value: '1' } })

    // 【処理実行】: 実行ボタンをクリック
    fireEvent.click(screen.getByTestId('run-clustering-btn'))

    // 【確認内容】: k-error が表示されて onRunClustering は呼ばれないこと
    expect(screen.getByTestId('k-error')).toHaveTextContent('Please specify k >= 2')
    expect(props.onRunClustering).not.toHaveBeenCalled()
  })

  // TC-902-E02: error prop があればエラーメッセージが表示される
  test('TC-902-E02: error prop があれば cluster-error が表示される', () => {
    // 【テスト目的】: 計算失敗エラーが UI に表示されることを確認 🟢
    render(<ClusterPanel {...makeProps({ error: 'Clustering failed' })} />)
    // 【確認内容】: cluster-error にエラーメッセージが含まれること
    expect(screen.getByTestId('cluster-error')).toHaveTextContent('Clustering failed')
  })
})
