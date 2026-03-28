/**
 * PDPChart テスト (TASK-804)
 *
 * 【テスト対象】: PDPChart — 部分依存プロット UIコンポーネント
 * 【テスト方針】:
 *   - echarts-for-react を vi.mock でモック（jsdom でも動作可能）
 *   - ローディング状態・空状態・R²警告・ONNX警告・ICEハイライトを検証
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';

// -------------------------------------------------------------------------
// echarts-for-react モック（__mocks__/echarts-for-react.tsx を自動使用）
// -------------------------------------------------------------------------

vi.mock('echarts-for-react');

import { PDPChart, getModelQuality } from './PDPChart';
import type { PdpData1d, PdpData2d } from './PDPChart';

// -------------------------------------------------------------------------
// テストヘルパー
// -------------------------------------------------------------------------

/** 【ヘルパー】: シンプルな1変数PDPデータを生成する */
function makePdpData1d(overrides?: Partial<PdpData1d>): PdpData1d {
  return {
    paramName: 'x1',
    objectiveName: 'obj0',
    grid: [0.0, 0.25, 0.5, 0.75, 1.0],
    values: [0.1, 0.3, 0.5, 0.7, 0.9],
    rSquared: 0.85,
    ...overrides,
  };
}

/** 【ヘルパー】: ICE ラインありの1変数PDPデータを生成する */
function makePdpData1dWithIce(highlightCount = 2): PdpData1d {
  return {
    paramName: 'x1',
    objectiveName: 'obj0',
    grid: [0.0, 0.5, 1.0],
    values: [0.1, 0.5, 0.9],
    rSquared: 0.9,
    iceLines: [
      [0.08, 0.48, 0.88],  // ICE ライン 0
      [0.12, 0.52, 0.92],  // ICE ライン 1
      [0.05, 0.45, 0.85],  // ICE ライン 2
    ].slice(0, highlightCount + 1),
  };
}

/** 【ヘルパー】: 2変数PDPデータを生成する */
function makePdpData2d(): PdpData2d {
  return {
    param1Name: 'x1',
    param2Name: 'x2',
    objectiveName: 'obj0',
    grid1: [0.0, 0.5, 1.0],
    grid2: [0.0, 0.5, 1.0],
    values: [
      [0.1, 0.3, 0.5],
      [0.3, 0.5, 0.7],
      [0.5, 0.7, 0.9],
    ],
    rSquared: 0.88,
  };
}

// -------------------------------------------------------------------------
// 正常系
// -------------------------------------------------------------------------

describe('PDPChart — 正常系', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  // TC-804-01: data=null, data2d=null でエラーなくレンダリング
  test('TC-804-01: data1d=null・data2d 未指定でエラーなくレンダリングされる', () => {
    // 【テスト目的】: データなし・ローディングなしでもクラッシュしないこと 🟢
    expect(() => render(<PDPChart data1d={null} />)).not.toThrow();
  });

  // TC-804-02: data1d ありで ECharts が表示される
  test('TC-804-02: data1d あり・isLoading=false で ECharts が表示される', () => {
    // 【テスト目的】: 1変数PDPデータがある場合にチャートが描画されること 🟢
    render(<PDPChart data1d={makePdpData1d()} />);

    // 【確認内容】: ECharts コンテナが表示されること
    expect(screen.getByTestId('echarts')).toBeInTheDocument();
  });

  // TC-804-03: data2d あり（data1d=null）で ECharts が表示される
  test('TC-804-03: data2d あり・data1d=null で ECharts が表示される', () => {
    // 【テスト目的】: 2変数PDPデータがある場合にヒートマップが表示されること 🟢
    render(<PDPChart data1d={null} data2d={makePdpData2d()} />);

    // 【確認内容】: ECharts コンテナが表示されること
    expect(screen.getByTestId('echarts')).toBeInTheDocument();
  });

  // TC-804-04: モデル品質パネルが表示される
  test('TC-804-04: モデル品質パネル（data-testid=model-quality-panel）が表示される', () => {
    // 【テスト目的】: R² と評価ラベルを含むパネルが常に表示されること 🟢
    render(<PDPChart data1d={makePdpData1d({ rSquared: 0.85 })} />);

    // 【確認内容】: パネルと R² 値が表示されること
    expect(screen.getByTestId('model-quality-panel')).toBeInTheDocument();
    expect(screen.getByTestId('r2-value')).toBeInTheDocument();
    expect(screen.getByTestId('quality-label')).toBeInTheDocument();
  });

  // TC-804-05: R² 値がパネルに表示される
  test('TC-804-05: モデル品質パネルに R² 値が表示される', () => {
    // 【テスト目的】: R² の数値が画面に表示されることを検証する 🟢
    render(<PDPChart data1d={makePdpData1d({ rSquared: 0.923 })} />);

    // 【確認内容】: R² が小数点3桁で表示されること
    expect(screen.getByTestId('r2-value').textContent).toBe('0.923');
  });
});

