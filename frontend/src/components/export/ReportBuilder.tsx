/**
 * Documentation.
 *
 * Documentation.
 * Design:
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 */

import React, { useState, useRef } from 'react'
import { useExportStore } from '../../stores/exportStore'
import type { ReportSection } from '../../stores/exportStore'

// -------------------------------------------------------------------------
// Type definitions
// -------------------------------------------------------------------------

interface ReportBuilderProps {
  /** Documentation. */
  paretoIndices: Uint32Array
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/** Documentation. */
const SECTION_LABELS: Record<ReportSection, string> = {
  summary: 'Statistics Summary',
  pareto: 'Pareto Solutions',
  pinned: 'Pinned Trials',
  history: 'Optimization History',
  cluster: 'Cluster Analysis',
}

/** Documentation. */
const ALL_SECTIONS: ReportSection[] = ['summary', 'pareto', 'pinned', 'history', 'cluster']

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 */
export const ReportBuilder: React.FC<ReportBuilderProps> = ({ paretoIndices }) => {
  const {
    reportSections,
    isGeneratingReport,
    reportError,
    setReportSections,
    generateHtmlReport,
    clearReportError,
  } = useExportStore()

  // Documentation.
  const [enabledSections, setEnabledSections] = useState<Set<ReportSection>>(
    new Set(reportSections),
  )

  // Documentation.
  const dragIndex = useRef<number | null>(null)

  // -------------------------------------------------------------------------
  // Documentation.
  // -------------------------------------------------------------------------

  /** Documentation. */
  const handleToggleSection = (sec: ReportSection) => {
    const next = new Set(enabledSections)
    if (next.has(sec)) {
      next.delete(sec)
    } else {
      next.add(sec)
    }
    setEnabledSections(next)
    // Documentation.
    setReportSections(reportSections.filter((s) => next.has(s)))
  }

  /** Documentation. */
  const handleDragStart = (e: React.DragEvent, index: number) => {
    dragIndex.current = index
    e.dataTransfer.effectAllowed = 'move'
  }

  /** Documentation. */
  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault()
    e.dataTransfer.dropEffect = 'move'
  }

  /** Documentation. */
  const handleDrop = (e: React.DragEvent, targetIndex: number) => {
    e.preventDefault()
    if (dragIndex.current === null || dragIndex.current === targetIndex) return

    const next = [...reportSections]
    const [dragged] = next.splice(dragIndex.current, 1)
    next.splice(targetIndex, 0, dragged)
    setReportSections(next)
    dragIndex.current = null
  }

  /** Documentation. */
  const handleGenerate = () => {
    clearReportError()
    generateHtmlReport(paretoIndices)
  }

  // -------------------------------------------------------------------------
  // Documentation.
  // -------------------------------------------------------------------------

  /** Documentation. */
  const enabledList = reportSections.filter((s) => enabledSections.has(s))
  const disabledList = ALL_SECTIONS.filter((s) => !enabledSections.has(s))

  // -------------------------------------------------------------------------
  // Rendering
  // -------------------------------------------------------------------------

  return (
    <div data-testid="report-builder" className="p-4 space-y-4">
      {/* Documentation. */}
      <h3 className="text-base font-semibold text-gray-800">Generate HTML Report</h3>

      {/* Documentation. */}
      <div>
        <p className="text-xs text-gray-500 mb-2">
          Drag to reorder the sections included in the report
        </p>

        {/* Documentation. */}
        <ul className="space-y-1" data-testid="section-list">
          {enabledList.map((sec, index) => (
            <li
              key={sec}
              data-testid={`section-item-${sec}`}
              draggable
              onDragStart={(e) => handleDragStart(e, index)}
              onDragOver={handleDragOver}
              onDrop={(e) => handleDrop(e, index)}
              className="flex items-center gap-2 p-2 bg-white border border-blue-200 rounded cursor-grab active:cursor-grabbing"
            >
              {/* Documentation. */}
              <span className="text-gray-400 text-xs select-none">⠿</span>
              {/* Documentation. */}
              <input
                type="checkbox"
                id={`sec-${sec}`}
                data-testid={`section-checkbox-${sec}`}
                checked
                onChange={() => handleToggleSection(sec)}
                className="cursor-pointer"
              />
              <label htmlFor={`sec-${sec}`} className="text-sm cursor-pointer flex-1">
                {SECTION_LABELS[sec]}
              </label>
            </li>
          ))}
        </ul>

        {/* Documentation. */}
        {disabledList.length > 0 && (
          <ul className="space-y-1 mt-2 opacity-50" data-testid="disabled-section-list">
            {disabledList.map((sec) => (
              <li
                key={sec}
                data-testid={`section-item-${sec}`}
                className="flex items-center gap-2 p-2 bg-gray-50 border border-gray-200 rounded"
              >
                <span className="text-gray-300 text-xs select-none">⠿</span>
                <input
                  type="checkbox"
                  id={`sec-${sec}`}
                  data-testid={`section-checkbox-${sec}`}
                  checked={false}
                  onChange={() => handleToggleSection(sec)}
                  className="cursor-pointer"
                />
                <label htmlFor={`sec-${sec}`} className="text-sm cursor-pointer flex-1">
                  {SECTION_LABELS[sec]}
                </label>
              </li>
            ))}
          </ul>
        )}
      </div>

      {/* Documentation. */}
      {reportError && (
        <p data-testid="report-error" className="text-red-600 text-xs">
          {reportError}
        </p>
      )}

      {/* Documentation. */}
      <div className="flex gap-2">
        {/* Documentation. */}
        <button
          data-testid="generate-report-btn"
          onClick={handleGenerate}
          disabled={isGeneratingReport || enabledList.length === 0}
          className="flex items-center gap-1.5 px-3 py-1.5 bg-blue-600 text-white text-sm rounded
                     hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          {isGeneratingReport ? (
            <>
              {/* Documentation. */}
              <span
                data-testid="report-spinner"
                className="inline-block w-3.5 h-3.5 border-2 border-white border-t-transparent rounded-full animate-spin"
              />
              Generating...
            </>
          ) : (
            <>Download HTML</>
          )}
        </button>
      </div>

      {/* Documentation. */}
      <p className="text-xs text-gray-400">
        * After downloading, open in a browser and select "Print" to save as PDF.
      </p>
    </div>
  )
}

export default ReportBuilder
