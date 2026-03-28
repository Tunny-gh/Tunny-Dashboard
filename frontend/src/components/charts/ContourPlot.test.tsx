/**
 * ContourPlot テスト
 *
 * 【テスト対象】: ContourPlot — 2パラメータ相関散布図（コンタープロット簡易版）
 * 【テスト方針】: echarts-for-react を vi.mock でモック
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';

// -------------------------------------------------------------------------
// echarts-for-react モック（__mocks__/echarts-for-react.tsx を自動使用）
// -------------------------------------------------------------------------

vi.mock('echarts-for-react');

import { ContourPlot } from './ContourPlot';
import type { ContourTrial } from './ContourPlot';

// -------------------------------------------------------------------------
// テストデータ
// -------------------------------------------------------------------------

const SAMPLE_TRIALS: ContourTrial[] = [
  { params: { x: 0.1, y: 0.2 }, values: [0.5] },
  { params: { x: 0.5, y: 0.6 }, values: [0.3] },
  { params: { x: 0.9, y: 0.1 }, values: [0.8] },
];

// -------------------------------------------------------------------------
// 正常系
// -------------------------------------------------------------------------

describe('ContourPlot — 正常系', () => {
  beforeEach(() => vi.clearAllMocks());
  afterEach(() => cleanup());

  // TC-CONTOUR-01: データありで ECharts が表示される
  test('TC-CONTOUR-01: データありで contour-plot コンテナと ECharts が表示される', () => {
    // 【テスト目的】: 有効なデータがある場合にチャートが描画されること 🟢
    render(<ContourPlot trials={SAMPLE_TRIALS} paramNames={['x', 'y']} objectiveNames={['obj']} />);

    // 【確認内容】: コンテナと ECharts が存在する
    expect(screen.getByTestId('contour-plot')).toBeInTheDocument();
    expect(screen.getByTestId('echarts')).toBeInTheDocument();
  });

  // TC-CONTOUR-02: Python 注意バナーが表示される
  test('TC-CONTOUR-02: Python/scikit-learn 注意バナーが表示される', () => {
    // 【テスト目的】: optuna-dashboard との機能差を明示する注意バナーがあること 🟢
    render(<ContourPlot trials={SAMPLE_TRIALS} paramNames={['x', 'y']} objectiveNames={['obj']} />);

    // 【確認内容】: 注意バナーが存在する
    expect(screen.getByTestId('contour-note')).toBeInTheDocument();
  });

  // TC-CONTOUR-03: X/Y パラメータ選択ドロップダウンが表示される
  test('TC-CONTOUR-03: X / Y パラメータ選択ドロップダウンが表示される', () => {
    // 【テスト目的】: 2軸のパラメータ切り替えUIが存在すること 🟢
    render(<ContourPlot trials={SAMPLE_TRIALS} paramNames={['x', 'y']} objectiveNames={['obj']} />);

    // 【確認内容】: X / Y 両方のドロップダウンが存在する
    expect(screen.getByTestId('contour-x-select')).toBeInTheDocument();
    expect(screen.getByTestId('contour-y-select')).toBeInTheDocument();
  });

  // TC-CONTOUR-04: 初期表示で xAxis 名が paramNames[0] になる
  test('TC-CONTOUR-04: 初期表示で xAxis 名が paramNames[0] になる', () => {
    // 【テスト目的】: デフォルトのX軸パラメータが正しく設定されること 🟢
    render(<ContourPlot trials={SAMPLE_TRIALS} paramNames={['x', 'y']} objectiveNames={['obj']} />);

    const option = JSON.parse(screen.getByTestId('echarts').getAttribute('data-option') ?? '{}');
    // 【確認内容】: xAxis 名が最初のパラメータ
    expect(option.xAxis.name).toBe('x');
  });

  // TC-CONTOUR-05: X 選択変更後に軸名が更新される
  test('TC-CONTOUR-05: X を 3 番目パラメータに変更後 xAxis 名が更新される', () => {
    // 【テスト目的】: X軸変更後にグラフが正しく更新されること 🟢
    render(
      <ContourPlot
        trials={[{ params: { x: 1, y: 2, z: 3 }, values: [0.5] }]}
        paramNames={['x', 'y', 'z']}
        objectiveNames={['obj']}
      />,
    );

    fireEvent.change(screen.getByTestId('contour-x-select'), { target: { value: '2' } });

    const option = JSON.parse(screen.getByTestId('echarts').getAttribute('data-option') ?? '{}');
    // 【確認内容】: xAxis 名が変更後のパラメータ名
    expect(option.xAxis.name).toBe('z');
  });

  // TC-CONTOUR-06: 多目的のとき目的関数選択が表示される
  test('TC-CONTOUR-06: 目的関数が 2 つのとき目的関数選択ドロップダウンが表示される', () => {
    // 【テスト目的】: 多目的の場合に目的関数切り替えUIが表示されること 🟢
    const multiObjTrials = SAMPLE_TRIALS.map(t => ({ ...t, values: [0.5, 0.3] }));
    render(
      <ContourPlot
        trials={multiObjTrials}
        paramNames={['x', 'y']}
        objectiveNames={['obj1', 'obj2']}
      />,
    );

    // 【確認内容】: 目的関数選択が表示される
    expect(screen.getByTestId('contour-obj-select')).toBeInTheDocument();
  });

  // TC-CONTOUR-07: scatter の data 点数が有効なトライアル数と一致する
  test('TC-CONTOUR-07: scatter series の data 点数が有効トライアル数と一致する', () => {
    // 【テスト目的】: 実際のデータ点が正しくチャートに渡されること 🟢
    render(<ContourPlot trials={SAMPLE_TRIALS} paramNames={['x', 'y']} objectiveNames={['obj']} />);

    const option = JSON.parse(screen.getByTestId('echarts').getAttribute('data-option') ?? '{}');
    // 【確認内容】: 3試行すべてが scatter データに含まれる
    expect(option.series[0].data).toHaveLength(3);
  });
});

// -------------------------------------------------------------------------
// 異常系
// -------------------------------------------------------------------------

describe('ContourPlot — 異常系', () => {
  beforeEach(() => vi.clearAllMocks());
  afterEach(() => cleanup());

  // TC-CONTOUR-E01: trials=[] で空状態UI
  test('TC-CONTOUR-E01: trials=[] のとき空状態UIを表示する', () => {
    // 【テスト目的】: データなし時に適切な空状態UIが表示されること 🟢
    render(<ContourPlot trials={[]} paramNames={['x', 'y']} objectiveNames={['obj']} />);

    // 【確認内容】: 空状態メッセージ / ECharts は非表示
    expect(screen.getByText(/データがありません/)).toBeInTheDocument();
    expect(screen.queryByTestId('echarts')).toBeNull();
  });

  // TC-CONTOUR-E02: paramNames が 1 つだけで空状態UI
  test('TC-CONTOUR-E02: paramNames が 1 つのとき空状態UIを表示する（2 つ必要）', () => {
    // 【テスト目的】: パラメータ不足時に適切な空状態UIが表示されること 🟢
    render(<ContourPlot trials={SAMPLE_TRIALS} paramNames={['x']} objectiveNames={['obj']} />);

    // 【確認内容】: 2パラメータ必要のメッセージ
    expect(screen.getByText(/データがありません/)).toBeInTheDocument();
  });
});
