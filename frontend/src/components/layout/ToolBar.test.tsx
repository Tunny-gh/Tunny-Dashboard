/**
 * ToolBar テスト (TASK-401 / TASK-002)
 *
 * 【テスト対象】: ToolBar — ファイル読込・Study選択・レイアウト切替 UI
 * 【テスト方針】: studyStore / layoutStore を vi.mock でモック、LayoutTabBar はモック
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';

// -------------------------------------------------------------------------
// studyStore / layoutStore モック
// -------------------------------------------------------------------------

const { mockLoadJournalTB, mockStartLive, mockStopLive } = vi.hoisted(() => {
  const mockLoadJournalTB = vi.fn().mockResolvedValue(undefined);
  const mockStartLive = vi.fn();
  const mockStopLive = vi.fn();
  return { mockLoadJournalTB, mockStartLive, mockStopLive };
});

vi.mock('../../stores/studyStore', () => ({
  useStudyStore: vi.fn().mockImplementation((selector: (s: { loadJournal: typeof mockLoadJournalTB; isLoading: boolean }) => unknown) =>
    selector({ loadJournal: mockLoadJournalTB, isLoading: false }),
  ),
}));

vi.mock('../../stores/liveUpdateStore', () => ({
  useLiveUpdateStore: vi.fn().mockImplementation((selector: (s: {
    isLive: boolean;
    isSupported: boolean;
    startLive: typeof mockStartLive;
    stopLive: typeof mockStopLive;
  }) => unknown) =>
    selector({ isLive: false, isSupported: true, startLive: mockStartLive, stopLive: mockStopLive }),
  ),
}));

// LayoutTabBar をモックして ToolBar の単体テストを分離する
vi.mock('./LayoutTabBar', () => ({ LayoutTabBar: () => <div data-testid="layout-tab-bar" /> }));

import { ToolBar } from './ToolBar';
import { useLiveUpdateStore } from '../../stores/liveUpdateStore';

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

  // TC-401-07: LayoutTabBar が ToolBar 内に存在する（TASK-002）
  test('TC-401-07: data-testid="layout-tab-bar" が ToolBar 内に存在する', () => {
    // 【テスト目的】: REQ-001 — LayoutTabBar が ToolBar に統合されていること
    render(<ToolBar />);
    expect(screen.getByTestId('layout-tab-bar')).toBeInTheDocument();
  });

  // TC-401-08: 旧レイアウトボタン（layout-btn-*）が存在しない（TASK-002）
  test('TC-401-08: layout-btn-A 〜 layout-btn-D が DOM に存在しない', () => {
    // 【テスト目的】: REQ-405 — 旧ボタン群が削除されていること
    render(<ToolBar />);
    expect(screen.queryByTestId('layout-btn-A')).not.toBeInTheDocument();
    expect(screen.queryByTestId('layout-btn-B')).not.toBeInTheDocument();
    expect(screen.queryByTestId('layout-btn-C')).not.toBeInTheDocument();
    expect(screen.queryByTestId('layout-btn-D')).not.toBeInTheDocument();
  });

  // TC-401-10: ライブ更新ボタンが DOM に存在する
  test('TC-401-10: ライブ更新ボタンが data-testid="live-update-btn" で存在する', () => {
    // 【テスト目的】: REQ-104-G — ToolBar にライブ更新ボタンが表示される
    render(<ToolBar />);
    expect(screen.getByTestId('live-update-btn')).toBeInTheDocument();
  });

  // TC-401-11: isLive=false のとき startLive が呼ばれる
  test('TC-401-11: isLive=false のときボタンをクリックすると startLive が呼ばれる', () => {
    // 【テスト目的】: REQ-104-K — isLive=false 時のクリックで startLive() を呼ぶ
    render(<ToolBar />);
    fireEvent.click(screen.getByTestId('live-update-btn'));
    expect(mockStartLive).toHaveBeenCalledOnce();
    expect(mockStopLive).not.toHaveBeenCalled();
  });

  // TC-401-12: isLive=true のとき stopLive が呼ばれる
  test('TC-401-12: isLive=true のときボタンをクリックすると stopLive が呼ばれる', () => {
    // 【テスト目的】: REQ-104-J — isLive=true 時のクリックで stopLive() を呼ぶ
    vi.mocked(useLiveUpdateStore).mockImplementation((selector) =>
      selector({ isLive: true, isSupported: true, startLive: mockStartLive, stopLive: mockStopLive }),
    );
    render(<ToolBar />);
    fireEvent.click(screen.getByTestId('live-update-btn'));
    expect(mockStopLive).toHaveBeenCalledOnce();
    expect(mockStartLive).not.toHaveBeenCalled();
  });

  // TC-401-13: isSupported=false のときボタンが disabled になる
  test('TC-401-13: isSupported=false のときライブ更新ボタンが disabled になる', () => {
    // 【テスト目的】: REQ-104-I — isSupported=false 時はボタンを disabled にする
    vi.mocked(useLiveUpdateStore).mockImplementation((selector) =>
      selector({ isLive: false, isSupported: false, startLive: mockStartLive, stopLive: mockStopLive }),
    );
    render(<ToolBar />);
    expect(screen.getByTestId('live-update-btn')).toBeDisabled();
  });
});
