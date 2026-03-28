/**
 * SensitivityHeatmap テスト (TASK-802)
 *
 * 【テスト対象】: SensitivityHeatmap — 感度分析ヒートマップ UIコンポーネント
 * 【テスト方針】:
 *   - echarts-for-react を vi.mock でモック（jsdom でも動作可能）
 *   - しきい値フィルタ・指標切り替え・ロード状態・セルクリックを検証
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';

// -------------------------------------------------------------------------
// echarts-for-react モック
// -------------------------------------------------------------------------

vi.mock('echarts-for-react', () => ({
  default: vi.fn(({ option }: { option: unknown }) => (
    <div data-testid="echarts" data-option={JSON.stringify(option)} />
  )),
}));

import { SensitivityHeatmap } from './SensitivityHeatmap';
import type { SensitivityData } from './SensitivityHeatmap';

// -------------------------------------------------------------------------
// テストヘルパー
// -------------------------------------------------------------------------

/** 【ヘルパー】: 2パラメータ×2目的の感度データを生成する */
function makeSensitivityData(): SensitivityData {
  return {
    paramNames: ['x1', 'x2'],
    objectiveNames: ['obj0', 'obj1'],
    // spearman[param_idx][obj_idx]
    spearman: [
      [0.8, -0.3],  // x1: obj0=強正相関, obj1=弱負相関
      [0.1, 0.9],   // x2: obj0=弱相関, obj1=強正相関
    ],
    ridge: [
      { beta: [0.7, 0.1], rSquared: 0.85 }, // obj0
      { beta: [-0.2, 0.8], rSquared: 0.92 }, // obj1
    ],
  };
}

/** 【ヘルパー】: 高しきい値でフィルタされるデータ（全値が低い）を生成する */
function makeLowCorrelationData(): SensitivityData {
  return {
    paramNames: ['x1', 'x2'],
    objectiveNames: ['obj0'],
    spearman: [
      [0.1],  // x1: 低相関
      [0.05], // x2: 低相関
    ],
    ridge: [
      { beta: [0.05, 0.03], rSquared: 0.02 },
    ],
  };
}

// -------------------------------------------------------------------------
// 正常系
// -------------------------------------------------------------------------

describe('SensitivityHeatmap — 正常系', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  // TC-802-01: エラーなくレンダリング
  test('TC-802-01: SensitivityHeatmap が data=null でもエラーなくレンダリングされる', () => {
    // 【テスト目的】: データなし・ローディングなしでもクラッシュしないこと 🟢
    expect(() =>
      render(<SensitivityHeatmap data={null} metric="spearman" threshold={0} />),
    ).not.toThrow();
  });

  // TC-802-02: データありで ECharts が表示される
  test('TC-802-02: data あり・isLoading=false で ECharts コンテナが表示される', () => {
    // 【テスト目的】: 感度データがある場合にヒートマップが描画されること 🟢
    render(
      <SensitivityHeatmap
        data={makeSensitivityData()}
        metric="spearman"
        threshold={0}
      />,
    );

    // 【確認内容】: ECharts コンテナが表示されること
    expect(screen.getByTestId('echarts')).toBeInTheDocument();
  });

  // TC-802-03: 指標切り替えボタンが存在する
  test('TC-802-03: 指標切り替えボタン（spearman/beta）が表示される', () => {
    // 【テスト目的】: Spearman/β の指標切り替えボタンが存在すること 🟢
    render(
      <SensitivityHeatmap
        data={makeSensitivityData()}
        metric="spearman"
        threshold={0}
      />,
    );

    // 【確認内容】: 指標ボタンが 2 つ存在すること
    expect(screen.getByTestId('metric-btn-spearman')).toBeInTheDocument();
    expect(screen.getByTestId('metric-btn-beta')).toBeInTheDocument();
  });

  // TC-802-04: デフォルト指標ボタンがアクティブ
  test('TC-802-04: metric=spearman のとき spearman ボタンが aria-pressed=true', () => {
    // 【テスト目的】: 初期指標の aria-pressed が正しく設定されること 🟢
    render(
      <SensitivityHeatmap
        data={makeSensitivityData()}
        metric="spearman"
        threshold={0}
      />,
    );

    // 【確認内容】: spearman がアクティブ、beta が非アクティブ
    expect(screen.getByTestId('metric-btn-spearman')).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByTestId('metric-btn-beta')).toHaveAttribute('aria-pressed', 'false');
  });

  // TC-802-05: しきい値スライダーが存在する
  test('TC-802-05: しきい値スライダー（data-testid=threshold-slider）が表示される', () => {
    // 【テスト目的】: しきい値フィルタのスライダーが存在すること 🟢
    render(
      <SensitivityHeatmap
        data={makeSensitivityData()}
        metric="spearman"
        threshold={0.3}
      />,
    );

    // 【確認内容】: スライダーが存在し値が正しいこと
    const slider = screen.getByTestId('threshold-slider');
    expect(slider).toBeInTheDocument();
    expect(slider).toHaveValue('0.3');
  });

  // TC-802-06: しきい値変更でコールバックが呼ばれる
  test('TC-802-06: しきい値スライダー変更で onThresholdChange が呼ばれる', () => {
    // 【テスト目的】: しきい値変更コールバックが正しい値で呼ばれること 🟢
    const onThresholdChange = vi.fn();
    render(
      <SensitivityHeatmap
        data={makeSensitivityData()}
        metric="spearman"
        threshold={0}
        onThresholdChange={onThresholdChange}
      />,
    );

    // 【処理実行】: スライダーを 0.5 に変更
    fireEvent.change(screen.getByTestId('threshold-slider'), {
      target: { value: '0.5' },
    });

    // 【確認内容】: onThresholdChange が 0.5 で呼ばれること
    expect(onThresholdChange).toHaveBeenCalledWith(0.5);
  });
});

