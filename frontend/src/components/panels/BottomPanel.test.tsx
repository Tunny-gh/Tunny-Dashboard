/**
 * BottomPanel テスト (TASK-402)
 *
 * 【テスト対象】: BottomPanel — 仮想スクロールテーブル（trial一覧）
 * 【テスト方針】: selectionStore / studyStore を vi.mock でモック
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';

// -------------------------------------------------------------------------
// selectionStore モック
// -------------------------------------------------------------------------

const { mockSetHighlight } = vi.hoisted(() => {
  const mockSetHighlight = vi.fn();
  return { mockSetHighlight };
});

vi.mock('../../stores/selectionStore', () => ({
  useSelectionStore: vi.fn().mockImplementation(
    (selector: (s: {
      selectedIndices: Uint32Array;
      highlighted: number | null;
      setHighlight: typeof mockSetHighlight;
    }) => unknown) =>
      selector({
        selectedIndices: new Uint32Array([0, 1, 2]),
        highlighted: null,
        setHighlight: mockSetHighlight,
      }),
  ),
}));

// -------------------------------------------------------------------------
// studyStore モック — trial データ
// -------------------------------------------------------------------------

vi.mock('../../stores/studyStore', () => ({
  useStudyStore: vi.fn().mockImplementation(
    (selector: (s: {
      currentStudy: {
        paramNames: string[];
        objectiveNames: string[];
      } | null;
    }) => unknown) =>
      selector({
        currentStudy: {
          paramNames: ['x1'],
          objectiveNames: ['obj1'],
        },
      }),
  ),
}));

import { BottomPanel } from './BottomPanel';
import { useStudyStore } from '../../stores/studyStore';

// -------------------------------------------------------------------------
// 正常系
// -------------------------------------------------------------------------

describe('BottomPanel — 正常系', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  // TC-402-05: BottomPanel がレンダリングされる
  test('TC-402-05: BottomPanel がエラーなくレンダリングされる', () => {
    // 【テスト目的】: BottomPanel が正常にレンダリングできること 🟢
    expect(() => render(<BottomPanel />)).not.toThrow();
  });

  // TC-402-06: テーブルヘッダーが表示される
  test('TC-402-06: テーブルに trial_id ヘッダーが表示される', () => {
    // 【テスト目的】: テーブルの列ヘッダーが正しく表示されること 🟢
    render(<BottomPanel />);
    expect(screen.getByText('trial_id')).toBeInTheDocument();
  });

  // TC-402-07: 行クリックで setHighlight が呼ばれる
  test('TC-402-07: テーブル行クリックで selectionStore.setHighlight が呼ばれる', () => {
    // 【テスト目的】: 行クリックがハイライト操作に連携されること 🟢
    render(<BottomPanel />);

    // 【処理実行】: 最初の行をクリック
    const firstRow = screen.getByTestId('trial-row-0');
    fireEvent.click(firstRow);

    // 【確認内容】: setHighlight が trial index 0 で呼ばれた
    expect(mockSetHighlight).toHaveBeenCalledWith(0);
  });
});

// -------------------------------------------------------------------------
// 異常系
// -------------------------------------------------------------------------

describe('BottomPanel — 異常系', () => {
  afterEach(() => {
    cleanup();
  });

  // TC-402-E02: currentStudy=null のとき空状態UIを表示
  test('TC-402-E02: currentStudy=null のとき「データが読み込まれていません」を表示する', () => {
    // 【テスト目的】: Study なし時に適切な空状態UIが表示されること 🟢
    (useStudyStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (s: { currentStudy: null }) => unknown) => selector({ currentStudy: null }),
    );

    render(<BottomPanel />);
    expect(screen.getByText('データが読み込まれていません')).toBeInTheDocument();
  });
});
