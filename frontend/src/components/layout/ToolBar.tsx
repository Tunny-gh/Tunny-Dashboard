/**
 * ToolBar — file loading, study selection, and layout switching UI (TASK-401)
 *
 * Provides journal file selection and layout mode A–D switching.
 * Connects to studyStore.loadJournal() and layoutStore.setLayoutMode().
 * File input change event triggers loadJournal(file).
 */

import type { ChangeEvent } from 'react'
import { useStudyStore } from '../../stores/studyStore'
import { useLiveUpdateStore } from '../../stores/liveUpdateStore'
import { LayoutTabBar } from './LayoutTabBar'

// -------------------------------------------------------------------------
// Component
// -------------------------------------------------------------------------

/**
 * Top-level operation bar. TC-401-03–06
 */
export function ToolBar() {
  // Read store state and actions
  const isLive = useLiveUpdateStore((s) => s.isLive)
  const isSupported = useLiveUpdateStore((s) => s.isSupported)
  const startLive = useLiveUpdateStore((s) => s.startLive)
  const stopLive = useLiveUpdateStore((s) => s.stopLive)
  const loadJournal = useStudyStore((s) => s.loadJournal)
  const isLoading = useStudyStore((s) => s.isLoading)
  const loadError = useStudyStore((s) => s.loadError)
  const currentStudy = useStudyStore((s) => s.currentStudy)
  const allStudies = useStudyStore((s) => s.allStudies)
  const selectStudy = useStudyStore((s) => s.selectStudy)

  /**
   * Calls loadJournal when the file input changes. Ignores empty file lists.
   */
  const handleFileChange = (e: ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (file) {
      // fire-and-forget
      void loadJournal(file)
    }
  }

  return (
    <div
      data-testid="toolbar"
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: '8px',
        padding: '4px 12px',
        background: 'var(--bg-header)',
        borderBottom: '1px solid var(--border)',
      }}
    >
      {/* Journal file selector */}
      <input
        data-testid="file-input"
        type="file"
        accept=".log,.journal,.txt"
        onChange={handleFileChange}
        style={{ fontSize: '14px' }}
      />

      {/* Loading indicator */}
      {isLoading && (
        <span
          data-testid="toolbar-loading"
          style={{ fontSize: '13px', color: 'var(--text-muted)' }}
        >
          読み込み中...
        </span>
      )}

      {/* Error message */}
      {loadError && (
        <span
          data-testid="toolbar-error"
          style={{
            fontSize: '13px',
            color: '#dc2626',
            maxWidth: '300px',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
          title={loadError}
        >
          エラー: {loadError}
        </span>
      )}

      {/* Study selector: shown as a dropdown when more than one study is available */}
      {(allStudies?.length ?? 0) > 1 ? (
        <select
          data-testid="study-select"
          value={currentStudy?.studyId ?? ''}
          onChange={(e) => selectStudy?.(Number(e.target.value))}
          style={{
            fontSize: '13px',
            padding: '2px 6px',
            border: '1px solid var(--border)',
            borderRadius: '4px',
            color: 'var(--accent)',
            background: 'var(--bg)',
            fontWeight: 600,
          }}
        >
          {allStudies!.map((s) => (
            <option key={s.studyId} value={s.studyId}>
              {s.name} — {s.completedTrials} trials
            </option>
          ))}
        </select>
      ) : (
        /* Single study: show as a label */
        currentStudy && (
          <span
            data-testid="toolbar-study-info"
            style={{ fontSize: '13px', color: 'var(--accent)', fontWeight: 600 }}
          >
            {currentStudy.name} — {currentStudy.completedTrials} trials
          </span>
        )
      )}

      {/* LayoutTabBar and live-update button, right-aligned */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginLeft: 'auto' }}>
        {/* Layout tab switcher — REQ-001 */}
        <LayoutTabBar />

        {/* Live update toggle: polls the journal file for diffs — REQ-104 */}
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
  )
}