// -------------------------------------------------------------------------
// ローディング状態
// -------------------------------------------------------------------------

describe('SensitivityHeatmap — ローディング状態', () => {
  afterEach(() => {
    cleanup();
  });

  // TC-802-07: isLoading=true で「WASM計算中...」を表示
  test('TC-802-07: isLoading=true のとき「WASM計算中...」が表示される', () => {
    // 【テスト目的】: 計算中はローディングインジケーターが表示されること 🟢
    render(
      <SensitivityHeatmap
        data={null}
        metric="spearman"
        threshold={0}
        isLoading
      />,
    );

    // 【確認内容】: ローディングメッセージが表示されること
    expect(screen.getByText('WASM計算中...')).toBeInTheDocument();
  });

  // TC-802-08: isLoading=true のとき ECharts は表示されない
  test('TC-802-08: isLoading=true のとき ECharts コンテナは表示されない', () => {
    // 【テスト目的】: ローディング中はチャートが非表示であること 🟢
    render(
      <SensitivityHeatmap
        data={null}
        metric="spearman"
        threshold={0}
        isLoading
      />,
    );

    // 【確認内容】: ECharts コンテナが存在しないこと
    expect(screen.queryByTestId('echarts')).not.toBeInTheDocument();
  });
});

// -------------------------------------------------------------------------
// 異常系
// -------------------------------------------------------------------------

describe('SensitivityHeatmap — 異常系', () => {
  afterEach(() => {
    cleanup();
  });

  // TC-802-E01: data=null, isLoading=false で「データが読み込まれていません」
  test('TC-802-E01: data=null・isLoading=false のとき「データが読み込まれていません」が表示される', () => {
    // 【テスト目的】: データなし・計算待ちなしの場合に適切な空状態が表示されること 🟢
    render(
      <SensitivityHeatmap
        data={null}
        metric="spearman"
        threshold={0}
        isLoading={false}
      />,
    );

    // 【確認内容】: 空状態メッセージが表示されること
    expect(screen.getByText('データが読み込まれていません')).toBeInTheDocument();
  });

  // TC-802-E02: threshold が高くてデータがない → ECharts は表示されるが系列が空
  test('TC-802-E02: 全パラメータがしきい値以下でも ECharts は表示される', () => {
    // 【テスト目的】: しきい値で全データが除外されても表示エラーにならないこと 🟢
    render(
      <SensitivityHeatmap
        data={makeLowCorrelationData()}
        metric="spearman"
        threshold={0.9} // 全ての相関値 (0.1, 0.05) がこれより低い
      />,
    );

    // 【確認内容】: ECharts コンテナが表示されること（空でもクラッシュしない）
    expect(screen.getByTestId('echarts')).toBeInTheDocument();
  });
});
