# TDD開発メモ: objective-pair-matrix

## 概要

- 機能名: ObjectivePairMatrix（目的ペア行列）
- 開発開始: 2026-03-27
- 現在のフェーズ: 完了

## 関連ファイル

- 実装ファイル:
  - `frontend/src/components/charts/ObjectivePairMatrix.tsx`
- テストファイル:
  - `frontend/src/components/charts/ObjectivePairMatrix.test.tsx`

## テストケース（6件）

| ID | 分類 | 内容 |
|---|---|---|
| TC-502-01 | 正常系 | ObjectivePairMatrix 4目的でエラーなくレンダリング |
| TC-502-02 | 正常系 | 4目的で4×4グリッド（16セル）が表示される |
| TC-502-03 | 正常系 | セルクリックで onCellClick が正しい軸名で呼ばれる |
| TC-502-04 | 正常系 | 2目的で2×2グリッド（4セル）が表示される |
| TC-502-E01 | 異常系 | 1目的のときコンポーネントが非表示（null）になる |
| TC-502-E02 | 異常系 | currentStudy=null で「データが読み込まれていません」表示 |

## 主要な設計決定

1. **Props直接渡しパターン**
   - ParetoScatter2D/3D と同様に `gpuBuffer` と `currentStudy` を props で受け取る
   - ストア接続なし（AppShell が連携を担う）

2. **onCellClick コールバック**
   - `onCellClick?(xAxisName: string, yAxisName: string)` で軸選択を通知
   - 呼び出し元（AppShell 等）が 3D ビュー軸割り当てに使用

3. **グリッド構造**
   - 対角 (row === col): 目的名ラベル（将来的に 1D ヒストグラム）
   - 下三角 (row > col): deck.gl ScatterplotLayer 2D 散布図
   - 上三角 (row < col): 空セル（将来的に統計情報）

4. **表示制御**
   - 1目的以下: `null` を返して非表示
   - currentStudy=null: 「データが読み込まれていません」表示

## 最終テスト結果

```
Test Files: 14 passed (14)
Tests: 72 passed (72)
Duration: 4.14s
```

## 品質評価

✅ **高品質**
- テスト: 72/72 通過
- セキュリティ: 重大な脆弱性なし
- TypeScript（新規ファイル）: エラーなし
- ESLint: エラーなし
- REQ-070, REQ-075 準拠