// -------------------------------------------------------------------------
// ローディング状態
// -------------------------------------------------------------------------

describe('PDPChart — ローディング状態', () => {
  afterEach(() => {
    cleanup();
  });

  // TC-804-06: isLoading=true で「PDP計算中...」が表示される
  test('TC-804-06: isLoading=true のとき「PDP計算中...」が表示される', () => {
    // 【テスト目的】: 計算中はローディングインジケーターが表示されること 🟢
    render(<PDPChart data1d={null} isLoading />);

    // 【確認内容】: ローディングメッセージが表示されること
    expect(screen.getByText('PDP計算中...')).toBeInTheDocument();
  });

  // TC-804-07: isLoading=true のとき ECharts は表示されない
  test('TC-804-07: isLoading=true のとき ECharts コンテナは表示されない', () => {
    // 【テスト目的】: ローディング中はチャートが非表示であること 🟢
    render(<PDPChart data1d={makePdpData1d()} isLoading />);

    // 【確認内容】: ECharts コンテナが存在しないこと
    expect(screen.queryByTestId('echarts')).not.toBeInTheDocument();
  });
});

// -------------------------------------------------------------------------
// R² 警告バッジ
// -------------------------------------------------------------------------

describe('PDPChart — R² 警告バッジ', () => {
  afterEach(() => {
    cleanup();
  });

  // TC-804-08: R² < 0.8 のとき警告バッジが表示される
  test('TC-804-08: R² < 0.8 のとき「PDPの解釈に注意が必要です」警告バッジが表示される', () => {
    // 【テスト目的】: R² 低下時に警告バッジが表示されることを検証する 🟢
    render(<PDPChart data1d={makePdpData1d({ rSquared: 0.65 })} />);

    // 【確認内容】: 警告バッジが表示されること
    expect(screen.getByTestId('r2-warning-badge')).toBeInTheDocument();
    expect(screen.getByText(/PDPの解釈に注意が必要です/)).toBeInTheDocument();
  });

  // TC-804-09: R² < 0.8 の警告バッジに R² 値が含まれる
  test('TC-804-09: 警告バッジに R² の数値が含まれる', () => {
    // 【テスト目的】: 警告バッジがどの R² 値によるものか確認できること 🟢
    render(<PDPChart data1d={makePdpData1d({ rSquared: 0.72 })} />);

    // 【確認内容】: バッジに R²=0.72 の表示が含まれること
    const badge = screen.getByTestId('r2-warning-badge');
    expect(badge.textContent).toContain('0.72');
  });

  // TC-804-10: R² >= 0.8 のとき警告バッジが表示されない
  test('TC-804-10: R² >= 0.8 のとき警告バッジは表示されない', () => {
    // 【テスト目的】: R² が十分高い場合は警告バッジを表示しないこと 🟢
    render(<PDPChart data1d={makePdpData1d({ rSquared: 0.85 })} />);

    // 【確認内容】: 警告バッジが存在しないこと
    expect(screen.queryByTestId('r2-warning-badge')).not.toBeInTheDocument();
  });
});

