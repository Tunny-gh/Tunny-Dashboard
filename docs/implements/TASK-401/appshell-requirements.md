# TASK-401 要件定義: AppShell・ToolBar

## 1. 機能概要

- **対象コンポーネント**:
  1. `AppShell` — 4エリア CSS Grid レイアウトのアプリ骨格
  2. `ToolBar` — ファイル読込・Study選択・レイアウト切替 UI
- **参照要件**: REQ-030, REQ-032
- **依存**: TASK-002 ✅（型定義）

## 2. 入出力仕様

### AppShell

**動作**:
- `layoutStore.layoutMode` に応じて4エリア（ToolBar/LeftPanel/MainCanvas/BottomPanel）のグリッドを変更
- `onDrop` でファイルドラッグ&ドロップを受け取り → `studyStore.loadJournal(file)` を呼ぶ
- `data-testid="app-shell"` と `data-layout={layoutMode}` 属性でテスト可能にする

**レイアウトモード**:
- モード A: 4エリア全表示（ToolBar + LeftPanel + MainCanvas + BottomPanel）
- モード B〜D: LeftPanel 非表示 / BottomPanel 非表示 など（将来実装）

### ToolBar

**動作**:
- `<input type="file">` でファイル選択 → `studyStore.loadJournal(file)` 呼び出し
- レイアウトモード A/B/C/D ボタン → `layoutStore.setLayoutMode(mode)` 呼び出し
- `isLoading=true` のときローディングインジケータを表示

## 3. テスト要件

1. ファイルドロップで `loadJournal` が呼ばれる
2. ToolBar ファイル input 変更で `loadJournal` が呼ばれる
3. レイアウトモードボタンクリックで `setLayoutMode` が呼ばれる
4. `isLoading=true` のとき Loading 表示が出る
