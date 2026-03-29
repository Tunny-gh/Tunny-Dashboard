/**
 * ComparisonStore — 複数Study比較管理 (TASK-1401)
 *
 * 【役割】: 複数 Study の比較状態を管理し、Pareto 支配率を計算する
 * 【設計方針】:
 *   - Main Study は studyStore.currentStudy を参照（Store 間連携）
 *   - 比較対象は最大 MAX_COMPARISON_STUDIES Study（UI 色バッジ制限）
 *   - canComparePareto: 目的数・最適化方向の完全一致確認
 *   - Study 切替時は reset() で比較状態をクリア
 * 🟢 REQ-120〜REQ-124 に準拠
 */

import { create } from 'zustand'
import type { ComparisonMode, StudyComparisonResult, Study, Trial } from '../types'

// -------------------------------------------------------------------------
// 定数
// -------------------------------------------------------------------------

/**
 * 【比較Study色パレット】: Main Study 以外の比較 Study に割り当てる色
 * 最大 4 Study × 色 → index 0 = 1番目の比較Study
 */
export const COMPARISON_COLORS = [
  '#ef4444', // red-500
  '#22c55e', // green-500
  '#a855f7', // purple-500
  '#f59e0b', // amber-500
] as const

/** 【最大比較Study数】: UI 色バッジが 4 色まで */
export const MAX_COMPARISON_STUDIES = 4

// -------------------------------------------------------------------------
// ユーティリティ（エクスポート）
// -------------------------------------------------------------------------

/**
 * 【Pareto比較可能判定】: 2Study の目的数・最適化方向が完全一致するか確認する
 * 一致しない場合は Pareto 重畳表示が無効になる
 * 🟢 REQ-121: 目的数不一致時は History・変数分布のみ比較可能
 */
export function canComparePareto(a: Study, b: Study): boolean {
  // 【目的数チェック】: 目的数が異なれば即 false
  if (a.directions.length !== b.directions.length) return false
  // 【方向チェック】: 各次元の最適化方向が一致するか確認
  return a.directions.every((d, i) => d === b.directions[i])
}

/**
 * 【Pareto支配判定】: 点 a が点 b を支配するか（各最適化方向を考慮）
 * 支配条件: a[i] ≤ b[i] (全次元) かつ a[j] < b[j] (少なくとも1次元)
 */
function dominates(a: number[], b: number[], isMinimize: boolean[]): boolean {
  let strictlyBetter = false
  for (let i = 0; i < a.length; i++) {
    // 【方向正規化】: maximize を minimize に変換（符号反転）
    const ai = isMinimize[i] ? a[i] : -a[i]
    const bi = isMinimize[i] ? b[i] : -b[i]
    if (ai > bi) return false // 劣っている次元がある → 支配しない
    if (ai < bi) strictlyBetter = true // 優れている次元
  }
  return strictlyBetter
}

/**
 * 【Pareto支配率計算】: 合流 Pareto Front での出身Study割合を返す
 *
 * 【処理フロー】:
 *   1. mainTrials + compareTrials を合流タグ付きリストに変換
 *   2. どの点にも支配されない点集合（Pareto Front）を抽出
 *   3. Front 内の出身Study 割合を集計して返す
 *
 * 🟢 REQ-122: Pareto支配率テーブルで表示する値の基盤
 */
export function computeDominanceRatio(
  mainTrials: Trial[],
  compareTrials: Trial[],
  directions: Study['directions'],
): StudyComparisonResult['paretoDominanceRatio'] {
  const isMin = directions.map((d) => d === 'minimize')

  // 【有効試行フィルタ】: values が非 null かつ全次元が有限値の試行のみ使用
  const mainPts = mainTrials
    .filter((t) => t.values !== null && t.values.every((v) => Number.isFinite(v)))
    .map((t) => t.values as number[])
  const compPts = compareTrials
    .filter((t) => t.values !== null && t.values.every((v) => Number.isFinite(v)))
    .map((t) => t.values as number[])

  if (mainPts.length === 0 || compPts.length === 0) return null

  // 【タグ付き合流】: 出身Study を識別するタグを付与
  type Tagged = { pt: number[]; isMain: boolean }
  const combined: Tagged[] = [
    ...mainPts.map((pt) => ({ pt, isMain: true })),
    ...compPts.map((pt) => ({ pt, isMain: false })),
  ]

  // 【Pareto Front 抽出】: どの点にも支配されない点集合
  const front = combined.filter(
    (a) => !combined.some((b) => b !== a && dominates(b.pt, a.pt, isMin)),
  )

  if (front.length === 0) return null

  // 【出身Study割合計算】: Front 内の各点の出身 Study 割合
  const mainInFront = front.filter((p) => p.isMain).length
  const compInFront = front.filter((p) => !p.isMain).length
  const nonDom = front.length - mainInFront - compInFront // 通常0（計算誤差対策）

  return {
    mainDominatesComparison: Math.round((mainInFront / front.length) * 100),
    nonDominated: Math.round((nonDom / front.length) * 100),
    comparisonDominatesMain: Math.round((compInFront / front.length) * 100),
  }
}

