/**
 * LayoutStore tests (TASK-302)
 *
 * Tests useLayoutStore — layout mode and visible chart management.
 * No WASM dependency (pure state management only).
 */

import { describe, test, expect, beforeEach } from 'vitest';
import { useLayoutStore, DEFAULT_FREE_LAYOUT } from './layoutStore';

// -------------------------------------------------------------------------
// Test helpers
// -------------------------------------------------------------------------

function resetStore() {
  useLayoutStore.setState({
    layoutMode: 'A',
    visibleCharts: new Set(['pareto-front', 'parallel-coords', 'scatter-matrix', 'history']),
    panelSizes: { leftPanel: 280, bottomPanel: 200 },
    freeModeLayout: null,
  });
}

// -------------------------------------------------------------------------
// Happy path
// -------------------------------------------------------------------------

describe('LayoutStore — 正常系', () => {
  beforeEach(() => {
    resetStore();
  });

  // TC-302-11: setLayoutMode updates layoutMode
  test('TC-302-11: setLayoutMode("B") が layoutMode を B に更新する', () => {
    useLayoutStore.getState().setLayoutMode('B');
    expect(useLayoutStore.getState().layoutMode).toBe('B');
  });

  // TC-302-12: toggleChart updates visibleCharts
  test('TC-302-12: toggleChart("history") が history を visibleCharts から除去する', () => {
    expect(useLayoutStore.getState().visibleCharts.has('history')).toBe(true);

    useLayoutStore.getState().toggleChart('history');
    expect(useLayoutStore.getState().visibleCharts.has('history')).toBe(false);

    useLayoutStore.getState().toggleChart('history');
    expect(useLayoutStore.getState().visibleCharts.has('history')).toBe(true);
  });

  // TC-302-13: saveLayout / loadLayout round-trips the layout config
  test('TC-302-13: saveLayout → 状態変更 → loadLayout で元の設定が復元される', () => {
    useLayoutStore.getState().setLayoutMode('C');
    useLayoutStore.getState().toggleChart('history');

    const config = useLayoutStore.getState().saveLayout();

    useLayoutStore.getState().setLayoutMode('D');

    useLayoutStore.getState().loadLayout(config);

    expect(useLayoutStore.getState().layoutMode).toBe('C');
    expect(useLayoutStore.getState().visibleCharts.has('history')).toBe(false);
  });
});

// -------------------------------------------------------------------------
// TASK-1501 additional tests
// -------------------------------------------------------------------------