// -------------------------------------------------------------------------
// ONNX 未使用警告バナー
// -------------------------------------------------------------------------

describe('PDPChart — 線形近似警告バナー', () => {
  afterEach(() => {
    cleanup();
  });

  // TC-804-11: useOnnx=false で警告バナーが表示される
  test('TC-804-11: useOnnx=false のとき「線形近似で表示中」警告バナーが表示される', () => {
    // 【テスト目的】: ONNX 未使用時に警告バナーが表示されることを検証する 🟢
    render(<PDPChart data1d={makePdpData1d()} useOnnx={false} />);

    // 【確認内容】: 警告バナーが表示されること
    expect(screen.getByTestId('linear-approx-banner')).toBeInTheDocument();
    expect(screen.getByText(/線形近似で表示中/)).toBeInTheDocument();
  });

  // TC-804-12: useOnnx=true のとき警告バナーが表示されない
  test('TC-804-12: useOnnx=true のとき「線形近似」警告バナーは表示されない', () => {
    // 【テスト目的】: ONNX 使用中は警告バナーを表示しないこと 🟢
    render(<PDPChart data1d={makePdpData1d()} useOnnx={true} />);

    // 【確認内容】: 警告バナーが存在しないこと
    expect(screen.queryByTestId('linear-approx-banner')).not.toBeInTheDocument();
  });

  // TC-804-13: onOnnxRequest コールバックが渡されたとき .onnx 読み込みボタンが表示される
  test('TC-804-13: onOnnxRequest あり・useOnnx=false のとき .onnx 読み込みボタンが表示される', () => {
    // 【テスト目的】: ONNX リクエストボタンが警告バナーに追加されること 🟢
    const onOnnxRequest = vi.fn();
    render(<PDPChart data1d={makePdpData1d()} useOnnx={false} onOnnxRequest={onOnnxRequest} />);

    // 【確認内容】: ボタンが表示されること
    expect(screen.getByTestId('onnx-request-btn')).toBeInTheDocument();
  });

  // TC-804-14: .onnx 読み込みボタンクリックで onOnnxRequest が呼ばれる
  test('TC-804-14: .onnx ボタンクリックで onOnnxRequest が呼ばれる', () => {
    // 【テスト目的】: コールバックが正しく呼び出されることを検証する 🟢
    const onOnnxRequest = vi.fn();
    render(<PDPChart data1d={makePdpData1d()} useOnnx={false} onOnnxRequest={onOnnxRequest} />);

    // 【処理実行】: ボタンをクリックする
    fireEvent.click(screen.getByTestId('onnx-request-btn'));

    // 【確認内容】: コールバックが1回呼ばれること
    expect(onOnnxRequest).toHaveBeenCalledOnce();
  });
});

// -------------------------------------------------------------------------
// ICE ライン ハイライト連動
// -------------------------------------------------------------------------

describe('PDPChart — ICE ラインハイライト連動', () => {
  afterEach(() => {
    cleanup();
  });

  // TC-804-15: highlightedIndices を渡してもクラッシュしない
  test('TC-804-15: highlightedIndices=[0,1] を渡してもクラッシュしない', () => {
    // 【テスト目的】: ICE ラインハイライト時にコンポーネントがクラッシュしないこと 🟢
    expect(() =>
      render(
        <PDPChart
          data1d={makePdpData1dWithIce(2)}
          highlightedIndices={[0, 1]}
        />,
      ),
    ).not.toThrow();
  });

  // TC-804-16: highlightedIndices が変わっても ECharts が表示される
  test('TC-804-16: highlightedIndices 変更後も ECharts コンテナが表示される', () => {
    // 【テスト目的】: Brushing 後もチャートが正常に描画されること 🟢
    const { rerender } = render(
      <PDPChart data1d={makePdpData1dWithIce(2)} highlightedIndices={[]} />,
    );

    // 【処理実行】: ハイライトを更新する（Brushing のシミュレーション）
    rerender(<PDPChart data1d={makePdpData1dWithIce(2)} highlightedIndices={[0, 1]} />);

    // 【確認内容】: ECharts コンテナが引き続き表示されること
    expect(screen.getByTestId('echarts')).toBeInTheDocument();
  });

  // TC-804-17: iceLines なし（省略）でもクラッシュしない
  test('TC-804-17: iceLines 省略でも highlightedIndices を渡してもクラッシュしない', () => {
    // 【テスト目的】: ICE ラインなしのデータで highlightedIndices を渡してもクラッシュしないこと 🟢
    const data = makePdpData1d(); // iceLines なし
    expect(() =>
      render(<PDPChart data1d={data} highlightedIndices={[0, 1, 2]} />),
    ).not.toThrow();
  });
});

