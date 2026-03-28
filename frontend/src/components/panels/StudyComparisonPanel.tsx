/**
 * StudyComparisonPanel — 複数Study比較パネル (TASK-1401)
 *
 * 【役割】: 複数 Study の比較設定・比較結果を表示する
 * 【設計方針】:
 *   - Study 選択チェックボックス（Main Study は除外）
 *   - 目的数不一致 Study に ⚠ 警告アイコンを表示
 *   - 選択 Study に色バッジを割り当て（最大4色）
 *   - 比較モード切替: 重畳（Overlay）/ 並列（Side-by-side）/ 差分（Diff）
 *   - Pareto 支配率サマリーテーブル
 *   - currentStudy が null のときは非表示
 * 🟢 REQ-120〜REQ-124 に準拠
 */

import React from 'react';
import { useStudyStore } from '../../stores/studyStore';
import {
  useComparisonStore,
  canComparePareto,
  COMPARISON_COLORS,
} from '../../stores/comparisonStore';
import type { ComparisonMode } from '../../types';

// -------------------------------------------------------------------------
// 定数
// -------------------------------------------------------------------------

/** 【比較モードラベル】 */
const MODE_LABELS: Record<ComparisonMode, string> = {
  overlay: '重畳',
  'side-by-side': '並列',
  diff: '差分',
};

// -------------------------------------------------------------------------
// StudyComparisonPanel コンポーネント
// -------------------------------------------------------------------------

/**
 * 【機能概要】: 複数 Study の比較設定と支配率サマリーを提供するパネル
 */
export const StudyComparisonPanel: React.FC = () => {
  const { currentStudy, allStudies } = useStudyStore();
  const { comparisonStudyIds, mode, results, setComparisonStudyIds, setMode } =
    useComparisonStore();

  // 【currentStudy 未選択時は非表示】
  if (!currentStudy) return null;

  // Main Study 以外の Study リスト
  const otherStudies = allStudies.filter((s) => s.studyId !== currentStudy.studyId);

  /**
   * 【チェックボックス切替】: Study の選択/解除を処理する
   */
  const handleToggle = (studyId: number) => {
    if (comparisonStudyIds.includes(studyId)) {
      setComparisonStudyIds(comparisonStudyIds.filter((id) => id !== studyId));
    } else {
      setComparisonStudyIds([...comparisonStudyIds, studyId]);
    }
  };

  return (
    <div data-testid="study-comparison-panel" className="flex flex-col gap-3 p-3">
      {/* ---------------------------------------------------------------- */}
      {/* Study 選択リスト                                                  */}
      {/* ---------------------------------------------------------------- */}
      <div>
        <p className="text-xs font-semibold text-gray-600 mb-1">比較対象 Study</p>
        <ul data-testid="comparison-study-list" className="flex flex-col gap-1">
          {otherStudies.map((study, idx) => {
            const isSelected = comparisonStudyIds.includes(study.studyId);
            const isIncompat = !canComparePareto(currentStudy, study);
            // 選択済みの場合に割り当てる色インデックス
            const colorIdx = comparisonStudyIds.indexOf(study.studyId);
            const color = colorIdx >= 0 ? COMPARISON_COLORS[colorIdx] : undefined;
            void idx; // suppress unused var

            return (
              <li
                key={study.studyId}
                data-testid={`comparison-study-item-${study.studyId}`}
                className="flex items-center gap-2 text-sm"
              >
                {/* チェックボックス */}
                <input
                  type="checkbox"
                  data-testid={`comparison-study-checkbox-${study.studyId}`}
                  checked={isSelected}
                  onChange={() => handleToggle(study.studyId)}
                  className="h-3.5 w-3.5 rounded border-gray-300"
                />

                {/* 色バッジ（選択済みのみ表示） */}
                {isSelected && color && (
                  <span
                    data-testid={`comparison-color-badge-${study.studyId}`}
                    className="inline-block h-3 w-3 rounded-full flex-shrink-0"
                    style={{ backgroundColor: color }}
                  />
                )}

                {/* Study名 */}
                <span className="flex-1 truncate text-gray-700">{study.name}</span>

                {/* 目的数表示 */}
                <span className="text-xs text-gray-400">
                  {study.directions.length}目的
                </span>

                {/* 不一致警告アイコン */}
                {isIncompat && (
                  <span
                    data-testid={`comparison-warning-${study.studyId}`}
                    title="History・変数分布のみ比較可能"
                    className="text-amber-500 text-xs"
                  >
                    ⚠
                  </span>
                )}
              </li>
            );
          })}
        </ul>
      </div>

      {/* ---------------------------------------------------------------- */}
      {/* 比較モード切替                                                    */}
      {/* ---------------------------------------------------------------- */}
      <div>
        <p className="text-xs font-semibold text-gray-600 mb-1">比較モード</p>
        <div className="flex gap-1" data-testid="comparison-mode-controls">
          {(['overlay', 'side-by-side', 'diff'] as ComparisonMode[]).map((m) => (
            <button
              key={m}
              data-testid={`comparison-mode-${m}`}
              onClick={() => setMode(m)}
              className={`px-2 py-0.5 text-xs rounded border transition-colors
                ${mode === m
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
      {/* Pareto 支配率サマリーテーブル（比較結果あり時のみ）               */}
      {/* ---------------------------------------------------------------- */}
      {results.length > 0 && (
        <div>
          <p className="text-xs font-semibold text-gray-600 mb-1">Pareto支配率</p>
          <table
            data-testid="comparison-summary-table"
            className="w-full text-xs border-collapse"
          >
            <thead>
              <tr className="bg-gray-50">
                <th className="text-left px-1 py-0.5 border border-gray-200">Study</th>
                <th className="text-right px-1 py-0.5 border border-gray-200">Main優勢</th>
                <th className="text-right px-1 py-0.5 border border-gray-200">Comp優勢</th>
              </tr>
            </thead>
            <tbody>
              {results.map((r) => {
                const compStudy = allStudies.find((s) => s.studyId === r.comparisonStudyId);
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
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
};

export default StudyComparisonPanel;
