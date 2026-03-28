# TDD開発メモ: free-layout (フリーレイアウト Mode D)

## 概要

- 機能名: フリーレイアウト（FreeLayoutCanvas + layoutStore 拡張）
- 開発開始: 2026-03-28
- 現在のフェーズ: 完了

## 関連ファイル

- 実装ファイル: `frontend/src/components/layout/FreeLayoutCanvas.tsx`
- 修正ファイル: `frontend/src/stores/layoutStore.ts` (新アクション追加)
- テストファイル: `frontend/src/components/layout/FreeLayoutCanvas.test.tsx`
- テストファイル: `frontend/src/stores/layoutStore.test.ts` (TC-1501-L01〜L05 追加)

## Redフェーズ（失敗するテスト作成）

### テストケース

- TC-1501-F01〜F07: FreeLayoutCanvas（カード表示/デフォルト/D&D/トースト/エラー/プリセット）
- TC-1501-L01〜L05: layoutStore（setFreeModeLayout/updateCellPosition/saveLoad/JSON不正/JSON有効）

## Greenフェーズ（最小実装）

### 実装方針

- `layoutStore.ts` に追加:
  - `DEFAULT_FREE_LAYOUT`: 4×4グリッドを4等分した初期配置 (export)
  - `layoutLoadError: string | null`: JSON読込エラー
  - `setFreeModeLayout(layout)`: freeModeLayout 直接設定
  - `updateCellPosition(chartId, gridRow, gridCol)`: 単一チャート位置更新
  - `loadLayoutFromJson(json)`: JSON parse + バリデーション + エラー時デフォルト復帰

- `FreeLayoutCanvas.tsx`:
  - 4×4 絶対配置ドロップゾーン（data-testid: `free-layout-dropzone-{r}-{c}`）
  - CSS Grid チャートカード層（data-testid: `free-layout-card-{chartId}`）
  - ドラッグハンドル（data-testid: `free-layout-drag-handle-{chartId}`）
  - dragging状態はuseState管理（dataTransfer不使用 = テスト容易）
  - 保存トースト: useState + setTimeout 2秒
  - プリセットボタン A/B/C: window.confirm → setFreeModeLayout

### 主要な設計ポイント

- `GRID_SIZE = 4` (4×4グリッド)
- `DEFAULT_FREE_LAYOUT`: 4チャートを2×2に均等配置
- `PRESET_LAYOUTS`: A(均等4分割)/B(左半分大+右2分割)/C(上均等+下横長)
- `updateCellPosition`: 現在スパンを維持、GRID_SIZE+1 でクランプ

### テスト結果

- FreeLayoutCanvas.test.tsx: 7/7 pass
- layoutStore.test.ts: 8/8 pass (既存3 + 新規5)
- 全スイート: 272/272 pass

## Refactorフェーズ（品質改善）

### セキュリティレビュー

- XSS: React JSX による自動エスケープ ✅
- `loadLayoutFromJson`: try-catch でパース失敗を安全に処理 ✅

### パフォーマンスレビュー

- D&D: ローカルstate管理のため軽量 ✅
- `updateCellPosition`: O(N) でチャート数に比例（通常4〜16件程度）✅

### 品質評価

- テスト: 272/272 pass
- セキュリティ: 重大な問題なし
- パフォーマンス: 問題なし
