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
 * Documentation.
 * Documentation.
 */

import React from 'react'
import { useStudyStore } from '../../stores/studyStore'
import {
  useComparisonStore,
  canComparePareto,
  COMPARISON_COLORS,
} from '../../stores/comparisonStore'
import type { ComparisonMode } from '../../types'

// -------------------------------------------------------------------------
// Constants
// -------------------------------------------------------------------------

/** Documentation. */
const MODE_LABELS: Record<ComparisonMode, string> = {
  overlay: 'Overlay',
  'side-by-side': 'Side by Side',
  diff: 'Diff',
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 */
export const StudyComparisonPanel: React.FC = () => {
  const { currentStudy, allStudies } = useStudyStore()
  const { comparisonStudyIds, mode, results, setComparisonStudyIds, setMode } = useComparisonStore()

  // Documentation.
  if (!currentStudy) return null

  // Documentation.
  const otherStudies = allStudies.filter((s) => s.studyId !== currentStudy.studyId)

  /**
   * Documentation.
   */
  const handleToggle = (studyId: number) => {
    if (comparisonStudyIds.includes(studyId)) {
      setComparisonStudyIds(comparisonStudyIds.filter((id) => id !== studyId))
    } else {
      setComparisonStudyIds([...comparisonStudyIds, studyId])
    }
  }

  return (
    <div data-testid="study-comparison-panel" className="flex flex-col gap-3 p-3">
      {/* ---------------------------------------------------------------- */}
      {/* Documentation. */}
      {/* ---------------------------------------------------------------- */}
      <div>
        <p className="text-xs font-semibold text-gray-600 mb-1">Comparison Studies</p>
        <ul data-testid="comparison-study-list" className="flex flex-col gap-1">
          {otherStudies.map((study, idx) => {
            const isSelected = comparisonStudyIds.includes(study.studyId)
            const isIncompat = !canComparePareto(currentStudy, study)
            // Documentation.
            const colorIdx = comparisonStudyIds.indexOf(study.studyId)
            const color = colorIdx >= 0 ? COMPARISON_COLORS[colorIdx] : undefined
            void idx // suppress unused var

            return (
              <li
                key={study.studyId}
                data-testid={`comparison-study-item-${study.studyId}`}
                className="flex items-center gap-2 text-sm"
              >
                {/* Documentation. */}
                <input
                  type="checkbox"
                  data-testid={`comparison-study-checkbox-${study.studyId}`}
                  checked={isSelected}
                  onChange={() => handleToggle(study.studyId)}
                  className="h-3.5 w-3.5 rounded border-gray-300"
                />

                {/* Documentation. */}
                {isSelected && color && (
                  <span
                    data-testid={`comparison-color-badge-${study.studyId}`}
                    className="inline-block h-3 w-3 rounded-full flex-shrink-0"
                    style={{ backgroundColor: color }}
                  />
                )}

                {/* Documentation. */}
                <span className="flex-1 truncate text-gray-700">{study.name}</span>

                {/* Documentation. */}
                <span className="text-xs text-gray-400">{study.directions.length} obj</span>

                {/* Documentation. */}
                {isIncompat && (
                  <span
                    data-testid={`comparison-warning-${study.studyId}`}
                    title="Only History and variable distribution can be compared"
                    className="text-amber-500 text-xs"
                  >
                    ⚠
                  </span>
                )}
              </li>
            )
          })}
        </ul>
      </div>

      {/* ---------------------------------------------------------------- */}
      {/* Documentation. */}
      {/* ---------------------------------------------------------------- */}
      <div>
        <p className="text-xs font-semibold text-gray-600 mb-1">Comparison Mode</p>
        <div className="flex gap-1" data-testid="comparison-mode-controls">
          {(['overlay', 'side-by-side', 'diff'] as ComparisonMode[]).map((m) => (
            <button
              key={m}
              data-testid={`comparison-mode-${m}`}
              onClick={() => setMode(m)}
              className={`px-2 py-0.5 text-xs rounded border transition-colors
                ${
                  mode === m
                    ? 'bg-blue-600 text-white border-blue-600'
                    : 'border-gray-300 text-gray-600 hover:bg-gray-50'
                }`}
            >
              {MODE_LABELS[m]}
            </button>
          ))}
        </div>
      </div>

      {/* ---------------------------------------------------------------- */}
      {/* Documentation. */}
      {/* ---------------------------------------------------------------- */}
      {results.length > 0 && (
        <div>
          <p className="text-xs font-semibold text-gray-600 mb-1">Pareto Dominance Ratio</p>
          <table data-testid="comparison-summary-table" className="w-full text-xs border-collapse">
            <thead>
              <tr className="bg-gray-50">
                <th className="text-left px-1 py-0.5 border border-gray-200">Study</th>
                <th className="text-right px-1 py-0.5 border border-gray-200">Main Dominates</th>
                <th className="text-right px-1 py-0.5 border border-gray-200">Comp Dominates</th>
              </tr>
            </thead>
            <tbody>
              {results.map((r) => {
                const compStudy = allStudies.find((s) => s.studyId === r.comparisonStudyId)
                return (
                  <tr
                    key={r.comparisonStudyId}
                    data-testid={`comparison-summary-row-${r.comparisonStudyId}`}
                  >
                    <td className="px-1 py-0.5 border border-gray-200 truncate max-w-[6rem]">
                      {compStudy?.name ?? `Study ${r.comparisonStudyId}`}
                    </td>
                    <td className="text-right px-1 py-0.5 border border-gray-200">
                      {r.canComparePareto && r.paretoDominanceRatio
                        ? `${r.paretoDominanceRatio.mainDominatesComparison}%`
                        : '—'}
                    </td>
                    <td className="text-right px-1 py-0.5 border border-gray-200">
                      {r.canComparePareto && r.paretoDominanceRatio
                        ? `${r.paretoDominanceRatio.comparisonDominatesMain}%`
                        : '—'}
                    </td>
                  </tr>
                )
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}

export default StudyComparisonPanel
