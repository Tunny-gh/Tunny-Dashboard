/**
 * ExportStore テスト (TASK-1101)
 *
 * 【テスト対象】: ExportStore — CSV エクスポート・ピン留め管理
 * 【テスト方針】: WasmLoader を vi.mock でモック、DOM API をスタブ
 */

import { describe, test, expect, beforeEach, vi } from 'vitest';

// -------------------------------------------------------------------------
// WasmLoader モック
// -------------------------------------------------------------------------

const { mockSerializeCsv, mockComputeReportStats } = vi.hoisted(() => {
  const mockSerializeCsv = vi.fn();
  const mockComputeReportStats = vi.fn().mockReturnValue('{"x1":{"min":1,"max":3,"mean":2,"std":1,"count":2}}');
  return { mockSerializeCsv, mockComputeReportStats };
});

vi.mock('../wasm/wasmLoader', () => ({
  WasmLoader: {
    getInstance: vi.fn().mockResolvedValue({
      serializeCsv: mockSerializeCsv,
      computeReportStats: mockComputeReportStats,
    }),
  },
}));

// -------------------------------------------------------------------------
// selectionStore / layoutStore モック（saveSession / loadSessionFromJson 用）
// -------------------------------------------------------------------------

const mockBrushSelect = vi.fn();
const mockAddAxisFilter = vi.fn();
const mockClearSelection = vi.fn();
const mockSetColorMode = vi.fn();
const mockSetLayoutMode = vi.fn();
const mockLoadLayout = vi.fn();

vi.mock('./selectionStore', () => ({
  useSelectionStore: {
    getState: vi.fn(() => ({
      selectedIndices: new Uint32Array([0, 1, 2]),
      filterRanges: { x1: { min: 0, max: 1 } },
      colorMode: 'objective',
      brushSelect: mockBrushSelect,
      addAxisFilter: mockAddAxisFilter,
      clearSelection: mockClearSelection,
      setColorMode: mockSetColorMode,
    })),
  },
}));

vi.mock('./layoutStore', () => ({
  useLayoutStore: {
    getState: vi.fn(() => ({
      layoutMode: 'A',
      visibleCharts: new Set(['pareto-front', 'history']),
      freeModeLayout: null,
      setLayoutMode: mockSetLayoutMode,
      loadLayout: mockLoadLayout,
    })),
  },
}));

// -------------------------------------------------------------------------
// DOM API スタブ（ダウンロードフロー用）
// -------------------------------------------------------------------------

// URL.createObjectURL / revokeObjectURL を jsdom でスタブ
Object.defineProperty(globalThis, 'URL', {
  value: {
    createObjectURL: vi.fn(() => 'blob:mock-url'),
    revokeObjectURL: vi.fn(),
  },
  writable: true,
});

// document.body.appendChild / removeChild スタブ（link.click 含む）
const mockClick = vi.fn();
const mockAppendChild = vi.fn();
const mockRemoveChild = vi.fn();
vi.spyOn(document.body, 'appendChild').mockImplementation(mockAppendChild);
vi.spyOn(document.body, 'removeChild').mockImplementation(mockRemoveChild);

// HTMLAnchorElement.click() が呼ばれないよう createElement をスタブ
const origCreateElement = document.createElement.bind(document);
vi.spyOn(document, 'createElement').mockImplementation((tag: string) => {
  const el = origCreateElement(tag);
  if (tag === 'a') {
    Object.defineProperty(el, 'click', { value: mockClick, writable: true });
  }
  return el;
});

import { useExportStore, MAX_PINS } from './exportStore';

// -------------------------------------------------------------------------
// 正常系
// -------------------------------------------------------------------------

