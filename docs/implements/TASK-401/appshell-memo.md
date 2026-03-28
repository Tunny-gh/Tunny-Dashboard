# TDD開発メモ: appshell

## 概要

- 機能名: AppShell・ToolBar（4エリアレイアウト + ファイル読込UI）
- 開発開始: 2026-03-27
- 現在のフェーズ: 完了

## 関連ファイル

- 要件定義: `docs/implements/TASK-401/appshell-requirements.md`
- 実装ファイル:
  - `frontend/src/components/layout/AppShell.tsx`
  - `frontend/src/components/layout/ToolBar.tsx`
- テストファイル:
  - `frontend/src/components/layout/AppShell.test.tsx`
  - `frontend/src/components/layout/ToolBar.test.tsx`

## Redフェーズ（失敗するテスト作成）

### テストケース（8件）

| ID | 分類 | 内容 |
|---|---|---|
| TC-401-01 | 正常系 | AppShell エラーなくレンダリング |
| TC-401-01b | 正常系 | data-layout 属性に layoutMode が反映 |
| TC-401-02 | 正常系 | ファイルドロップで loadJournal 呼び出し |
| TC-401-03 | 正常系 | ToolBar エラーなくレンダリング |
| TC-401-04 | 正常系 | ファイル input 変更で loadJournal 呼び出し |
| TC-401-05 | 正常系 | レイアウトボタン A → setLayoutMode('A') |
| TC-401-06 | 正常系 | レイアウトボタン B → setLayoutMode('B') |
| TC-401-E01 | 異常系 | isLoading=true でローディングインジケータ表示 |

## Greenフェーズ（最小実装）

### 主要な設計決定

1. **AppShell の CSS Grid 設計**
   - `gridTemplateRows: 'auto 1fr auto'` / `gridTemplateColumns: 'auto 1fr'`
   - ToolBar は `gridColumn: '1 / -1'`、BottomPanel も同様

2. **ドラッグ&ドロップ実装**
   - `onDragOver` で `e.preventDefault()` → ドロップを有効化
   - `onDrop` で `e.dataTransfer?.files?.[0]` を `loadJournal` に渡す

3. **isLoading インジケータ**
   - `position: fixed` + `top: 0` のプログレスバー
   - `data-testid="loading-indicator"` でテスト可能

4. **TypeScript cast パターン**
   - `vi.mocked(useStudyStore)` が Zustand の複雑な型に合わない場合
   - `(store as unknown as ReturnType<typeof vi.fn>).mockImplementation(...)` で対処

## Refactorフェーズ（品質改善）

### セキュリティレビュー

- **脆弱性**: なし
- `onDrop`: `dataTransfer.files[0]` を直接 `loadJournal` に渡すのみ（インジェクション不可）
- ファイル種別検証は `input[accept=".log,.journal"]` で UI レベルで制限

### パフォーマンスレビュー

- CSS Grid は宣言的で高速（再レンダリング時も DOM 変更最小）
- `loadJournal` は fire-and-forget（`void` で実行）

### 最終テスト結果

```
Test Files: 11 passed (11)
Tests: 57 passed (57)
Duration: 3.30s
```

### 品質評価

✅ **高品質**
- テスト: 57/57 通過
- セキュリティ: 重大な脆弱性なし
- TypeScript（新規ファイル）: エラーなし
- ESLint: エラーなし
- REQ-030, REQ-032 準拠
