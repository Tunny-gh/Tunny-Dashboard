/**
 * StudyComparisonPanel テスト (TASK-1401)
 *
 * 【テスト対象】: StudyComparisonPanel コンポーネント
 * 【テスト方針】: comparisonStore と studyStore を vi.mock でスタブ
 */

import { describe, test, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import '@testing-library/jest-dom';

// -------------------------------------------------------------------------
// comparisonStore モック
// -------------------------------------------------------------------------

const mockSetComparisonStudyIds = vi.fn();
const mockSetMode = vi.fn();

const mockComparisonState = {
  comparisonStudyIds: [] as number[],
  mode: 'overlay' as const,
  results: [] as import('../../types').StudyComparisonResult[],
  isComputing: false,
  setComparisonStudyIds: mockSetComparisonStudyIds,
  setMode: mockSetMode,
  computeResults: vi.fn(),
  reset: vi.fn(),
};

vi.mock('../../stores/comparisonStore', () => ({
  useComparisonStore: vi.fn(() => mockComparisonState),
  canComparePareto: (a: import('../../types').Study, b: import('../../types').Study) => {
    return (
      a.directions.length === b.directions.length &&
      a.directions.every((d: string, i: number) => d === b.directions[i])
    );
  },
  COMPARISON_COLORS: ['#ef4444', '#22c55e', '#a855f7', '#f59e0b'],
  MAX_COMPARISON_STUDIES: 4,
}));

// -------------------------------------------------------------------------
// studyStore モック
// -------------------------------------------------------------------------

const mockStudyState = {
  currentStudy: {
    studyId: 1,
    name: 'main-study',
    directions: ['minimize', 'minimize'] as import('../../types').OptimizationDirection[],
    completedTrials: 50,
    totalTrials: 50,
    paramNames: ['x', 'y'],
    objectiveNames: ['obj1', 'obj2'],
    userAttrNames: [],
    hasConstraints: false,
  },
  allStudies: [
    {
      studyId: 1,
      name: 'main-study',
      directions: ['minimize', 'minimize'] as import('../../types').OptimizationDirection[],
      completedTrials: 50,
      totalTrials: 50,
      paramNames: ['x', 'y'],
      objectiveNames: ['obj1', 'obj2'],
      userAttrNames: [],
      hasConstraints: false,
    },
    {
      studyId: 2,
      name: 'study-2',
      directions: ['minimize', 'minimize'] as import('../../types').OptimizationDirection[],
      completedTrials: 30,
      totalTrials: 30,
      paramNames: ['x', 'y'],
      objectiveNames: ['obj1', 'obj2'],
      userAttrNames: [],
      hasConstraints: false,
    },
    {
      studyId: 3,
      name: 'study-3',
      directions: ['minimize', 'minimize'] as import('../../types').OptimizationDirection[],
      completedTrials: 20,
      totalTrials: 20,
      paramNames: ['x', 'y'],
      objectiveNames: ['obj1', 'obj2'],
      userAttrNames: [],
      hasConstraints: false,
    },
    {
      studyId: 4,
      name: 'study-incompatible',
      directions: ['minimize'] as import('../../types').OptimizationDirection[],  // 目的数不一致
      completedTrials: 15,
      totalTrials: 15,
      paramNames: ['x'],
      objectiveNames: ['obj1'],
      userAttrNames: [],
      hasConstraints: false,
    },
  ],
};

vi.mock('../../stores/studyStore', () => ({
  useStudyStore: vi.fn(() => mockStudyState),
}));

import { useStudyStore } from '../../stores/studyStore';

import { StudyComparisonPanel } from './StudyComparisonPanel';

// -------------------------------------------------------------------------
// テスト
// -------------------------------------------------------------------------

describe('StudyComparisonPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockComparisonState.comparisonStudyIds = [];
    mockComparisonState.mode = 'overlay';
    mockComparisonState.results = [];
  });

  // TC-1401-P01: 目的数不一致Study選択時に警告アイコンが表示される
  test('TC-1401-P01: 目的数不一致Studyに警告アイコンが表示される', () => {
    // 【テスト目的】: 目的数不一致Study選択時に警告が表示されることを確認 🟢 REQ-121
    render(<StudyComparisonPanel />);
    // 【確認内容】: study-4（目的数不一致）に警告アイコンが表示される
    expect(screen.getByTestId('comparison-warning-4')).toBeInTheDocument();
    // 【確認内容】: study-2（互換）に警告アイコンが表示されないこと
    expect(screen.queryByTestId('comparison-warning-2')).not.toBeInTheDocument();
  });

  // TC-1401-P02: 3Study選択時に3つの色バッジが表示される
  test('TC-1401-P02: 3Study選択時に3つの色バッジが表示される', () => {
    // 【テスト目的】: 3Study重畳表示用に色バッジが正しく表示されることを確認 🟢 REQ-120
    mockComparisonState.comparisonStudyIds = [2, 3, 4];
    render(<StudyComparisonPanel />);
    // 【確認内容】: 選択された3Study分の色バッジが表示される
    expect(screen.getByTestId('comparison-color-badge-2')).toBeInTheDocument();
    expect(screen.getByTestId('comparison-color-badge-3')).toBeInTheDocument();
    expect(screen.getByTestId('comparison-color-badge-4')).toBeInTheDocument();
  });

  // TC-1401-P03: 比較モード切替ボタンが表示される
  test('TC-1401-P03: 比較モード切替ボタン（重畳/並列/差分）が表示される', () => {
    // 【テスト目的】: 比較モード切替UIが存在することを確認 🟢 REQ-123
    render(<StudyComparisonPanel />);
    expect(screen.getByTestId('comparison-mode-overlay')).toBeInTheDocument();
    expect(screen.getByTestId('comparison-mode-side-by-side')).toBeInTheDocument();
    expect(screen.getByTestId('comparison-mode-diff')).toBeInTheDocument();
  });

  // TC-1401-P04: MainStudyのチェックボックスは表示されない（比較対象外）
  test('TC-1401-P04: MainStudyは比較Study選択肢に表示されない', () => {
    // 【テスト目的】: MainStudy自身が比較対象として選択不可であることを確認 🟢
    render(<StudyComparisonPanel />);
    // 【確認内容】: main-study (studyId=1) のチェックボックスが存在しないこと
    expect(screen.queryByTestId('comparison-study-checkbox-1')).not.toBeInTheDocument();
    // 【確認内容】: 他のStudyのチェックボックスは存在すること
    expect(screen.getByTestId('comparison-study-checkbox-2')).toBeInTheDocument();
  });

  // TC-1401-P05: チェックボックスONでsetComparisonStudyIdsが呼ばれる
  test('TC-1401-P05: チェックボックスONでsetComparisonStudyIdsが呼ばれる', () => {
    // 【テスト目的】: Study選択操作が正しくStoreに反映されることを確認 🟢
    render(<StudyComparisonPanel />);
    fireEvent.click(screen.getByTestId('comparison-study-checkbox-2'));
    // 【確認内容】: setComparisonStudyIds が呼ばれること
    expect(mockSetComparisonStudyIds).toHaveBeenCalledWith([2]);
  });

  // TC-1401-P06: モードボタンクリックでsetModeが呼ばれる
  test('TC-1401-P06: 比較モードボタンクリックでsetModeが呼ばれる', () => {
    // 【テスト目的】: 比較モード切替がStoreに正しく反映されることを確認 🟢 REQ-123
    render(<StudyComparisonPanel />);
    fireEvent.click(screen.getByTestId('comparison-mode-side-by-side'));
    // 【確認内容】: setMode('side-by-side') が呼ばれること
    expect(mockSetMode).toHaveBeenCalledWith('side-by-side');
  });

  // TC-1401-P07: currentStudyがない場合はパネルが非表示
  test('TC-1401-P07: currentStudy=nullのときStudyComparisonPanelは非表示', () => {
    // 【テスト目的】: Study未選択時にパネルが非表示であることを確認 🟢
    (useStudyStore as ReturnType<typeof vi.fn>).mockReturnValue({
      ...mockStudyState,
      currentStudy: null,
    });
    render(<StudyComparisonPanel />);
    expect(screen.queryByTestId('study-comparison-panel')).not.toBeInTheDocument();
  });
});
