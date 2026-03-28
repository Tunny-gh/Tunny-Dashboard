/**
 * OptimizationHistory テスト (TASK-1001)
 *
 * 【テスト対象】: OptimizationHistory — 単目的最適化の収束履歴チャート
 * 【テスト方針】: echarts-for-react を vi.mock でモック、表示モード切り替えと
 *               フェーズ自動検出（境界値）を検証する
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';

// -------------------------------------------------------------------------
// echarts-for-react モック（__mocks__/echarts-for-react.tsx を自動使用）
// -------------------------------------------------------------------------

// 【ECharts モック】: jsdom 環境で canvas なしにレンダリング可能にする 🟢
vi.mock('echarts-for-react');

import { OptimizationHistory, detectPhase } from './OptimizationHistory';

// -------------------------------------------------------------------------
// テストヘルパー
// -------------------------------------------------------------------------

/** 【ヘルパー】: Best値が単調減少するテストデータを生成する */
function makeConvergingData(count: number): { trial: number; value: number }[] {
  return Array.from({ length: count }, (_, i) => ({
    trial: i + 1,
    value: 100 - i,
  }));
}

// -------------------------------------------------------------------------
// 正常系
// -------------------------------------------------------------------------

describe('OptimizationHistory — 正常系', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  // TC-1001-01: エラーなくレンダリング
  test('TC-1001-01: OptimizationHistory がエラーなくレンダリングされる', () => {
    // 【テスト目的】: データなし・minimize 方向でもクラッシュしないこと 🟢
    expect(() =>
      render(<OptimizationHistory data={[]} direction="minimize" />),
    ).not.toThrow();
  });

  // TC-1001-02: 4つの表示モードボタンが存在する
  test('TC-1001-02: 4つの表示モードボタン（best/all/moving-avg/improvement）が表示される', () => {
    // 【テスト目的】: Best値推移・全試行値・移動平均・改善率の 4 モードボタンが存在すること 🟢
    render(<OptimizationHistory data={makeConvergingData(10)} direction="minimize" />);

    // 【確認内容】: 4 つのモードボタンが存在すること
    expect(screen.getByTestId('mode-btn-best')).toBeInTheDocument();
    expect(screen.getByTestId('mode-btn-all')).toBeInTheDocument();
    expect(screen.getByTestId('mode-btn-moving-avg')).toBeInTheDocument();
    expect(screen.getByTestId('mode-btn-improvement')).toBeInTheDocument();
  });

  // TC-1001-03: モード切り替えで aria-pressed が変わる
  test('TC-1001-03: moving-avg ボタンクリックで moving-avg モードがアクティブになる', () => {
    // 【テスト目的】: 表示モード切り替えボタンのアクティブ状態が変わること 🟢
    render(<OptimizationHistory data={makeConvergingData(10)} direction="minimize" />);

    // 【初期確認】: デフォルトは 'best' モードがアクティブ
    expect(screen.getByTestId('mode-btn-best')).toHaveAttribute('aria-pressed', 'true');

    // 【処理実行】: moving-avg ボタンをクリック
    fireEvent.click(screen.getByTestId('mode-btn-moving-avg'));

    // 【確認内容】: moving-avg がアクティブになり、best が非アクティブになること
    expect(screen.getByTestId('mode-btn-moving-avg')).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByTestId('mode-btn-best')).toHaveAttribute('aria-pressed', 'false');
  });
});

// -------------------------------------------------------------------------
// フェーズ自動検出（境界値テスト）
// -------------------------------------------------------------------------

describe('detectPhase — フェーズ自動検出 境界値テスト', () => {
  // TC-1001-04: 探索期（progress < 0.3）
  test('TC-1001-04: progress=0.1 のとき exploration（探索期）が返される', () => {
    // 【テスト目的】: 試行数の先頭 30% を探索期として検出すること 🟢
    expect(detectPhase(10, 100)).toBe('exploration'); // 10/100 = 0.1
  });

  // TC-1001-05: 精緻化期（0.3 <= progress < 0.7）
  test('TC-1001-05: progress=0.5 のとき exploitation（精緻化期）が返される', () => {
    // 【テスト目的】: 試行数の中間 40% を精緻化期として検出すること 🟢
    expect(detectPhase(50, 100)).toBe('exploitation'); // 50/100 = 0.5
  });

  // TC-1001-06: 収束期（progress >= 0.7）
  test('TC-1001-06: progress=0.8 のとき convergence（収束期）が返される', () => {
    // 【テスト目的】: 試行数の末尾 30% を収束期として検出すること 🟢
    expect(detectPhase(80, 100)).toBe('convergence'); // 80/100 = 0.8
  });
});
