/**
 * LayoutStore — layout mode, visible charts, and panel size management (TASK-302)
 *
 * Manages layout modes A–D, the visible chart Set, and panel sizes.
 * Pure state management with no WASM calls.
 * Conforms to the LayoutStore interface in types/index.ts.
 */

import { create } from 'zustand';
import type { LayoutMode, ChartId, LayoutConfig, PanelSizes, FreeModeLayout } from '../types';

// -------------------------------------------------------------------------
// Constants (exported)
// -------------------------------------------------------------------------

/**
 * Default free-mode layout used when freeModeLayout is null.
 * A 2×2 arrangement that evenly divides a 4×4 grid.
 */
export const DEFAULT_FREE_LAYOUT: FreeModeLayout = {
  cells: [
    { cellId: crypto.randomUUID(), chartId: 'pareto-front', gridRow: [1, 3], gridCol: [1, 3] },
    { cellId: crypto.randomUUID(), chartId: 'parallel-coords', gridRow: [1, 3], gridCol: [3, 5] },
    { cellId: crypto.randomUUID(), chartId: 'scatter-matrix', gridRow: [3, 5], gridCol: [1, 3] },
    { cellId: crypto.randomUUID(), chartId: 'history', gridRow: [3, 5], gridCol: [3, 5] },
  ],
};

// -------------------------------------------------------------------------
// Types
// -------------------------------------------------------------------------

/**
 * LayoutStore state type. Conforms to the LayoutStore interface.
 */
interface LayoutState {
  layoutMode: LayoutMode;
  visibleCharts: Set<ChartId>;
  panelSizes: PanelSizes;
  freeModeLayout: FreeModeLayout | null;
  /** Error message from the last layout JSON load attempt */
  layoutLoadError: string | null;

  setLayoutMode: (mode: LayoutMode) => void;
  toggleChart: (chartId: ChartId) => void;
  saveLayout: () => LayoutConfig;
  loadLayout: (config: LayoutConfig) => void;
  /** Replaces the entire free-mode layout */
  setFreeModeLayout: (layout: FreeModeLayout | null) => void;
  /**
   * Updates the grid position of the specified cell (cellId).
   * Called after a drag-and-drop move. REQ-032
   */
  updateCellPosition: (
    cellId: string,
    gridRow: [number, number],
    gridCol: [number, number],
  ) => void;
  /**
   * Adds a new chart cell to freeModeLayout. cellId is auto-generated.
   * Falls back to DEFAULT_FREE_LAYOUT as the base when freeModeLayout is null.
   */
  addCell: (
    chartId: ChartId,
    gridRow: [number, number],
    gridCol: [number, number],
  ) => void;
  /**
   * Removes the cell with the given cellId.
   * Does nothing if the cellId does not exist.
   */
  removeCell: (cellId: string) => void;
  /**
   * Parses a JSON string and calls loadLayout.
   * On parse or validation failure, sets layoutLoadError and resets to the default layout.
   */
  loadLayoutFromJson: (json: string) => { success: boolean; error?: string };
}

// -------------------------------------------------------------------------
// Constants
// -------------------------------------------------------------------------

/** Initial visible charts for layout mode A */
const DEFAULT_VISIBLE_CHARTS: ChartId[] = [
  'pareto-front',
  'parallel-coords',
  'scatter-matrix',
  'history',
];

// -------------------------------------------------------------------------
// Store implementation
// -------------------------------------------------------------------------

export const useLayoutStore = create<LayoutState>()((set, get) => ({
  // -------------------------------------------------------------------------
  // Initial state
  // -------------------------------------------------------------------------
  layoutMode: 'A' as LayoutMode,
  visibleCharts: new Set<ChartId>(DEFAULT_VISIBLE_CHARTS),
  panelSizes: { leftPanel: 280, bottomPanel: 200 },
  freeModeLayout: null,
  layoutLoadError: null,

  // -------------------------------------------------------------------------
  // Actions
  // -------------------------------------------------------------------------

  /** Switches the layout mode (A–D) */
  setLayoutMode: (mode) => set({ layoutMode: mode }),

  /** Toggles a chart in visibleCharts: removes it if present, adds it if absent */
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
   * Returns the current layout as a LayoutConfig.
   * Converts the visibleCharts Set to an Array for JSON compatibility.
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
   * Applies a saved LayoutConfig, converting visibleCharts from Array back to Set.
   */
  loadLayout: (config) => {
    set({
      layoutMode: config.mode,
      visibleCharts: new Set(config.visibleCharts),
      panelSizes: config.panelSizes,
      freeModeLayout: config.freeModeLayout,
    });
  },

  /** Directly sets freeModeLayout (used when applying a preset or resetting) */
  setFreeModeLayout: (layout) => set({ freeModeLayout: layout }),

  /**
   * Updates only the grid position of the specified cell.
   * Called when a drag-and-drop operation completes. REQ-032
   */
  updateCellPosition: (cellId, gridRow, gridCol) => {
    const { freeModeLayout } = get();
    if (!freeModeLayout) return;
    const cells = freeModeLayout.cells.map((cell) =>
      cell.cellId === cellId ? { ...cell, gridRow, gridCol } : cell,
    );
    set({ freeModeLayout: { cells } });
  },

  /**
   * Adds a new chart cell to freeModeLayout.
   * cellId is auto-generated via crypto.randomUUID().
   */
  addCell: (chartId, gridRow, gridCol) => {
    const base = get().freeModeLayout ?? DEFAULT_FREE_LAYOUT;
    const cellId = crypto.randomUUID();
    set({ freeModeLayout: { cells: [...base.cells, { cellId, chartId, gridRow, gridCol }] } });
  },

  /**
   * Removes the cell with the given cellId.
   * Does not mutate state if the cellId does not exist.
   */
  removeCell: (cellId) => {
    const { freeModeLayout } = get();
    if (!freeModeLayout) return;
    const cells = freeModeLayout.cells.filter((cell) => cell.cellId !== cellId);
    if (cells.length === freeModeLayout.cells.length) return; // no change
    set({ freeModeLayout: { cells } });
  },

  /**
   * Parses a JSON string and calls loadLayout.
   * On parse or validation failure, sets layoutLoadError and resets to the default layout.
   * NFR-032: error message is shown when loading fails.
   */
  loadLayoutFromJson: (json) => {
    const ERR = 'レイアウトを読み込めませんでした';
    try {
      const config = JSON.parse(json) as LayoutConfig;
      // Validate required fields
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