describe('ExportStore — 正常系', () => {
  beforeEach(() => {
    // 【状態リセット】: 各テスト前に Store を初期状態に戻す
    useExportStore.setState({
      csvTarget: 'all',
      selectedColumns: [],
      isExporting: false,
      exportError: null,
      pinnedTrials: [],
      pinError: null,
    });
    vi.clearAllMocks();
    mockSerializeCsv.mockReturnValue('trial_id,x1\n0,1.5\n');
  });

  // TC-1101-11: setCsvTarget で csvTarget が更新される
  test('TC-1101-11: setCsvTarget で csvTarget が "selected" に変更される', () => {
    // 【テスト目的】: CSV 対象設定が Store に反映されることを確認 🟢
    useExportStore.getState().setCsvTarget('selected');
    // 【確認内容】: csvTarget が 'selected' に更新されること
    expect(useExportStore.getState().csvTarget).toBe('selected');
  });

  // TC-1101-12: exportCsv で serialize_csv が呼ばれる
  test('TC-1101-12: exportCsv で WasmLoader.serialize_csv が呼ばれる', async () => {
    // 【テスト目的】: CSV エクスポート実行時に WASM 関数が呼ばれることを確認 🟢
    await useExportStore.getState().exportCsv(new Uint32Array([0, 1, 2]));
    // 【確認内容】: serialize_csv が呼ばれること
    expect(mockSerializeCsv).toHaveBeenCalledOnce();
  });

  // TC-1101-13: exportCsv 完了後 isExporting が false に戻る
  test('TC-1101-13: exportCsv 完了後 isExporting が false に戻る', async () => {
    // 【テスト目的】: エクスポート完了後にフラグがリセットされることを確認 🟢
    await useExportStore.getState().exportCsv(new Uint32Array([0]));
    // 【確認内容】: isExporting が false
    expect(useExportStore.getState().isExporting).toBe(false);
  });

  // TC-1101-14: pinTrial でピン留めが追加される
  test('TC-1101-14: pinTrial でピン留めが追加される', () => {
    // 【テスト目的】: pinTrial が pinnedTrials に1件追加されることを確認 🟢
    useExportStore.getState().pinTrial(0, 100);
    // 【確認内容】: pinnedTrials に1件追加されること
    const { pinnedTrials } = useExportStore.getState();
    expect(pinnedTrials).toHaveLength(1);
    expect(pinnedTrials[0].trialId).toBe(100);
  });

  // TC-1101-15: unpinTrial でピン留めが削除される
  test('TC-1101-15: unpinTrial でピン留めが削除される', () => {
    // 【テスト目的】: unpinTrial が指定インデックスのピン留めを削除することを確認 🟢
    useExportStore.getState().pinTrial(0, 100);
    useExportStore.getState().unpinTrial(0);
    // 【確認内容】: pinnedTrials が空になること
    expect(useExportStore.getState().pinnedTrials).toHaveLength(0);
  });

  // TC-1101-16: updatePinMemo でメモが更新される
  test('TC-1101-16: updatePinMemo でピン留めメモが更新される', () => {
    // 【テスト目的】: ピン留めメモ更新が反映されることを確認 🟢
    useExportStore.getState().pinTrial(0, 100);
    useExportStore.getState().updatePinMemo(0, 'テストメモ');
    // 【確認内容】: memo が 'テストメモ' に更新されること
    const pin = useExportStore.getState().pinnedTrials[0];
    expect(pin.memo).toBe('テストメモ');
  });

  // TC-1101-17: 同じインデックスを重複ピン留めしても1件のまま
  test('TC-1101-17: 同じインデックスを重複ピン留めしても1件のまま', () => {
    // 【テスト目的】: 重複ピン留めが無視されることを確認 🟢
    useExportStore.getState().pinTrial(0, 100);
    useExportStore.getState().pinTrial(0, 100);
    // 【確認内容】: pinnedTrials は1件のまま
    expect(useExportStore.getState().pinnedTrials).toHaveLength(1);
  });
});

// -------------------------------------------------------------------------
// 異常系
// -------------------------------------------------------------------------

