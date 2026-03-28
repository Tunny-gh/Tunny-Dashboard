/**
 * SelectionStore — Brushing & Linking の中核 Zustand Store (TASK-302)
 *
 * 【役割】: selectedIndices / filterRanges / colorMode / highlighted を一元管理する
 * 【設計方針】: subscribeWithSelector を使い React サイクル外で GPU alpha を直接更新可能にする
 * 【WASM 呼び出し】: addAxisFilter / removeAxisFilter は fire-and-forget で非同期実行
 * 🟢 REQ-040〜REQ-044 に準拠
 */

import { create } from 'zustand';
import { subscribeWithSelector } from 'zustand/middleware';
import type { ColorMode, Range } from '../types';
import { WasmLoader } from '../wasm/wasmLoader';

// -------------------------------------------------------------------------
// 型定義
// -------------------------------------------------------------------------

/**
 * 【Store 状態型】: 公開 SelectionStore インターフェース + 内部状態
 * 🟢 公開フィールドは types/index.ts § SelectionStore に準拠
 */
interface SelectionState {
  // --- 公開状態 ---
  /** 現在選択中の trial インデックス（N個） */
  selectedIndices: Uint32Array;
  /** 軸名 → {min, max} フィルタ範囲 */
  filterRanges: Record<string, Range>;
  /** ハイライト中の trial インデックス（null = なし） */
  highlighted: number | null;
  /** カラーリングモード */
  colorMode: ColorMode;

  // --- 公開アクション ---
  brushSelect: (indices: Uint32Array) => void;
  addAxisFilter: (axis: string, min: number, max: number) => void;
  removeAxisFilter: (axis: string) => void;
  clearSelection: () => void;
  setHighlight: (index: number | null) => void;
  setColorMode: (mode: ColorMode) => void;

  // --- 内部状態（公開インターフェース外）---
  /** clearSelection で全インデックス生成に使用する trial 数 */
  _trialCount: number;
  /** studyStore から呼び出して trialCount を初期化する */
  _setTrialCount: (n: number) => void;
}

// -------------------------------------------------------------------------
// Store 実装
// -------------------------------------------------------------------------

/**
 * 【Store 作成】: subscribeWithSelector を使って selector ベースの subscribe を有効化
 * 【subscribeWithSelector 用途】:
 *   `useSelectionStore.subscribe((s) => s.selectedIndices, (indices) => gpuBuf.updateAlphas(indices))`
 *   という形でチャートコンポーネントが React サイクル外で GPU を直接更新する
 * 🟢 REQ-044 に準拠（React 再レンダリングなしに alpha 更新）
 */
export const useSelectionStore = create<SelectionState>()(
  subscribeWithSelector((set, get) => ({
    // -------------------------------------------------------------------------
    // 初期状態
    // -------------------------------------------------------------------------
    selectedIndices: new Uint32Array(0),
    filterRanges: {},
    highlighted: null,
    colorMode: 'objective' as ColorMode,
    _trialCount: 0,

    // -------------------------------------------------------------------------
    // アクション実装
    // -------------------------------------------------------------------------

    /**
     * 【Brush 選択】: グラフ上のブラシ操作から呼ばれる
     * 🟢 selectedIndices を同期更新 → subscribe が GPU alpha を更新
     */
    brushSelect: (indices) => {
      set({ selectedIndices: indices });
    },

    /**
     * 【軸フィルタ追加】: Left Panel のスライダー操作から呼ばれる
     * 1. filterRanges を同期更新（UI に即反映）
     * 2. WASM filterByRanges を非同期呼び出し → selectedIndices を更新
     * 【WASM 未初期化時】: filterRanges は更新済み、selectedIndices は変更しない
     * 🟢 REQ-042 に準拠（非同期で UI をブロックしない）
     */
    addAxisFilter: (axis, min, max) => {
      // 【同期更新】: filterRanges を即座に更新
      const newRanges: Record<string, Range> = {
        ...get().filterRanges,
        [axis]: { min, max },
      };
      set({ filterRanges: newRanges });

      // 【非同期 WASM 呼び出し】: fire-and-forget
      WasmLoader.getInstance()
        .then((wasm) => {
          const indices = wasm.filterByRanges(JSON.stringify(newRanges));
          set({ selectedIndices: indices });
        })
        .catch(() => {
          // 【WASM 未初期化】: filterRanges のみ更新済み、selectedIndices は変更しない
        });
    },

    /**
     * 【軸フィルタ除去】: フィルタクリアボタンから呼ばれる
     * フィルタが全て除去された場合は全インデックスを選択状態にする
     * 🟢 残りフィルタがある場合は WASM を再呼び出し
     */
    removeAxisFilter: (axis) => {
      // 【フィルタ除去】: 指定軸を除いた新しい filterRanges を生成
      const current = get().filterRanges;
      // eslint-disable-next-line @typescript-eslint/no-unused-vars
      const { [axis]: _removed, ...newRanges } = current;
      set({ filterRanges: newRanges as Record<string, Range> });

      if (Object.keys(newRanges).length === 0) {
        // 【全フィルタ除去】: 全インデックスを選択状態に
        const n = get()._trialCount;
        set({ selectedIndices: _makeAllIndices(n) });
        return;
      }

      // 【残りフィルタ再適用】: WASM を再呼び出し
      WasmLoader.getInstance()
        .then((wasm) => {
          const indices = wasm.filterByRanges(JSON.stringify(newRanges));
          set({ selectedIndices: indices });
        })
        .catch(() => {
          // WASM 未初期化時は無視
        });
    },

    /**
     * 【選択クリア】: 全点を選択状態に戻し filterRanges を空にする
     * 🟢 _trialCount から全インデックス [0, 1, ..., N-1] を生成
     */
    clearSelection: () => {
      const n = get()._trialCount;
      set({
        selectedIndices: _makeAllIndices(n),
        filterRanges: {},
      });
    },

    /** 【ハイライト設定】 */
    setHighlight: (index) => set({ highlighted: index }),

    /** 【カラーモード設定】 */
    setColorMode: (mode) => set({ colorMode: mode }),

    /** 【内部】: trialCount を studyStore から設定する */
    _setTrialCount: (n) => set({ _trialCount: n }),
  })),
);

// -------------------------------------------------------------------------
// 内部ユーティリティ
// -------------------------------------------------------------------------

/**
 * 【全インデックス生成】: [0, 1, ..., n-1] の Uint32Array を生成する
 * clearSelection / removeAxisFilter（全除去時）で使用
 */
function _makeAllIndices(n: number): Uint32Array {
  const arr = new Uint32Array(n);
  for (let i = 0; i < n; i++) arr[i] = i;
  return arr;
}
