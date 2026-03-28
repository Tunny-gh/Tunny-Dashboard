/**
 * LayoutTabBar — タブ型レイアウト切替コンポーネント (TASK-001)
 *
 * 【役割】: A〜D レイアウトモードをタブUIで切り替える
 * 【設計方針】:
 *   - プリセットタブ（A/B/C）クリック: setLayoutMode + setFreeModeLayout を同時実行
 *   - フリータブ（D）クリック: setLayoutMode のみ（freeModeLayout 維持）
 *   - アクティブタブ再クリック: べき等（何もしない）
 * 🟢 REQ-001, REQ-002, REQ-101〜106, REQ-401〜405, REQ-501〜506, REQ-601, REQ-602
 */

import { useLayoutStore } from '../../stores/layoutStore';
import type { LayoutMode, FreeModeLayout } from '../../types';

// -------------------------------------------------------------------------
// プリセットレイアウト定義
// -------------------------------------------------------------------------

/**
 * 【プリセット生成ヘルパー】: cellId を UUID で自動付与してレイアウトを生成する
 */
const makePresetLayout = (cells: Array<Omit<FreeModeLayout['cells'][number], 'cellId'>>): FreeModeLayout => ({
  cells: cells.map((c) => ({ ...c, cellId: crypto.randomUUID() })),
});

/** 【プリセットレイアウト】: Mode A〜C に対応するレイアウト定義（クリック毎に新 UUID 生成） */
const PRESET_LAYOUTS: Record<Exclude<LayoutMode, 'D'>, () => FreeModeLayout> = {
  A: () => makePresetLayout([
    { chartId: 'pareto-front', gridRow: [1, 3], gridCol: [1, 3] },
    { chartId: 'parallel-coords', gridRow: [1, 3], gridCol: [3, 5] },
    { chartId: 'scatter-matrix', gridRow: [3, 5], gridCol: [1, 3] },
    { chartId: 'history', gridRow: [3, 5], gridCol: [3, 5] },
  ]),
  B: () => makePresetLayout([
    { chartId: 'pareto-front', gridRow: [1, 5], gridCol: [1, 3] },
    { chartId: 'parallel-coords', gridRow: [1, 3], gridCol: [3, 5] },
    { chartId: 'hypervolume', gridRow: [3, 5], gridCol: [3, 5] },
  ]),
  C: () => makePresetLayout([
    { chartId: 'pareto-front', gridRow: [1, 3], gridCol: [1, 3] },
    { chartId: 'parallel-coords', gridRow: [1, 3], gridCol: [3, 5] },
    { chartId: 'scatter-matrix', gridRow: [3, 5], gridCol: [1, 5] },
  ]),
};

// -------------------------------------------------------------------------
// タブ定義
// -------------------------------------------------------------------------

/** 【タブ一覧】: 表示順と対応モード・ラベル 🟢 REQ-002, REQ-506 */
const LAYOUT_TABS: Array<{ mode: LayoutMode; label: string }> = [
  { mode: 'A', label: '4分割' },
  { mode: 'B', label: '左大' },
  { mode: 'C', label: '縦並び' },
  { mode: 'D', label: 'フリー' },
];

// -------------------------------------------------------------------------
// コンポーネント実装
// -------------------------------------------------------------------------

/**
 * 【機能概要】: レイアウトモードを切り替えるタブバー
 * 【テスト対応】: TC-LT-01〜07
 */
export function LayoutTabBar() {
  // 【Store接続】: layoutMode と各アクションを取得
  const layoutMode = useLayoutStore((s) => s.layoutMode);
  const setLayoutMode = useLayoutStore((s) => s.setLayoutMode);
  const setFreeModeLayout = useLayoutStore((s) => s.setFreeModeLayout);

  /**
   * 【クリックハンドラ】: タブクリックでモード切替とプリセット適用を行う
   * - アクティブタブ再クリックはべき等（REQ-106）
   * - Mode D は freeModeLayout を変更しない（REQ-104）
   * - Mode A/B/C はプリセットを即時適用（REQ-101〜103, REQ-602）
   */
  const handleClick = (mode: LayoutMode) => {
    if (mode === layoutMode) return;
    setLayoutMode(mode);
    if (mode !== 'D') {
      setFreeModeLayout(PRESET_LAYOUTS[mode]());
    }
  };

  return (
    <div
      data-testid="layout-tab-bar"
      style={{ display: 'flex', gap: '2px' }}
    >
      {LAYOUT_TABS.map(({ mode, label }) => {
        const isActive = layoutMode === mode;
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
        );
      })}
    </div>
  );
}