/**
 * 【StudyComparisonResult 構築】: mainStudy と compareStudy の比較結果オブジェクトを生成する
 */
export function buildComparisonResult(
  mainStudy: Study,
  mainTrials: Trial[],
  compareStudy: Study,
  compareTrials: Trial[],
): StudyComparisonResult {
  const compatible = canComparePareto(mainStudy, compareStudy)

  // 【警告メッセージ生成】: 不一致の理由に応じたメッセージを設定
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
    // 【支配率計算】: 互換Study のみ計算（不一致の場合は null）
    paretoDominanceRatio: compatible
      ? computeDominanceRatio(mainTrials, compareTrials, mainStudy.directions)
      : null,
  }
}

// -------------------------------------------------------------------------
// Store 型定義
// -------------------------------------------------------------------------

interface ComparisonState {
  // --- 状態 ---
  /** 比較対象 Study ID リスト（Main Study 除く、最大 MAX_COMPARISON_STUDIES） */
  comparisonStudyIds: number[]
  /** 比較表示モード（重畳 / 並列 / 差分） */
  mode: ComparisonMode
  /** 各比較 Study との比較計算結果 */
  results: StudyComparisonResult[]
  /** 比較計算中フラグ */
  isComputing: boolean

  // --- アクション ---
  /** 比較対象 Study ID を設定する（MAX 超は切り詰め） */
  setComparisonStudyIds: (ids: number[]) => void
  /** 比較モードを切り替える */
  setMode: (mode: ComparisonMode) => void
  /**
   * 各比較 Study との比較結果を計算する
   * @param mainStudy - メイン Study のメタ情報
   * @param mainTrials - メイン Study の試行リスト
   * @param allStudies - 全 Study のメタ情報リスト
   * @param trialMap - StudyId → Trial[] のマップ（比較 Study の試行データ）
   */
  computeResults: (
    mainStudy: Study,
    mainTrials: Trial[],
    allStudies: Study[],
    trialMap: Map<number, Trial[]>,
  ) => void
  /** 比較状態をリセットする（Study 切替時に呼ぶ）🟢 REQ-124 */
  reset: () => void
}

// -------------------------------------------------------------------------
// Store 実装
// -------------------------------------------------------------------------

/**
 * 【ComparisonStore】: 複数 Study 比較状態を管理する Zustand Store
 */
export const useComparisonStore = create<ComparisonState>()((set, get) => ({
  // -------------------------------------------------------------------------
  // 初期状態
  // -------------------------------------------------------------------------
  comparisonStudyIds: [],
  mode: 'overlay',
  results: [],
  isComputing: false,

  // -------------------------------------------------------------------------
  // アクション実装
  // -------------------------------------------------------------------------

  /**
   * 【比較Study設定】: 最大 MAX_COMPARISON_STUDIES 件に切り詰めて設定する
   */
  setComparisonStudyIds: (ids) => {
    // 【最大件数制限】: 色バッジが 4 色までのため4件に制限
    set({ comparisonStudyIds: ids.slice(0, MAX_COMPARISON_STUDIES) })
  },

  /** 【比較モード設定】 */
  setMode: (mode) => set({ mode }),

  /**
   * 【比較結果計算】: 各比較 Study との StudyComparisonResult を生成する
   *
   * 【処理フロー】:
   *   1. isComputing = true に設定
   *   2. 各比較 Study ID に対して buildComparisonResult を呼ぶ
   *   3. 計算完了後 results と isComputing を更新
   */
  computeResults: (mainStudy, mainTrials, allStudies, trialMap) => {
    set({ isComputing: true })
    const { comparisonStudyIds } = get()

    const results: StudyComparisonResult[] = comparisonStudyIds.map((id) => {
      const compareStudy = allStudies.find((s) => s.studyId === id)

      // 【Study未発見】: 見つからない場合は無効な結果を返す
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
   * 【全状態リセット】: Study 切替時に呼び出して比較状態を初期化する
   * studyStore.selectStudy から呼ばれる 🟢 REQ-124
   */
  reset: () => set({ comparisonStudyIds: [], results: [], mode: 'overlay', isComputing: false }),
}))
