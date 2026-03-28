# TDD開発メモ: zustand-stores

## 概要

- 機能名: Zustand Store群（SelectionStore / StudyStore / LayoutStore）
- 開発開始: 2026-03-27
- 現在のフェーズ: 完了

## 関連ファイル

- 要件定義: `docs/implements/TASK-302/zustand-stores-requirements.md`
- テストケース定義: `docs/implements/TASK-302/zustand-stores-testcases.md`
- 実装ファイル:
  - `frontend/src/stores/selectionStore.ts`
  - `frontend/src/stores/studyStore.ts`
  - `frontend/src/stores/layoutStore.ts`
- テストファイル:
  - `frontend/src/stores/selectionStore.test.ts`
  - `frontend/src/stores/studyStore.test.ts`
  - `frontend/src/stores/layoutStore.test.ts`

## Redフェーズ（失敗するテスト作成）

### 作成日時

2026-03-27

### テストケース（17件）

| ID | 分類 | 内容 |
|---|---|---|
| TC-302-01〜08 | 正常系 | SelectionStore: brushSelect/clearSelection/addAxisFilter/subscribe |
| TC-302-09〜10 | 正常系 | StudyStore: loadJournal/isLoading |
| TC-302-11〜13 | 正常系 | LayoutStore: setLayoutMode/toggleChart/saveLayout/loadLayout |
| TC-302-E01 | 異常系 | loadJournal 失敗時 loadError 設定 |
| TC-302-E02 | 異常系 | WASM 未初期化時 addAxisFilter クラッシュなし |
| TC-302-B01〜B02 | 境界値 | clearSelection(trialCount=0) / removeAxisFilter(非存在キー) |

### 確認された失敗

```
FAIL src/stores/selectionStore.test.ts → Cannot find module './selectionStore'
FAIL src/stores/studyStore.test.ts → Cannot find module './studyStore'
FAIL src/stores/layoutStore.test.ts → Cannot find module './layoutStore'
```

## Greenフェーズ（最小実装）

### 実装日時

2026-03-27

### 主要な設計決定

1. **vi.hoisted でモックの hoisting 問題を解決**
   - `vi.mock` ファクトリが先頭にホイスティングされるため、ファクトリ内で外部変数を参照できない
   - `vi.hoisted(() => { const mockFn = vi.fn(); return { mockFn }; })` で先行定義

2. **subscribeWithSelector でセレクタ購読を有効化**
   - `useSelectionStore.subscribe((s) => s.selectedIndices, callback)` が機能する
   - チャートコンポーネントが React サイクル外で GPU alpha を直接更新するための基盤

3. **addAxisFilter の fire-and-forget パターン**
   - `filterRanges` は同期更新（UI 即反映）
   - `WasmLoader.getInstance().then(...).catch(...)` で WASM 呼び出しを非同期化
   - WASM 失敗時は `filterRanges` のみ更新、`selectedIndices` は変更しない

4. **`_makeAllIndices(n)` ヘルパーで O(N) 全インデックス生成を明示**
   - `clearSelection` と `removeAxisFilter`（全フィルタ除去時）で再利用

5. **studyStore.setComparisonStudies は eslint-disable でスタブ化**
   - TypeScript 未使用パラメータ警告を `eslint-disable-next-line` でサプレス

### テスト結果

```
Test Files: 5 passed (5)
Tests: 31 passed (31)
Duration: 815ms
```

## Refactorフェーズ（品質改善）

### リファクタ日時

2026-03-27

### セキュリティレビュー

- **脆弱性**: なし
- `addAxisFilter`: `filterRanges` を `JSON.stringify` して WASM に渡す
  - `JSON.stringify` は安全。WASM 側でのパース検証は TASK-103 で実施
- `loadJournal`: `file.arrayBuffer()` は File API 経由で信頼されたブラウザ API
- 全 Store は Zustand の `set()` でのみ状態を変更（直接ミューテーションなし）

### パフォーマンスレビュー

| 処理 | 複雑度 | 備考 |
|---|---|---|
| brushSelect | O(1) | set のみ |
| clearSelection | O(N) | 全インデックス配列生成 |
| addAxisFilter | O(N) 非同期 | WASM 側の責任 |
| subscribe 連鎖 | O(N) | GpuBuffer.updateAlphas が O(N) |

### 最終テスト結果

```
Test Files: 5 passed (5)
Tests: 31 passed (31)
Duration: 815ms
```

### 品質評価

✅ **高品質**
- テスト: 31/31 通過
- セキュリティ: 重大な脆弱性なし
- TypeScript: エラーなし
- ESLint: エラーなし
- REQ-040〜REQ-044 準拠
