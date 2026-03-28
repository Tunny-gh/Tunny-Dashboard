/**
 * ObjectivePairMatrix テスト (TASK-502)
 *
 * 【テスト対象】: ObjectivePairMatrix — N×N目的ペア行列（対角: ヒストグラム、下三角: 2D散布図）
 * 【テスト方針】: deck.gl をvi.mockでモック、props直接渡しでテスト
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';

// -------------------------------------------------------------------------
// deck.gl モック — WebGL不要のダミーコンポーネント
// -------------------------------------------------------------------------

// 【deck.gl モック】: jsdom 環境でWebGLなしにレンダリング可能にする 🟢
vi.mock('deck.gl', () => ({
  DeckGL: vi.fn(({ children }: { children?: unknown }) => (
    <div data-testid="deck-gl">{children as never}</div>
  )),
  ScatterplotLayer: vi.fn().mockImplementation((props: { id: string }) => ({
    id: props.id,
    type: 'ScatterplotLayer',
  })),
}));

vi.mock('../../wasm/gpuBuffer', () => ({
  GpuBuffer: vi.fn(),
}));

import { ObjectivePairMatrix } from './ObjectivePairMatrix';
import type { GpuBuffer } from '../../wasm/gpuBuffer';
import type { Study } from '../../types';

// -------------------------------------------------------------------------
// テストヘルパー
// -------------------------------------------------------------------------

/** 【ヘルパー】: テスト用 GpuBuffer スタブを生成する */
function makeGpuBuffer(): GpuBuffer {
  return {
    trialCount: 3,
    positions: new Float32Array(3 * 2),
    positions3d: new Float32Array(3 * 3),
    colors: new Float32Array(3 * 4),
    sizes: new Float32Array(3),
    updateAlphas: vi.fn(),
    resetAlphas: vi.fn(),
  } as unknown as GpuBuffer;
}

/** 【ヘルパー】: 4目的のテスト用 Study を生成する */
function makeStudy4(): Study {
  return {
    studyId: 1,
    name: 'test-study',
    directions: ['minimize', 'minimize', 'minimize', 'minimize'],
    completedTrials: 3,
    totalTrials: 3,
    paramNames: ['x1'],
    objectiveNames: ['f1', 'f2', 'f3', 'f4'],
    userAttrNames: [],
    hasConstraints: false,
  };
}

/** 【ヘルパー】: 2目的のテスト用 Study を生成する */
function makeStudy2(): Study {
  return {
    studyId: 2,
    name: 'test-study-2',
    directions: ['minimize', 'minimize'],
    completedTrials: 3,
    totalTrials: 3,
    paramNames: ['x1'],
    objectiveNames: ['f1', 'f2'],
    userAttrNames: [],
    hasConstraints: false,
  };
}

/** 【ヘルパー】: 1目的のテスト用 Study を生成する */
function makeStudy1(): Study {
  return {
    studyId: 3,
    name: 'test-study-1',
    directions: ['minimize'],
    completedTrials: 3,
    totalTrials: 3,
    paramNames: ['x1'],
    objectiveNames: ['f1'],
    userAttrNames: [],
    hasConstraints: false,
  };
}

// -------------------------------------------------------------------------
// 正常系
// -------------------------------------------------------------------------

describe('ObjectivePairMatrix — 正常系', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  // TC-502-01: 4目的でエラーなくレンダリング
  test('TC-502-01: ObjectivePairMatrix が4目的でエラーなくレンダリングされる', () => {
    // 【テスト目的】: 4目的のStudyで正常にレンダリングできること 🟢
    expect(() =>
      render(<ObjectivePairMatrix gpuBuffer={null} currentStudy={makeStudy4()} />),
    ).not.toThrow();
  });

  // TC-502-02: 4目的で4×4グリッド（16セル）
  test('TC-502-02: 4目的のとき4×4グリッド（16セル）が表示される', () => {
    // 【テスト目的】: objectiveNames.length=4 で 16 個のセルが生成されること 🟢
    render(<ObjectivePairMatrix gpuBuffer={null} currentStudy={makeStudy4()} />);

    // 【確認内容】: data-testid="matrix-cell-{row}-{col}" が 16 個存在すること
    const cells = screen.getAllByTestId(/^matrix-cell-/);
    expect(cells).toHaveLength(16);
  });

  // TC-502-03: セルクリックでonCellClickが呼ばれる
  test('TC-502-03: セルクリックでonCellClickが正しい軸名で呼ばれる', () => {
    // 【テスト目的】: セルクリックが xAxis/yAxis 名で onCellClick に連携されること 🟢
    const onCellClick = vi.fn();
    render(
      <ObjectivePairMatrix
        gpuBuffer={makeGpuBuffer()}
        currentStudy={makeStudy4()}
        onCellClick={onCellClick}
      />,
    );

    // 【処理実行】: row=1, col=0 のセルをクリック → xAxis='f1', yAxis='f2'
    const cell = screen.getByTestId('matrix-cell-1-0');
    fireEvent.click(cell);

    // 【確認内容】: onCellClick が ('f1', 'f2') で呼ばれた 🟢
    expect(onCellClick).toHaveBeenCalledWith('f1', 'f2');
  });

  // TC-502-04: 2目的で2×2グリッド（4セル）
  test('TC-502-04: 2目的のとき2×2グリッド（4セル）が表示される', () => {
    // 【テスト目的】: objectiveNames.length=2 で 4 個のセルが生成されること 🟢
    render(<ObjectivePairMatrix gpuBuffer={null} currentStudy={makeStudy2()} />);

    // 【確認内容】: data-testid="matrix-cell-{row}-{col}" が 4 個存在すること
    const cells = screen.getAllByTestId(/^matrix-cell-/);
    expect(cells).toHaveLength(4);
  });
});

// -------------------------------------------------------------------------
// 異常系
// -------------------------------------------------------------------------

describe('ObjectivePairMatrix — 異常系', () => {
  afterEach(() => {
    cleanup();
  });

  // TC-502-E01: 1目的でコンポーネントが非表示
  test('TC-502-E01: 1目的のときコンポーネントが非表示になる', () => {
    // 【テスト目的】: 1目的以下の場合は行列を表示しないこと 🟢
    render(<ObjectivePairMatrix gpuBuffer={null} currentStudy={makeStudy1()} />);

    // 【確認内容】: objective-pair-matrix コンテナが存在しないこと
    expect(screen.queryByTestId('objective-pair-matrix')).not.toBeInTheDocument();
  });

  // TC-502-E02: currentStudy=null で空状態UI
  test('TC-502-E02: currentStudy=null のとき「データが読み込まれていません」を表示する', () => {
    // 【テスト目的】: Study なし時に適切な空状態UIが表示されること 🟢
    render(<ObjectivePairMatrix gpuBuffer={null} currentStudy={null} />);

    // 【確認内容】: 空状態メッセージが表示されること
    expect(screen.getByText('データが読み込まれていません')).toBeInTheDocument();
  });
});
