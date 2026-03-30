/**
 * AppShell — 4-area CSS Grid application skeleton (TASK-401)
 *
 * Manages the ToolBar, LeftPanel, MainCanvas, and BottomPanel areas.
 * Grid layout switches based on the current layoutStore mode.
 * File drag-and-drop triggers studyStore.loadJournal().
 * LeftPanel width and BottomPanel height are resizable via drag handles.
 */

import { useStudyStore } from '../../stores/studyStore'
import { useLayoutStore } from '../../stores/layoutStore'
import { ToolBar } from './ToolBar'
import { LeftPanel } from '../panels/LeftPanel'
import { BottomPanel } from '../panels/BottomPanel'
import { FreeLayoutCanvas } from './FreeLayoutCanvas'
import { ChartCatalogPanel } from './ChartCatalogPanel'
import type { DragEvent } from 'react'
import { useState, useEffect, useRef, useCallback } from 'react'

// Min/max constraints for panel sizes
const LEFT_MIN = 120
const LEFT_MAX = 600
const BOTTOM_MIN = 60
const BOTTOM_MAX = 600

// -------------------------------------------------------------------------
// Component
// -------------------------------------------------------------------------

/**
 * CSS Grid skeleton for the whole application. TC-401-01, TC-401-02, TC-401-E01
 */
export function AppShell() {
  const layoutMode = useLayoutStore((s) => s.layoutMode)
  const panelSizes = useLayoutStore((s) => s.panelSizes)
  const setPanelSizes = useLayoutStore((s) => s.setPanelSizes)
  const loadJournal = useStudyStore((s) => s.loadJournal)
  const isLoading = useStudyStore((s) => s.isLoading)
  const [isDragging, setIsDragging] = useState(false)
  let dragCounter = 0

  // -------------------------------------------------------------------------
  // Panel resize logic
  // -------------------------------------------------------------------------

  const [resizing, setResizing] = useState<'left' | 'bottom' | null>(null)
  const resizeDragRef = useRef<{
    startX: number
    startY: number
    startLeft: number
    startBottom: number
  } | null>(null)

  const startLeftResize = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault()
      resizeDragRef.current = {
        startX: e.clientX,
        startY: e.clientY,
        startLeft: panelSizes.leftPanel,
        startBottom: panelSizes.bottomPanel,
      }
      setResizing('left')
    },
    [panelSizes],
  )

  const startBottomResize = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault()
      resizeDragRef.current = {
        startX: e.clientX,
        startY: e.clientY,
        startLeft: panelSizes.leftPanel,
        startBottom: panelSizes.bottomPanel,
      }
      setResizing('bottom')
    },
    [panelSizes],
  )

  useEffect(() => {
    if (!resizing) return

    const onMove = (e: MouseEvent) => {
      const drag = resizeDragRef.current
      if (!drag) return
      if (resizing === 'left') {
        const delta = e.clientX - drag.startX
        const newLeft = Math.max(LEFT_MIN, Math.min(LEFT_MAX, drag.startLeft + delta))
        setPanelSizes({ leftPanel: newLeft, bottomPanel: drag.startBottom })
      } else {
        const delta = e.clientY - drag.startY
        const newBottom = Math.max(BOTTOM_MIN, Math.min(BOTTOM_MAX, drag.startBottom - delta))
        setPanelSizes({ leftPanel: drag.startLeft, bottomPanel: newBottom })
      }
    }

    const onUp = () => setResizing(null)

    document.addEventListener('mousemove', onMove)
    document.addEventListener('mouseup', onUp)
    return () => {
      document.removeEventListener('mousemove', onMove)
      document.removeEventListener('mouseup', onUp)
    }
  }, [resizing, setPanelSizes])

  // -------------------------------------------------------------------------
  // File drag-and-drop
  // -------------------------------------------------------------------------

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

  const handleDrop = (e: DragEvent<HTMLDivElement>) => {
    e.preventDefault()
    dragCounter = 0
    setIsDragging(false)
    const file = e.dataTransfer?.files?.[0]
    if (file) {
      void loadJournal(file)
    }
  }

  // -------------------------------------------------------------------------
  // Grid dimensions
  // -------------------------------------------------------------------------

  const leftWidth = panelSizes.leftPanel
  const bottomHeight = panelSizes.bottomPanel
  // 5px columns/rows for the drag handles
  const HANDLE_PX = 5

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
        gridTemplateRows: `auto 1fr ${HANDLE_PX}px ${bottomHeight}px`,
        gridTemplateColumns: `${leftWidth}px ${HANDLE_PX}px 1fr auto`,
        height: '100vh',
        width: '100vw',
        // Prevent text selection while resizing
        userSelect: resizing ? 'none' : undefined,
        cursor: resizing === 'left' ? 'col-resize' : resizing === 'bottom' ? 'row-resize' : undefined,
      }}
    >
      {/* ToolBar area: full width, top row */}
      <div style={{ gridColumn: '1 / -1', gridRow: '1' }}>
        <ToolBar />
      </div>

      {/* Left panel area */}
      <div
        data-testid="left-panel"
        style={{
          gridColumn: '1',
          gridRow: '2',
          overflowY: 'auto',
          borderRight: 'none',
          background: 'var(--bg-panel)',
        }}
      >
        <LeftPanel />
      </div>

      {/* Vertical resize handle (between LeftPanel and MainCanvas) */}
      <div
        data-testid="left-resize-handle"
        onMouseDown={startLeftResize}
        style={{
          gridColumn: '2',
          gridRow: '2',
          cursor: 'col-resize',
          background: resizing === 'left' ? 'var(--accent, #4f46e5)' : 'var(--border)',
          transition: resizing ? 'none' : 'background 0.15s',
          zIndex: 10,
        }}
        onMouseEnter={(e) => {
          if (!resizing) (e.currentTarget as HTMLDivElement).style.background = 'var(--accent, #4f46e5)'
        }}
        onMouseLeave={(e) => {
          if (!resizing) (e.currentTarget as HTMLDivElement).style.background = 'var(--border)'
        }}
      />

      {/* Main canvas area */}
      <div
        data-testid="main-canvas"
        style={{ gridColumn: '3', gridRow: '2', overflow: 'hidden', position: 'relative' }}
      >
        <FreeLayoutCanvas />
      </div>

      {/* Chart catalog panel: collapsible, right side */}
      <div style={{ gridColumn: '4', gridRow: '2' }}>
        <ChartCatalogPanel />
      </div>

      {/* Horizontal resize handle (between main area and BottomPanel) */}
      <div
        data-testid="bottom-resize-handle"
        onMouseDown={startBottomResize}
        style={{
          gridColumn: '1 / -1',
          gridRow: '3',
          cursor: 'row-resize',
          background: resizing === 'bottom' ? 'var(--accent, #4f46e5)' : 'var(--border)',
          transition: resizing ? 'none' : 'background 0.15s',
          zIndex: 10,
        }}
        onMouseEnter={(e) => {
          if (!resizing) (e.currentTarget as HTMLDivElement).style.background = 'var(--accent, #4f46e5)'
        }}
        onMouseLeave={(e) => {
          if (!resizing) (e.currentTarget as HTMLDivElement).style.background = 'var(--border)'
        }}
      />

      {/* Bottom panel area: full width, bottom row */}
      <div
        data-testid="bottom-panel"
        style={{
          gridColumn: '1 / -1',
          gridRow: '4',
          overflowY: 'auto',
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
