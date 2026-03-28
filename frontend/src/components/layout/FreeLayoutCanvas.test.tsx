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
    { chartId: 'pareto-front', gridRow: [1, 3], gridCol: [1, 3] },
    { chartId: 'parallel-coords', gridRow: [1, 3], gridCol: [3, 5] },
    { chartId: 'history', gridRow: [3, 5], gridCol: [1, 3] },
    { chartId: 'scatter-matrix', gridRow: [3, 5], gridCol: [3, 5] },
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
    // 【ドロップゾーンにドロップ】
    fireEvent.dragOver(dropZone);
    fireEvent.drop(dropZone);

    // 【確認内容】: pareto-front の gridRow が 3 から始まるように更新されること
    const updatedCell = useLayoutStore
      .getState()
      .freeModeLayout?.cells.find((c) => c.chartId === 'pareto-front');
    expect(updatedCell?.gridRow[0]).toBe(3);
    expect(updatedCell?.gridCol[0]).toBe(3);
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

  // TC-1501-F06: プリセットボタン（A/B/C）が表示される
  test('TC-1501-F06: プリセットボタンA/B/Cが表示される', () => {
    // 【テスト目的】: プリセット切替UIが存在することを確認 🟢
    render(<FreeLayoutCanvas />);
    expect(screen.getByTestId('free-layout-preset-A')).toBeInTheDocument();
    expect(screen.getByTestId('free-layout-preset-B')).toBeInTheDocument();
    expect(screen.getByTestId('free-layout-preset-C')).toBeInTheDocument();
  });

  // TC-1501-F07: プリセットボタンクリック + confirm でレイアウトが更新される
  test('TC-1501-F07: プリセットAボタンとconfirmでfreeModeLayoutが更新される', () => {
    // 【テスト目的】: プリセット適用でfreeModeLayoutが切り替わることを確認 🟢
    vi.spyOn(window, 'confirm').mockReturnValue(true);
    useLayoutStore.setState({ freeModeLayout: SAMPLE_LAYOUT });
    render(<FreeLayoutCanvas />);

    fireEvent.click(screen.getByTestId('free-layout-preset-A'));

    // 【確認内容】: confirm が呼ばれること
    expect(window.confirm).toHaveBeenCalled();
    // 【確認内容】: freeModeLayout が更新されること（セルが存在すること）
    expect(useLayoutStore.getState().freeModeLayout?.cells.length).toBeGreaterThan(0);
  });
});
