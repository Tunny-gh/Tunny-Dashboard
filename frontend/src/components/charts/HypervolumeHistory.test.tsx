/**
 * HypervolumeHistory tests (TASK-501)
 *
 * Target: HypervolumeHistory — ECharts line chart for hypervolume history
 * Strategy: mock echarts-for-react with vi.mock
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';

// -------------------------------------------------------------------------
// echarts-for-react mock (uses __mocks__/echarts-for-react.tsx automatically)
// ECharts uses Canvas, which cannot be rendered in jsdom
// -------------------------------------------------------------------------

vi.mock('echarts-for-react');

import { HypervolumeHistory } from './HypervolumeHistory';

// -------------------------------------------------------------------------
// Happy path
// -------------------------------------------------------------------------

describe('HypervolumeHistory — 正常系', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  // TC-501-05: with data, ECharts container is rendered
  test('TC-501-05: データありで ECharts コンテナが表示される', () => {
    const data = [
      { trial: 1, hypervolume: 0.3 },
      { trial: 2, hypervolume: 0.5 },
      { trial: 3, hypervolume: 0.7 },
    ];

    render(<HypervolumeHistory data={data} />);

    expect(screen.getByTestId('echarts')).toBeInTheDocument();
  });

  // TC-501-06: data is passed as ECharts series data
  test('TC-501-06: データが ECharts の series データとして渡される', () => {
    const data = [
      { trial: 1, hypervolume: 0.3 },
      { trial: 5, hypervolume: 0.8 },
    ];

    render(<HypervolumeHistory data={data} />);

    const echartsEl = screen.getByTestId('echarts');
    const option = JSON.parse(echartsEl.getAttribute('data-option') ?? '{}');

    expect(option.series).toBeDefined();
    expect(option.series[0].data).toHaveLength(2);
    expect(option.series[0].data[0]).toEqual([1, 0.3]);
    expect(option.series[0].data[1]).toEqual([5, 0.8]);
  });
});

// -------------------------------------------------------------------------
// Error cases
// -------------------------------------------------------------------------

describe('HypervolumeHistory — 異常系', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  // TC-501-E03: data=[] shows empty state
  test('TC-501-E03: data=[] のとき「データがありません」を表示する', () => {
    render(<HypervolumeHistory data={[]} />);

    expect(screen.getByText('データがありません')).toBeInTheDocument();
    expect(screen.queryByTestId('echarts')).toBeNull();
  });
});
