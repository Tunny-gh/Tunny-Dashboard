# TDD開発メモ: comparison (複数Study比較)

## 概要

- 機能名: 複数Study比較機能（ComparisonStore + StudyComparisonPanel）
- 開発開始: 2026-03-28
- 現在のフェーズ: 完了

## 関連ファイル

- 実装ファイル: `frontend/src/stores/comparisonStore.ts`
- 実装ファイル: `frontend/src/components/panels/StudyComparisonPanel.tsx`
- 修正ファイル: `frontend/src/stores/studyStore.ts` (selectStudy に reset() 追加)
- テストファイル: `frontend/src/stores/comparisonStore.test.ts`
- テストファイル: `frontend/src/components/panels/StudyComparisonPanel.test.tsx`

## Redフェーズ（失敗するテスト作成）

### テストケース

- TC-1401-C01〜C03: canComparePareto（同一/目的数不一致/方向不一致）
- TC-1401-D01〜D02: computeDominanceRatio（支配率計算/空リスト）
- TC-1401-B01〜B02: buildComparisonResult（不一致警告/互換結果）
- TC-1401-S01〜S05: ComparisonStore（setStudyIds/MAX制限/3Study比較/不一致/reset）
- TC-1401-P01〜P07: StudyComparisonPanel（警告/色バッジ/モード/MainStudy除外/チェック/モードクリック/null）

## Greenフェーズ（最小実装）

### 実装方針

- `comparisonStore.ts`: 純粋 TypeScript（WASM 依存なし）
  - `canComparePareto`: directions の配列比較
  - `computeDominanceRatio`: 合流 Pareto Front での出身Study割合
  - `buildComparisonResult`: 警告メッセージ付き比較結果構築
  - `useComparisonStore`: Zustand store（setMode/setIds/computeResults/reset）
- `StudyComparisonPanel.tsx`: チェックボックス + 色バッジ + モードボタン + 支配率テーブル
- `studyStore.ts`: `selectStudy` に `useComparisonStore.getState().reset()` を追加

### 主要な設計ポイント

- COMPARISON_COLORS: 4色固定（#ef4444, #22c55e, #a855f7, #f59e0b）
- MAX_COMPARISON_STUDIES = 4
- `reset()`: mode も 'overlay' に戻す（完全リセット）
- StudyComparisonPanel: Main Study は otherStudies から除外してチェックボックス非表示

### テスト結果

- comparisonStore.test.ts: 12/12 pass
- StudyComparisonPanel.test.tsx: 7/7 pass
- 全スイート: 260/260 pass

## Refactorフェーズ（品質改善）

### セキュリティレビュー

- XSS: JSX テンプレートリテラル使用なし（React が自動エスケープ）🟢
- データフロー: 外部入力なし（全 Study データは内部 Store から）🟢

### パフォーマンスレビュー

- Pareto Front 計算: O(N²) アルゴリズム。試行数が多い場合は要最適化 🟡
- computeResults は同期処理 → 将来的には Web Worker に移行検討 🟡

### 品質評価

- テスト: 260/260 pass
- セキュリティ: 重大な問題なし
- パフォーマンス: 小中規模では問題なし
