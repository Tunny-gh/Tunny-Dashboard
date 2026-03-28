# TDD開発メモ: single-objective

## 概要

- 機能名: 単目的モード専用コンポーネント
- 開発開始: 2026-03-27
- 現在のフェーズ: 完了

## 関連ファイル

- 実装ファイル:
  - `frontend/src/components/charts/OptimizationHistory.tsx`
  - `frontend/src/components/panels/ConvergenceDiagnosis.tsx`
  - `frontend/src/components/panels/BestTrialHistory.tsx`
- テストファイル:
  - `frontend/src/components/charts/OptimizationHistory.test.tsx`
  - `frontend/src/components/panels/ConvergenceDiagnosis.test.tsx`
  - `frontend/src/components/panels/BestTrialHistory.test.tsx`

## テストケース（16件）

| ID | 分類 | 内容 |
|---|---|---|
| TC-1001-01 | 正常系 | OptimizationHistory がエラーなくレンダリング |
| TC-1001-02 | 正常系 | 4つのモードボタン（best/all/moving-avg/improvement）が表示される |
| TC-1001-03 | 正常系 | moving-avg ボタンクリックで aria-pressed が切り替わる |
| TC-1001-04 | 境界値 | detectPhase(10, 100)=0.1 → exploration |
| TC-1001-05 | 境界値 | detectPhase(50, 100)=0.5 → exploitation |
| TC-1001-06 | 境界値 | detectPhase(80, 100)=0.8 → convergence |
| TC-1001-07 | 正常系 | data=[] で「判定不可（試行数不足）」を表示 |
| TC-1001-08 | 正常系 | 収束済みデータで badge-converged が表示される |
| TC-1001-09 | 正常系 | 収束中データで badge-converging が表示される |
| TC-1001-10 | 境界値 | data.length=5 → insufficient |
| TC-1001-11 | 正常系 | 末尾20%で改善なし → converged |
| TC-1001-12 | 正常系 | 緩やかに改善中 → converged にはならない |
| TC-1001-13 | 正常系 | BestTrialHistory がエラーなくレンダリング |
| TC-1001-14 | 正常系 | minimize方向でBest更新行（3件）がテーブルに表示される |
| TC-1001-15 | 正常系 | Best更新のない試行はテーブルに表示されない |
| TC-1001-16 | 正常系 | 行クリックで onRowClick が正しいデータで呼ばれる |
| TC-1001-E01 | 異常系 | data=[] で best-trial-table が表示されるが行0件 |

## 主要な設計決定

1. **4 表示モード（OptimizationHistory）**
   - best: 累積Best値の折れ線グラフ（`computeBestSeries()`）
   - all: 全試行値の散布図
   - moving-avg: MOVING_AVG_WINDOW=5 の移動平均（`computeMovingAverage()`）
   - improvement: Best値改善率の棒グラフ（`computeImprovementRate()`）

2. **detectPhase() — フェーズ自動検出**
   - exploration: progress < 0.3（先頭30%）
   - exploitation: 0.3 ≤ progress < 0.7（中間40%）
   - convergence: progress ≥ 0.7（末尾30%）

3. **diagnoseConvergence() — 収束診断（ConvergenceDiagnosis）**
   - insufficient: data.length < 10
   - 末尾20%の改善率で判定: < 0.1% → converged, < 1% → converging, それ以外 → not-converged
   - バッジ: converged=緑, converging=黄, not-converged=赤, insufficient=グレー

4. **extractBestTrials() — Best更新試行抽出（BestTrialHistory）**
   - minimize/maximize 方向に応じてBest値が更新された試行のみ抽出
   - 初回試行は常にBest更新として含む

## 最終テスト結果

```
Test Files: 19 passed (19)
Tests: 103 passed (103)
Duration: 5.38s
```

## 品質評価

✅ **高品質**
- テスト: 103/103 通過
- セキュリティ: 重大な脆弱性なし
- TypeScript: エラーなし
- REQ-110〜REQ-113 準拠
