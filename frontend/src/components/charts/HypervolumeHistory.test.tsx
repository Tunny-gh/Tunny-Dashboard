/**
 * HypervolumeHistory テスト (TASK-501)
 *
 * 【テスト対象】: HypervolumeHistory — ECharts折れ線グラフ（Hypervolume推移）
 * 【テスト方針】: echarts-for-react を vi.mock でモック
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';

// -------------------------------------------------------------------------
// echarts-for-react モック
// 【モック理由】: ECharts は Canvas を使用するため jsdom では描画できない
// -------------------------------------------------------------------------

const { mockReactECharts } = vi.hoisted(() => {
  const mockReactECharts = vi.fn(({ option }: { option: unknown }) => (
    <div data-testid="echarts" data-option={JSON.stringify(option)} />
  ));
  return { mockReactECharts };
});

vi.mock('echarts-for-react', () => ({
  default: mockReactECharts,
}));

import { HypervolumeHistory } from './HypervolumeHistory';

// -------------------------------------------------------------------------
// 正常系
// -------------------------------------------------------------------------

describe('HypervolumeHistory — 正常系', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  // TC-501-05: データありで ReactECharts が表示される
  test('TC-501-05: データありで ECharts コンテナが表示される', () => {
    // 【テスト目的】: Hypervolume データがある場合にグラフが表示されること 🟢
    const data = [
      { trial: 1, hypervolume: 0.3 },
      { trial: 2, hypervolume: 0.5 },
      { trial: 3, hypervolume: 0.7 },
    ];

    // 【処理実行】
    render(<HypervolumeHistory data={data} />);

    // 【確認内容】: ECharts コンテナが存在する
    expect(screen.getByTestId('echarts')).toBeInTheDocument();
  });

  // TC-501-06: データが折れ線グラフの option に渡される
  test('TC-501-06: データが ECharts の series データとして渡される', () => {
    // 【テスト目的】: グラフデータが正しく ECharts に渡されること 🟢
    const data = [
      { trial: 1, hypervolume: 0.3 },
      { trial: 5, hypervolume: 0.8 },
    ];

    // 【処理実行】
    render(<HypervolumeHistory data={data} />);

    // 【確認内容】: ECharts の option が設定されている
    const echartsEl = screen.getByTestId('echarts');
    const option = JSON.parse(echartsEl.getAttribute('data-option') ?? '{}');

    // 【確認内容】: series に正しいデータが含まれている
    expect(option.series).toBeDefined();
    expect(option.series[0].data).toHaveLength(2);
    expect(option.series[0].data[0]).toEqual([1, 0.3]);
    expect(option.series[0].data[1]).toEqual([5, 0.8]);
  });
});

// -------------------------------------------------------------------------
// 異常系
// -------------------------------------------------------------------------

describe('HypervolumeHistory — 異常系', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  // TC-501-E03: data=[] で空状態UIを表示
  test('TC-501-E03: data=[] のとき「データがありません」を表示する', () => {
    // 【テスト目的】: データなし時に適切な空状態UIが表示されること 🟢

    // 【処理実行】
    render(<HypervolumeHistory data={[]} />);

    // 【確認内容】: 空状態メッセージが表示されている
    expect(screen.getByText('データがありません')).toBeInTheDocument();
    // 【確認内容】: ECharts コンポーネントは表示されない
    expect(screen.queryByTestId('echarts')).toBeNull();
  });
});
