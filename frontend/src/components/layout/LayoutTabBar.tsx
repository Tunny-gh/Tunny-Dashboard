/**
 * LayoutTabBar — tab-based layout switcher component (TASK-001)
 *
 * Switches between layout modes A–D via a tab UI.
 * - Preset tabs (A/B/C): calls setLayoutMode + setFreeModeLayout together
 * - Free tab (D): calls setLayoutMode only (freeModeLayout is preserved)
 * - Re-clicking the active tab: idempotent (no-op)
 * REQ-001, REQ-002, REQ-101–106, REQ-401–405, REQ-501–506, REQ-601, REQ-602
 */

import { useLayoutStore } from '../../stores/layoutStore'
import type { LayoutMode, FreeModeLayout } from '../../types'

// -------------------------------------------------------------------------
// Preset layout definitions
// -------------------------------------------------------------------------

/**
 * Generates a FreeModeLayout by auto-assigning a UUID to each cell's cellId.
 */
const makePresetLayout = (
  cells: Array<Omit<FreeModeLayout['cells'][number], 'cellId'>>,
): FreeModeLayout => ({
  cells: cells.map((c) => ({ ...c, cellId: crypto.randomUUID() })),
})

/** Preset layouts for modes A–C. New UUIDs are generated on each click. */
const PRESET_LAYOUTS: Record<Exclude<LayoutMode, 'D'>, () => FreeModeLayout> = {
  A: () =>
    makePresetLayout([
      { chartId: 'pareto-front', gridRow: [1, 3], gridCol: [1, 3] },
      { chartId: 'parallel-coords', gridRow: [1, 3], gridCol: [3, 5] },
      { chartId: 'scatter-matrix', gridRow: [3, 5], gridCol: [1, 3] },
      { chartId: 'history', gridRow: [3, 5], gridCol: [3, 5] },
    ]),
  B: () =>
    makePresetLayout([
      { chartId: 'pareto-front', gridRow: [1, 5], gridCol: [1, 3] },
      { chartId: 'parallel-coords', gridRow: [1, 3], gridCol: [3, 5] },
      { chartId: 'hypervolume', gridRow: [3, 5], gridCol: [3, 5] },
    ]),
  C: () =>
    makePresetLayout([
      { chartId: 'pareto-front', gridRow: [1, 3], gridCol: [1, 3] },
      { chartId: 'parallel-coords', gridRow: [1, 3], gridCol: [3, 5] },
      { chartId: 'scatter-matrix', gridRow: [3, 5], gridCol: [1, 5] },
    ]),
}

// -------------------------------------------------------------------------
// Tab definitions
// -------------------------------------------------------------------------

/** Display order, mode, and label for each tab. REQ-002, REQ-506 */
const LAYOUT_TABS: Array<{ mode: LayoutMode; label: string }> = [
  { mode: 'A', label: 'Quad' },
  { mode: 'B', label: 'Left Main' },
  { mode: 'C', label: 'Vertical' },
  { mode: 'D', label: 'Free' },
]

// -------------------------------------------------------------------------
// Component
// -------------------------------------------------------------------------

/**
 * Tab bar for switching the layout mode. TC-LT-01–07
 */
export function LayoutTabBar() {
  // Read layoutMode and actions from the store
  const layoutMode = useLayoutStore((s) => s.layoutMode)
  const setLayoutMode = useLayoutStore((s) => s.setLayoutMode)
  const setFreeModeLayout = useLayoutStore((s) => s.setFreeModeLayout)

  /**
   * Handles a tab click: switches mode and applies the preset layout when relevant.
   * Re-clicking the active tab is a no-op (REQ-106).
   * Mode D leaves freeModeLayout unchanged (REQ-104).
   * Modes A/B/C apply the preset immediately (REQ-101–103, REQ-602).
   */
  const handleClick = (mode: LayoutMode) => {
    if (mode === layoutMode) return
    setLayoutMode(mode)
    if (mode !== 'D') {
      setFreeModeLayout(PRESET_LAYOUTS[mode]())
    }
  }

  return (
    <div data-testid="layout-tab-bar" style={{ display: 'flex', gap: '2px' }}>
      {LAYOUT_TABS.map(({ mode, label }) => {
        const isActive = layoutMode === mode
        return (
          <button
            key={mode}
            data-testid={`layout-tab-${mode}`}
            aria-selected={isActive}
            onClick={() => handleClick(mode)}
            style={{
              padding: '4px 14px',
              fontSize: '13px',
              border: isActive ? 'none' : '1px solid var(--border)',
              borderRadius: '4px 4px 0 0',
              cursor: 'pointer',
              fontWeight: isActive ? 700 : 500,
              background: isActive ? 'var(--accent)' : 'var(--bg)',
              color: isActive ? '#fff' : 'var(--text-muted)',
            }}
          >
            {label}
          </button>
        )
      })}
    </div>
  )
}
