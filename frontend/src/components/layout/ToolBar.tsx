/**
 * ToolBar — ファイル読込・Study選択・レイアウト切替 UI (TASK-401)
 *
 * 【役割】: Journalファイル選択・レイアウトモード A〜D の切り替えボタンを提供
 * 【設計方針】: studyStore.loadJournal() と layoutStore.setLayoutMode() に接続
 * 🟢 ファイル input の change イベント → loadJournal(file) を呼び出す
 */

import type { ChangeEvent } from 'react';
import { useStudyStore } from '../../stores/studyStore';
import { useLayoutStore } from '../../stores/layoutStore';
import type { LayoutMode } from '../../types';

// -------------------------------------------------------------------------
// レイアウトモード定義
// -------------------------------------------------------------------------

/** 🟢 レイアウトモードの一覧（A〜D） */
const LAYOUT_MODES: LayoutMode[] = ['A', 'B', 'C', 'D'];

// -------------------------------------------------------------------------
// コンポーネント実装
// -------------------------------------------------------------------------

/**
 * 【機能概要】: アプリ上部の操作バー
 * 【テスト対応】: TC-401-03〜06
 */
export function ToolBar() {
  // 【Store接続】: loadJournal と setLayoutMode を取得 🟢
  const loadJournal = useStudyStore((s) => s.loadJournal);
  const setLayoutMode = useLayoutStore((s) => s.setLayoutMode);

  /**
   * 【ファイル選択ハンドラ】: input[type=file] の change イベントで loadJournal を呼ぶ
   * 【ファイル検証】: files が空の場合は無視
   */
  const handleFileChange = (e: ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      // 【Journal読み込み開始】: fire-and-forget
      void loadJournal(file);
    }
  };

  // 【レンダリング】: ファイル input + レイアウトモードボタン群 🟢
  return (
    <div
      data-testid="toolbar"
      style={{ display: 'flex', alignItems: 'center', gap: '8px', padding: '4px 8px', borderBottom: '1px solid #e5e7eb' }}
    >
      {/* 【ファイル入力】: Journalファイルを選択する input 🟢 */}
      <input
        data-testid="file-input"
        type="file"
        accept=".log,.journal"
        onChange={handleFileChange}
        style={{ fontSize: '14px' }}
      />

      {/* 【レイアウトモードボタン群】: A〜D の切り替えボタン 🟢 */}
      <div style={{ display: 'flex', gap: '4px', marginLeft: 'auto' }}>
        {LAYOUT_MODES.map((mode) => (
          <button
            key={mode}
            data-testid={`layout-btn-${mode}`}
            onClick={() => setLayoutMode(mode)}
            style={{
              padding: '2px 8px',
              border: '1px solid #d1d5db',
              borderRadius: '4px',
              cursor: 'pointer',
              fontSize: '13px',
            }}
          >
            {mode}
          </button>
        ))}
      </div>
    </div>
  );
}
