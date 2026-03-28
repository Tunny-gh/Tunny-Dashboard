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
  const isLoading = useStudyStore((s) => s.isLoading);
  const loadError = useStudyStore((s) => s.loadError);
  const currentStudy = useStudyStore((s) => s.currentStudy);
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
      style={{ display: 'flex', alignItems: 'center', gap: '8px', padding: '4px 12px', background: 'var(--bg-header)', borderBottom: '1px solid var(--border)' }}
    >
      {/* 【ファイル入力】: Journalファイルを選択する input 🟢 */}
      <input
        data-testid="file-input"
        type="file"
        accept=".log,.journal,.txt"
        onChange={handleFileChange}
        style={{ fontSize: '14px' }}
      />

      {/* 【ローディング表示】 */}
      {isLoading && (
        <span data-testid="toolbar-loading" style={{ fontSize: '13px', color: 'var(--text-muted)' }}>
          読み込み中...
        </span>
      )}

      {/* 【エラー表示】 */}
      {loadError && (
        <span data-testid="toolbar-error" style={{ fontSize: '13px', color: '#dc2626', maxWidth: '300px', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }} title={loadError}>
          エラー: {loadError}
        </span>
      )}

      {/* 【Study情報】 */}
      {currentStudy && (
        <span data-testid="toolbar-study-info" style={{ fontSize: '13px', color: 'var(--accent)', fontWeight: 600 }}>
          {currentStudy.name} — {currentStudy.completedTrials} trials
        </span>
      )}

      {/* 【レイアウトモードボタン群】: A〜D の切り替えボタン 🟢 */}
      <div style={{ display: 'flex', gap: '4px', marginLeft: 'auto' }}>
        {LAYOUT_MODES.map((mode) => (
          <button
            key={mode}
            data-testid={`layout-btn-${mode}`}
            onClick={() => setLayoutMode(mode)}
            style={{
              padding: '2px 10px',
              border: '1px solid var(--accent)',
              borderRadius: '4px',
              cursor: 'pointer',
              fontSize: '13px',
              background: 'var(--bg)',
              color: 'var(--accent)',
              fontWeight: 500,
            }}
          >
            {mode}
          </button>
        ))}
      </div>
    </div>
  );
}
