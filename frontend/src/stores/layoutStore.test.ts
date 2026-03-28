/**
 * LayoutStore テスト (TASK-302)
 *
 * 【テスト対象】: useLayoutStore — レイアウトモード・チャート表示管理
 * 【テスト方針】: WASM 不要（純粋な状態管理のみ）
 */

import { describe, test, expect, beforeEach } from 'vitest';
import { useLayoutStore, DEFAULT_FREE_LAYOUT } from './layoutStore';

// -------------------------------------------------------------------------
// テストヘルパー
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
// 正常系
// -------------------------------------------------------------------------

describe('LayoutStore — 正常系', () => {
  beforeEach(() => {
    resetStore();
  });

  // TC-302-11: setLayoutMode が layoutMode を更新する
  test('TC-302-11: setLayoutMode("B") が layoutMode を B に更新する', () => {
    // 【テスト目的】: setLayoutMode アクションが正しく動作すること 🟢
    useLayoutStore.getState().setLayoutMode('B');
    expect(useLayoutStore.getState().layoutMode).toBe('B');
  });

  // TC-302-12: toggleChart が visibleCharts を更新する
  test('TC-302-12: toggleChart("history") が history を visibleCharts から除去する', () => {
    // 【テスト目的】: toggleChart が Set への追加・削除を正しく行うこと 🟢

    // 【前提確認】: 初期状態で history が含まれている
    expect(useLayoutStore.getState().visibleCharts.has('history')).toBe(true);

    // 【処理実行】: 削除
    useLayoutStore.getState().toggleChart('history');
    expect(useLayoutStore.getState().visibleCharts.has('history')).toBe(false);

    // 【処理実行】: 再追加
    useLayoutStore.getState().toggleChart('history');
    expect(useLayoutStore.getState().visibleCharts.has('history')).toBe(true);
  });

  // TC-302-13: saveLayout / loadLayout でレイアウト保存・復元できる
  test('TC-302-13: saveLayout → 状態変更 → loadLayout で元の設定が復元される', () => {
    // 【テスト目的】: レイアウト設定の保存・復元が完全に機能すること 🟢

    // 【初期設定】
    useLayoutStore.getState().setLayoutMode('C');
    useLayoutStore.getState().toggleChart('history'); // 削除

    // 【保存】
    const config = useLayoutStore.getState().saveLayout();

    // 【状態変更】: 別のレイアウトに変更
    useLayoutStore.getState().setLayoutMode('D');

    // 【復元】
    useLayoutStore.getState().loadLayout(config);

    // 【確認内容】: 元の設定に戻っている
    expect(useLayoutStore.getState().layoutMode).toBe('C');
    expect(useLayoutStore.getState().visibleCharts.has('history')).toBe(false);
  });
});

// -------------------------------------------------------------------------
// TASK-1501 追加テスト
// -------------------------------------------------------------------------

describe('LayoutStore — フリーモードレイアウト操作 (TASK-1501)', () => {
  beforeEach(() => {
    resetStore();
    useLayoutStore.setState({ layoutLoadError: null });
  });

  // TC-1501-L01: setFreeModeLayout でレイアウトが設定される
  test('TC-1501-L01: setFreeModeLayoutでfreeModeLayoutが設定される', () => {
    // 【テスト目的】: freeModeLayout が正しく設定されることを確認 🟢 REQ-032
    useLayoutStore.getState().setFreeModeLayout(DEFAULT_FREE_LAYOUT);
    expect(useLayoutStore.getState().freeModeLayout).toEqual(DEFAULT_FREE_LAYOUT);
  });

  // TC-1501-L02: updateCellPosition で指定チャートのグリッド位置が更新される
  test('TC-1501-L02: updateCellPositionで指定チャートのグリッド位置が更新される', () => {
    // 【テスト目的】: ドラッグ&ドロップ後の位置更新が正しく機能することを確認 🟢 REQ-032 (TC-1501-S01相当)
    useLayoutStore.getState().setFreeModeLayout(DEFAULT_FREE_LAYOUT);

    // 【処理実行】: pareto-front を (3, 3) に移動
    useLayoutStore.getState().updateCellPosition('pareto-front', [3, 5], [3, 5]);

    // 【確認内容】: pareto-front のグリッド位置が更新されること
    const cell = useLayoutStore
      .getState()
      .freeModeLayout?.cells.find((c) => c.chartId === 'pareto-front');
    expect(cell?.gridRow).toEqual([3, 5]);
    expect(cell?.gridCol).toEqual([3, 5]);
    // 【確認内容】: 他のチャートは変化しないこと
    const histCell = useLayoutStore
      .getState()
      .freeModeLayout?.cells.find((c) => c.chartId === 'history');
    expect(histCell?.gridRow).toEqual([3, 5]);
  });

  // TC-1501-L03: saveLayout → loadLayout で freeModeLayout が保存・復元される (TC-1501-S02相当)
  test('TC-1501-L03: saveLayout→loadLayoutでfreeModeLayoutが正しく保存・復元される', () => {
    // 【テスト目的】: レイアウトJSON保存・復元テスト 🟢 (TC-1501-S02相当)
    useLayoutStore.getState().setFreeModeLayout(DEFAULT_FREE_LAYOUT);
    useLayoutStore.getState().setLayoutMode('D');

    const config = useLayoutStore.getState().saveLayout();

    // 【状態変更】: 一旦クリアして異なる状態にする
    useLayoutStore.getState().setFreeModeLayout(null);

    // 【復元】
    useLayoutStore.getState().loadLayout(config);

    // 【確認内容】: freeModeLayout が正しく復元されること
    expect(useLayoutStore.getState().freeModeLayout).toEqual(DEFAULT_FREE_LAYOUT);
    expect(useLayoutStore.getState().layoutMode).toBe('D');
  });

  // TC-1501-L04: loadLayoutFromJson で不正 JSON はエラーを返す
  test('TC-1501-L04: loadLayoutFromJsonで不正JSONはエラーメッセージを設定してデフォルトに戻す', () => {
    // 【テスト目的】: 不正 JSON 時にエラー表示されデフォルトに戻ることを確認 🟢 NFR-032
    const result = useLayoutStore.getState().loadLayoutFromJson('{ invalid json');

    // 【確認内容】: 失敗を返すこと
    expect(result.success).toBe(false);
    // 【確認内容】: エラーメッセージが設定されること
    expect(useLayoutStore.getState().layoutLoadError).toContain('読み込めませんでした');
    // 【確認内容】: freeModeLayout がデフォルトに戻ること
    expect(useLayoutStore.getState().freeModeLayout).toEqual(DEFAULT_FREE_LAYOUT);
  });

  // TC-1501-L05: loadLayoutFromJson で有効 JSON は正常に読み込まれる
  test('TC-1501-L05: loadLayoutFromJsonで有効なJSONは正常に読み込まれる', () => {
    // 【テスト目的】: 正常な JSON が正しく読み込まれることを確認 🟢
    const config = {
      mode: 'D',
      visibleCharts: ['pareto-front'],
      panelSizes: { leftPanel: 280, bottomPanel: 200 },
      freeModeLayout: DEFAULT_FREE_LAYOUT,
    };
    const result = useLayoutStore.getState().loadLayoutFromJson(JSON.stringify(config));

    // 【確認内容】: 成功を返すこと
    expect(result.success).toBe(true);
    // 【確認内容】: エラーなし
    expect(useLayoutStore.getState().layoutLoadError).toBeNull();
    expect(useLayoutStore.getState().freeModeLayout).toEqual(DEFAULT_FREE_LAYOUT);
  });
});
