# TDD開発メモ: clustering-ui

## 概要

- 機能名: クラスタリングUIコンポーネント（ClusterPanel + ClusterList）
- 開発開始: 2026-03-27
- 現在のフェーズ: 完了

## 関連ファイル

- 実装ファイル:
  - `frontend/src/components/panels/ClusterPanel.tsx`: 設定パネル（空間選択・k設定・実行・Elbow）
  - `frontend/src/components/panels/ClusterList.tsx`: クラスタ一覧（件数・centroid±std・有意差★）
- テストファイル:
  - `frontend/src/components/panels/ClusterPanel.test.tsx`
  - `frontend/src/components/panels/ClusterList.test.tsx`

## テストケース（18件）

| ID | 分類 | 内容 |
|---|---|---|
| TC-902-01 | 正常系 | 実行ボタンクリックで onRunClustering が正しい引数で呼ばれる |
| TC-902-02 | 正常系 | isRunning=true でプログレスバーが表示される |
| TC-902-03 | 正常系 | progress=68 で「計算中...68%」テキストが表示される |
| TC-902-05 | 正常系 | elbowResult あり時に推薦 k テキストが表示される |
| TC-902-06 | 正常系 | isRunning=false でプログレスバーは非表示 |
| TC-902-07 | 正常系 | 目的関数ラジオ選択後に onRunClustering が "objective" で呼ばれる |
| TC-902-08 | 正常系 | elbowResult=null のとき elbow-recommended が非表示 |
| TC-902-L01 | ローディング | isRunning=true のとき実行ボタンが disabled |
| TC-902-E01 | 異常系 | k=1 で実行すると k-error 警告が表示される |
| TC-902-E02 | 異常系 | error prop があれば cluster-error が表示される |
| TC-902-09 | 正常系 | clusterStats あり時にクラスタバッジが表示される |
| TC-902-10 | 正常系 | クラスタ行クリックで brushSelect が呼ばれる |
| TC-902-11 | 正常系 | クラスタ行クリック後に行の background が選択色になる |
| TC-902-12 | 正常系 | significantFeatures=true の特徴に★マークが表示される |
| TC-902-13 | 正常系 | centroid ± std のテキストが各セルに表示される |
| TC-902-14 | 正常系 | Ctrl+クリックで複数クラスタの全インデックスが brushSelect に渡される |
| TC-902-15 | 純粋関数 | getClusterColor(0) は "#4f46e5" を返す（8色以上は循環） |
| TC-902-E01 | 異常系 | clusterStats=[] のとき「クラスタリングが実行されていません」が表示される |

## 主要な設計決定

1. **ClusterPanel: Props ベース純粋コンポーネント**
   - WASM 呼び出しは親コンポーネントに委譲（onRunClustering コールバック）
   - テスト容易性のためストア依存なし

2. **ClusterList: selectionStore.brushSelect 直結**
   - BottomPanel パターンに倣って useSelectionStore で直接 brushSelect を取得
   - Ctrl+クリックによる複数選択: Set<number> でクラスタIDを管理

3. **Elbow チャート: ECharts markPoint で推薦 k 強調**
   - k=2 スタートで横軸 category 型
   - markPoint で推薦 k 位置に赤点 + ラベル

4. **クラスタ識別色: 8 色循環**
   - `CLUSTER_COLORS` 配列 (indigo / rose / emerald / amber / cyan / violet / orange / teal)
   - `getClusterColor(id)` をエクスポートしてテスト可能

5. **k=1 バリデーション**
   - `handleRun()` 内でクライアント側バリデーション
   - k<2 なら kError をセットして onRunClustering を呼ばない

## 最終テスト結果

```
Test Files  24 passed
Tests       159 passed; 0 failed

ClusterPanel: 10/10 passed
ClusterList:  8/8 passed
```

## 品質評価

✅ **高品質**
- テスト: 18/18 通過（全 159 フロントエンドテスト通過）
- セキュリティ: 重大な脆弱性なし
- REQ-083〜REQ-087 準拠
