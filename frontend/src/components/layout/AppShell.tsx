/**
 * AppShell — 4-area CSS Grid application skeleton (TASK-401)
 *
 * Manages the ToolBar, LeftPanel, MainCanvas, and BottomPanel areas.
 * Grid layout switches based on the current layoutStore mode.
 * File drag-and-drop triggers studyStore.loadJournal().
 */

import { useStudyStore } from '../../stores/studyStore'
import { useLayoutStore } from '../../stores/layoutStore'
import { ToolBar } from './ToolBar'
import { LeftPanel } from '../panels/LeftPanel'
import { BottomPanel } from '../panels/BottomPanel'
import { FreeLayoutCanvas } from './FreeLayoutCanvas'
import { ChartCatalogPanel } from './ChartCatalogPanel'
import type { DragEvent } from 'react'
import { useState } from 'react'

// -------------------------------------------------------------------------
// Component
// -------------------------------------------------------------------------

/**
 * CSS Grid skeleton for the whole application. TC-401-01, TC-401-02, TC-401-E01
 */
export function AppShell() {
  // Read layoutMode and loadJournal from their respective stores
  const layoutMode = useLayoutStore((s) => s.layoutMode)
  const loadJournal = useStudyStore((s) => s.loadJournal)
  const isLoading = useStudyStore((s) => s.isLoading)
  const [isDragging, setIsDragging] = useState(false)
  let dragCounter = 0

  /**
   * Prevents the default browser behavior to allow file drop.
   */
  const handleDragOver = (e: DragEvent<HTMLDivElement>) => {
    e.preventDefault()
  }

  const handleDragEnter = (e: DragEvent<HTMLDivElement>) => {
    e.preventDefault()
    dragCounter++
    setIsDragging(true)
  }

  const handleDragLeave = (e: DragEvent<HTMLDivElement>) => {
    e.preventDefault()
    dragCounter--
    if (dragCounter <= 0) {
      dragCounter = 0
      setIsDragging(false)
    }
  }

  /**
   * Passes the first dropped file to loadJournal. Multiple files are ignored.
   */
  const handleDrop = (e: DragEvent<HTMLDivElement>) => {
    e.preventDefault()
    dragCounter = 0
    setIsDragging(false)
    const file = e.dataTransfer?.files?.[0]
    if (file) {
      // fire-and-forget async load
      void loadJournal(file)
    }
  }

  return (
    <div
      data-testid="app-shell"
      data-layout={layoutMode}
      onDragOver={handleDragOver}
      onDragEnter={handleDragEnter}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      style={{
        display: 'grid',
        gridTemplateRows: 'auto 1fr auto',
        gridTemplateColumns: 'auto 1fr auto',
        height: '100vh',
        width: '100vw',
      }}
    >
      {/* ToolBar area: full width, top row */}
      <div style={{ gridColumn: '1 / 4' }}>
        <ToolBar />
      </div>

      {/* Left panel area */}
      <div
        data-testid="left-panel"
        style={{
          width: '260px',
          overflowY: 'auto',
          borderRight: '1px solid var(--border)',
          background: 'var(--bg-panel)',
        }}
      >
        <LeftPanel />
      </div>

      {/* Main canvas area */}
      <div
        data-testid="main-canvas"
        style={{ overflow: 'hidden', position: 'relative', height: '100%' }}
      >
        <FreeLayoutCanvas />
      </div>

      {/* Chart catalog panel: collapsible, right side */}
      <ChartCatalogPanel />

      {/* Bottom panel area: full width, bottom row */}
      <div
        data-testid="bottom-panel"
        style={{
          gridColumn: '1 / 4',
          height: '220px',
          overflowY: 'auto',
          borderTop: '1px solid var(--border)',
        }}
      >
        <BottomPanel />
      </div>

      {/* Progress bar shown while loading */}
      {isLoading && (
        <div
          data-testid="loading-indicator"
          style={{
            position: 'fixed',
            top: 0,
            left: 0,
            right: 0,
            height: '4px',
            background: 'var(--accent, #4f46e5)',
            zIndex: 9999,
          }}
        />
      )}

      {/* Drag-and-drop overlay */}
      {isDragging && (
        <div
          data-testid="drop-overlay"
          style={{
            position: 'fixed',
            inset: 0,
            background: 'rgba(79, 70, 229, 0.12)',
            border: '3px dashed var(--accent, #4f46e5)',
            borderRadius: '8px',
            zIndex: 9998,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            pointerEvents: 'none',
          }}
        >
          <span style={{ fontSize: '20px', fontWeight: 700, color: 'var(--accent, #4f46e5)' }}>
            Drop journal file here
          </span>
        </div>
      )}
    </div>
  )
}
