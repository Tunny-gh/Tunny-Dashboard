/**
 * ComparisonStore テスト (TASK-1401)
 *
 * 【テスト対象】: ComparisonStore + ユーティリティ (canComparePareto, computeDominanceRatio)
 * 【テスト方針】: 純粋な TypeScript ロジック（WASM 依存なし）
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
// ヘルパー
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
// canComparePareto テスト
// -------------------------------------------------------------------------

describe('canComparePareto', () => {
  // TC-1401-C01: 同一目的数・方向で true
  test('TC-1401-C01: 同一目的数・同一方向のStudyでtrueを返す', () => {
    // 【テスト目的】: 互換Studyを正しく検出できることを確認 🟢 REQ-121
    const a = makeStudy(1, ['minimize', 'minimize'])
    const b = makeStudy(2, ['minimize', 'minimize'])
    expect(canComparePareto(a, b)).toBe(true)
  })

  // TC-1401-C02: 目的数不一致で false
  test('TC-1401-C02: 目的数が異なるStudyでfalseを返す', () => {
    // 【テスト目的】: 目的数不一致時にPareto比較が無効になることを確認 🟢 REQ-121
    const a = makeStudy(1, ['minimize', 'minimize'])
    const b = makeStudy(2, ['minimize'])
    // 【確認内容】: 目的数不一致のため false
    expect(canComparePareto(a, b)).toBe(false)
  })

  // TC-1401-C03: 目的方向不一致で false
  test('TC-1401-C03: 最適化方向が異なるStudyでfalseを返す', () => {
    // 【テスト目的】: 最適化方向不一致時にPareto比較が無効になることを確認 🟢
    const a = makeStudy(1, ['minimize', 'minimize'])
    const b = makeStudy(2, ['minimize', 'maximize'])
    expect(canComparePareto(a, b)).toBe(false)
  })
})

// -------------------------------------------------------------------------
// computeDominanceRatio テスト
// -------------------------------------------------------------------------

describe('computeDominanceRatio', () => {
  // TC-1401-D01: Main が全点を支配する場合 mainDominatesComparison=100
  test('TC-1401-D01: Main点が全てComparisonを支配する場合mainDominatesComparisonが高い', () => {
    // 【テスト目的】: Pareto支配率が正しく計算されることを確認 🟢 REQ-122
    const mainTrials = [makeTrial(1, [1.0, 1.0])] // 優れた点
    const compTrials = [makeTrial(2, [2.0, 2.0])] // 劣った点
    const dirs: Study['directions'] = ['minimize', 'minimize']

    const ratio = computeDominanceRatio(mainTrials, compTrials, dirs)
    // 【確認内容】: Main点のみが Pareto Front に残るため mainDominatesComparison が高い
    expect(ratio).not.toBeNull()
    expect(ratio!.mainDominatesComparison).toBeGreaterThan(ratio!.comparisonDominatesMain)
  })

  // TC-1401-D02: 有効な試行がない場合 null を返す
  test('TC-1401-D02: 有効な試行がない場合nullを返す', () => {
    // 【テスト目的】: 空のtrialリストで安全に処理されることを確認 🟢
    const ratio = computeDominanceRatio([], [], ['minimize'])
    expect(ratio).toBeNull()
  })
})

// -------------------------------------------------------------------------
// buildComparisonResult テスト
// -------------------------------------------------------------------------

describe('buildComparisonResult', () => {
  // TC-1401-B01: 目的数不一致で warningMessage がセットされ canComparePareto=false
  test('TC-1401-B01: 目的数不一致StudyのcomparisonResultはcanComparePareto=falseでwarningあり', () => {
    // 【テスト目的】: 不一致Study比較時に適切な警告が生成されることを確認 🟢 REQ-121
    const main = makeStudy(1, ['minimize', 'minimize'])
    const comp = makeStudy(2, ['minimize'])
    const result = buildComparisonResult(main, [], comp, [])

    // 【確認内容】: canComparePareto が false
    expect(result.canComparePareto).toBe(false)
    // 【確認内容】: warningMessage が設定されている
    expect(result.warningMessage).not.toBeNull()
    expect(result.warningMessage).toContain('目的数が異なります')
    // 【確認内容】: Pareto支配率は null
    expect(result.paretoDominanceRatio).toBeNull()
  })

  // TC-1401-B02: 互換Studyは canComparePareto=true
  test('TC-1401-B02: 互換Studyの比較結果はcanComparePareto=true', () => {
    // 【テスト目的】: 互換Study比較時に正しい結果が生成されることを確認 🟢
    const main = makeStudy(1, ['minimize'])
    const comp = makeStudy(2, ['minimize'])
    const result = buildComparisonResult(main, [makeTrial(1, [1.0])], comp, [makeTrial(2, [2.0])])

    // 【確認内容】: canComparePareto が true
    expect(result.canComparePareto).toBe(true)
    // 【確認内容】: warningMessage は null
    expect(result.warningMessage).toBeNull()
  })
})

// -------------------------------------------------------------------------
// ComparisonStore テスト
// -------------------------------------------------------------------------

describe('ComparisonStore — setComparisonStudyIds', () => {
  beforeEach(() => resetStore())

  // TC-1401-S01: MAX 以下は全件設定される
  test('TC-1401-S01: MAX_COMPARISON_STUDIES以下のIDは全件設定される', () => {
    // 【テスト目的】: Study ID が正しく設定されることを確認 🟢
    useComparisonStore.getState().setComparisonStudyIds([2, 3])
    expect(useComparisonStore.getState().comparisonStudyIds).toEqual([2, 3])
  })

  // TC-1401-S02: MAX 超は切り詰め
  test('TC-1401-S02: MAX_COMPARISON_STUDIESを超えるIDは切り詰められる', () => {
    // 【テスト目的】: 5件以上のStudy選択が制限されることを確認 🟡 REQ-124
    const ids = [2, 3, 4, 5, 6] // 5件 > MAX(4)
    useComparisonStore.getState().setComparisonStudyIds(ids)
    // 【確認内容】: MAX_COMPARISON_STUDIES 件に切り詰められること
    expect(useComparisonStore.getState().comparisonStudyIds).toHaveLength(MAX_COMPARISON_STUDIES)
  })
})

describe('ComparisonStore — computeResults', () => {
  beforeEach(() => resetStore())

  // TC-1401-S03: 3Study比較でresultsが3件格納される
  test('TC-1401-S03: 3Study選択時にresultsが3件格納される', () => {
    // 【テスト目的】: 3Study重畳表示のためにresultsが正しく構築されることを確認 🟢 REQ-120
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

    // 【確認内容】: results に3件の比較結果が格納されること
    expect(useComparisonStore.getState().results).toHaveLength(3)
    // 【確認内容】: isComputing が false に戻ること
    expect(useComparisonStore.getState().isComputing).toBe(false)
  })

  // TC-1401-S04: 目的数不一致StudyのresultはcanComparePareto=false
  test('TC-1401-S04: 目的数不一致Study比較時にPareto比較が無効化される', () => {
    // 【テスト目的】: 目的数不一致Study比較時の無効化を確認 🟢 REQ-121 (TC-1401-S01相当)
    const main = makeStudy(1, ['minimize', 'minimize'])
    const incompatible = makeStudy(2, ['minimize']) // 目的数不一致
    const allStudies = [main, incompatible]

    useComparisonStore.getState().setComparisonStudyIds([2])
    useComparisonStore.getState().computeResults(main, [], allStudies, new Map())

    const result = useComparisonStore.getState().results[0]
    // 【確認内容】: canComparePareto が false
    expect(result.canComparePareto).toBe(false)
    // 【確認内容】: warningMessage が設定されている
    expect(result.warningMessage).toContain('目的数が異なります')
  })
})

describe('ComparisonStore — reset', () => {
  beforeEach(() => resetStore())

  // TC-1401-S05: Study切り替え時のリセット（reset() で全状態初期化）
  test('TC-1401-S05: reset()で比較状態が完全に初期化される', () => {
    // 【テスト目的】: Study切り替え時にcomparisonStoreがリセットされることを確認 🟢 (TC-1401-S03相当)
    useComparisonStore.getState().setComparisonStudyIds([2, 3])
    useComparisonStore.getState().setMode('side-by-side')

    useComparisonStore.getState().reset()

    // 【確認内容】: comparisonStudyIds が空になること
    expect(useComparisonStore.getState().comparisonStudyIds).toHaveLength(0)
    // 【確認内容】: results が空になること
    expect(useComparisonStore.getState().results).toHaveLength(0)
    // 【確認内容】: mode がデフォルトに戻ること
    expect(useComparisonStore.getState().mode).toBe('overlay')
  })
})