describe('ExportStore — 異常系', () => {
  beforeEach(() => {
    useExportStore.setState({
      csvTarget: 'all',
      selectedColumns: [],
      isExporting: false,
      exportError: null,
      pinnedTrials: [],
      pinError: null,
    });
    vi.clearAllMocks();
    mockSerializeCsv.mockReturnValue('trial_id\n');
  });

  // TC-1101-E01: 空の indices で exportCsv すると exportError がセットされる
  test('TC-1101-E01: 空 indices で exportCsv → exportError「対象データがありません」', async () => {
    // 【テスト目的】: 選択0件時に適切なエラーが表示されることを確認 🟢
    await useExportStore.getState().exportCsv(new Uint32Array([]));
    // 【確認内容】: exportError が「対象データがありません」
    expect(useExportStore.getState().exportError).toBe('対象データがありません');
    // 【確認内容】: WASM は呼ばれないこと
    expect(mockSerializeCsv).not.toHaveBeenCalled();
  });

  // TC-1101-E02: MAX_PINS 超過時に pinError がセットされる
  test(`TC-1101-E02: ピン留め${MAX_PINS}件超過で pinError が表示される`, () => {
    // 【テスト目的】: ピン留め上限超過時にエラーメッセージが表示されることを確認 🟢
    // MAX_PINS 件追加
    for (let i = 0; i < MAX_PINS; i++) {
      useExportStore.getState().pinTrial(i, i);
    }
    // 【処理実行】: MAX_PINS + 1 件目のピン留み試行
    useExportStore.getState().pinTrial(MAX_PINS, MAX_PINS);
    // 【確認内容】: pinError がセットされること
    expect(useExportStore.getState().pinError).toMatch(/上限/);
    // 【確認内容】: pinnedTrials は MAX_PINS 件のまま
    expect(useExportStore.getState().pinnedTrials).toHaveLength(MAX_PINS);
  });
});

// -------------------------------------------------------------------------
// TASK-1102: generateHtmlReport テスト
// -------------------------------------------------------------------------

describe('ExportStore — generateHtmlReport (TASK-1102)', () => {
  beforeEach(() => {
    useExportStore.setState({
      csvTarget: 'all',
      selectedColumns: [],
      isExporting: false,
      exportError: null,
      pinnedTrials: [],
      pinError: null,
      reportSections: ['summary', 'pareto', 'pinned', 'history', 'cluster'],
      isGeneratingReport: false,
      reportError: null,
    });
    vi.clearAllMocks();
    mockComputeReportStats.mockReturnValue('{"x1":{"min":1,"max":3,"mean":2,"std":1,"count":2}}');
  });

  // TC-1102-S01: generateHtmlReport で isGeneratingReport が変化する
  test('TC-1102-S01: generateHtmlReport 完了後 isGeneratingReport が false になる', async () => {
    // 【テスト目的】: レポート生成フラグが完了後に false に戻ることを確認 🟢
    await useExportStore.getState().generateHtmlReport(new Uint32Array([0, 1]));
    // 【確認内容】: isGeneratingReport が false
    expect(useExportStore.getState().isGeneratingReport).toBe(false);
  });

  // TC-1102-S02: compute_report_stats が呼ばれる
  test('TC-1102-S02: generateHtmlReport で compute_report_stats が呼ばれる', async () => {
    // 【テスト目的】: WASM 統計関数が呼ばれることを確認 🟢 REQ-154
    await useExportStore.getState().generateHtmlReport(new Uint32Array([0, 1]));
    // 【確認内容】: compute_report_stats が呼ばれること
    expect(mockComputeReportStats).toHaveBeenCalledOnce();
  });

  // TC-1102-S03: ダウンロードリンクが作成される
  test('TC-1102-S03: generateHtmlReport でダウンロードリンクが作成される', async () => {
    // 【テスト目的】: <a download> による HTML ダウンロードが発火することを確認 🟢 REQ-155
    await useExportStore.getState().generateHtmlReport(new Uint32Array([0, 1]));
    // 【確認内容】: Blob URL が作成されること
    expect(URL.createObjectURL).toHaveBeenCalledOnce();
    // 【確認内容】: Blob URL がクリーンアップされること
    expect(URL.revokeObjectURL).toHaveBeenCalledOnce();
  });

  // TC-1102-S04: ピン留め試行が HTML に含まれる
  test('TC-1102-S04: ピン留め試行が generateHtmlReport で生成された HTML に含まれる', async () => {
    // 【テスト目的】: 注目解（ピン留め）セクションにピン留め情報が含まれることを確認 🟢
    useExportStore.getState().pinTrial(0, 42);
    useExportStore.getState().updatePinMemo(0, '最良の解');

    // Blob への引数を捕捉する
    const capturedContent: string[] = [];
    const origBlob = globalThis.Blob;
    globalThis.Blob = class MockBlob {
      constructor(parts: BlobPart[], _opts?: BlobPropertyBag) {
        capturedContent.push(...(parts as string[]));
      }
    } as typeof Blob;

    await useExportStore.getState().generateHtmlReport(new Uint32Array([0, 1]));

    globalThis.Blob = origBlob;

    // 【確認内容】: 生成 HTML に trial_id=42 が含まれること
    const html = capturedContent.join('');
    expect(html).toContain('42');
    // 【確認内容】: メモ '最良の解' が含まれること
    expect(html).toContain('最良の解');
  });

  // TC-1102-S05: setReportSections で順序が更新される
  test('TC-1102-S05: setReportSections でセクション順序が更新される', () => {
    // 【テスト目的】: セクション並び替えが Store に反映されることを確認 🟢
    useExportStore.getState().setReportSections(['pinned', 'summary']);
    // 【確認内容】: reportSections が ['pinned', 'summary'] になること
    expect(useExportStore.getState().reportSections).toEqual(['pinned', 'summary']);
  });
});

