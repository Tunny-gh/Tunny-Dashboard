# TASK-302 テストケース一覧: Zustand Store群

## 開発言語・フレームワーク

- **プログラミング言語**: TypeScript 🟢
- **テストフレームワーク**: Vitest 🟢
- **テスト実行コマンド**: `cd frontend && npx vitest run --reporter=verbose 2>&1` 🟢

---

## 正常系テストケース

### TC-302-01: brushSelect が selectedIndices を更新する

- **入力**: `brushSelect(new Uint32Array([1, 3, 5]))`
- **期待結果**: `selectedIndices === [1, 3, 5]` 🟢

### TC-302-02: clearSelection が全インデックスを返す

- **入力**: `_trialCount=5` に初期化後、`clearSelection()`
- **期待結果**: `selectedIndices === [0,1,2,3,4]`, `filterRanges === {}` 🟢

### TC-302-03: addAxisFilter が filterRanges を同期更新する

- **入力**: `addAxisFilter('x', 0.1, 0.9)`
- **期待結果**: `filterRanges['x'] === { min: 0.1, max: 0.9 }` （同期で確認可能） 🟢

### TC-302-04: addAxisFilter が WASM filterByRanges を呼び出す

- **入力**: WasmLoader モック済み、`addAxisFilter('x', 0, 1)`
- **期待結果**: `wasm.filterByRanges` が呼ばれ、返値で `selectedIndices` が更新される 🟢

### TC-302-05: removeAxisFilter が filterRanges からキーを除去する

- **入力**: `addAxisFilter('x', 0, 1)` 後に `removeAxisFilter('x')`
- **期待結果**: `filterRanges['x']` が undefined になる 🟢

### TC-302-06: setHighlight が highlighted を更新する

- **入力**: `setHighlight(7)`
- **期待結果**: `highlighted === 7` 🟢

### TC-302-07: setColorMode が colorMode を更新する

- **入力**: `setColorMode('cluster')`
- **期待結果**: `colorMode === 'cluster'` 🟢

### TC-302-08: selectionStore.subscribe で GpuBuffer が自動更新される

- **入力**: GpuBuffer 作成後、`selectionStore.subscribe` でアルファ更新ロジックを登録、`brushSelect([2, 4])` 呼び出し
- **期待結果**: `gpuBuffer.colors[2*4+3] === 1.0`（選択）、`gpuBuffer.colors[0*4+3] === 0.2`（非選択） 🟢

### TC-302-09: studyStore.loadJournal が WASM parseJournal を呼び出す

- **入力**: WasmLoader モック済み、`loadJournal(mockFile)`
- **期待結果**: `parseJournal` が呼ばれ、`allStudies` が更新される 🟢

### TC-302-10: studyStore.loadJournal 中は isLoading=true になる

- **入力**: loadJournal 開始前後の isLoading 状態
- **期待結果**: 開始時 `isLoading=true`、完了後 `isLoading=false` 🟢

### TC-302-11: layoutStore.setLayoutMode が layoutMode を更新する

- **入力**: `setLayoutMode('B')`
- **期待結果**: `layoutMode === 'B'` 🟢

### TC-302-12: layoutStore.toggleChart が visibleCharts を更新する

- **入力**: `toggleChart('history')`（初期状態で含まれている場合）
- **期待結果**: `visibleCharts` から 'history' が除去される 🟢

### TC-302-13: layoutStore.saveLayout / loadLayout でレイアウト保存・復元できる

- **入力**: モード・チャート・サイズを設定後 `saveLayout()` → 別のレイアウトに変更 → `loadLayout(config)`
- **期待結果**: 元のレイアウト設定が完全に復元される 🟢

---

## 異常系テストケース

### TC-302-E01: studyStore.loadJournal が失敗した場合 loadError が設定される

- **入力**: WasmLoader.parseJournal がエラー throw するモック
- **期待結果**: `loadError` にエラーメッセージが設定される、`isLoading=false` 🟢

### TC-302-E02: addAxisFilter WASM 未初期化でもクラッシュしない

- **入力**: WasmLoader.getInstance() が reject するモック
- **期待結果**: `filterRanges` は更新される、クラッシュなし 🟢

---

## 境界値テストケース

### TC-302-B01: clearSelection で _trialCount=0 の場合も crash しない

- **入力**: `_trialCount=0` の状態で `clearSelection()`
- **期待結果**: `selectedIndices.length === 0`、クラッシュなし 🟢

### TC-302-B02: removeAxisFilter 存在しないキーでも crash しない

- **入力**: filterRanges が空の状態で `removeAxisFilter('nonexistent')`
- **期待結果**: filterRanges が空のまま、クラッシュなし 🟢

---

## テストケースサマリー

| 分類 | 件数 |
|---|---|
| 正常系 | 13件 |
| 異常系 | 2件 |
| 境界値 | 2件 |
| **合計** | **17件** |

---

## 品質判定

✅ **高品質**
- selectionStore テストは WasmLoader を vi.mock でモック
- studyStore テストは File API と WasmLoader を vi.mock でモック
- subscribe 連鎖テストは Zustand の実 subscribe API を使用（モック不要）
