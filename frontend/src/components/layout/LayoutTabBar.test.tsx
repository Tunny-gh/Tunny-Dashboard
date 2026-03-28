/**
 * LayoutTabBar テスト (TASK-001)
 *
 * 【テスト対象】: LayoutTabBar — タブ型レイアウト切替コンポーネント
 * 【テスト方針】: layoutStore を vi.mock でモック
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';

// -------------------------------------------------------------------------
// layoutStore モック
// -------------------------------------------------------------------------

const { mockSetLayoutMode, mockSetFreeModeLayout } = vi.hoisted(() => ({
  mockSetLayoutMode: vi.fn(),
  mockSetFreeModeLayout: vi.fn(),
}));

vi.mock('../../stores/layoutStore', () => ({
  useLayoutStore: vi.fn().mockImplementation(
    (selector: (s: {
      layoutMode: string;
      setLayoutMode: typeof mockSetLayoutMode;
      setFreeModeLayout: typeof mockSetFreeModeLayout;
    }) => unknown) =>
      selector({ layoutMode: 'A', setLayoutMode: mockSetLayoutMode, setFreeModeLayout: mockSetFreeModeLayout }),
  ),
}));

import { LayoutTabBar } from './LayoutTabBar';
import { useLayoutStore } from '../../stores/layoutStore';

// -------------------------------------------------------------------------
// 正常系
// -------------------------------------------------------------------------

describe('LayoutTabBar — 正常系', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useLayoutStore).mockImplementation((selector) =>
      selector({ layoutMode: 'A', setLayoutMode: mockSetLayoutMode, setFreeModeLayout: mockSetFreeModeLayout }),
    );
  });

  afterEach(() => {
    cleanup();
  });

  // TC-LT-01: layout-tab-bar コンテナが DOM に存在する
  test('TC-LT-01: data-testid="layout-tab-bar" が存在する', () => {
    // 【テスト目的】: コンテナ要素が正しくレンダリングされること
    render(<LayoutTabBar />);
    expect(screen.getByTestId('layout-tab-bar')).toBeInTheDocument();
  });

  // TC-LT-02: 4つのタブ（A〜D）が存在する
  test('TC-LT-02: layout-tab-A 〜 layout-tab-D が全て存在する', () => {
    // 【テスト目的】: 4つのタブが全て描画されること
    render(<LayoutTabBar />);
    expect(screen.getByTestId('layout-tab-A')).toBeInTheDocument();
    expect(screen.getByTestId('layout-tab-B')).toBeInTheDocument();
    expect(screen.getByTestId('layout-tab-C')).toBeInTheDocument();
    expect(screen.getByTestId('layout-tab-D')).toBeInTheDocument();
  });

  // TC-LT-03: 各タブのラベルが説明的な日本語ラベルであること
  test('TC-LT-03: 各タブのラベルが「4分割」「左大」「縦並び」「フリー」であること', () => {
    // 【テスト目的】: REQ-002 — 不透明な A/B/C/D ではなく説明的ラベルが表示されること
    render(<LayoutTabBar />);
    expect(screen.getByTestId('layout-tab-A')).toHaveTextContent('4分割');
    expect(screen.getByTestId('layout-tab-B')).toHaveTextContent('左大');
    expect(screen.getByTestId('layout-tab-C')).toHaveTextContent('縦並び');
    expect(screen.getByTestId('layout-tab-D')).toHaveTextContent('フリー');
  });

  // TC-LT-04: layoutMode=B のとき layout-tab-B が aria-selected="true"
  test('TC-LT-04: layoutMode=B のとき layout-tab-B の aria-selected が "true" で他は "false"', () => {
    // 【テスト目的】: REQ-105 — アクティブタブが視覚的に区別されること
    vi.mocked(useLayoutStore).mockImplementation((selector) =>
      selector({ layoutMode: 'B', setLayoutMode: mockSetLayoutMode, setFreeModeLayout: mockSetFreeModeLayout }),
    );
    render(<LayoutTabBar />);
    expect(screen.getByTestId('layout-tab-B')).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByTestId('layout-tab-A')).toHaveAttribute('aria-selected', 'false');
    expect(screen.getByTestId('layout-tab-C')).toHaveAttribute('aria-selected', 'false');
    expect(screen.getByTestId('layout-tab-D')).toHaveAttribute('aria-selected', 'false');
  });

  // TC-LT-05: プリセットタブ（A）クリックで setLayoutMode + setFreeModeLayout が呼ばれる
  test('TC-LT-05: layout-tab-A クリックで setLayoutMode("A") と setFreeModeLayout が呼ばれる', () => {
    // 【テスト目的】: REQ-101 — タブクリックでモード切替とレイアウト適用が同時に行われること
    // 【前提条件】: layoutMode = 'B' (A は非アクティブなのでクリック可能)
    vi.mocked(useLayoutStore).mockImplementation((selector) =>
      selector({ layoutMode: 'B', setLayoutMode: mockSetLayoutMode, setFreeModeLayout: mockSetFreeModeLayout }),
    );
    render(<LayoutTabBar />);
    fireEvent.click(screen.getByTestId('layout-tab-A'));
    expect(mockSetLayoutMode).toHaveBeenCalledOnce();
    expect(mockSetLayoutMode).toHaveBeenCalledWith('A');
    expect(mockSetFreeModeLayout).toHaveBeenCalledOnce();
  });

  // TC-LT-06: フリータブ（D）クリックで setLayoutMode のみ呼ばれる
  test('TC-LT-06: layout-tab-D クリックで setLayoutMode("D") が呼ばれ setFreeModeLayout は呼ばれない', () => {
    // 【テスト目的】: REQ-104 — フリーモード切替では freeModeLayout を変更しないこと
    // 【前提条件】: layoutMode = 'A' (D は非アクティブ)
    render(<LayoutTabBar />);
    fireEvent.click(screen.getByTestId('layout-tab-D'));
    expect(mockSetLayoutMode).toHaveBeenCalledOnce();
    expect(mockSetLayoutMode).toHaveBeenCalledWith('D');
    expect(mockSetFreeModeLayout).not.toHaveBeenCalled();
  });

  // TC-LT-07: アクティブタブ再クリックで何も呼ばれない（べき等）
  test('TC-LT-07: アクティブタブクリックで setLayoutMode も setFreeModeLayout も呼ばれない', () => {
    // 【テスト目的】: REQ-106 — アクティブタブ再クリックはべき等であること
    // 【前提条件】: layoutMode = 'A' (A がアクティブ)
    render(<LayoutTabBar />);
    fireEvent.click(screen.getByTestId('layout-tab-A'));
    expect(mockSetLayoutMode).not.toHaveBeenCalled();
    expect(mockSetFreeModeLayout).not.toHaveBeenCalled();
  });
});
