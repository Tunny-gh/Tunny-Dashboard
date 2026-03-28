# TDD開発メモ: panels

## 概要

- 機能名: LeftPanel・BottomPanel（フィルタスライダー・トライアル一覧）
- 開発開始: 2026-03-27
- 現在のフェーズ: 完了

## 関連ファイル

- 実装ファイル:
  - `frontend/src/components/panels/LeftPanel.tsx`
  - `frontend/src/components/panels/BottomPanel.tsx`
- テストファイル:
  - `frontend/src/components/panels/LeftPanel.test.tsx`
  - `frontend/src/components/panels/BottomPanel.test.tsx`

## テストケース（9件）

| ID | 分類 | 内容 |
|---|---|---|
| TC-402-01 | 正常系 | LeftPanel エラーなくレンダリング |
| TC-402-02 | 正常系 | selected カウンタが selectedIndices.length を表示 |
| TC-402-03 | 正常系 | スライダー変更で addAxisFilter が呼ばれる |
| TC-402-04 | 正常系 | カラーモード 'cluster' 選択で setColorMode が呼ばれる |
| TC-402-05 | 正常系 | BottomPanel エラーなくレンダリング |
| TC-402-06 | 正常系 | テーブルに trial_id ヘッダーが表示される |
| TC-402-07 | 正常系 | テーブル行クリックで setHighlight が呼ばれる |
| TC-402-E01 | 異常系 | currentStudy=null で「データが読み込まれていません」表示 |
| TC-402-E02 | 異常系 | currentStudy=null で「データが読み込まれていません」表示 |

## 主要な設計決定

1. **スライダーで addAxisFilter（min=0 固定）**
   - スライダー value → `addAxisFilter(axis, 0, value)` で最大値フィルタ
   - 将来的に min/max 両スライダー化（TASK-402b）

2. **BottomPanel の行データはプレースホルダー**
   - 現在の `positions` には x/y しかないためパラメータ値は「—」
   - WASM `get_column()` 実装（TASK-102）後に実データに変更

3. **highlighted 行のハイライト**
   - `highlighted === idx` で背景色を `#eff6ff` に変更

## 最終テスト結果

```
Test Files: 13 passed (13)
Tests: 66 passed (66)
Duration: 3.77s
```

## 品質評価

✅ **高品質**
- テスト: 66/66 通過
- セキュリティ: 重大な脆弱性なし
- TypeScript（新規ファイル）: エラーなし
- ESLint: エラーなし
- REQ-031, REQ-033, REQ-034 準拠
