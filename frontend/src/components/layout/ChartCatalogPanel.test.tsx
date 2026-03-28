/**
 * ChartCatalogPanel テスト (chart-catalog TASK-003)
 *
 * 【テスト対象】: ChartCatalogPanel — 右側収納可能なチャートカタログパネル
 * 【テスト方針】: useLayoutStore を vi.mock でモック
 */

import { describe, test, expect, beforeEach, vi } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';
import '@testing-library/jest-dom';

// -------------------------------------------------------------------------
// layoutStore モック
// -------------------------------------------------------------------------

const { mockFreeModeLayout } = vi.hoisted(() => {
  const mockFreeModeLayout = { cells: [] as Array<{ cellId: string; chartId: string; gridRow: [number, number]; gridCol: [number, number] }> };
  return { mockFreeModeLayout };
});

vi.mock('../../stores/layoutStore', () => ({
  useLayoutStore: vi.fn().mockImplementation((selector: (s: { freeModeLayout: typeof mockFreeModeLayout | null }) => unknown) =>
    selector({ freeModeLayout: mockFreeModeLayout }),
  ),
}));

import { ChartCatalogPanel, CHART_CATALOG } from './ChartCatalogPanel';
import { useLayoutStore } from '../../stores/layoutStore';

// -------------------------------------------------------------------------
// テスト
// -------------------------------------------------------------------------

describe('ChartCatalogPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockFreeModeLayout.cells = [];
    vi.mocked(useLayoutStore).mockImplementation((selector) =>
      selector({ freeModeLayout: mockFreeModeLayout }),
    );
    cleanup();
  });

  // TC-CC-P01: トグルボタンが存在する
  test('TC-CC-P01: data-testid="catalog-toggle-btn" が DOM に存在する', () => {
    // 【テスト目的】: REQ-004 — トグルボタンが存在すること
    render(<ChartCatalogPanel />);
    expect(screen.getByTestId('catalog-toggle-btn')).toBeInTheDocument();
  });

  // TC-CC-P02: 初期状態でカタログリストが非表示
  test('TC-CC-P02: 初期状態でカタログリストが非表示', () => {
    // 【テスト目的】: REQ-203 — isOpen 初期値が false
    render(<ChartCatalogPanel />);
    expect(screen.getByTestId('catalog-list')).not.toBeVisible();
  });

  // TC-CC-P03: トグルボタンクリックでカタログリストが表示される
  test('TC-CC-P03: トグルボタンクリックでカタログリストが表示される', () => {
    // 【テスト目的】: REQ-004, REQ-201 — トグルで開閉できること
    render(<ChartCatalogPanel />);
    fireEvent.click(screen.getByTestId('catalog-toggle-btn'));
    expect(screen.getByTestId('catalog-list')).toBeVisible();
  });

  // TC-CC-P04: 14チャートアイテム全てが存在する
  test('TC-CC-P04: 14チャートアイテム全てが存在する', () => {
    // 【テスト目的】: REQ-002 — 14種のチャートが表示されること
    render(<ChartCatalogPanel />);
    fireEvent.click(screen.getByTestId('catalog-toggle-btn'));
    expect(CHART_CATALOG).toHaveLength(14);
    CHART_CATALOG.forEach(({ chartId }) => {
      expect(screen.getByTestId(`catalog-item-${chartId}`)).toBeInTheDocument();
    });
  });

  // TC-CC-P05: cells に 'slice' が2件あるとき data-count="2" が付与される
  test('TC-CC-P05: cells に slice が2件あるとき data-count="2" が付与される', () => {
    // 【テスト目的】: REQ-108 — インスタンス数が表示されること
    mockFreeModeLayout.cells = [
      { cellId: 'cell-1', chartId: 'slice', gridRow: [1, 3], gridCol: [1, 3] },
      { cellId: 'cell-2', chartId: 'slice', gridRow: [3, 5], gridCol: [1, 3] },
    ];
    vi.mocked(useLayoutStore).mockImplementation((selector) =>
      selector({ freeModeLayout: mockFreeModeLayout }),
    );
    render(<ChartCatalogPanel />);
    fireEvent.click(screen.getByTestId('catalog-toggle-btn'));
    expect(screen.getByTestId('catalog-item-slice')).toHaveAttribute('data-count', '2');
  });

  // TC-CC-P06: cells に 'slice' が0件のとき data-count="0" でインスタンス数テキストが非表示
  test('TC-CC-P06: cells に slice が0件のとき data-count="0" でインスタンス数テキスト非表示', () => {
    // 【テスト目的】: REQ-108 — 0件のときインスタンス数を表示しないこと
    render(<ChartCatalogPanel />);
    fireEvent.click(screen.getByTestId('catalog-toggle-btn'));
    const sliceItem = screen.getByTestId('catalog-item-slice');
    expect(sliceItem).toHaveAttribute('data-count', '0');
    // インスタンス数テキスト（N個）が表示されていないこと
    expect(sliceItem.textContent).not.toContain('個）');
  });

  // TC-CC-P07: freeModeLayout が null のとき全アイテムが data-count="0"
  test('TC-CC-P07: freeModeLayout が null のとき全アイテムが data-count="0"', () => {
    // 【テスト目的】: エラーハンドリング — null 時に count=0 で安全に表示
    vi.mocked(useLayoutStore).mockImplementation((selector) =>
      selector({ freeModeLayout: null }),
    );
    render(<ChartCatalogPanel />);
    fireEvent.click(screen.getByTestId('catalog-toggle-btn'));
    CHART_CATALOG.forEach(({ chartId }) => {
      expect(screen.getByTestId(`catalog-item-${chartId}`)).toHaveAttribute('data-count', '0');
    });
  });
});
