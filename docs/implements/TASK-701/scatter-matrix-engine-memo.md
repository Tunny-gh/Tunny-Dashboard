# TDD開発メモ: scatter-matrix-engine

## 概要

- 機能名: ScatterMatrix WebWorker・OffscreenCanvas基盤
- 開発開始: 2026-03-27
- 現在のフェーズ: 完了

## 関連ファイル

- 実装ファイル:
  - `frontend/src/wasm/workers/ScatterMatrixEngine.ts`
  - `frontend/src/wasm/workers/scatterMatrixWorker.ts`
- テストファイル:
  - `frontend/src/wasm/workers/ScatterMatrixEngine.test.ts`

## テストケース（7件）

| ID | 分類 | 内容 |
|---|---|---|
| TC-701-01 | 正常系 | workerCount=4 で 4 個の Worker が生成される |
| TC-701-02 | 正常系 | renderCell() が正しいワーカーに postMessage を送信する |
| TC-701-03 | 正常系 | Worker の done 応答で renderCell の Promise が解決する |
| TC-701-04 | 正常系 | 行グループに応じて正しいワーカーインデックスが返される |
| TC-701-05 | 正常系 | thumbnail/preview/full のサイズが 80/300/600 である |
| TC-701-E01 | 異常系 | Worker エラー時に renderCell の Promise が reject される |
| TC-701-E02 | 異常系 | dispose() が全ワーカーを terminate する |

## 主要な設計決定

1. **workerFactory 注入パターン**
   - `new Worker(url)` の代わりにファクトリ関数を注入
   - jsdom 環境でテスト可能（MockWorker をファクトリで注入）

2. **行グループ割り当て**
   - `Math.floor(row / WORKER_ROW_GROUP) % workerCount`
   - rows 0-9 → Worker[0], 10-19 → Worker[1], 20-29 → Worker[2], 30-33 → Worker[3]

3. **Promise ベース API**
   - `renderCell(row, col, size): Promise<ImageData | null>` で非同期結果を返す
   - Worker エラー時は Promise.reject でエラーを伝播

4. **OffscreenCanvas プレースホルダー**
   - `scatterMatrixWorker.ts` は現在グレー背景のみ描画
   - TASK-102 (WASM DataFrame) 完成後に実データ描画を追加予定

## 最終テスト結果

```
Test Files: 15 passed (15)
Tests: 79 passed (79)
Duration: 4.83s
```

## 品質評価

✅ **高品質**
- テスト: 79/79 通過
- セキュリティ: 重大な脆弱性なし
- TypeScript（新規ファイル）: エラーなし
- REQ-052, REQ-060〜REQ-066 準拠
