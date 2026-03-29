/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 */

import { describe, test, expect, beforeEach } from 'vitest'
import {
  useComparisonStore,
  canComparePareto,
  computeDominanceRatio,
  buildComparisonResult,
  MAX_COMPARISON_STUDIES,
} from './comparisonStore'
import type { Study, Trial } from '../types'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

function makeStudy(studyId: number, directions: Study['directions']): Study {
  return {
    studyId,
    name: `study-${studyId}`,
    directions,
    completedTrials: 10,
    totalTrials: 10,
    paramNames: ['x'],
    objectiveNames: directions.map((_, i) => `obj${i}`),
    userAttrNames: [],
    hasConstraints: false,
  }
}

function makeTrial(trialId: number, values: number[] | null): Trial {
  return {
    trialId,
    state: 'COMPLETE',
    params: {},
    values,
    paretoRank: null,
    clusterId: null,
    isFeasible: true,
    userAttrs: {},
    artifactIds: [],
  }
}

function resetStore() {
  useComparisonStore.setState({
    comparisonStudyIds: [],
    mode: 'overlay',
    results: [],
    isComputing: false,
  })
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('canComparePareto', () => {
  // Documentation.
  test('TC-1401-C01', () => {
    // Documentation.
    const a = makeStudy(1, ['minimize', 'minimize'])
    const b = makeStudy(2, ['minimize', 'minimize'])
    expect(canComparePareto(a, b)).toBe(true)
  })

  // Documentation.
  test('TC-1401-C02', () => {
    // Documentation.
    const a = makeStudy(1, ['minimize', 'minimize'])
    const b = makeStudy(2, ['minimize'])
    // Documentation.
    expect(canComparePareto(a, b)).toBe(false)
  })

  // Documentation.
  test('TC-1401-C03', () => {
    // Documentation.
    const a = makeStudy(1, ['minimize', 'minimize'])
    const b = makeStudy(2, ['minimize', 'maximize'])
    expect(canComparePareto(a, b)).toBe(false)
  })
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('computeDominanceRatio', () => {
  // Documentation.
  test('TC-1401-D01', () => {
    // Documentation.
    const mainTrials = [makeTrial(1, [1.0, 1.0])] // Documentation.
    const compTrials = [makeTrial(2, [2.0, 2.0])] // Documentation.
    const dirs: Study['directions'] = ['minimize', 'minimize']

    const ratio = computeDominanceRatio(mainTrials, compTrials, dirs)
    // Documentation.
    expect(ratio).not.toBeNull()
    expect(ratio!.mainDominatesComparison).toBeGreaterThan(ratio!.comparisonDominatesMain)
  })

  // Documentation.
  test('TC-1401-D02', () => {
    // Documentation.
    const ratio = computeDominanceRatio([], [], ['minimize'])
    expect(ratio).toBeNull()
  })
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('buildComparisonResult', () => {
  // Documentation.
  test('TC-1401-B01', () => {
    // Documentation.
    const main = makeStudy(1, ['minimize', 'minimize'])
    const comp = makeStudy(2, ['minimize'])
    const result = buildComparisonResult(main, [], comp, [])

    // Documentation.
    expect(result.canComparePareto).toBe(false)
    // Documentation.
    expect(result.warningMessage).not.toBeNull()
    expect(result.warningMessage).toContain('Objective count differs')
    // Documentation.
    expect(result.paretoDominanceRatio).toBeNull()
  })

  // Documentation.
  test('TC-1401-B02', () => {
    // Documentation.
    const main = makeStudy(1, ['minimize'])
    const comp = makeStudy(2, ['minimize'])
    const result = buildComparisonResult(main, [makeTrial(1, [1.0])], comp, [makeTrial(2, [2.0])])

    // Documentation.
    expect(result.canComparePareto).toBe(true)
    // Documentation.
    expect(result.warningMessage).toBeNull()
  })
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('ComparisonStore — setComparisonStudyIds', () => {
  beforeEach(() => resetStore())

  // Documentation.
  test('TC-1401-S01', () => {
    // Documentation.
    useComparisonStore.getState().setComparisonStudyIds([2, 3])
    expect(useComparisonStore.getState().comparisonStudyIds).toEqual([2, 3])
  })

  // Documentation.
  test('TC-1401-S02', () => {
    // Documentation.
    const ids = [2, 3, 4, 5, 6] // Documentation.
    useComparisonStore.getState().setComparisonStudyIds(ids)
    // Documentation.
    expect(useComparisonStore.getState().comparisonStudyIds).toHaveLength(MAX_COMPARISON_STUDIES)
  })
})

describe('ComparisonStore — computeResults', () => {
  beforeEach(() => resetStore())

  // Documentation.
  test('TC-1401-S03', () => {
    // Documentation.
    const main = makeStudy(1, ['minimize'])
    const allStudies = [
      main,
      makeStudy(2, ['minimize']),
      makeStudy(3, ['minimize']),
      makeStudy(4, ['minimize']),
    ]
    const trialMap = new Map<number, Trial[]>([
      [2, [makeTrial(10, [2.0])]],
      [3, [makeTrial(20, [3.0])]],
      [4, [makeTrial(30, [4.0])]],
    ])

    useComparisonStore.getState().setComparisonStudyIds([2, 3, 4])
    useComparisonStore.getState().computeResults(main, [makeTrial(1, [1.0])], allStudies, trialMap)

    // Documentation.
    expect(useComparisonStore.getState().results).toHaveLength(3)
    // Documentation.
    expect(useComparisonStore.getState().isComputing).toBe(false)
  })

  // Documentation.
  test('TC-1401-S04', () => {
    // Documentation.
    const main = makeStudy(1, ['minimize', 'minimize'])
    const incompatible = makeStudy(2, ['minimize']) // Documentation.
    const allStudies = [main, incompatible]

    useComparisonStore.getState().setComparisonStudyIds([2])
    useComparisonStore.getState().computeResults(main, [], allStudies, new Map())

    const result = useComparisonStore.getState().results[0]
    // Documentation.
    expect(result.canComparePareto).toBe(false)
    // Documentation.
    expect(result.warningMessage).toContain('Objective count differs')
  })
})

describe('ComparisonStore — reset', () => {
  beforeEach(() => resetStore())

  // Documentation.
  test('TC-1401-S05', () => {
    // Documentation.
    useComparisonStore.getState().setComparisonStudyIds([2, 3])
    useComparisonStore.getState().setMode('side-by-side')

    useComparisonStore.getState().reset()

    // Documentation.
    expect(useComparisonStore.getState().comparisonStudyIds).toHaveLength(0)
    // Documentation.
    expect(useComparisonStore.getState().results).toHaveLength(0)
    // Documentation.
    expect(useComparisonStore.getState().mode).toBe('overlay')
  })
})