describe('LayoutStore — フリーモードレイアウト操作 (TASK-1501)', () => {
  beforeEach(() => {
    resetStore();
    useLayoutStore.setState({ layoutLoadError: null });
  });

  // TC-1501-L01: setFreeModeLayout sets freeModeLayout
  test('TC-1501-L01: setFreeModeLayoutでfreeModeLayoutが設定される', () => {
    useLayoutStore.getState().setFreeModeLayout(DEFAULT_FREE_LAYOUT);
    expect(useLayoutStore.getState().freeModeLayout).toEqual(DEFAULT_FREE_LAYOUT);
  });

  // TC-1501-L02: updateCellPosition updates the grid position of the specified cell
  test('TC-1501-L02: updateCellPositionで指定セルのグリッド位置が更新される', () => {
    useLayoutStore.getState().setFreeModeLayout(DEFAULT_FREE_LAYOUT);

    // cellId is a UUID so it must be retrieved dynamically
    const paretoCell = DEFAULT_FREE_LAYOUT.cells.find((c) => c.chartId === 'pareto-front')!;
    useLayoutStore.getState().updateCellPosition(paretoCell.cellId, [3, 5], [3, 5]);

    const cell = useLayoutStore
      .getState()
      .freeModeLayout?.cells.find((c) => c.chartId === 'pareto-front');
    expect(cell?.gridRow).toEqual([3, 5]);
    expect(cell?.gridCol).toEqual([3, 5]);
    // Other charts must remain unchanged
    const histCell = useLayoutStore
      .getState()
      .freeModeLayout?.cells.find((c) => c.chartId === 'history');
    expect(histCell?.gridRow).toEqual([3, 5]);
  });

  // TC-1501-L03: saveLayout → loadLayout round-trips freeModeLayout
  test('TC-1501-L03: saveLayout→loadLayoutでfreeModeLayoutが正しく保存・復元される', () => {
    useLayoutStore.getState().setFreeModeLayout(DEFAULT_FREE_LAYOUT);
    useLayoutStore.getState().setLayoutMode('D');

    const config = useLayoutStore.getState().saveLayout();

    useLayoutStore.getState().setFreeModeLayout(null);

    useLayoutStore.getState().loadLayout(config);

    expect(useLayoutStore.getState().freeModeLayout).toEqual(DEFAULT_FREE_LAYOUT);
    expect(useLayoutStore.getState().layoutMode).toBe('D');
  });

  // TC-1501-L06: addCell appends a new cell
  test('TC-1501-L06: addCellでセルが追加される', () => {
    useLayoutStore.getState().setFreeModeLayout(DEFAULT_FREE_LAYOUT);
    const before = useLayoutStore.getState().freeModeLayout!.cells.length;

    useLayoutStore.getState().addCell('slice', [1, 3], [1, 3]);

    const after = useLayoutStore.getState().freeModeLayout!.cells;
    expect(after.length).toBe(before + 1);
    const added = after[after.length - 1];
    expect(added.chartId).toBe('slice');
    expect(added.gridRow).toEqual([1, 3]);
    expect(added.gridCol).toEqual([1, 3]);
    expect(typeof added.cellId).toBe('string');
    expect(added.cellId.length).toBeGreaterThan(0);
  });

  // TC-1501-L07: calling addCell twice with the same chartId adds 2 entries with distinct cellIds (REQ-106)
  test('TC-1501-L07: addCellを2回呼ぶと同一chartIdでも2エントリ追加される', () => {
    useLayoutStore.getState().setFreeModeLayout({ cells: [] });

    useLayoutStore.getState().addCell('slice', [1, 3], [1, 3]);
    useLayoutStore.getState().addCell('slice', [3, 5], [1, 3]);

    const cells = useLayoutStore.getState().freeModeLayout!.cells;
    expect(cells.length).toBe(2);
    expect(cells[0].chartId).toBe('slice');
    expect(cells[1].chartId).toBe('slice');
    expect(cells[0].cellId).not.toBe(cells[1].cellId);
  });

  // TC-1501-L08: when freeModeLayout is null, addCell uses DEFAULT_FREE_LAYOUT as the base
  test('TC-1501-L08: freeModeLayout=nullのときaddCellはDEFAULT_FREE_LAYOUTをベースに追加する', () => {
    // freeModeLayout is null (set by resetStore())
    useLayoutStore.getState().addCell('edf', [1, 3], [1, 3]);

    const cells = useLayoutStore.getState().freeModeLayout!.cells;
    // DEFAULT_FREE_LAYOUT has 4 cells + 1 newly added
    expect(cells.length).toBe(DEFAULT_FREE_LAYOUT.cells.length + 1);
    expect(cells[cells.length - 1].chartId).toBe('edf');
  });

  // TC-1501-L09: removeCell removes only the specified cell
  test('TC-1501-L09: removeCellで指定cellIdのセルが削除される', () => {
    useLayoutStore.getState().setFreeModeLayout(DEFAULT_FREE_LAYOUT);

    // cellIds are UUIDs, so retrieve them dynamically
    const paretoCellId = DEFAULT_FREE_LAYOUT.cells.find((c) => c.chartId === 'pareto-front')!.cellId;
    const historyCellId = DEFAULT_FREE_LAYOUT.cells.find((c) => c.chartId === 'history')!.cellId;

    useLayoutStore.getState().removeCell(paretoCellId);

    const cells = useLayoutStore.getState().freeModeLayout!.cells;
    expect(cells.length).toBe(DEFAULT_FREE_LAYOUT.cells.length - 1);
    expect(cells.find((c) => c.cellId === paretoCellId)).toBeUndefined();
    // Other cells remain
    expect(cells.find((c) => c.cellId === historyCellId)).toBeDefined();
  });

  // TC-1501-L10: passing a non-existent cellId to removeCell does not change state
  test('TC-1501-L10: 存在しないcellIdをremoveCellに渡してもstateが変化しない', () => {
    useLayoutStore.getState().setFreeModeLayout(DEFAULT_FREE_LAYOUT);
    const before = useLayoutStore.getState().freeModeLayout!.cells.length;

    useLayoutStore.getState().removeCell('nonexistent-id');

    const after = useLayoutStore.getState().freeModeLayout!.cells.length;
    expect(after).toBe(before);
  });

  // TC-1501-L04: loadLayoutFromJson returns an error for invalid JSON
  test('TC-1501-L04: loadLayoutFromJsonで不正JSONはエラーメッセージを設定してデフォルトに戻す', () => {
    const result = useLayoutStore.getState().loadLayoutFromJson('{ invalid json');

    expect(result.success).toBe(false);
    expect(useLayoutStore.getState().layoutLoadError).toContain('読み込めませんでした');
    expect(useLayoutStore.getState().freeModeLayout).toEqual(DEFAULT_FREE_LAYOUT);
  });

  // TC-1501-L05: loadLayoutFromJson successfully loads valid JSON
  test('TC-1501-L05: loadLayoutFromJsonで有効なJSONは正常に読み込まれる', () => {
    const config = {
      mode: 'D',
      visibleCharts: ['pareto-front'],
      panelSizes: { leftPanel: 280, bottomPanel: 200 },
      freeModeLayout: DEFAULT_FREE_LAYOUT,
    };
    const result = useLayoutStore.getState().loadLayoutFromJson(JSON.stringify(config));

    expect(result.success).toBe(true);
    expect(useLayoutStore.getState().layoutLoadError).toBeNull();
    expect(useLayoutStore.getState().freeModeLayout).toEqual(DEFAULT_FREE_LAYOUT);
  });
});
