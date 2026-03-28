/**
 * ToolBar — ファイル読込・Study選択・レイアウト切替 UI (TASK-401)
 *
 * 【役割】: Journalファイル選択・レイアウトモード A〜D の切り替えボタンを提供
 * 【設計方針】: studyStore.loadJournal() と layoutStore.setLayoutMode() に接続
 * 🟢 ファイル input の change イベント → loadJournal(file) を呼び出す
 */

import type { ChangeEvent } from 'react';
import { useStudyStore } from '../../stores/studyStore';
import { useLiveUpdateStore } from '../../stores/liveUpdateStore';
import { LayoutTabBar } from './LayoutTabBar';

// -------------------------------------------------------------------------
// コンポーネント実装
// -------------------------------------------------------------------------

/**
 * 【機能概要】: アプリ上部の操作バー
 * 【テスト対応】: TC-401-03〜06
 */
export function ToolBar() {
  // 【Store接続】: loadJournal と setLayoutMode を取得 🟢
  const isLive = useLiveUpdateStore((s) => s.isLive);
  const isSupported = useLiveUpdateStore((s) => s.isSupported);
  const startLive = useLiveUpdateStore((s) => s.startLive);
  const stopLive = useLiveUpdateStore((s) => s.stopLive);
  const loadJournal = useStudyStore((s) => s.loadJournal);
  const isLoading = useStudyStore((s) => s.isLoading);
  const loadError = useStudyStore((s) => s.loadError);
  const currentStudy = useStudyStore((s) => s.currentStudy);
  const allStudies = useStudyStore((s) => s.allStudies);
  const selectStudy = useStudyStore((s) => s.selectStudy);

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

      {/* 【Study セレクタ】: 2件以上のときドロップダウンを表示 */}
      {(allStudies?.length ?? 0) > 1 ? (
        <select
          data-testid="study-select"
          value={currentStudy?.studyId ?? ''}
          onChange={(e) => selectStudy?.(Number(e.target.value))}
          style={{ fontSize: '13px', padding: '2px 6px', border: '1px solid var(--border)', borderRadius: '4px', color: 'var(--accent)', background: 'var(--bg)', fontWeight: 600 }}
        >
          {allStudies!.map((s) => (
            <option key={s.studyId} value={s.studyId}>
              {s.name} — {s.completedTrials} trials
            </option>
          ))}
        </select>
      ) : (
        /* 【Study情報】: 単一 Study のときはラベル表示 */
        currentStudy && (
          <span data-testid="toolbar-study-info" style={{ fontSize: '13px', color: 'var(--accent)', fontWeight: 600 }}>
            {currentStudy.name} — {currentStudy.completedTrials} trials
          </span>
        )
      )}

      {/* 【レイアウトタブバー + ライブ更新ボタン】: 右端に配置 🟢 */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginLeft: 'auto' }}>
        {/* 【LayoutTabBar】: タブ型レイアウト切替 🟢 REQ-001 */}
        <LayoutTabBar />

        {/* 【ライブ更新ボタン】: Journal ファイルの差分ポーリング ON/OFF 🟢 REQ-104 */}
        <button
          data-testid="live-update-btn"
          disabled={!isSupported}
          title={isSupported ? undefined : 'このブラウザは対応していません（Chrome/Edge 推奨）'}
          onClick={isLive ? stopLive : startLive}
          style={{
            padding: '2px 10px',
            border: '1px solid var(--border)',
            borderRadius: '4px',
            cursor: isSupported ? 'pointer' : 'not-allowed',
            fontSize: '13px',
            fontWeight: 500,
            background: isLive ? '#c0392b' : '#27ae60',
            color: '#fff',
            opacity: isSupported ? 1 : 0.5,
          }}
        >
          {isLive ? 'ライブ停止' : 'ライブ開始'}
        </button>
      </div>
    </div>
  );
}
