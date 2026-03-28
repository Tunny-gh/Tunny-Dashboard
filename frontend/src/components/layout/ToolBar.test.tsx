/**
 * ToolBar テスト (TASK-401)
 *
 * 【テスト対象】: ToolBar — ファイル読込・Study選択・レイアウト切替 UI
 * 【テスト方針】: studyStore / layoutStore を vi.mock でモック
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';

// -------------------------------------------------------------------------
// studyStore / layoutStore モック
// -------------------------------------------------------------------------

const { mockLoadJournalTB, mockSetLayoutModeTB } = vi.hoisted(() => {
  const mockLoadJournalTB = vi.fn().mockResolvedValue(undefined);
  const mockSetLayoutModeTB = vi.fn();
  return { mockLoadJournalTB, mockSetLayoutModeTB };
});

vi.mock('../../stores/studyStore', () => ({
  useStudyStore: vi.fn().mockImplementation((selector: (s: { loadJournal: typeof mockLoadJournalTB; isLoading: boolean }) => unknown) =>
    selector({ loadJournal: mockLoadJournalTB, isLoading: false }),
  ),
}));

vi.mock('../../stores/layoutStore', () => ({
  useLayoutStore: vi.fn().mockImplementation((selector: (s: { layoutMode: string; setLayoutMode: typeof mockSetLayoutModeTB }) => unknown) =>
    selector({ layoutMode: 'A', setLayoutMode: mockSetLayoutModeTB }),
  ),
}));

import { ToolBar } from './ToolBar';

// -------------------------------------------------------------------------
// 正常系
// -------------------------------------------------------------------------

describe('ToolBar — 正常系', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  // TC-401-03: ToolBar がレンダリングされる
  test('TC-401-03: ToolBar がエラーなくレンダリングされる', () => {
    // 【テスト目的】: ToolBar コンポーネントが正常にレンダリングできること 🟢
    expect(() => render(<ToolBar />)).not.toThrow();
  });

  // TC-401-04: ファイル input 変更で loadJournal が呼ばれる
  test('TC-401-04: ファイル input の変更で studyStore.loadJournal が呼ばれる', () => {
    // 【テスト目的】: ファイル選択ダイアログで loadJournal が呼ばれること 🟢
    render(<ToolBar />);

    // 【テストデータ準備】: ファイルモック
    const file = new File(['content'], 'journal.log', { type: 'text/plain' });

    // 【処理実行】: file input の change イベント
    const fileInput = screen.getByTestId('file-input');
    Object.defineProperty(fileInput, 'files', { value: [file] });
    fireEvent.change(fileInput);

    // 【確認内容】: loadJournal が呼ばれた
    expect(mockLoadJournalTB).toHaveBeenCalledOnce();
    expect(mockLoadJournalTB).toHaveBeenCalledWith(file);
  });

  // TC-401-05: レイアウトモードボタン A で setLayoutMode('A') が呼ばれる
  test("TC-401-05: レイアウトモードボタン A クリックで setLayoutMode('A') が呼ばれる", () => {
    // 【テスト目的】: レイアウト切り替えボタンが正しく動作すること 🟢
    render(<ToolBar />);

    // 【処理実行】: ボタン A をクリック
    fireEvent.click(screen.getByTestId('layout-btn-A'));

    // 【確認内容】: setLayoutMode が 'A' で呼ばれた
    expect(mockSetLayoutModeTB).toHaveBeenCalledWith('A');
  });

  // TC-401-06: レイアウトモードボタン B で setLayoutMode('B') が呼ばれる
  test("TC-401-06: レイアウトモードボタン B クリックで setLayoutMode('B') が呼ばれる", () => {
    // 【テスト目的】: 異なるレイアウトモードへの切り替えが動作すること 🟢
    render(<ToolBar />);

    // 【処理実行】: ボタン B をクリック
    fireEvent.click(screen.getByTestId('layout-btn-B'));

    // 【確認内容】: setLayoutMode が 'B' で呼ばれた
    expect(mockSetLayoutModeTB).toHaveBeenCalledWith('B');
  });
});
