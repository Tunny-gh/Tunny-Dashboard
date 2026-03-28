/**
 * LayoutStore — レイアウトモード・表示チャート・パネルサイズ管理 (TASK-302)
 *
 * 【役割】: レイアウトモード A〜D / 表示チャート Set / パネルサイズを管理する
 * 【設計方針】: 純粋な状態管理（WASM 呼び出しなし）
 * 🟢 LayoutStore インターフェース（types/index.ts）に準拠
 */

import { create } from 'zustand';
import type { LayoutMode, ChartId, LayoutConfig, PanelSizes, FreeModeLayout } from '../types';

// -------------------------------------------------------------------------
// 定数（エクスポート）
// -------------------------------------------------------------------------

/**
 * 【デフォルトフリーモードレイアウト】: freeModeLayout が null のときに使用する初期配置
 * 4×4 グリッドを 4 等分した 2×2 配置
 */
export const DEFAULT_FREE_LAYOUT: FreeModeLayout = {
  cells: [
    { chartId: 'pareto-front', gridRow: [1, 3], gridCol: [1, 3] },
    { chartId: 'parallel-coords', gridRow: [1, 3], gridCol: [3, 5] },
    { chartId: 'scatter-matrix', gridRow: [3, 5], gridCol: [1, 3] },
    { chartId: 'history', gridRow: [3, 5], gridCol: [3, 5] },
  ],
};

// -------------------------------------------------------------------------
// 型定義
// -------------------------------------------------------------------------

/**
 * 【Store 状態型】: LayoutStore インターフェースに準拠
 */
interface LayoutState {
  layoutMode: LayoutMode;
  visibleCharts: Set<ChartId>;
  panelSizes: PanelSizes;
  freeModeLayout: FreeModeLayout | null;
  /** レイアウト JSON 読み込みエラーメッセージ */
  layoutLoadError: string | null;

  setLayoutMode: (mode: LayoutMode) => void;
  toggleChart: (chartId: ChartId) => void;
  saveLayout: () => LayoutConfig;
  loadLayout: (config: LayoutConfig) => void;
  /** フリーモードレイアウト全体を設定する */
  setFreeModeLayout: (layout: FreeModeLayout | null) => void;
  /**
   * 指定チャートのグリッド位置を更新する
   * ドラッグ&ドロップによる移動時に呼ぶ 🟢 REQ-032
   */
  updateCellPosition: (
    chartId: ChartId,
    gridRow: [number, number],
    gridCol: [number, number],
  ) => void;
  /**
   * JSON 文字列からレイアウトを読み込む
   * 形式エラー時は layoutLoadError をセットしてデフォルトに戻す
   */
  loadLayoutFromJson: (json: string) => { success: boolean; error?: string };
}

// -------------------------------------------------------------------------
// 定数
// -------------------------------------------------------------------------

/**
 * 【デフォルト表示チャート】: レイアウトモード A の初期表示チャート
 * 🟢 主要な4チャートを初期表示
 */
const DEFAULT_VISIBLE_CHARTS: ChartId[] = [
  'pareto-front',
  'parallel-coords',
  'scatter-matrix',
  'history',
];

// -------------------------------------------------------------------------
// Store 実装
// -------------------------------------------------------------------------

export const useLayoutStore = create<LayoutState>()((set, get) => ({
  // -------------------------------------------------------------------------
  // 初期状態
  // -------------------------------------------------------------------------
  layoutMode: 'A' as LayoutMode,
  visibleCharts: new Set<ChartId>(DEFAULT_VISIBLE_CHARTS),
  panelSizes: { leftPanel: 280, bottomPanel: 200 },
  freeModeLayout: null,
  layoutLoadError: null,

  // -------------------------------------------------------------------------
  // アクション実装
  // -------------------------------------------------------------------------

  /**
   * 【レイアウトモード設定】: モード A〜D を切り替える
   * 🟢 モード切り替えは即座に反映
   */
  setLayoutMode: (mode) => set({ layoutMode: mode }),

  /**
   * 【チャート表示切り替え】: visibleCharts Set への追加・削除
   * 含まれていれば削除、含まれていなければ追加
   */
  toggleChart: (chartId) => {
    const current = get().visibleCharts;
    const next = new Set(current);
    if (next.has(chartId)) {
      next.delete(chartId);
    } else {
      next.add(chartId);
    }
    set({ visibleCharts: next });
  },

  /**
   * 【レイアウト保存】: 現在のレイアウト設定を LayoutConfig として返す
   * Set は Array に変換して JSON 互換にする
   * 🟢 saveLayout / loadLayout でセッション保存・復元が可能
   */
  saveLayout: (): LayoutConfig => {
    const { layoutMode, visibleCharts, panelSizes, freeModeLayout } = get();
    return {
      mode: layoutMode,
      visibleCharts: Array.from(visibleCharts),
      panelSizes,
      freeModeLayout,
    };
  },

  /**
   * 【レイアウト復元】: 保存された LayoutConfig を適用する
   * visibleCharts は Array から Set に変換する
   */
  loadLayout: (config) => {
    set({
      layoutMode: config.mode,
      visibleCharts: new Set(config.visibleCharts),
      panelSizes: config.panelSizes,
      freeModeLayout: config.freeModeLayout,
    });
  },

  /**
   * 【フリーモードレイアウト設定】: freeModeLayout を直接更新する
   * プリセット適用・リセット時に使用
   */
  setFreeModeLayout: (layout) => set({ freeModeLayout: layout }),

  /**
   * 【グリッド位置更新】: 指定チャートのグリッド位置のみを更新する
   * ドラッグ&ドロップの完了時に呼ぶ 🟢 REQ-032
   */
  updateCellPosition: (chartId, gridRow, gridCol) => {
    const { freeModeLayout } = get();
    if (!freeModeLayout) return;
    const cells = freeModeLayout.cells.map((cell) =>
      cell.chartId === chartId ? { ...cell, gridRow, gridCol } : cell,
    );
    set({ freeModeLayout: { cells } });
  },

  /**
   * 【JSON レイアウト読み込み】: JSON 文字列をパースして loadLayout を呼ぶ
   * パース失敗・バリデーション失敗時は layoutLoadError をセットしてデフォルトに戻す
   * 🟢 NFR-032: エラー時は「レイアウトを読み込めませんでした」を表示
   */
  loadLayoutFromJson: (json) => {
    const ERR = 'レイアウトを読み込めませんでした';
    try {
      const config = JSON.parse(json) as LayoutConfig;
      // 【バリデーション】: 必須フィールドの確認
      if (!config.mode || !Array.isArray(config.visibleCharts)) {
        set({ layoutLoadError: ERR, freeModeLayout: DEFAULT_FREE_LAYOUT });
        return { success: false, error: ERR };
      }
      get().loadLayout(config);
      set({ layoutLoadError: null });
      return { success: true };
    } catch {
      set({ layoutLoadError: ERR, freeModeLayout: DEFAULT_FREE_LAYOUT });
      return { success: false, error: ERR };
    }
  },
}));
