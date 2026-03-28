# TDD開発メモ: pareto-charts

## 概要

- 機能名: Pareto Charts（ParetoScatter3D / ParetoScatter2D / HypervolumeHistory）
- 開発開始: 2026-03-27
- 現在のフェーズ: 完了

## 関連ファイル

- 要件定義: `docs/implements/TASK-501/pareto-charts-requirements.md`
- テストケース定義: `docs/implements/TASK-501/pareto-charts-testcases.md`
- 実装ファイル:
  - `frontend/src/components/charts/ParetoScatter3D.tsx`
  - `frontend/src/components/charts/ParetoScatter2D.tsx`
  - `frontend/src/components/charts/HypervolumeHistory.tsx`
- テストファイル:
  - `frontend/src/components/charts/ParetoScatter3D.test.tsx`
  - `frontend/src/components/charts/ParetoScatter2D.test.tsx`
  - `frontend/src/components/charts/HypervolumeHistory.test.tsx`
- テスト設定:
  - `frontend/vitest.config.ts` — jsdom 環境 + @testing-library/jest-dom 設定
  - `frontend/src/test/setup.ts` — jest-dom matcher 有効化

## 環境セットアップ（準備作業）

### 追加パッケージ

```
dependencies: deck.gl@9.2.11, echarts, echarts-for-react
devDependencies: @testing-library/react, @testing-library/jest-dom, jsdom
```

### vitest.config.ts 設定

- `environment: 'jsdom'` — React コンポーネントのDOM環境を有効化
- `setupFiles: ['./src/test/setup.ts']` — jest-dom matcher 追加
- `globals: true` — describe/test/expect の explicit import は維持

## Redフェーズ（失敗するテスト作成）

### 作成日時

2026-03-27

### テストケース（11件）

| ID | 分類 | 内容 |
|---|---|---|
| TC-501-01 | 正常系 | ParetoScatter3D: null データでもエラーなくレンダリング |
| TC-501-02 | 正常系 | ParetoScatter3D: gpuBufferありでDeckGLコンテナ表示 |
| TC-501-03 | 正常系 | ParetoScatter3D: mountでselectionStore.subscribe呼び出し |
| TC-501-04 | 正常系 | ParetoScatter2D: エラーなくレンダリング |
| TC-501-04b | 正常系 | ParetoScatter2D: gpuBufferありでDeckGLコンテナ表示 |
| TC-501-05 | 正常系 | HypervolumeHistory: データありでEChartsコンテナ表示 |
| TC-501-06 | 正常系 | HypervolumeHistory: データがseries.dataに渡される |
| TC-501-E01 | 異常系 | ParetoScatter3D: gpuBuffer=null で空状態UI表示 |
| TC-501-E02 | 異常系 | ParetoScatter2D: gpuBuffer=null で空状態UI表示 |
| TC-501-E03 | 異常系 | HypervolumeHistory: data=[] で空状態UI表示 |
| TC-501-B01 | 境界値 | ParetoScatter3D: アンマウント時にunsubscribe呼び出し |

### 確認された失敗

```
FAIL src/components/charts/ParetoScatter3D.test.tsx → Cannot find module './ParetoScatter3D'
FAIL src/components/charts/ParetoScatter2D.test.tsx → Cannot find module './ParetoScatter2D'
FAIL src/components/charts/HypervolumeHistory.test.tsx → Cannot find module './HypervolumeHistory'
```

## Greenフェーズ（最小実装）

### 実装日時

2026-03-27

### 主要な設計決定

1. **deck.gl v9 の DeckGL + Layer クラス API**
   - `DeckGL` コンポーネントに `layers={[layer]}` を渡す
   - `PointCloudLayer` / `ScatterplotLayer` にアクセサ関数でデータを渡す
   - テストでは `vi.mock('deck.gl', ...)` で WebGL 不要のモック化

2. **selectionStore subscribe パターン（Brushing & Linking）**
   - `useEffect` 内で `useSelectionStore.subscribe(selector, callback)` を設定
   - `useRef` で unsubscribe 関数を保持（cleanup 用）
   - アンマウント時に `unsubscribeRef.current()` で確実に解除

3. **HypervolumeHistory の ECharts データ形式**
   - `{ trial, hypervolume }` → `[trial, hypervolume]` に変換して `series[0].data` に渡す
   - テストで `data-option={JSON.stringify(option)}` としてオプション内容を検証

4. **JSX transform（react-jsx）の活用**
   - `"jsx": "react-jsx"` 設定のため `import React` 不要
   - コンポーネントおよびテストファイルから `import React` を削除

### テスト結果

```
Test Files: 8 passed (8)
Tests: 42 passed (42)
Duration: 3.10s
```

## Refactorフェーズ（品質改善）

### リファクタ日時

2026-03-27

### セキュリティレビュー

- **脆弱性**: なし
- `ParetoScatter3D/2D`: GpuBuffer の Float32Array データのみアクセス（ユーザー入力なし）
- `HypervolumeHistory`: `JSON.stringify(option)` でテスト用データをシリアライズ（安全）
- deck.gl のピッカー機能 (`pickable: true`) はクリックイベントで trial データを表示するための標準機能

### パフォーマンスレビュー

| 処理 | 設計 | 備考 |
|---|---|---|
| alpha 更新 | O(N) React外 | subscribe パターンで再レンダリングなし |
| layer 更新 | O(1) | DeckGL が差分検知して再描画 |
| unmount cleanup | O(1) | ref 経由で即座に unsubscribe |

### 修正内容

1. **ESLint修正**: 未使用の `currentStudy` 破壊的代入 → `{ gpuBuffer }` のみに変更
2. **TypeScript修正**: `import React` 削除（react-jsx transform 使用のため不要）
3. **テストモック**: `React.ReactNode` → `unknown` でモック型を簡素化

### 最終テスト結果

```
Test Files: 8 passed (8)
Tests: 42 passed (42)
Duration: 3.10s
```

### 品質評価

✅ **高品質**
- テスト: 42/42 通過
- セキュリティ: 重大な脆弱性なし
- TypeScript（新規ファイル）: エラーなし
- ESLint: エラーなし
- REQ-050, REQ-070〜REQ-075 準拠
