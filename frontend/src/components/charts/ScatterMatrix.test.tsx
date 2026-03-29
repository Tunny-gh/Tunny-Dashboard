/**
 * ScatterMatrix テスト (TASK-702)
 *
 * 【テスト対象】: ScatterMatrix — 散布図行列 UIシェル（モード切り替え・軸ソート）
 * 【テスト方針】:
 *   - ScatterMatrixEngine はモックオブジェクトを props で注入してテスト
 *   - モード切り替え・ソート変更・エラー状態を検証
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest'
import { render, screen, cleanup, fireEvent } from '@testing-library/react'

import { ScatterMatrix } from './ScatterMatrix'
import type { ScatterMatrixEngine } from '../../wasm/workers/ScatterMatrixEngine'
import type { Study } from '../../types'

// -------------------------------------------------------------------------
// テストヘルパー
// -------------------------------------------------------------------------

/** 【ヘルパー】: renderCell が即座に null で resolve するモックエンジンを生成する */
function makeMockEngine(): ScatterMatrixEngine {
  return {
    renderCell: vi.fn().mockResolvedValue(null),
    workerIndex: vi.fn().mockReturnValue(0),
    dispose: vi.fn(),
  } as unknown as ScatterMatrixEngine
}

/** 【ヘルパー】: テスト用 Study を生成する（paramNames 2 個、objectiveNames 2 個） */
function makeStudy(): Study {
  return {
    studyId: 1,
    name: 'test-study',
    directions: ['minimize', 'minimize'],
    completedTrials: 5,
    totalTrials: 5,
    paramNames: ['x1', 'x2'],
    objectiveNames: ['f1', 'f2'],
    userAttrNames: [],
    hasConstraints: false,
  }
}

// -------------------------------------------------------------------------
// 正常系
// -------------------------------------------------------------------------

describe('ScatterMatrix — 正常系', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    cleanup()
  })

  // TC-702-01: エラーなくレンダリング
  test('TC-702-01: ScatterMatrix が currentStudy あり・engine=null でエラーなくレンダリングされる', () => {
    // 【テスト目的】: Study あり・engine 未初期化状態でもクラッシュしないこと 🟢
    expect(() => render(<ScatterMatrix engine={null} currentStudy={makeStudy()} />)).not.toThrow()
  })

  // TC-702-02: 3つのモードボタンが表示される
  test('TC-702-02: 3つのモードボタン（mode1/mode2/mode3）が表示される', () => {
    // 【テスト目的】: 全 3 表示モードのボタンが存在すること 🟢
    render(<ScatterMatrix engine={null} currentStudy={makeStudy()} />)

    // 【確認内容】: モードボタンが 3 つ存在すること
    expect(screen.getByTestId('mode-btn-mode1')).toBeInTheDocument()
    expect(screen.getByTestId('mode-btn-mode2')).toBeInTheDocument()
    expect(screen.getByTestId('mode-btn-mode3')).toBeInTheDocument()
  })

  // TC-702-03: モードボタンクリックで状態が切り替わる
  test('TC-702-03: Mode 2 ボタンクリックで mode2 が aria-pressed=true になる', () => {
    // 【テスト目的】: モード切り替えボタンのアクティブ状態が変わること 🟢
    render(<ScatterMatrix engine={null} currentStudy={makeStudy()} />)

    // 【初期確認】: mode1 がデフォルトでアクティブ、mode2 は非アクティブ
    expect(screen.getByTestId('mode-btn-mode1')).toHaveAttribute('aria-pressed', 'true')
    expect(screen.getByTestId('mode-btn-mode2')).toHaveAttribute('aria-pressed', 'false')

    // 【処理実行】: mode2 ボタンをクリック
    fireEvent.click(screen.getByTestId('mode-btn-mode2'))

    // 【確認内容】: mode2 がアクティブになり、mode1 が非アクティブになること
    expect(screen.getByTestId('mode-btn-mode2')).toHaveAttribute('aria-pressed', 'true')
    expect(screen.getByTestId('mode-btn-mode1')).toHaveAttribute('aria-pressed', 'false')
  })

  // TC-702-04: ソートセレクタが表示される
  test('TC-702-04: 軸ソートセレクタ（data-testid=sort-select）が表示される', () => {
    // 【テスト目的】: 軸ソートオプションのセレクタが存在すること 🟢
    render(<ScatterMatrix engine={null} currentStudy={makeStudy()} />)

    // 【確認内容】: ソートセレクタが存在し、デフォルト値が 'alphabetical' であること
    const sortSelect = screen.getByTestId('sort-select')
    expect(sortSelect).toBeInTheDocument()
    expect(sortSelect).toHaveValue('alphabetical')
  })

  // TC-702-05: ソートオプション変更でセレクタ値が更新される
  test('TC-702-05: ソートオプションを correlation に変更するとセレクタ値が変わる', () => {
    // 【テスト目的】: 軸ソート選択が UI に反映されること 🟢
    render(<ScatterMatrix engine={null} currentStudy={makeStudy()} />)

    // 【処理実行】: ソートを 'correlation' に変更
    const sortSelect = screen.getByTestId('sort-select')
    fireEvent.change(sortSelect, { target: { value: 'correlation' } })

    // 【確認内容】: ソートセレクタの値が 'correlation' に変わること
    expect(sortSelect).toHaveValue('correlation')
  })
})

// -------------------------------------------------------------------------
// 異常系
// -------------------------------------------------------------------------

describe('ScatterMatrix — 異常系', () => {
  afterEach(() => {
    cleanup()
  })

  // TC-702-E01: currentStudy=null で「データが読み込まれていません」
  test('TC-702-E01: currentStudy=null のとき「データが読み込まれていません」を表示する', () => {
    // 【テスト目的】: Study 未読み込み時に適切な空状態UIが表示されること 🟢
    render(<ScatterMatrix engine={null} currentStudy={null} />)

    // 【確認内容】: 空状態メッセージが表示されること
    expect(screen.getByText('Data not loaded')).toBeInTheDocument()
  })

  // TC-702-E02: currentStudy + engine ありでグリッドが表示される
  test('TC-702-E02: engine + currentStudy があるとき散布図グリッドが表示される', () => {
    // 【テスト目的】: Study と engine が揃っているときにグリッドが表示されること 🟢
    const engine = makeMockEngine()
    render(<ScatterMatrix engine={engine} currentStudy={makeStudy()} />)

    // 【確認内容】: scatter-grid コンテナが存在すること
    expect(screen.getByTestId('scatter-grid')).toBeInTheDocument()
  })
})
