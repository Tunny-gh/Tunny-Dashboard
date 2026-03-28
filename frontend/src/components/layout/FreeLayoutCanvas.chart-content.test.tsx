/**
 * FreeLayoutCanvas — ChartContent 追加チャートのテスト
 *
 * 【テスト対象】: ChartContent の objective-pair-matrix / importance / EmptyState メッセージ
 * 【テスト方針】: studyStore / layoutStore をモックしてチャート内容を検証
 */

import { describe, test, expect, beforeEach, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import '@testing-library/jest-dom';

// -------------------------------------------------------------------------
// echarts-for-react モック（__mocks__/echarts-for-react.tsx を使用）
// -------------------------------------------------------------------------

vi.mock('echarts-for-react');

// -------------------------------------------------------------------------
// deck.gl モック — WebGL不要ダミー
// -------------------------------------------------------------------------

vi.mock('deck.gl', () => ({
  DeckGL: vi.fn(({ children }: { children?: unknown }) => (
    <div data-testid="deck-gl">{children as never}</div>
  )),
  ScatterplotLayer: vi.fn().mockImplementation((props: { id: string }) => ({
    id: props.id,
    type: 'ScatterplotLayer',
  })),
}));

// -------------------------------------------------------------------------
// studyStore モック
// -------------------------------------------------------------------------

const mockStudySelector = vi.fn();

vi.mock('../../stores/studyStore', () => ({
  useStudyStore: (selector: (s: unknown) => unknown) => mockStudySelector(selector),
}));

// -------------------------------------------------------------------------
// wasmLoader モック（getTrials は studyStore 経由で使うが念のため）
// -------------------------------------------------------------------------

vi.mock('../../wasm/wasmLoader', () => ({
  WasmLoader: {
    getInstance: vi.fn().mockResolvedValue({
      getTrials: vi.fn().mockReturnValue([]),
      computeHvHistory: vi.fn().mockReturnValue({
        trialIds: new Uint32Array([0, 1, 2]),
        hvValues: new Float64Array([0.5, 0.7, 0.9]),
      }),
    }),
  },
}));

// -------------------------------------------------------------------------
// layoutStore モック（チャートカードを表示するため）
// -------------------------------------------------------------------------

import { useLayoutStore, DEFAULT_FREE_LAYOUT } from '../../stores/layoutStore';
import { FreeLayoutCanvas } from './FreeLayoutCanvas';
import type { Study, TrialData } from '../../types';
import type { GpuBuffer } from '../../wasm/gpuBuffer';

// DEFAULT_FREE_LAYOUT の型エラー回避（使用していないが import 解決のため）
void DEFAULT_FREE_LAYOUT;

// -------------------------------------------------------------------------
// テストヘルパー
// -------------------------------------------------------------------------

/** 2目的 Study スタブ */
function makeMultiObjectiveStudy(): Study {
  return {
    studyId: 1,
    name: 'test-study',
    directions: ['minimize', 'minimize'],
    completedTrials: 5,
    totalTrials: 5,
    paramNames: ['x', 'y'],
    objectiveNames: ['obj0', 'obj1'],
    userAttrNames: [],
    hasConstraints: false,
  };
}

/** 1目的 Study スタブ */
function makeSingleObjectiveStudy(): Study {
  return {
    studyId: 1,
    name: 'test-study',
    directions: ['minimize'],
    completedTrials: 5,
    totalTrials: 5,
    paramNames: ['x', 'y'],
    objectiveNames: ['value'],
    userAttrNames: [],
    hasConstraints: false,
  };
}

/** GpuBuffer スタブ（5トライアル分） */
function makeGpuBuffer(): GpuBuffer {
  const n = 5;
  const positions = new Float32Array(n * 2).fill(0.5);
  return {
    trialCount: n,
    positions,
    positions3d: new Float32Array(n * 3),
    sizes: new Float32Array(n),
  } as unknown as GpuBuffer;
}

/** テスト用 TrialData */
function makeTrialData(): TrialData[] {
  return [
    { trialId: 0, params: { x: 1.5, y: 0.5 }, values: [10.0], paretoRank: null },
    { trialId: 1, params: { x: 2.5, y: 1.5 }, values: [5.0], paretoRank: null },
  ];
}

/** studyStore モックを指定の currentStudy / gpuBuffer / trialRows で設定 */
function setStudyStore(
  currentStudy: Study | null,
  gpuBuffer: GpuBuffer | null,
  trialRows: TrialData[] = [],
) {
  mockStudySelector.mockImplementation((selector: (s: {
    currentStudy: Study | null;
    gpuBuffer: GpuBuffer | null;
    trialRows: TrialData[];
  }) => unknown) =>
    selector({ currentStudy, gpuBuffer, trialRows }),
  );
}

// -------------------------------------------------------------------------
// テスト
// -------------------------------------------------------------------------

describe('FreeLayoutCanvas — ChartContent', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // レイアウトをデフォルトにリセット
    useLayoutStore.setState({
      layoutMode: 'D',
      freeModeLayout: null,
      layoutLoadError: null,
      visibleCharts: new Set(),
      panelSizes: { leftPanel: 280, bottomPanel: 200 },
    });
  });

  // ----------------------------------------------------------------
  // EmptyState メッセージ
  // ----------------------------------------------------------------

  describe('EmptyState メッセージ', () => {
    test('TC-CC-001: データ未ロード時に「データを読み込んでください」が表示される', () => {
      // 【目的】: REQ-C03 — currentStudy === null のとき正しいメッセージが表示される
      setStudyStore(null, null);
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'pareto-front', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      });
      render(<FreeLayoutCanvas />);
      expect(screen.getByTestId('empty-state')).toHaveTextContent('データを読み込んでください');
    });

    test('TC-CC-002: 未知のchartIdのとき「このチャートは準備中です」が表示される', () => {
      // 【目的】: REQ-C03 — 未実装チャートで適切なメッセージを表示する
      setStudyStore(makeSingleObjectiveStudy(), makeGpuBuffer());
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'unknown-chart' as never, gridRow: [1, 3], gridCol: [1, 3] }],
        },
      });
      render(<FreeLayoutCanvas />);
      expect(screen.getByTestId('empty-state')).toHaveTextContent('このチャートは準備中です');
    });
  });

  // ----------------------------------------------------------------
  // objective-pair-matrix
  // ----------------------------------------------------------------

  describe('objective-pair-matrix', () => {
    test('TC-CC-010: 2目的 Study のとき ObjectivePairMatrix がレンダリングされる', () => {
      // 【目的】: REQ-C01-A — 多目的 Study で ObjectivePairMatrix が表示される
      setStudyStore(makeMultiObjectiveStudy(), makeGpuBuffer());
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'objective-pair-matrix', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      });
      render(<FreeLayoutCanvas />);
      expect(screen.getByTestId('objective-pair-matrix')).toBeInTheDocument();
    });

    test('TC-CC-011: 1目的 Study のとき EmptyState が表示される', () => {
      // 【目的】: REQ-C01-B — 単目的 Study では ObjectivePairMatrix を表示しない
      setStudyStore(makeSingleObjectiveStudy(), makeGpuBuffer());
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'objective-pair-matrix', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      });
      render(<FreeLayoutCanvas />);
      expect(screen.getByTestId('empty-state')).toHaveTextContent('多目的 Study でのみ利用可能です');
    });

    test('TC-CC-012: gpuBuffer === null でもクラッシュしない', () => {
      // 【目的】: REQ-C01-C — gpuBuffer なし時に ObjectivePairMatrix 内部の — プレースホルダーが表示される
      setStudyStore(makeMultiObjectiveStudy(), null);
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'objective-pair-matrix', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      });
      // gpuBuffer=null のとき currentStudy || gpuBuffer チェックで EmptyState になる
      // (ChartContent は currentStudy && gpuBuffer がある場合のみ switch に進む)
      expect(() => render(<FreeLayoutCanvas />)).not.toThrow();
    });
  });

  // ----------------------------------------------------------------
  // importance
  // ----------------------------------------------------------------

  describe('importance', () => {
    test('TC-CC-020: paramNames が存在するとき ECharts バーチャートがレンダリングされる', () => {
      // 【目的】: REQ-C02-A — paramNames をラベルとするバーチャートが表示される
      setStudyStore(makeSingleObjectiveStudy(), makeGpuBuffer());
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'importance', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      });
      render(<FreeLayoutCanvas />);
      const echartsEl = screen.getByTestId('echarts');
      expect(echartsEl).toBeInTheDocument();
      // option に paramNames が含まれているか確認
      const option = JSON.parse(echartsEl.dataset.option ?? '{}');
      expect(option.xAxis?.data).toEqual(['x', 'y']);
    });

    test('TC-CC-021: タイトルに「重要度（暫定・WASM未計算）」が含まれる', () => {
      // 【目的】: REQ-C02-B — 暫定表示であることをタイトルで明示する
      setStudyStore(makeSingleObjectiveStudy(), makeGpuBuffer());
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'importance', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      });
      render(<FreeLayoutCanvas />);
      const option = JSON.parse(screen.getByTestId('echarts').dataset.option ?? '{}');
      expect(option.title?.text).toContain('重要度');
    });

    test('TC-CC-022: paramNames が空のとき EmptyState が表示される', () => {
      // 【目的】: REQ-C02-C — パラメータなし時は EmptyState を表示する
      const studyNoParams: Study = {
        ...makeSingleObjectiveStudy(),
        paramNames: [],
      };
      setStudyStore(studyNoParams, makeGpuBuffer());
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'importance', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      });
      render(<FreeLayoutCanvas />);
      expect(screen.getByTestId('empty-state')).toBeInTheDocument();
    });

    test('TC-CC-023: バーの数が paramNames の数と一致する', () => {
      // 【目的】: REQ-C02-A — paramNames の数だけバーが表示される
      setStudyStore(makeSingleObjectiveStudy(), makeGpuBuffer());
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'importance', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      });
      render(<FreeLayoutCanvas />);
      const option = JSON.parse(screen.getByTestId('echarts').dataset.option ?? '{}');
      // series[0].data の長さが paramNames.length と一致すること
      expect(option.series?.[0]?.data?.length).toBe(2); // x, y の2パラメータ
    });
  });

  // ----------------------------------------------------------------
  // slice
  // ----------------------------------------------------------------

  describe('slice', () => {
    test('TC-CC-030: trialData が存在するとき SlicePlot がレンダリングされる', () => {
      // 【目的】: REQ-C05-A — trialRows があるとき SlicePlot が表示される
      setStudyStore(makeSingleObjectiveStudy(), makeGpuBuffer(), makeTrialData());
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'slice', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      });
      render(<FreeLayoutCanvas />);
      expect(screen.getByTestId('slice-plot')).toBeInTheDocument();
    });

    test('TC-CC-031: trialData が空のとき EmptyState が表示される', () => {
      // 【目的】: REQ-C05-D — trialRows 空のとき EmptyState
      setStudyStore(makeSingleObjectiveStudy(), makeGpuBuffer(), []);
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'slice', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      });
      render(<FreeLayoutCanvas />);
      expect(screen.getByTestId('empty-state')).toBeInTheDocument();
    });
  });

  // ----------------------------------------------------------------
  // contour
  // ----------------------------------------------------------------

  // ----------------------------------------------------------------
  // hypervolume
  // ----------------------------------------------------------------

  describe('hypervolume', () => {
    test('TC-CC-060: 2目的 Study のとき HypervolumeHistory がレンダリングされる', async () => {
      // 【目的】: REQ-103-G — 多目的 Study で hypervolume チャートが表示される
      setStudyStore(makeMultiObjectiveStudy(), makeGpuBuffer());
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'hypervolume', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      });
      render(<FreeLayoutCanvas />);
      await waitFor(() => {
        expect(screen.getByTestId('hypervolume-chart')).toBeInTheDocument();
      });
    });

    test('TC-CC-061: 1目的 Study のとき EmptyState「多目的 Study でのみ利用可能です」が表示される', () => {
      // 【目的】: REQ-103-I — 単目的 Study では hypervolume チャートを表示しない
      setStudyStore(makeSingleObjectiveStudy(), makeGpuBuffer());
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'hypervolume', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      });
      render(<FreeLayoutCanvas />);
      expect(screen.getByTestId('empty-state')).toHaveTextContent('多目的 Study でのみ利用可能です');
    });

    test('TC-CC-062: computeHvHistory が reject したとき EmptyState「HV 計算エラー」が表示される', async () => {
      // 【目的】: REQ-103-J — WASM エラー時に EmptyState を表示してクラッシュしない
      const { WasmLoader } = await import('../../wasm/wasmLoader');
      vi.mocked(WasmLoader.getInstance).mockResolvedValueOnce({
        getTrials: vi.fn().mockReturnValue([]),
        computeHvHistory: vi.fn().mockRejectedValue(new Error('HV failed')),
        filterByRanges: vi.fn().mockReturnValue(new Uint32Array([])),
        serializeCsv: vi.fn().mockReturnValue(''),
        appendJournalDiff: vi.fn().mockReturnValue({ new_completed: 0, consumed_bytes: 0 }),
        computeReportStats: vi.fn().mockReturnValue('{}'),
        parseJournal: vi.fn().mockReturnValue({ studies: [], durationMs: 0 }),
        selectStudy: vi.fn().mockReturnValue({}),
        computeParetoRanks: vi.fn().mockReturnValue({ ranks: [] }),
        computeHvHistory: vi.fn().mockRejectedValue(new Error('HV failed')),
      } as never);
      setStudyStore(makeMultiObjectiveStudy(), makeGpuBuffer());
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'hypervolume', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      });
      render(<FreeLayoutCanvas />);
      await waitFor(() => {
        expect(screen.getByTestId('empty-state')).toHaveTextContent('HV 計算エラー');
      });
    });
  });

  describe('contour', () => {
    test('TC-CC-040: trialData が存在するとき ContourPlot がレンダリングされる', () => {
      // 【目的】: REQ-C06-A — trialRows があり paramNames >= 2 のとき ContourPlot が表示される
      setStudyStore(makeSingleObjectiveStudy(), makeGpuBuffer(), makeTrialData());
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'contour', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      });
      render(<FreeLayoutCanvas />);
      expect(screen.getByTestId('contour-plot')).toBeInTheDocument();
    });

    test('TC-CC-041: paramNames < 2 のとき EmptyState が表示される', () => {
      // 【目的】: REQ-C06-C — パラメータ不足時 EmptyState
      const studyOneParam: Study = { ...makeSingleObjectiveStudy(), paramNames: ['x'] };
      setStudyStore(studyOneParam, makeGpuBuffer(), makeTrialData());
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'contour', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      });
      render(<FreeLayoutCanvas />);
      expect(screen.getByTestId('empty-state')).toHaveTextContent('パラメータが2つ以上必要です');
    });

    test('TC-CC-042: trialData が空のとき EmptyState が表示される', () => {
      // 【目的】: REQ-C06-D — trialRows 空のとき EmptyState
      setStudyStore(makeSingleObjectiveStudy(), makeGpuBuffer(), []);
      useLayoutStore.setState({
        freeModeLayout: {
          cells: [{ chartId: 'contour', gridRow: [1, 3], gridCol: [1, 3] }],
        },
      });
      render(<FreeLayoutCanvas />);
      expect(screen.getByTestId('empty-state')).toBeInTheDocument();
    });
  });
});
