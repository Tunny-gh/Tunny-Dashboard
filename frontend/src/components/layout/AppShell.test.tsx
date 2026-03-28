/**
 * AppShell テスト (TASK-401)
 *
 * 【テスト対象】: AppShell — 4エリア CSS Grid レイアウトのアプリ骨格
 * 【テスト方針】: studyStore / layoutStore を vi.mock でモック
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';

// -------------------------------------------------------------------------
// studyStore モック
// -------------------------------------------------------------------------

const { mockLoadJournal } = vi.hoisted(() => {
  const mockLoadJournal = vi.fn().mockResolvedValue(undefined);
  return { mockLoadJournal };
});

vi.mock('../../stores/studyStore', () => ({
  useStudyStore: vi.fn().mockImplementation((selector: (s: { loadJournal: typeof mockLoadJournal; isLoading: boolean }) => unknown) =>
    selector({ loadJournal: mockLoadJournal, isLoading: false }),
  ),
}));

// -------------------------------------------------------------------------
// layoutStore モック
// -------------------------------------------------------------------------

const { mockSetLayoutMode } = vi.hoisted(() => {
  const mockSetLayoutMode = vi.fn();
  return { mockSetLayoutMode };
});

vi.mock('../../stores/layoutStore', () => ({
  useLayoutStore: vi.fn().mockImplementation((selector: (s: { layoutMode: string; setLayoutMode: typeof mockSetLayoutMode }) => unknown) =>
    selector({ layoutMode: 'A', setLayoutMode: mockSetLayoutMode }),
  ),
}));

import { AppShell } from './AppShell';
import { useStudyStore } from '../../stores/studyStore';

// -------------------------------------------------------------------------
// 正常系
// -------------------------------------------------------------------------

describe('AppShell — 正常系', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  // TC-401-01: AppShell が4エリアでレンダリングされる
  test('TC-401-01: AppShell がエラーなくレンダリングされる', () => {
    // 【テスト目的】: AppShell が正常にレンダリングできること 🟢
    expect(() => render(<AppShell />)).not.toThrow();
  });

  test('TC-401-01b: AppShell の data-layout 属性に layoutMode が反映される', () => {
    // 【テスト目的】: レイアウトモードが DOM 属性として反映されること 🟢
    render(<AppShell />);
    const shell = screen.getByTestId('app-shell');
    expect(shell).toHaveAttribute('data-layout', 'A');
  });

  // TC-401-02: ファイルドロップで loadJournal が呼ばれる
  test('TC-401-02: ファイルドロップで studyStore.loadJournal が呼ばれる', () => {
    // 【テスト目的】: drag & drop でファイルが読み込まれること 🟢
    render(<AppShell />);
    const shell = screen.getByTestId('app-shell');

    // 【テストデータ準備】: ドロップ用のファイルモック
    const file = new File(['content'], 'journal.log', { type: 'text/plain' });
    const dataTransfer = { files: [file] };

    // 【処理実行】: dragOver + drop イベントをシミュレート
    fireEvent.dragOver(shell, { preventDefault: vi.fn() });
    fireEvent.drop(shell, { dataTransfer });

    // 【確認内容】: loadJournal が呼ばれた
    expect(mockLoadJournal).toHaveBeenCalledOnce();
    expect(mockLoadJournal).toHaveBeenCalledWith(file);
  });
});

// -------------------------------------------------------------------------
// 異常系
// -------------------------------------------------------------------------

describe('AppShell — 異常系', () => {
  afterEach(() => {
    cleanup();
  });

  // TC-401-E01: isLoading=true のときローディング表示
  test('TC-401-E01: isLoading=true のとき Loading インジケータが表示される', () => {
    // 【テスト目的】: 読み込み中の状態が正しく表示されること 🟢

    // 【前提条件】: isLoading=true の state をモックで設定
    // Zustand の複雑な型を回避するため unknown 経由でキャスト
    (useStudyStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (s: { loadJournal: typeof mockLoadJournal; isLoading: boolean }) => unknown) =>
        selector({ loadJournal: mockLoadJournal, isLoading: true }),
    );

    // 【処理実行】
    render(<AppShell />);

    // 【確認内容】: Loading インジケータが表示されている
    expect(screen.getByTestId('loading-indicator')).toBeInTheDocument();
  });
});
