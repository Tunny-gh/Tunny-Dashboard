/**
 * FreeLayoutCanvas テスト (TASK-1501)
 *
 * 【テスト対象】: FreeLayoutCanvas コンポーネント
 * 【テスト方針】: useLayoutStore を実ストアで使用し、ドラッグ&ドロップ動作を確認
 */

import { describe, test, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import '@testing-library/jest-dom';
import { useLayoutStore } from '../../stores/layoutStore';
import { FreeLayoutCanvas } from './FreeLayoutCanvas';
import type { FreeModeLayout } from '../../types';

// -------------------------------------------------------------------------
// ヘルパー
// -------------------------------------------------------------------------

function resetStore() {
  useLayoutStore.setState({
    layoutMode: 'D',
    freeModeLayout: null,
    layoutLoadError: null,
    visibleCharts: new Set(['pareto-front', 'parallel-coords', 'history', 'scatter-matrix']),
    panelSizes: { leftPanel: 280, bottomPanel: 200 },
  });
}

const SAMPLE_LAYOUT: FreeModeLayout = {
  cells: [
    { cellId: 'pareto-front', chartId: 'pareto-front', gridRow: [1, 3], gridCol: [1, 3] },
    { cellId: 'parallel-coords', chartId: 'parallel-coords', gridRow: [1, 3], gridCol: [3, 5] },
    { cellId: 'history', chartId: 'history', gridRow: [3, 5], gridCol: [1, 3] },
    { cellId: 'scatter-matrix', chartId: 'scatter-matrix', gridRow: [3, 5], gridCol: [3, 5] },
  ],
};

// -------------------------------------------------------------------------
// テスト
// -------------------------------------------------------------------------

describe('FreeLayoutCanvas', () => {
  beforeEach(() => {
    resetStore();
    vi.clearAllMocks();
  });

  // TC-1501-F01: freeModeLayout のチャートカードが表示される
  test('TC-1501-F01: freeModeLayoutのチャートカードが表示される', () => {
    // 【テスト目的】: freeModeLayout で指定したチャートのカードが表示されることを確認 🟢 REQ-032
    useLayoutStore.setState({ freeModeLayout: SAMPLE_LAYOUT });
    render(<FreeLayoutCanvas />);
    // 【確認内容】: 4件のカードが表示されること
    expect(screen.getByTestId('free-layout-card-pareto-front')).toBeInTheDocument();
    expect(screen.getByTestId('free-layout-card-parallel-coords')).toBeInTheDocument();
    expect(screen.getByTestId('free-layout-card-history')).toBeInTheDocument();
    expect(screen.getByTestId('free-layout-card-scatter-matrix')).toBeInTheDocument();
  });

  // TC-1501-F02: freeModeLayoutがnullのときデフォルトレイアウトが表示される
  test('TC-1501-F02: freeModeLayout=nullのときデフォルトレイアウトのカードが表示される', () => {
    // 【テスト目的】: デフォルトレイアウトが適用されることを確認 🟢
    render(<FreeLayoutCanvas />);
    // 【確認内容】: デフォルトの4件のカードが表示されること
    expect(screen.getByTestId('free-layout-card-pareto-front')).toBeInTheDocument();
  });

  // TC-1501-F03: ドラッグ&ドロップでチャートが移動する
  test('TC-1501-F03: ドラッグ&ドロップでチャートのグリッド位置が更新される', () => {
    // 【テスト目的】: ドラッグ&ドロップ操作でfreeModeLayoutが更新されることを確認 🟢 REQ-032 (TC-1501-S01相当)
    useLayoutStore.setState({ freeModeLayout: SAMPLE_LAYOUT });
    render(<FreeLayoutCanvas />);

    const dragHandle = screen.getByTestId('free-layout-drag-handle-pareto-front');
    const dropZone = screen.getByTestId('free-layout-dropzone-3-3');

    // 【ドラッグ開始】
    fireEvent.dragStart(dragHandle);
    // 【ドロップゾーンにドロップ】（move-chart ペイロードなしでも draggingCellId state で動作）
    fireEvent.dragOver(dropZone);
    fireEvent.drop(dropZone);

    // 【確認内容】: pareto-front の gridRow が 3 から始まるように更新されること
    const updatedCell = useLayoutStore
      .getState()
      .freeModeLayout?.cells.find((c) => c.chartId === 'pareto-front');
    expect(updatedCell?.gridRow[0]).toBe(3);
    expect(updatedCell?.gridCol[0]).toBe(3);
  });

  // TC-1501-F08: 削除ボタンがタイルに存在する
  test('TC-1501-F08: 各タイルに data-testid="chart-close-btn-{cellId}" ボタンが存在する', () => {
    // 【テスト目的】: REQ-301 — 削除ボタンが存在すること
    useLayoutStore.setState({ freeModeLayout: SAMPLE_LAYOUT });
    render(<FreeLayoutCanvas />);
    expect(screen.getByTestId('chart-close-btn-pareto-front')).toBeInTheDocument();
    expect(screen.getByTestId('chart-close-btn-history')).toBeInTheDocument();
  });

  // TC-1501-F09: 削除ボタンクリックでタイルが除去される
  test('TC-1501-F09: 削除ボタンクリックで対応するセルが freeModeLayout から除去される', () => {
    // 【テスト目的】: REQ-302 — removeCell が正しい cellId で呼ばれること
    useLayoutStore.setState({ freeModeLayout: SAMPLE_LAYOUT });
    render(<FreeLayoutCanvas />);

    fireEvent.click(screen.getByTestId('chart-close-btn-pareto-front'));

    const cells = useLayoutStore.getState().freeModeLayout?.cells;
    expect(cells?.find((c) => c.cellId === 'pareto-front')).toBeUndefined();
  });

  // TC-1501-F10: カタログからの add-chart ドロップでセルが追加される
  test('TC-1501-F10: add-chart ペイロードのドロップで新しいセルが追加される', () => {
    // 【テスト目的】: REQ-102 — カタログドロップで addCell が呼ばれること
    useLayoutStore.setState({ freeModeLayout: { cells: [] }, layoutMode: 'D' });
    render(<FreeLayoutCanvas />);

    const dropZone = screen.getByTestId('free-layout-dropzone-1-1');
    fireEvent.dragOver(dropZone);
    fireEvent.drop(dropZone, {
      dataTransfer: { getData: () => JSON.stringify({ type: 'add-chart', chartId: 'edf' }) },
    });

    const cells = useLayoutStore.getState().freeModeLayout?.cells;
    expect(cells?.length).toBe(1);
    expect(cells?.[0].chartId).toBe('edf');
  });

  // TC-1501-F11: 同一 chartId を2回ドロップすると2つのセルが追加される
  test('TC-1501-F11: 同一chartIdを2回ドロップすると2エントリが追加される', () => {
    // 【テスト目的】: REQ-106 — 重複 chartId を許容すること
    useLayoutStore.setState({ freeModeLayout: { cells: [] }, layoutMode: 'D' });
    render(<FreeLayoutCanvas />);

    const dropZone1 = screen.getByTestId('free-layout-dropzone-1-1');
    const dropZone2 = screen.getByTestId('free-layout-dropzone-3-1');
    const payload = JSON.stringify({ type: 'add-chart', chartId: 'slice' });

    fireEvent.drop(dropZone1, { dataTransfer: { getData: () => payload } });
    fireEvent.drop(dropZone2, { dataTransfer: { getData: () => payload } });

    const cells = useLayoutStore.getState().freeModeLayout?.cells;
    expect(cells?.length).toBe(2);
    expect(cells?.[0].chartId).toBe('slice');
    expect(cells?.[1].chartId).toBe('slice');
    expect(cells?.[0].cellId).not.toBe(cells?.[1].cellId);
  });

  // TC-1501-F04: 「レイアウト保存」ボタンでトーストが表示される
  test('TC-1501-F04: レイアウト保存ボタンクリックでトースト通知が表示される', () => {
    // 【テスト目的】: 保存操作後にトースト通知が表示されることを確認 🟢 NFR-031
    useLayoutStore.setState({ freeModeLayout: SAMPLE_LAYOUT });
    render(<FreeLayoutCanvas />);

    const saveBtn = screen.getByTestId('save-free-layout-btn');
    fireEvent.click(saveBtn);

    // 【確認内容】: トーストが表示されること
    expect(screen.getByTestId('layout-saved-toast')).toBeInTheDocument();
  });

  // TC-1501-F05: layoutLoadError があるときエラーメッセージが表示される
  test('TC-1501-F05: layoutLoadErrorがあるときエラーメッセージが表示される', () => {
    // 【テスト目的】: レイアウト読み込みエラー時にエラーメッセージが表示されることを確認 🟢
    useLayoutStore.setState({ layoutLoadError: 'レイアウトを読み込めませんでした' });
    render(<FreeLayoutCanvas />);
    // 【確認内容】: エラーメッセージが表示されること
    expect(screen.getByTestId('layout-error-msg')).toBeInTheDocument();
    expect(screen.getByTestId('layout-error-msg')).toHaveTextContent(
      'レイアウトを読み込めませんでした',
    );
  });

  // TC-1501-F06: プリセットボタン（A/B/C）が存在しない（TASK-003 削除後）
  test('TC-1501-F06: プリセットボタン free-layout-preset-A/B/C が DOM に存在しない', () => {
    // 【テスト目的】: REQ-003 — FreeLayoutCanvas からプリセットボタンが削除されていること
    render(<FreeLayoutCanvas />);
    expect(screen.queryByTestId('free-layout-preset-A')).not.toBeInTheDocument();
    expect(screen.queryByTestId('free-layout-preset-B')).not.toBeInTheDocument();
    expect(screen.queryByTestId('free-layout-preset-C')).not.toBeInTheDocument();
  });

  // TC-1501-F07: レイアウト保存ボタンが退行なく存在する
  test('TC-1501-F07: save-free-layout-btn が引き続き存在する', () => {
    // 【テスト目的】: プリセット削除後も保存ボタンが退行なく動作すること
    render(<FreeLayoutCanvas />);
    expect(screen.getByTestId('save-free-layout-btn')).toBeInTheDocument();
  });
});
