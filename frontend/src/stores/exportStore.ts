/**
 * Documentation.
 *
 * Documentation.
 * Design:
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 */

import { create } from 'zustand'
import { WasmLoader } from '../wasm/wasmLoader'
import type { SessionState, ClusterConfig } from '../types'
import { useSelectionStore } from './selectionStore'
import { useLayoutStore } from './layoutStore'

// -------------------------------------------------------------------------
// Type definitions
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 */
export type CsvTarget = 'all' | 'selected' | 'pareto' | 'cluster'

/**
 * Documentation.
 * Documentation.
 */
export interface PinnedTrial {
  /** Documentation. */
  index: number
  /** Optuna trial_id */
  trialId: number
  /** Documentation. */
  memo: string
  /** Documentation. */
  pinnedAt: string
}

/**
 * Documentation.
 * Documentation.
 */
export type ReportSection = 'summary' | 'pareto' | 'pinned' | 'history' | 'cluster'

/**
 * Documentation.
 */
interface ExportState {
  // Documentation.
  /** Documentation. */
  csvTarget: CsvTarget
  /** Documentation. */
  selectedColumns: string[]
  /** Documentation. */
  isExporting: boolean
  /** Documentation. */
  exportError: string | null

  // Documentation.
  /** Documentation. */
  pinnedTrials: PinnedTrial[]
  /** Documentation. */
  pinError: string | null

  // Documentation.
  /** Documentation. */
  reportSections: ReportSection[]
  /** Documentation. */
  isGeneratingReport: boolean
  /** Documentation. */
  reportError: string | null

  // --- Actions ---
  setCsvTarget: (target: CsvTarget) => void
  setSelectedColumns: (columns: string[]) => void
  exportCsv: (indices: Uint32Array) => Promise<void>
  pinTrial: (index: number, trialId: number) => void
  unpinTrial: (index: number) => void
  updatePinMemo: (index: number, memo: string) => void
  clearPinError: () => void
  clearExportError: () => void
  /** Documentation. */
  setReportSections: (sections: ReportSection[]) => void
  /** Documentation. */
  generateHtmlReport: (paretoIndices: Uint32Array) => Promise<void>
  /** Documentation. */
  clearReportError: () => void

  // Documentation.
  /** Documentation. */
  sessionState: SessionState | null
  /** Documentation. */
  isSavingSession: boolean
  /** Documentation. */
  sessionError: string | null
  /** Documentation. */
  sessionWarning: string | null

  /** Documentation. */
  saveSession: (studyId: number, journalPath: string) => Promise<void>
  /** Documentation. */
  loadSessionFromJson: (json: string) => void
  /** Documentation. */
  clearSessionMessages: () => void
}

// -------------------------------------------------------------------------
// Constants
// -------------------------------------------------------------------------

/** Documentation. */
export const MAX_PINS = 20

