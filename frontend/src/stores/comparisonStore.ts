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

import { create } from 'zustand'
import type { ComparisonMode, StudyComparisonResult, Study, Trial } from '../types'

// -------------------------------------------------------------------------
// Constants
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 */
export const COMPARISON_COLORS = [
  '#ef4444', // red-500
  '#22c55e', // green-500
  '#a855f7', // purple-500
  '#f59e0b', // amber-500
] as const

/** Documentation. */
export const MAX_COMPARISON_STUDIES = 4

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 * Documentation.
 */
export function canComparePareto(a: Study, b: Study): boolean {
  // Documentation.
  if (a.directions.length !== b.directions.length) return false
  // Documentation.
  return a.directions.every((d, i) => d === b.directions[i])
}

/**
 * Documentation.
 * Documentation.
 */
function dominates(a: number[], b: number[], isMinimize: boolean[]): boolean {
  let strictlyBetter = false
  for (let i = 0; i < a.length; i++) {
    // Documentation.
    const ai = isMinimize[i] ? a[i] : -a[i]
    const bi = isMinimize[i] ? b[i] : -b[i]
    if (ai > bi) return false // Documentation.
    if (ai < bi) strictlyBetter = true // Documentation.
  }
  return strictlyBetter
}

/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 *
 * Documentation.
 */
export function computeDominanceRatio(
  mainTrials: Trial[],
  compareTrials: Trial[],
  directions: Study['directions'],
): StudyComparisonResult['paretoDominanceRatio'] {
  const isMin = directions.map((d) => d === 'minimize')

  // Documentation.
  const mainPts = mainTrials
    .filter((t) => t.values !== null && t.values.every((v) => Number.isFinite(v)))
    .map((t) => t.values as number[])
  const compPts = compareTrials
    .filter((t) => t.values !== null && t.values.every((v) => Number.isFinite(v)))
    .map((t) => t.values as number[])

  if (mainPts.length === 0 || compPts.length === 0) return null

  // Documentation.
  type Tagged = { pt: number[]; isMain: boolean }
  const combined: Tagged[] = [
    ...mainPts.map((pt) => ({ pt, isMain: true })),
    ...compPts.map((pt) => ({ pt, isMain: false })),
  ]

  // Documentation.
  const front = combined.filter(
    (a) => !combined.some((b) => b !== a && dominates(b.pt, a.pt, isMin)),
  )

  if (front.length === 0) return null

  // Documentation.
  const mainInFront = front.filter((p) => p.isMain).length
  const compInFront = front.filter((p) => !p.isMain).length
  const nonDom = front.length - mainInFront - compInFront // Documentation.

  return {
    mainDominatesComparison: Math.round((mainInFront / front.length) * 100),
    nonDominated: Math.round((nonDom / front.length) * 100),
    comparisonDominatesMain: Math.round((compInFront / front.length) * 100),
  }
}

/**
 * Documentation.
 */
export function buildComparisonResult(
  mainStudy: Study,
  mainTrials: Trial[],
  compareStudy: Study,
  compareTrials: Trial[],
): StudyComparisonResult {
  const compatible = canComparePareto(mainStudy, compareStudy)

  // Documentation.
  let warningMessage: string | null = null
  if (!compatible) {
    const mLen = mainStudy.directions.length
    const cLen = compareStudy.directions.length
    if (mLen !== cLen) {
      warningMessage = `Objective count differs (${mainStudy.name}: ${mLen} objectives, ${compareStudy.name}: ${cLen} objectives). Only History and variable distribution can be compared.`
    } else {
      warningMessage = 'Optimization directions differ. Pareto comparison is disabled.'
    }
  }

  return {
    mainStudyId: mainStudy.studyId,
    comparisonStudyId: compareStudy.studyId,
    canComparePareto: compatible,
    warningMessage,
    // Documentation.
    paretoDominanceRatio: compatible
      ? computeDominanceRatio(mainTrials, compareTrials, mainStudy.directions)
      : null,
  }
}

// -------------------------------------------------------------------------
// Store Type definitions
// -------------------------------------------------------------------------

interface ComparisonState {
  // --- State ---
  /** Documentation. */
  comparisonStudyIds: number[]
  /** Documentation. */
  mode: ComparisonMode
  /** Documentation. */
  results: StudyComparisonResult[]
  /** Documentation. */
  isComputing: boolean

  // --- Actions ---
  /** Documentation. */
  setComparisonStudyIds: (ids: number[]) => void
  /** Documentation. */
  setMode: (mode: ComparisonMode) => void
  /**
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   */
  computeResults: (
    mainStudy: Study,
    mainTrials: Trial[],
    allStudies: Study[],
    trialMap: Map<number, Trial[]>,
  ) => void
  /** Documentation. */
  reset: () => void
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 */
export const useComparisonStore = create<ComparisonState>()((set, get) => ({
  // -------------------------------------------------------------------------
  // Documentation.
  // -------------------------------------------------------------------------
  comparisonStudyIds: [],
  mode: 'overlay',
  results: [],
  isComputing: false,

  // -------------------------------------------------------------------------
  // Documentation.
  // -------------------------------------------------------------------------

  /**
   * Documentation.
   */
  setComparisonStudyIds: (ids) => {
    // Documentation.
    set({ comparisonStudyIds: ids.slice(0, MAX_COMPARISON_STUDIES) })
  },

  /** Documentation. */
  setMode: (mode) => set({ mode }),

  /**
   * Documentation.
   *
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   */
  computeResults: (mainStudy, mainTrials, allStudies, trialMap) => {
    set({ isComputing: true })
    const { comparisonStudyIds } = get()

    const results: StudyComparisonResult[] = comparisonStudyIds.map((id) => {
      const compareStudy = allStudies.find((s) => s.studyId === id)

      // Documentation.
      if (!compareStudy) {
        return {
          mainStudyId: mainStudy.studyId,
          comparisonStudyId: id,
          canComparePareto: false,
          warningMessage: `Study ${id} not found`,
          paretoDominanceRatio: null,
        }
      }

      const compareTrials = trialMap.get(id) ?? []
      return buildComparisonResult(mainStudy, mainTrials, compareStudy, compareTrials)
    })

    set({ results, isComputing: false })
  },

  /**
   * Documentation.
   * Documentation.
   */
  reset: () => set({ comparisonStudyIds: [], results: [], mode: 'overlay', isComputing: false }),
}))