// -------------------------------------------------------------------------
// TASK-1103: saveSession / loadSessionFromJson テスト
// -------------------------------------------------------------------------

import { SESSION_VERSION } from './exportStore';

describe('ExportStore — セッション保存 saveSession (TASK-1103)', () => {
  beforeEach(() => {
    useExportStore.setState({
      csvTarget: 'all',
      selectedColumns: [],
      isExporting: false,
      exportError: null,
      pinnedTrials: [],
      pinError: null,
      reportSections: ['summary', 'pareto', 'pinned', 'history', 'cluster'],
      isGeneratingReport: false,
      reportError: null,
      sessionState: null,
      isSavingSession: false,
      sessionError: null,
      sessionWarning: null,
    });
    vi.clearAllMocks();
  });

  // TC-1103-01: saveSession で isSavingSession が変化する
  test('TC-1103-01: saveSession 完了後 isSavingSession が false になる', async () => {
    // 【テスト目的】: 保存フラグが完了後に false に戻ることを確認 🟢
    await useExportStore.getState().saveSession(0, 'test.log');
    // 【確認内容】: isSavingSession が false
    expect(useExportStore.getState().isSavingSession).toBe(false);
  });

  // TC-1103-02: saveSession で sessionState が更新される
  test('TC-1103-02: saveSession 完了後 sessionState が設定される', async () => {
    // 【テスト目的】: 保存後に sessionState が Store に保存されることを確認 🟢
    await useExportStore.getState().saveSession(1, '/path/to/journal.log');
    const state = useExportStore.getState().sessionState;
    // 【確認内容】: sessionState が null でないこと
    expect(state).not.toBeNull();
    // 【確認内容】: selectedStudyId が 1 であること
    expect(state?.selectedStudyId).toBe(1);
  });

  // TC-1103-03: saveSession で <a download> が起動される
  test('TC-1103-03: saveSession でダウンロードが起動される', async () => {
    // 【テスト目的】: session_YYYYMMDD.json がダウンロードされることを確認 🟢
    await useExportStore.getState().saveSession(0, 'test.log');
    // 【確認内容】: Blob URL が作成されること
    expect(URL.createObjectURL).toHaveBeenCalledOnce();
  });

  // TC-1103-04: saveSession で SESSION_VERSION が埋め込まれる
  test('TC-1103-04: saveSession で sessionState.version が SESSION_VERSION になる', async () => {
    // 【テスト目的】: バージョン情報が正しく埋め込まれることを確認 🟢
    await useExportStore.getState().saveSession(0, 'journal.log');
    expect(useExportStore.getState().sessionState?.version).toBe(SESSION_VERSION);
  });
});

