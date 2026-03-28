/**
 * StudyStore — Journal 読み込み・Study 選択・DataFrame 管理 (TASK-302)
 *
 * 【役割】: Journal ファイルのパース、Study 選択、GpuBuffer の初期化を担う
 * 【設計方針】: WASM は WasmLoader.getInstance() 経由でアクセス
 * 🟢 StudyStore インターフェース（types/index.ts）に準拠
 */

import { create } from 'zustand';
import type { Study, DataFrameInfo, StudyMode, TrialData } from '../types';
import { WasmLoader } from '../wasm/wasmLoader';
import { GpuBuffer } from '../wasm/gpuBuffer';
import { useSelectionStore } from './selectionStore';
import { useComparisonStore } from './comparisonStore';

// -------------------------------------------------------------------------
// 型定義
// -------------------------------------------------------------------------

/**
 * 【Store 状態型】: StudyStore インターフェース + 内部状態
 */
interface StudyState {
  // --- 公開状態 ---
  currentStudy: Study | null;
  allStudies: Study[];
  studyMode: StudyMode;
  isLoading: boolean;
  loadError: string | null;

  // --- アクション ---
  loadJournal: (file: File) => Promise<void>;
  selectStudy: (studyId: number) => void;
  setComparisonStudies: (studyIds: number[]) => void;
  getDataFrameInfo: () => DataFrameInfo | null;

  // --- 内部状態 ---
  /** 現在選択中の Study の GpuBuffer（チャートコンポーネントが参照） */
  gpuBuffer: GpuBuffer | null;
  /** 現在選択中の Study の per-trial データ（slice / contour チャートが参照） */
  trialRows: TrialData[];
}

// -------------------------------------------------------------------------
// Store 実装
// -------------------------------------------------------------------------

export const useStudyStore = create<StudyState>()((set, get) => ({
  // -------------------------------------------------------------------------
  // 初期状態
  // -------------------------------------------------------------------------
  currentStudy: null,
  allStudies: [],
  studyMode: 'single-objective' as StudyMode,
  isLoading: false,
  loadError: null,
  gpuBuffer: null,
  trialRows: [],

  // -------------------------------------------------------------------------
  // アクション実装
  // -------------------------------------------------------------------------

  /**
   * 【Journal 読み込み】: File → WASM parseJournal → allStudies を更新する
   * 【isLoading 管理】: 開始時 true、完了/失敗時 false
   * 【エラー処理】: WASM 失敗時 loadError にメッセージを格納
   * 🟢 loadJournal は Promise<void> を返す（StudyStore インターフェース準拠）
   */
  loadJournal: async (file) => {
    // 【ローディング開始】: 同期的に isLoading を true に設定
    set({ isLoading: true, loadError: null });

    try {
      // 【ファイル読み込み】: File API で ArrayBuffer を取得
      const arrayBuffer = await file.arrayBuffer();
      const data = new Uint8Array(arrayBuffer);

      // 【WASM 呼び出し】: WasmLoader 経由で parseJournal を実行
      const wasm = await WasmLoader.getInstance();
      const result = wasm.parseJournal(data);

      // 【状態更新】: allStudies に設定し、最初の Study を自動選択
      set({ allStudies: result.studies, isLoading: false });
      if (result.studies.length > 0) {
        get().selectStudy(result.studies[0].studyId);
      }
    } catch (err) {
      // 【エラー処理】: loadError にメッセージを格納
      set({ isLoading: false, loadError: String(err) });
    }
  },

  /**
   * 【Study 選択】: studyId に対応する Study を選択し GpuBuffer を初期化する
   * 【SelectionStore 初期化】: trialCount と全インデックスを selectionStore に設定
   * 🟡 TASK-103 WASM 実装後に selectStudy の戻り値型が確定
   */
  selectStudy: (studyId) => {
    WasmLoader.getInstance()
      .then((wasm) => {
        // 【WASM selectStudy】: positions / sizes / trialCount を取得
        const result = wasm.selectStudy(studyId);
        const gpuBuffer = new GpuBuffer(result);

        // 【currentStudy 更新】
        const study = get().allStudies.find((s) => s.studyId === studyId) ?? null;
        // 【studyMode 判定】: 目的関数数 > 1 なら multi-objective
        const studyMode: StudyMode =
          study && study.directions.length > 1 ? 'multi-objective' : 'single-objective';

        // 【getTrials 呼び出し】: per-trial データを取得して trialRows に保持
        let trialRows: TrialData[] = [];
        try {
          trialRows = wasm.getTrials();
        } catch {
          // WASM getTrials 未実装の場合は空配列のまま
        }

        set({ currentStudy: study, gpuBuffer, studyMode, trialRows });

        // 【SelectionStore 初期化】: trialCount と全インデックスを設定
        useSelectionStore.getState()._setTrialCount(gpuBuffer.trialCount);
        useSelectionStore.getState().clearSelection();

        // 【ComparisonStore リセット】: Study 切替時に比較状態をクリア 🟢 REQ-124
        useComparisonStore.getState().reset();
      })
      .catch(() => {
        // 【WASM 未初期化時】: 無視
      });
  },

  /**
   * 【比較Study設定】: 🟡 スタブ実装（後続タスクで完成）
   */
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  setComparisonStudies: (_studyIds) => {
    // TODO: TASK-1201 複数Study比較で実装
  },

  /**
   * 【DataFrame情報取得】: 🟡 スタブ実装（後続タスクで完成）
   */
  getDataFrameInfo: () => null,
}));