/** Documentation. */
export const SESSION_VERSION = '1.0'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 */
export const useExportStore = create<ExportState>()((set, get) => ({
  // -------------------------------------------------------------------------
  // Documentation.
  // -------------------------------------------------------------------------
  csvTarget: 'all',
  selectedColumns: [],
  isExporting: false,
  exportError: null,
  pinnedTrials: [],
  pinError: null,
  reportSections: ['summary', 'pareto', 'pinned', 'history', 'cluster'],
  isGeneratingReport: false,
  reportError: null,
  sessionState: null,
  isSavingSession: false,
  sessionError: null,
  sessionWarning: null,

  // -------------------------------------------------------------------------
  // Documentation.
  // -------------------------------------------------------------------------

  /**
   * Documentation.
   */
  setCsvTarget: (target) => set({ csvTarget: target }),

  /**
   * Documentation.
   * Documentation.
   */
  setSelectedColumns: (columns) => set({ selectedColumns: columns }),

  /**
   * Documentation.
   *
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   */
  exportCsv: async (indices) => {
    // Documentation.
    if (indices.length === 0) {
      set({ exportError: 'No data to export' })
      return
    }

    set({ isExporting: true, exportError: null })

    try {
      const wasm = await WasmLoader.getInstance()
      const { selectedColumns } = get()

      // Documentation.
      const columnsJson =
        selectedColumns.length > 0 ? JSON.stringify(selectedColumns) : JSON.stringify([])

      // Documentation.
      const csvContent = wasm.serializeCsv(Array.from(indices), columnsJson)

      // Documentation.
      _downloadCsv(csvContent, `tunny-export-${Date.now()}.csv`)
    } catch (e) {
      set({ exportError: e instanceof Error ? e.message : 'Export failed' })
    } finally {
      set({ isExporting: false })
    }
  },

  /**
   * Documentation.
   * Documentation.
   */
  pinTrial: (index, trialId) => {
    const { pinnedTrials } = get()

    // Documentation.
    if (pinnedTrials.some((p) => p.index === index)) {
      return
    }

    // Documentation.
    if (pinnedTrials.length >= MAX_PINS) {
      set({ pinError: `Limit is ${MAX_PINS}. Please remove an old pin first.` })
      return
    }

    const newPin: PinnedTrial = {
      index,
      trialId,
      memo: '',
      pinnedAt: new Date().toISOString(),
    }

    set({ pinnedTrials: [...pinnedTrials, newPin], pinError: null })
  },

  /**
   * Documentation.
   */
  unpinTrial: (index) => {
    set((s) => ({
      pinnedTrials: s.pinnedTrials.filter((p) => p.index !== index),
    }))
  },

  /**
   * Documentation.
   */
  updatePinMemo: (index, memo) => {
    set((s) => ({
      pinnedTrials: s.pinnedTrials.map((p) => (p.index === index ? { ...p, memo } : p)),
    }))
  },

  /** Documentation. */
  clearPinError: () => set({ pinError: null }),

  /** Documentation. */
  clearExportError: () => set({ exportError: null }),

  /**
   * Documentation.
   */
  setReportSections: (sections) => set({ reportSections: sections }),

  /**
   * Documentation.
   *
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   *
   * 🟢 REQ-154〜REQ-155
   */
  generateHtmlReport: async (paretoIndices) => {
    set({ isGeneratingReport: true, reportError: null })

    try {
      const wasm = await WasmLoader.getInstance()
      const { reportSections, pinnedTrials } = get()

      // Documentation.
      const stats = wasm.computeReportStats()

      // Documentation.
      const html = _buildHtmlReport(reportSections, stats, pinnedTrials, paretoIndices)

      // Documentation.
      _downloadFile(html, `tunny-report-${Date.now()}.html`, 'text/html;charset=utf-8')
    } catch (e) {
      set({
        reportError: e instanceof Error ? e.message : 'Report generation failed',
      })
    } finally {
      set({ isGeneratingReport: false })
    }
  },

  /** Documentation. */
  clearReportError: () => set({ reportError: null }),

  /**
   * Documentation.
   *
   * Documentation.
   *   - filterRanges / selectedIndices / colorMode (selectionStore)
   *   - layoutMode / freeModeLayout / visibleCharts (layoutStore)
   *   - pinnedTrials (exportStore)
   *
   * 🟢 REQ-157
   */
  saveSession: async (studyId, journalPath) => {
    set({ isSavingSession: true, sessionError: null })

    try {
      const { pinnedTrials } = get()
      const sel = useSelectionStore.getState()
      const layout = useLayoutStore.getState()

      const session: SessionState = {
        version: SESSION_VERSION,
        journalPath,
        selectedStudyId: studyId,
        filterRanges: sel.filterRanges,
        selectedIndices: Array.from(sel.selectedIndices),
        colorMode: sel.colorMode,
        clusterConfig: null as ClusterConfig | null, // Documentation.
        layoutMode: layout.layoutMode,
        visibleCharts: Array.from(layout.visibleCharts),
        pinnedTrials: pinnedTrials.map((p) => ({
          trialId: p.trialId,
          note: p.memo,
          pinnedAt: new Date(p.pinnedAt).getTime(),
        })),
        freeModeLayout: layout.freeModeLayout,
        savedAt: new Date().toISOString(),
      }

      const json = JSON.stringify(session, null, 2)
      const dateStr = new Date().toISOString().slice(0, 10).replace(/-/g, '')
      _downloadFile(json, `session_${dateStr}.json`, 'application/json;charset=utf-8')

      set({ sessionState: session })
    } catch (e) {
      set({
        sessionError: e instanceof Error ? e.message : 'Failed to save session',
      })
    } finally {
      set({ isSavingSession: false })
    }
  },

  /**
   * Documentation.
   *
   * Documentation.
   * Documentation.
   * Documentation.
   *
   * 🟢 REQ-157
   */
  loadSessionFromJson: (json) => {
    set({ sessionError: null, sessionWarning: null })

    let session: SessionState
    try {
      session = JSON.parse(json) as SessionState
    } catch {
      set({ sessionError: 'Invalid session file format' })
      return
    }

    // Documentation.
    if (session.version !== SESSION_VERSION) {
      set({
        sessionWarning: 'Old session version. Some settings may not be restored.',
      })
    }

    // Documentation.
    const sel = useSelectionStore.getState()
    sel.brushSelect(new Uint32Array(session.selectedIndices ?? []))
    // Documentation.
    sel.clearSelection()
    if (session.filterRanges) {
      Object.entries(session.filterRanges).forEach(([axis, range]) => {
        sel.addAxisFilter(axis, range.min, range.max)
      })
    }
    if (session.colorMode) {
      sel.setColorMode(session.colorMode)
    }

    // Documentation.
    const layout = useLayoutStore.getState()
    if (session.layoutMode) {
      layout.setLayoutMode(session.layoutMode)
    }
    if (session.freeModeLayout !== undefined) {
      layout.loadLayout({
        mode: session.layoutMode ?? 'A',
        visibleCharts: session.visibleCharts ?? [],
        panelSizes: { leftPanel: 280, bottomPanel: 200 },
        freeModeLayout: session.freeModeLayout,
      })
    }

    // Documentation.
    if (Array.isArray(session.pinnedTrials)) {
      const restored = session.pinnedTrials.map((p, i) => ({
        index: i, // Documentation.
        trialId: p.trialId,
        memo: p.note ?? '',
        pinnedAt:
          typeof p.pinnedAt === 'number'
            ? new Date(p.pinnedAt).toISOString()
            : new Date().toISOString(),
      }))
      set({ pinnedTrials: restored.slice(0, MAX_PINS) })
    }

    set({ sessionState: session })
  },

  /** Documentation. */
  clearSessionMessages: () => set({ sessionError: null, sessionWarning: null }),
}))

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 */
function _downloadCsv(content: string, filename: string): void {
  // Documentation.
  const bom = '\uFEFF'
  const blob = new Blob([bom + content], { type: 'text/csv;charset=utf-8;' })
  const url = URL.createObjectURL(blob)

  // Documentation.
  const link = document.createElement('a')
  link.href = url
  link.download = filename
  link.style.display = 'none'
  document.body.appendChild(link)
  link.click()

  // Documentation.
  document.body.removeChild(link)
  URL.revokeObjectURL(url)
}