describe('ExportStore — セッション復元 loadSessionFromJson (TASK-1103)', () => {
  beforeEach(() => {
    useExportStore.setState({
      csvTarget: 'all',
      selectedColumns: [],
      isExporting: false,
      exportError: null,
      pinnedTrials: [],
      pinError: null,
      reportSections: ['summary', 'pareto', 'pinned', 'history', 'cluster'],
      isGeneratingReport: false,
      reportError: null,
      sessionState: null,
      isSavingSession: false,
      sessionError: null,
      sessionWarning: null,
    });
    vi.clearAllMocks();
  });

  // TC-1103-05: 正常な JSON でセッションが復元される
  test('TC-1103-05: 正常な JSON でセッション復元後 sessionState が設定される', () => {
    // 【テスト目的】: 正常なセッション JSON が復元されることを確認 🟢
    const sessionJson = JSON.stringify({
      version: SESSION_VERSION,
      journalPath: 'test.log',
      selectedStudyId: 2,
      filterRanges: { x1: { min: 0.5, max: 1.5 } },
      selectedIndices: [0, 1, 2],
      colorMode: 'rank',
      clusterConfig: null,
      layoutMode: 'B',
      visibleCharts: ['pareto-front'],
      pinnedTrials: [{ trialId: 42, note: 'テスト', pinnedAt: 1234567890000 }],
      freeModeLayout: null,
      savedAt: new Date().toISOString(),
    });

    useExportStore.getState().loadSessionFromJson(sessionJson);

    const state = useExportStore.getState().sessionState;
    // 【確認内容】: sessionState が設定されること
    expect(state).not.toBeNull();
    expect(state?.selectedStudyId).toBe(2);
    // 【確認内容】: sessionError が null のこと
    expect(useExportStore.getState().sessionError).toBeNull();
  });

  // TC-1103-06: 不正な JSON で sessionError がセットされる
  test('TC-1103-06: 不正な JSON で sessionError がセットされる', () => {
    // 【テスト目的】: 不正 JSON 読み込み時にエラーが表示されることを確認 🟢
    useExportStore.getState().loadSessionFromJson('not valid json {{{');
    // 【確認内容】: sessionError が設定されること
    expect(useExportStore.getState().sessionError).toContain('形式が正しくありません');
  });

  // TC-1103-07: バージョン不一致で sessionWarning がセットされ復元は続行する
  test('TC-1103-07: バージョン不一致時に sessionWarning がセットされ復元は続行される', () => {
    // 【テスト目的】: 古いバージョンでも警告を表示して復元が続行されることを確認 🟢
    const oldSessionJson = JSON.stringify({
      version: '0.9',
      journalPath: 'old.log',
      selectedStudyId: 0,
      filterRanges: {},
      selectedIndices: [],
      colorMode: 'objective',
      clusterConfig: null,
      layoutMode: 'A',
      visibleCharts: [],
      pinnedTrials: [],
      freeModeLayout: null,
      savedAt: new Date().toISOString(),
    });

    useExportStore.getState().loadSessionFromJson(oldSessionJson);

    // 【確認内容】: sessionWarning がセットされること
    expect(useExportStore.getState().sessionWarning).toContain('古いバージョン');
    // 【確認内容】: 復元は続行されること（sessionState が設定される）
    expect(useExportStore.getState().sessionState).not.toBeNull();
  });

  // TC-1103-08: ピン留め試行が復元される
  test('TC-1103-08: ピン留め試行が loadSessionFromJson で復元される', () => {
    // 【テスト目的】: ピン留め試行がセッション復元時に復元されることを確認 🟢
    const sessionJson = JSON.stringify({
      version: SESSION_VERSION,
      journalPath: 'test.log',
      selectedStudyId: 0,
      filterRanges: {},
      selectedIndices: [],
      colorMode: 'objective',
      clusterConfig: null,
      layoutMode: 'A',
      visibleCharts: [],
      pinnedTrials: [
        { trialId: 10, note: 'メモ1', pinnedAt: 1000000000000 },
        { trialId: 20, note: 'メモ2', pinnedAt: 1000000001000 },
      ],
      freeModeLayout: null,
      savedAt: new Date().toISOString(),
    });

    useExportStore.getState().loadSessionFromJson(sessionJson);

    const { pinnedTrials } = useExportStore.getState();
    // 【確認内容】: ピン留め試行が 2 件復元されること
    expect(pinnedTrials).toHaveLength(2);
    expect(pinnedTrials[0].trialId).toBe(10);
    expect(pinnedTrials[1].trialId).toBe(20);
  });

  // TC-1103-09: clearSessionMessages でエラー・警告がクリアされる
  test('TC-1103-09: clearSessionMessages でエラー・警告が null になる', () => {
    // 【テスト目的】: メッセージクリア操作が正しく動作することを確認 🟢
    useExportStore.setState({ sessionError: 'エラー', sessionWarning: '警告' });
    useExportStore.getState().clearSessionMessages();
    expect(useExportStore.getState().sessionError).toBeNull();
    expect(useExportStore.getState().sessionWarning).toBeNull();
  });
});
