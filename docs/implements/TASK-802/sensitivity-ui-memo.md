# TDD開発メモ: sensitivity-ui

## 概要

- 機能名: 感度分析UIコンポーネント
- 開発開始: 2026-03-27
- 現在のフェーズ: 完了

## 関連ファイル

- 実装ファイル:
  - `frontend/src/components/charts/SensitivityHeatmap.tsx`
- テストファイル:
  - `frontend/src/components/charts/SensitivityHeatmap.test.tsx`

## テストケース（10件）

| ID | 分類 | 内容 |
|---|---|---|
| TC-802-01 | 正常系 | data=null でもエラーなくレンダリング |
| TC-802-02 | 正常系 | data あり・isLoading=false で ECharts が表示 |
| TC-802-03 | 正常系 | 指標切り替えボタン（spearman/beta）が表示 |
| TC-802-04 | 正常系 | metric=spearman のとき spearman が aria-pressed=true |
| TC-802-05 | 正常系 | しきい値スライダーが表示・値が正しい |
| TC-802-06 | 正常系 | スライダー変更で onThresholdChange が呼ばれる |
| TC-802-07 | ローディング | isLoading=true で「WASM計算中...」が表示 |
| TC-802-08 | ローディング | isLoading=true で ECharts は表示されない |
| TC-802-E01 | 異常系 | data=null・isLoading=false で「データが読み込まれていません」 |
| TC-802-E02 | 異常系 | しきい値で全データ除外でも ECharts は表示 |

## 主要な設計決定

1. **指標切り替え（metric）**
   - props で受け取った metric を初期値として useState で内部状態管理
   - spearman / beta の 2 種類、aria-pressed で状態表示

2. **しきい値フィルタ**
   - パラメータの最大|感度| < threshold → その行の値を 0 に置き換え（無相関色）
   - 行ごとの最大絶対値で判定（max across objectives）

3. **色設定（REQ-097）**
   - visualMap: 青(#2563eb)=負の相関・白=無相関・赤(#dc2626)=正の相関

4. **ローディング状態（REQ-098）**
   - isLoading=true → ECharts を非表示にして「WASM計算中...」スピナー

5. **セルクリック**
   - ECharts の onEvents で click イベントを受け取り onCellClick コールバック

## 最終テスト結果

```
Test Files: 20 passed (20)
Tests: 113 passed (113)
Duration: 5.73s
```

## 品質評価

✅ **高品質**
- テスト: 113/113 通過
- セキュリティ: 重大な脆弱性なし
- TypeScript: エラーなし
- REQ-096〜REQ-098 準拠
