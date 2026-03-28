/**
 * EdfPlot テスト
 *
 * 【テスト対象】: EdfPlot — 経験累積分布関数チャート
 * 【テスト方針】: echarts-for-react を vi.mock でモック。
 *               computeEdf 純粋関数は単独でテスト
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';

// -------------------------------------------------------------------------
// echarts-for-react モック（__mocks__/echarts-for-react.tsx を自動使用）
// -------------------------------------------------------------------------

vi.mock('echarts-for-react');

import { EdfPlot, computeEdf } from './EdfPlot';

// -------------------------------------------------------------------------
// computeEdf 純粋関数テスト
// -------------------------------------------------------------------------

describe('computeEdf — 純粋関数', () => {
  // TC-EDF-PURE-01: 空配列
  test('TC-EDF-PURE-01: 空配列は空を返す', () => {
    // 【テスト目的】: 空入力に対して空配列を返すこと 🟢
    expect(computeEdf([])).toEqual([]);
  });

  // TC-EDF-PURE-02: 1要素
  test('TC-EDF-PURE-02: 1要素のとき [value, 1.0] を返す', () => {
    // 【テスト目的】: 1試行のとき CDF が 1.0 になること 🟢
    expect(computeEdf([5])).toEqual([[5, 1.0]]);
  });

  // TC-EDF-PURE-03: 複数要素（ソート順を確認）
  test('TC-EDF-PURE-03: 3要素を昇順ソートして累積確率を計算する', () => {
    // 【テスト目的】: 順不同の入力でも正しくソートして CDF を計算すること 🟢
    const result = computeEdf([3, 1, 2]);

    // 【確認内容】: 昇順でソートされた CDF 値
    expect(result).toHaveLength(3);
    expect(result[0]).toEqual([1, 1 / 3]);
    expect(result[1]).toEqual([2, 2 / 3]);
    expect(result[2]).toEqual([3, 1.0]);
  });

  // TC-EDF-PURE-04: 重複値
  test('TC-EDF-PURE-04: 重複値でも CDF が単調増加になる', () => {
    // 【テスト目的】: 同一値が複数あっても CDF が正しく計算されること 🟢
    const result = computeEdf([1, 1, 2]);

    // 【確認内容】: 単調増加
    expect(result[0][1]).toBeLessThan(result[1][1]);
    expect(result[1][1]).toBeLessThan(result[2][1]);
    expect(result[2][1]).toBe(1.0);
  });
});

// -------------------------------------------------------------------------
// 正常系
// -------------------------------------------------------------------------

describe('EdfPlot — 正常系', () => {
  beforeEach(() => vi.clearAllMocks());
  afterEach(() => cleanup());

  // TC-EDF-01: データありで ECharts が表示される
  test('TC-EDF-01: データありで edf-plot コンテナと ECharts が表示される', () => {
    // 【テスト目的】: 有効なデータがある場合にチャートが描画されること 🟢
    render(<EdfPlot series={[{ name: 'obj1', values: [0.1, 0.5, 0.3] }]} />);

    // 【確認内容】: コンテナと ECharts が存在する
    expect(screen.getByTestId('edf-plot')).toBeInTheDocument();
    expect(screen.getByTestId('echarts')).toBeInTheDocument();
  });

  // TC-EDF-02: series 名が legend に渡される
  test('TC-EDF-02: series の名前が ECharts legend に渡される', () => {
    // 【テスト目的】: シリーズ名が凡例として正しく設定されること 🟢
    render(<EdfPlot series={[{ name: 'cost', values: [1, 2, 3] }]} />);

    const option = JSON.parse(screen.getByTestId('echarts').getAttribute('data-option') ?? '{}');
    // 【確認内容】: legend に series 名が含まれる
    expect(option.legend.data).toContain('cost');
  });

  // TC-EDF-03: データが step line として渡される
  test('TC-EDF-03: series データが step="end" の折れ線として渡される', () => {
    // 【テスト目的】: CDF らしいステップ折れ線になっていること 🟢
    render(<EdfPlot series={[{ name: 'obj', values: [2, 1, 3] }]} />);

    const option = JSON.parse(screen.getByTestId('echarts').getAttribute('data-option') ?? '{}');
    // 【確認内容】: step 折れ線 / データが昇順ソートされている
    expect(option.series[0].step).toBe('end');
    expect(option.series[0].data[0][0]).toBeLessThanOrEqual(option.series[0].data[1][0]);
  });

  // TC-EDF-04: 多目的 series が複数ラインとして描画される
  test('TC-EDF-04: 多目的のとき series が 2 本になる', () => {
    // 【テスト目的】: 複数の目的関数が独立した折れ線として描画されること 🟢
    render(
      <EdfPlot
        series={[
          { name: 'obj1', values: [1, 2, 3] },
          { name: 'obj2', values: [4, 5, 6] },
        ]}
      />,
    );

    const option = JSON.parse(screen.getByTestId('echarts').getAttribute('data-option') ?? '{}');
    // 【確認内容】: series が 2 本
    expect(option.series).toHaveLength(2);
  });
});

// -------------------------------------------------------------------------
// 異常系
// -------------------------------------------------------------------------

describe('EdfPlot — 異常系', () => {
  beforeEach(() => vi.clearAllMocks());
  afterEach(() => cleanup());

  // TC-EDF-E01: series=[] で空状態UI
  test('TC-EDF-E01: series=[] のとき「データがありません」を表示する', () => {
    // 【テスト目的】: シリーズなし時に適切な空状態UIが表示されること 🟢
    render(<EdfPlot series={[]} />);

    // 【確認内容】: 空状態メッセージ / ECharts は非表示
    expect(screen.getByText('データがありません')).toBeInTheDocument();
    expect(screen.queryByTestId('echarts')).toBeNull();
  });

  // TC-EDF-E02: values=[] の series だけのとき空状態UI
  test('TC-EDF-E02: 全 series の values が空のとき「データがありません」を表示する', () => {
    // 【テスト目的】: 全シリーズが空値の場合に空状態UIが表示されること 🟢
    render(<EdfPlot series={[{ name: 'obj', values: [] }]} />);

    // 【確認内容】: 空状態メッセージが表示される
    expect(screen.getByText('データがありません')).toBeInTheDocument();
  });
});
