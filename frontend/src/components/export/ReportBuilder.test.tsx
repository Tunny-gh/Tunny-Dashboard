/**
 * ReportBuilder テスト (TASK-1102)
 *
 * 【テスト対象】: ReportBuilder コンポーネント — HTMLレポート生成 UI
 * 【テスト方針】: exportStore を vi.hoisted + vi.mock でスタブし、UI の動作を検証
 */

import { describe, test, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import '@testing-library/jest-dom';

// -------------------------------------------------------------------------
// exportStore モック
// -------------------------------------------------------------------------

const {
  mockSetReportSections,
  mockGenerateHtmlReport,
  mockClearReportError,
} = vi.hoisted(() => ({
  mockSetReportSections: vi.fn(),
  mockGenerateHtmlReport: vi.fn().mockResolvedValue(undefined),
  mockClearReportError: vi.fn(),
}));

const mockStoreState = {
  reportSections: ['summary', 'pareto', 'pinned', 'history', 'cluster'] as const,
  isGeneratingReport: false,
  reportError: null as string | null,
  pinnedTrials: [],
  setReportSections: mockSetReportSections,
  generateHtmlReport: mockGenerateHtmlReport,
  clearReportError: mockClearReportError,
};

vi.mock('../../stores/exportStore', () => ({
  useExportStore: vi.fn(() => mockStoreState),
  ReportSection: {},
}));

import { ReportBuilder } from './ReportBuilder';

// -------------------------------------------------------------------------
// ユーティリティ
// -------------------------------------------------------------------------

function renderReportBuilder(paretoIndices = new Uint32Array([0, 1, 2])) {
  return render(<ReportBuilder paretoIndices={paretoIndices} />);
}

// -------------------------------------------------------------------------
// テスト
// -------------------------------------------------------------------------

describe('ReportBuilder — セクション表示', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockStoreState.isGeneratingReport = false;
    mockStoreState.reportError = null;
    mockStoreState.reportSections = ['summary', 'pareto', 'pinned', 'history', 'cluster'];
  });

  // TC-1102-R01: セクションリストが表示される
  test('TC-1102-R01: 全セクションのチェックボックスが表示される', () => {
    // 【テスト目的】: デフォルト状態で全セクションが表示されることを確認 🟢
    renderReportBuilder();

    // 【確認内容】: 全セクションのチェックボックスが存在すること
    expect(screen.getByTestId('section-checkbox-summary')).toBeInTheDocument();
    expect(screen.getByTestId('section-checkbox-pareto')).toBeInTheDocument();
    expect(screen.getByTestId('section-checkbox-pinned')).toBeInTheDocument();
    expect(screen.getByTestId('section-checkbox-history')).toBeInTheDocument();
    expect(screen.getByTestId('section-checkbox-cluster')).toBeInTheDocument();
  });

  // TC-1102-R02: セクションを無効化するとチェックが外れる
  test('TC-1102-R02: チェックボックスをクリックするとセクションが無効化される', () => {
    // 【テスト目的】: チェックボックス操作でセクションの有効/無効が切り替わることを確認 🟢
    renderReportBuilder();

    const checkbox = screen.getByTestId('section-checkbox-summary');
    expect(checkbox).toBeChecked();

    fireEvent.click(checkbox);

    // 【確認内容】: チェックが外れること
    expect(checkbox).not.toBeChecked();
  });

  // TC-1102-R03: レポートビルダーコンポーネントが表示される
  test('TC-1102-R03: report-builder が描画される', () => {
    // 【テスト目的】: コンポーネントが正しく描画されることを確認 🟢
    renderReportBuilder();
    expect(screen.getByTestId('report-builder')).toBeInTheDocument();
  });
});

describe('ReportBuilder — ダウンロードアクション', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockStoreState.isGeneratingReport = false;
    mockStoreState.reportError = null;
    mockStoreState.reportSections = ['summary', 'pareto', 'pinned', 'history', 'cluster'];
  });

  // TC-1102-R04: ダウンロードボタンが表示される
  test('TC-1102-R04: generate-report-btn が表示されている', () => {
    // 【テスト目的】: HTMLダウンロードボタンが存在することを確認 🟢
    renderReportBuilder();
    expect(screen.getByTestId('generate-report-btn')).toBeInTheDocument();
  });

  // TC-1102-R05: ボタンクリックで generateHtmlReport が呼ばれる
  test('TC-1102-R05: ダウンロードボタンクリックで generateHtmlReport が呼ばれる', () => {
    // 【テスト目的】: ボタン押下でレポート生成アクションが発火することを確認 🟢
    const paretoIndices = new Uint32Array([0, 1, 2]);
    renderReportBuilder(paretoIndices);

    fireEvent.click(screen.getByTestId('generate-report-btn'));

    // 【確認内容】: generateHtmlReport が呼ばれること
    expect(mockGenerateHtmlReport).toHaveBeenCalledOnce();
  });

  // TC-1102-R06: 生成中はスピナーが表示される
  test('TC-1102-R06: isGeneratingReport=true のときスピナーが表示される', () => {
    // 【テスト目的】: 生成中インジケーターが表示されることを確認 🟢
    mockStoreState.isGeneratingReport = true;
    renderReportBuilder();

    // 【確認内容】: スピナーが表示されること
    expect(screen.getByTestId('report-spinner')).toBeInTheDocument();
  });

  // TC-1102-R07: エラーが表示される
  test('TC-1102-R07: reportError があればエラーメッセージが表示される', () => {
    // 【テスト目的】: エラーメッセージが UI に反映されることを確認 🟢
    mockStoreState.reportError = 'テストエラーメッセージ';
    renderReportBuilder();

    // 【確認内容】: エラーメッセージが表示されること
    const errorEl = screen.getByTestId('report-error');
    expect(errorEl).toBeInTheDocument();
    expect(errorEl).toHaveTextContent('テストエラーメッセージ');
  });
});
