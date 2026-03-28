# TDD開発メモ: parallel-coordinates

## 概要

- 機能名: ParallelCoordinates（ECharts parallel座標 30軸）
- 開発開始: 2026-03-27
- 現在のフェーズ: 完了

## 関連ファイル

- 要件定義: `docs/implements/TASK-601/parallel-coordinates-requirements.md`
- テストケース定義: `docs/implements/TASK-601/parallel-coordinates-testcases.md`
- 実装ファイル: `frontend/src/components/charts/ParallelCoordinates.tsx`
- テストファイル: `frontend/src/components/charts/ParallelCoordinates.test.tsx`

## Redフェーズ（失敗するテスト作成）

### 作成日時

2026-03-27

### テストケース（7件）

| ID | 分類 | 内容 |
|---|---|---|
| TC-601-01 | 正常系 | null データでもエラーなくレンダリング |
| TC-601-02 | 正常系 | ECharts コンテナ表示 |
| TC-601-03 | 正常系 | parallelAxis に paramNames + objectiveNames が含まれる |
| TC-601-04 | 正常系 | axisareaselected イベントで addAxisFilter が呼ばれる |
| TC-601-E01 | 異常系 | gpuBuffer=null で空状態UI |
| TC-601-E02 | 異常系 | currentStudy=null で空状態UI |
| TC-601-B01 | 境界値 | 34軸（30変数+4目的）でも crash しない |

### 確認された失敗

```
FAIL src/components/charts/ParallelCoordinates.test.tsx → Cannot find module './ParallelCoordinates'
```

## Greenフェーズ（最小実装）

### 実装日時

2026-03-27

### 主要な設計決定

1. **ECharts `parallel` チャートタイプの採用**
   - `parallel` + `parallelAxis` + `series[{type:'parallel'}]` の三点セット
   - `parallelAxis` は `paramNames + objectiveNames` から `{dim, name, type}` を生成

2. **axisareaselected イベントハンドラで Brushing 実装**
   - `intervals` が空 → `removeAxisFilter(axisName)` でフィルタ解除
   - `intervals[0]` の `[min, max]` → `addAxisFilter(axisName, min, max)`
   - `axisIndex` → `axisNames[axisIndex]` でインデックスから軸名に変換

3. **テスト戦略: onEvents キャプチャパターン**
   - `vi.hoisted` で `captureOnEvents` 関数を定義
   - mock内で `capturedOnEvents = onEvents` として保持
   - テストから `captureOnEvents()['axisareaselected'](...)` で直接呼び出し

4. **jsdom 環境注意事項**
   - `npx vitest run` は `frontend/` ディレクトリで実行すること
   - ルートから実行すると jsdom なしで `document is not defined` エラーになる

### テスト結果

```
Test Files: 9 passed (9)
Tests: 49 passed (49)
Duration: 2.89s
```

## Refactorフェーズ（品質改善）

### リファクタ日時

2026-03-27

### セキュリティレビュー

- **脆弱性**: なし
- `axisareaselected` イベントデータ: ECharts 内部から来るため信頼可能
- `intervals[0]` のアクセスは `intervals.length > 0` の分岐後のみ（実行はブラシ操作時のみ）

### パフォーマンスレビュー

| 処理 | 複雑度 | 備考 |
|---|---|---|
| parallelAxis 構築 | O(N軸) | N=34 最大 |
| seriesData 構築 | O(N×軸数) | 最大 50000×34 — WASM実装後に改善予定 |
| handleAxisAreaSelected | O(1) | axesInfo は通常1軸分 |

### 最終テスト結果

```
Test Files: 9 passed (9)
Tests: 49 passed (49)
Duration: 2.89s
```

### 品質評価

✅ **高品質**
- テスト: 49/49 通過
- セキュリティ: 重大な脆弱性なし
- TypeScript（新規ファイル）: エラーなし
- ESLint: エラーなし
- REQ-051, REQ-041 準拠