// -------------------------------------------------------------------------
// 異常系
// -------------------------------------------------------------------------

describe('PDPChart — 異常系', () => {
  afterEach(() => {
    cleanup();
  });

  // TC-804-E01: data1d=null・data2d 未指定 で「データが読み込まれていません」
  test('TC-804-E01: data1d=null・data2d なしのとき「データが読み込まれていません」が表示される', () => {
    // 【テスト目的】: データなし・計算待ちなしの場合に適切な空状態が表示されること 🟢
    render(<PDPChart data1d={null} isLoading={false} />);

    // 【確認内容】: 空状態メッセージが表示されること
    expect(screen.getByText('データが読み込まれていません')).toBeInTheDocument();
  });

  // TC-804-E02: rSquared=0 のとき「推奨外」が表示される
  test('TC-804-E02: rSquared=0 のとき quality-label に「推奨外」が含まれる', () => {
    // 【テスト目的】: R²=0 の最悪ケースで適切な評価が表示されること 🟢
    render(<PDPChart data1d={makePdpData1d({ rSquared: 0.0 })} />);

    // 【確認内容】: 「推奨外」が表示されること
    expect(screen.getByTestId('quality-label').textContent).toContain('推奨外');
  });
});

// -------------------------------------------------------------------------
// getModelQuality 純粋関数テスト
// -------------------------------------------------------------------------

describe('getModelQuality — 品質評価関数', () => {
  // TC-804-Q01: R² >= 0.8 → '良好'
  test('TC-804-Q01: R² >= 0.8 のとき "良好" を返す', () => {
    // 【テスト目的】: R² しきい値 0.8 以上で「良好」が返ること 🟢
    expect(getModelQuality(0.8)).toBe('良好'); // 【確認内容】: 境界値 0.8 が「良好」
    expect(getModelQuality(0.95)).toBe('良好'); // 【確認内容】: 高い R² が「良好」
    expect(getModelQuality(1.0)).toBe('良好'); // 【確認内容】: 完璧な R² が「良好」
  });

  // TC-804-Q02: 0.5 <= R² < 0.8 → '要注意'
  test('TC-804-Q02: 0.5 <= R² < 0.8 のとき "要注意" を返す', () => {
    // 【テスト目的】: R² が中間範囲のとき「要注意」が返ること 🟢
    expect(getModelQuality(0.5)).toBe('要注意'); // 【確認内容】: 境界値 0.5 が「要注意」
    expect(getModelQuality(0.65)).toBe('要注意'); // 【確認内容】: 中間値が「要注意」
    expect(getModelQuality(0.79)).toBe('要注意'); // 【確認内容】: 上限境界が「要注意」
  });

  // TC-804-Q03: R² < 0.5 → '推奨外'
  test('TC-804-Q03: R² < 0.5 のとき "推奨外" を返す', () => {
    // 【テスト目的】: R² が低い場合「推奨外」が返ること 🟢
    expect(getModelQuality(0.0)).toBe('推奨外'); // 【確認内容】: R²=0 が「推奨外」
    expect(getModelQuality(0.3)).toBe('推奨外'); // 【確認内容】: 低い R² が「推奨外」
    expect(getModelQuality(0.49)).toBe('推奨外'); // 【確認内容】: 下限境界が「推奨外」
  });
});