/**
 * Documentation.
 */
function _downloadFile(content: string, filename: string, mimeType: string): void {
  const blob = new Blob([content], { type: mimeType })
  const url = URL.createObjectURL(blob)

  const link = document.createElement('a')
  link.href = url
  link.download = filename
  link.style.display = 'none'
  document.body.appendChild(link)
  link.click()
  document.body.removeChild(link)
  URL.revokeObjectURL(url)
}

/**
 * Documentation.
 *
 * Design:
 * Documentation.
 * Documentation.
 * Documentation.
 * 🟢 REQ-154〜REQ-155
 */
function _buildHtmlReport(
  sections: ReportSection[],
  statsJson: string,
  pinnedTrials: PinnedTrial[],
  paretoIndices: Uint32Array,
): string {
  // Documentation.
  const embeddedData = JSON.stringify({
    generatedAt: new Date().toISOString(),
    stats: JSON.parse(statsJson || '{}'),
    pinnedTrials: pinnedTrials.map((p) => ({
      trialId: p.trialId,
      memo: p.memo,
      pinnedAt: p.pinnedAt,
    })),
    paretoCount: paretoIndices.length,
  })

  // Documentation.
  const sectionHtml = sections
    .map((sec) => _buildSectionHtml(sec, pinnedTrials, paretoIndices))
    .join('\n')

  return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Tunny Dashboard Report</title>
<style>
body{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;margin:0;padding:2rem;background:#f8fafc;color:#1e293b;}
h1{font-size:1.75rem;font-weight:700;border-bottom:2px solid #3b82f6;padding-bottom:0.5rem;margin-bottom:1.5rem;}
h2{font-size:1.25rem;font-weight:600;color:#1e40af;margin-top:2rem;}
table{border-collapse:collapse;width:100%;margin-top:0.5rem;background:#fff;border-radius:0.5rem;overflow:hidden;box-shadow:0 1px 3px rgba(0,0,0,.1);}
th,td{border:1px solid #e2e8f0;padding:0.5rem 0.75rem;font-size:0.85rem;}
th{background:#1e40af;color:#fff;text-align:left;}
tr:nth-child(even){background:#f1f5f9;}
.note{color:#64748b;font-size:0.8rem;margin-top:0.25rem;}
@media print{body{background:#fff;padding:1rem;}.no-print{display:none;}}
</style>
</head>
<body>
<h1>Tunny Dashboard Report</h1>
<p class="note">Generated: ${new Date().toLocaleString('en-US')}</p>
<div class="no-print" style="margin:1rem 0;">
  <button onclick="window.print()" style="background:#3b82f6;color:#fff;border:none;padding:0.5rem 1rem;border-radius:0.375rem;cursor:pointer;">Print as PDF</button>
</div>
${sectionHtml}
<script>
// Documentation.
const REPORT_DATA = ${embeddedData};
</script>
</body>
</html>`
}

/** Documentation. */
function _buildSectionHtml(
  section: ReportSection,
  pinnedTrials: PinnedTrial[],
  paretoIndices: Uint32Array,
): string {
  switch (section) {
    case 'summary':
      return '<h2>Statistics Summary</h2><p class="note">Refer to embedded data (REPORT_DATA.stats) for parameter and objective statistics.</p>'
    case 'pareto':
      return `<h2>Pareto Solutions</h2><p>Pareto optimal solutions: ${paretoIndices.length}</p>`
    case 'pinned':
      if (pinnedTrials.length === 0) {
        return '<h2>Pinned Trials</h2><p class="note">No trials have been pinned.</p>'
      }
      return (
        '<h2>Pinned Trials</h2>' +
        '<table><thead><tr><th>Trial ID</th><th>Pinned At</th><th>Memo</th></tr></thead><tbody>' +
        pinnedTrials
          .map(
            (p) =>
              `<tr><td>${p.trialId}</td><td>${new Date(p.pinnedAt).toLocaleString('en-US')}</td><td>${_escapeHtml(p.memo)}</td></tr>`,
          )
          .join('') +
        '</tbody></table>'
      )
    case 'history':
      return '<h2>Optimization History</h2><p class="note">Optimization history charts are displayed when Plotly.js is available.</p>'
    case 'cluster':
      return '<h2>Cluster Analysis</h2><p class="note">Refer to the embedded data for cluster analysis results.</p>'
    default:
      return ''
  }
}

/** Documentation. */
function _escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;')
}
