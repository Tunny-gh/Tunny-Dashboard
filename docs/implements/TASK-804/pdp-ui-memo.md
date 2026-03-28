# TDD開発メモ: pdp-ui

## 概要

- 機能名: PDPコンポーネント（ECharts実装）
- 開発開始: 2026-03-27
- 現在のフェーズ: 完了

## 関連ファイル

- 実装ファイル:
  - `frontend/src/components/charts/PDPChart.tsx`: PDPチャートコンポーネント
- テストファイル:
  - `frontend/src/components/charts/PDPChart.test.tsx`

## テストケース（22件）

| ID | 分類 | 内容 |
|---|---|---|
| TC-804-01 | 正常系 | data1d=null でエラーなくレンダリング |
| TC-804-02 | 正常系 | data1d あり・isLoading=false で ECharts が表示 |
| TC-804-03 | 正常系 | data2d あり・data1d=null で ECharts が表示 |
| TC-804-04 | 正常系 | モデル品質パネルが表示される |
| TC-804-05 | 正常系 | R² 値がパネルに小数点3桁で表示される |
| TC-804-06 | ローディング | isLoading=true で「PDP計算中...」が表示 |
| TC-804-07 | ローディング | isLoading=true のとき ECharts は非表示 |
| TC-804-08 | 警告 | R² < 0.8 で警告バッジが表示される |
| TC-804-09 | 警告 | 警告バッジに R² 値が含まれる |
| TC-804-10 | 警告 | R² >= 0.8 では警告バッジが非表示 |
| TC-804-11 | 警告 | useOnnx=false で「線形近似で表示中」バナーが表示 |
| TC-804-12 | 警告 | useOnnx=true では線形近似バナーが非表示 |
| TC-804-13 | 警告 | onOnnxRequest あり時に .onnx ボタンが表示 |
| TC-804-14 | 警告 | .onnx ボタンクリックで onOnnxRequest が呼ばれる |
| TC-804-15 | ICE | highlightedIndices=[0,1] を渡してもクラッシュしない |
| TC-804-16 | ICE | highlightedIndices 変更後も ECharts が表示される |
| TC-804-17 | ICE | iceLines 省略で highlightedIndices を渡してもクラッシュしない |
| TC-804-E01 | 異常系 | data1d=null・data2d なしで「データが読み込まれていません」 |
| TC-804-E02 | 異常系 | rSquared=0 で quality-label が「推奨外」 |
| TC-804-Q01 | 純粋関数 | R² >= 0.8 → '良好' |
| TC-804-Q02 | 純粋関数 | 0.5 <= R² < 0.8 → '要注意' |
| TC-804-Q03 | 純粋関数 | R² < 0.5 → '推奨外' |

## 主要な設計決定

1. **PDP 曲線 + ICE ライン描画**
   - ECharts Line で ICE ラインを半透明グレーで描画（背景）
   - PDP 曲線を紫太線で前面に重ねて描画（z=10）
   - Brushing 連動: highlightedSet に含まれる ICE ラインをオレンジ太線でハイライト

2. **R² 品質評価**
   - R² >= 0.8: 良好（緑）
   - 0.5 <= R² < 0.8: 要注意（オレンジ）
   - R² < 0.5: 推奨外（赤）
   - `getModelQuality()` は純粋関数として export してテスト容易

3. **ONNX 警告バナー**
   - useOnnx=false: 常に警告バナーを表示
   - onOnnxRequest が提供された場合は「.onnx を読み込む」ボタンを追加

4. **2変数 PDP ヒートマップ**
   - data2d 提供時に ECharts HeatMap で交互作用を可視化
   - 色: 青=低・赤=高（SensitivityHeatmap と同じ配色）

## 最終テスト結果

```
Running PDPChart tests
test result: ok. 22 passed; 0 failed

All frontend tests: 141 passed; 0 failed
```

## 品質評価

✅ **高品質**
- テスト: 22/22 通過
- セキュリティ: 重大な脆弱性なし
- REQ-103〜REQ-106 準拠
