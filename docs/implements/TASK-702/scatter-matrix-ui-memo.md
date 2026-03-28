# TDD開発メモ: scatter-matrix-ui

## 概要

- 機能名: ScatterMatrix UIコンポーネント・表示モード
- 開発開始: 2026-03-27
- 現在のフェーズ: 完了

## 関連ファイル

- 実装ファイル:
  - `frontend/src/components/charts/ScatterMatrix.tsx`
- テストファイル:
  - `frontend/src/components/charts/ScatterMatrix.test.tsx`

## テストケース（7件）

| ID | 分類 | 内容 |
|---|---|---|
| TC-702-01 | 正常系 | currentStudy あり・engine=null でエラーなくレンダリング |
| TC-702-02 | 正常系 | 3つのモードボタン（mode1/mode2/mode3）が表示される |
| TC-702-03 | 正常系 | Mode 2 クリックで mode2 が aria-pressed=true になる |
| TC-702-04 | 正常系 | ソートセレクタが表示される |
| TC-702-05 | 正常系 | ソートオプション変更でセレクタ値が更新される |
| TC-702-E01 | 異常系 | currentStudy=null で「データが読み込まれていません」 |
| TC-702-E02 | 異常系 | engine+currentStudy ありでグリッドが表示される |

## 主要な設計決定

1. **3 表示モード**
   - Mode 1: 変数×変数（paramNames の正方行列）
   - Mode 2: 変数×目的（paramNames × objectiveNames の矩形行列）
   - Mode 3: 全変数（paramNames + objectiveNames の正方行列）

2. **軸ソート**
   - alphabetical: 即座に利用可能（`.sort()`）
   - correlation/importance: WASM 実装後に有効化（現在はプレースホルダー）

3. **ScatterCell の非同期描画**
   - `engine.renderCell(row, col, size)` を useEffect で呼び出す
   - resolved: ImageData → Canvas → dataURL → `<img>` タグ
   - null: グレーのローディングプレースホルダー
   - rejected: 「❌」表示

4. **workerFactory 注入パターン（TASK-701 の設計を継承）**
   - テストではモックエンジンを props で注入
   - engine=null の状態でも UI は正常にレンダリング可能

## 最終テスト結果

```
Test Files: 16 passed (16)
Tests: 86 passed (86)
Duration: 4.82s
```

## 品質評価

✅ **高品質**
- テスト: 86/86 通過
- セキュリティ: 重大な脆弱性なし
- TypeScript（新規ファイル）: エラーなし
- REQ-060〜REQ-066 準拠
