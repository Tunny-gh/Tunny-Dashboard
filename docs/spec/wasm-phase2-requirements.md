# WASM Phase 2 バインディング 要件定義書

## 概要

`ui-feature-gap-requirements.md` の REQ-101〜REQ-104 および本書独自の REQ-105 として定義する **WASM Phase 2** の実装要件。

すべての Rust 実装（`filter_by_ranges`・`serialize_csv`・`compute_hypervolume_history`・`append_journal_diff`・`compute_report_stats`）は `rust_core/src/` に既に実装済みである。
**本書のスコープ** は Rust → WASM bindgen 公開・TypeScript 型宣言・`wasmLoader.ts` バインディング・フロントエンド UI 配線の 4 層を一気通貫で実装することにある。

### 既実装の Rust 関数（対応ファイル）

| Rust 関数 | ファイル | ステータス |
|---|---|---|
| `filter_by_ranges(ranges_json)` | `filter.rs` | 実装済み・15+ テスト |
| `serialize_csv(indices, columns_json)` | `export.rs` | 実装済み・10+ テスト |
| `compute_hypervolume_history(is_minimize)` | `pareto.rs` | 実装済み |
| `append_journal_diff(data)` | `live_update.rs` | 実装済み・7 テスト |
| `compute_report_stats()` | `export.rs` | 実装済み |

### 現状ギャップ（`tunny_core.d.ts` の公開関数）

`tunny_core.d.ts` は現在 `main()` / `parseJournal()` / `selectStudy()` / `getTrials()` のみを公開しており、
上記 5 関数の WASM bindgen エクスポートが存在しない。
`wasmLoader.ts` の該当メソッドはすべて `_notImplemented(name)` スタブで実装されている。

---

## ユーザーストーリー

### ストーリー 1: スライダーで試行を絞り込む

- **である** パラメータ範囲を絞って最適解を探したいユーザー **として**
- **私は** LeftPanel のスライダーを動かして Pareto 散布図のポイントを即座に絞り込み **たい**
- **そうすることで** 実現可能な設計空間のみを確認できる

### ストーリー 2: フィルタ済み試行を CSV 出力する

- **である** 外部ツールで追加分析したいユーザー **として**
- **私は** ExportPanel の「エクスポート」ボタンを押して選択試行の CSV をダウンロード **したい**
- **そうすることで** Excel や Python で自由に集計できる

### ストーリー 3: Hypervolume 推移で最適化進捗を確認する

- **である** 多目的最適化のパフォーマンスを評価したいユーザー **として**
- **私は** Hypervolume 推移チャートを FreeLayoutCanvas に表示 **したい**
- **そうすることで** アルゴリズムが時間経過とともに改善しているかを定量的に把握できる

### ストーリー 4: Journal ファイルをリアルタイム監視する

- **である** 実行中の最適化を継続的に監視したいユーザー **として**
- **私は** ToolBar のライブ更新ボタンで差分ポーリングを ON/OFF し **たい**
- **そうすることで** Optuna が走っている間にダッシュボードを最新状態に保てる

### ストーリー 5: HTML レポートを生成する

- **である** 最適化結果を報告書にまとめたいユーザー **として**
- **私は** ExportPanel から HTML レポートを生成・ダウンロード **したい**
- **そうすることで** ブラウザで開ける単一ファイルの報告書を配布できる

---

## 機能要件（EARS 記法）

### REQ-101: `filterByRanges` WASM バインディング

#### Rust → WASM 公開（lib.rs）

- REQ-101-A: `rust_core/src/lib.rs` に `#[wasm_bindgen(js_name = "filterByRanges")]` 関数を追加しなければならない
- REQ-101-B: `wasm_filter_by_ranges(ranges_json: &str)` は `filter::filter_by_ranges(ranges_json)` を呼び出し、結果の `Vec<u32>` を `js_sys::Uint32Array` に変換して `JsValue` として返さなければならない
- REQ-101-C: `filter_by_ranges` がエラーを返した場合、`Err(JsValue::from_str(&e))` を返さなければならない

#### TypeScript 型宣言（tunny_core.d.ts）

- REQ-101-D: `tunny_core.d.ts` に `export function filterByRanges(ranges_json: string): Uint32Array;` を追加しなければならない

#### WasmLoader バインディング（wasmLoader.ts）

- REQ-101-E: `wasmLoader.ts` の `_initialize()` で `loader.filterByRanges` を `(rangesJson: string) => wasmFilterByRanges(rangesJson) as Uint32Array` にバインドしなければならない
- REQ-101-F: `filterByRanges` の `_notImplemented` スタブを削除しなければならない

#### 動作確認

- REQ-101-G: LeftPanel のスライダーを操作したとき、`selectionStore.addAxisFilter()` が `wasm.filterByRanges()` を呼び出し、結果の `Uint32Array` を `selectedIndices` に設定しなければならない

---

### REQ-102: `serializeCsv` WASM バインディング

#### Rust → WASM 公開（lib.rs）

- REQ-102-A: `rust_core/src/lib.rs` に `#[wasm_bindgen(js_name = "serializeCsv")]` 関数を追加しなければならない
- REQ-102-B: `wasm_serialize_csv(indices_js: js_sys::Array, columns_json: &str)` は `indices_js` を `Vec<u32>` に変換してから `export::serialize_csv(&indices, columns_json)` を呼び出し、結果の `String` を `JsValue` として返さなければならない
- REQ-102-C: `serialize_csv` がエラーを返した場合、`Err(JsValue::from_str(&e))` を返さなければならない

#### TypeScript 型宣言（tunny_core.d.ts）

- REQ-102-D: `tunny_core.d.ts` に `export function serializeCsv(indices: number[], columns_json: string): string;` を追加しなければならない

#### WasmLoader バインディング（wasmLoader.ts）

- REQ-102-E: `wasmLoader.ts` の `_initialize()` で `loader.serializeCsv` を `(indices: number[], columnsJson: string) => wasmSerializeCsv(indices, columnsJson) as string` にバインドしなければならない
- REQ-102-F: `serializeCsv` の `_notImplemented` スタブを削除しなければならない

#### 動作確認

- REQ-102-G: ExportPanel の「エクスポート」ボタン押下で `exportStore.exportCsv()` が `wasm.serializeCsv()` を呼び出し、UTF-8 CSV の Blob を `<a>` タグでダウンロードしなければならない

---

### REQ-103: `computeHvHistory` WASM バインディングと `hypervolume` チャート配線

#### Rust → WASM 公開（lib.rs）

- REQ-103-A: `rust_core/src/lib.rs` に `#[wasm_bindgen(js_name = "computeHvHistory")]` 関数を追加しなければならない
- REQ-103-B: `wasm_compute_hv_history(is_minimize_js: js_sys::Array)` は `is_minimize_js` を `Vec<bool>` に変換してから `pareto::compute_hypervolume_history(&is_minimize)` を呼び出し、`{ trialIds: Uint32Array, hvValues: Float64Array }` 形式の JS オブジェクトとして返さなければならない
- REQ-103-C: 目的関数数が 0 の場合など `compute_hypervolume_history` がエラーを返した場合は `Err(JsValue::from_str(&e))` を返さなければならない

#### TypeScript 型宣言（tunny_core.d.ts）

- REQ-103-D: `tunny_core.d.ts` に以下を追加しなければならない
  ```ts
  export interface HvHistoryResult {
    trialIds: Uint32Array;
    hvValues: Float64Array;
  }
  export function computeHvHistory(is_minimize: boolean[]): HvHistoryResult;
  ```

#### WasmLoader バインディング（wasmLoader.ts）

- REQ-103-E: `wasmLoader.ts` の `_initialize()` で `loader.computeHvHistory` を `(isMinimize: boolean[]) => wasmComputeHvHistory(isMinimize) as HvHistoryResult` にバインドしなければならない
- REQ-103-F: `computeHvHistory` の `_notImplemented` スタブを削除しなければならない

#### FreeLayoutCanvas チャート配線

- REQ-103-G: `FreeLayoutCanvas.tsx` の `ChartContent` switch 文に `case 'hypervolume':` を追加し、`HypervolumeHistory` コンポーネントをレンダリングしなければならない
- REQ-103-H: `hypervolume` チャートの表示時、`currentStudy.directions` から `isMinimize` 配列を導出して `wasm.computeHvHistory(isMinimize)` を呼び出し、結果を `HypervolumeDataPoint[]` に変換して `HypervolumeHistory` に渡さなければならない
- REQ-103-I: `currentStudy.directions.length < 2` の場合（単目的 Study）、`EmptyState` を `message="多目的 Study でのみ利用可能です"` で表示しなければならない
- REQ-103-J: WASM 呼び出し中はローディング状態を表示し、エラー時は `EmptyState` に `message="HV 計算エラー"` を表示しなければならない

---

### REQ-104: `appendJournalDiff` WASM バインディングとライブ更新 UI

#### Rust → WASM 公開（lib.rs）

- REQ-104-A: `rust_core/src/lib.rs` に `#[wasm_bindgen(js_name = "appendJournalDiff")]` 関数を追加しなければならない
- REQ-104-B: `wasm_append_journal_diff(data: &[u8])` は `live_update::append_journal_diff(data)` を呼び出し、`{ newCompleted: number, consumedBytes: number }` 形式の JS オブジェクトを返さなければならない
- REQ-104-C: `append_journal_diff` がエラーを返した場合は `Err(JsValue::from_str(&e))` を返さなければならない

#### TypeScript 型宣言（tunny_core.d.ts）

- REQ-104-D: `tunny_core.d.ts` に `export function appendJournalDiff(data: Uint8Array): { newCompleted: number; consumedBytes: number };` を追加しなければならない

#### WasmLoader バインディング（wasmLoader.ts）

- REQ-104-E: `wasmLoader.ts` の `_initialize()` で `loader.appendJournalDiff` を `(data: Uint8Array) => wasmAppendJournalDiff(data) as { new_completed: number; consumed_bytes: number }` にバインドしなければならない
- REQ-104-F: `appendJournalDiff` の `_notImplemented` スタブを削除しなければならない

#### ToolBar ライブ更新ボタン

- REQ-104-G: `ToolBar.tsx` に「ライブ更新」トグルボタンを追加しなければならない
- REQ-104-H: ボタンは `useLiveUpdateStore` の `isLive`・`isSupported`・`startLive()`・`stopLive()` を使用しなければならない
- REQ-104-I: `isSupported === false` の場合、ボタンは disabled かつ `title="このブラウザは対応していません（Chrome/Edge 推奨）"` を持たなければならない
- REQ-104-J: `isLive === true` の場合、ボタンは「ライブ停止」を示すスタイル（赤系）で表示し、クリックで `stopLive()` を呼び出さなければならない
- REQ-104-K: `isLive === false` の場合、ボタンは「ライブ開始」を示すスタイル（緑系）で表示し、クリックで `startLive()` を呼び出さなければならない
- REQ-104-L: ToolBar はスタイリングに Tailwind CSS を使用してはならない（インラインスタイルまたは CSS 変数のみ使用）

---

### REQ-105: `computeReportStats` WASM バインディング

#### Rust → WASM 公開（lib.rs）

- REQ-105-A: `rust_core/src/lib.rs` に `#[wasm_bindgen(js_name = "computeReportStats")]` 関数を追加しなければならない
- REQ-105-B: `wasm_compute_report_stats()` は `export::compute_report_stats()` を呼び出し、結果の JSON 文字列を `JsValue` として返さなければならない
- REQ-105-C: エラー時は `Err(JsValue::from_str(&e))` を返さなければならない

#### TypeScript 型宣言（tunny_core.d.ts）

- REQ-105-D: `tunny_core.d.ts` に `export function computeReportStats(): string;` を追加しなければならない

#### WasmLoader バインディング（wasmLoader.ts）

- REQ-105-E: `wasmLoader.ts` の `_initialize()` で `loader.computeReportStats` を `() => wasmComputeReportStats() as string` にバインドしなければならない
- REQ-105-F: `computeReportStats` の `_notImplemented` スタブを削除しなければならない

#### 動作確認

- REQ-105-G: ExportPanel の「レポート生成」ボタン押下で `exportStore.generateHtmlReport()` が `wasm.computeReportStats()` を呼び出し、HTML ドキュメントを生成してダウンロードしなければならない

---

## 非機能要件

### パフォーマンス

- NFR-001: `filterByRanges` は 50,000 試行 × 3 軸フィルタ条件を 5 ms 以内で完了しなければならない（Rust 側で既に保証済み）
- NFR-002: `computeHvHistory` は 1,000 試行で 200 ms 以内に完了しなければならない

### エラー耐性

- NFR-101: WASM 関数の呼び出しが失敗した場合、フロントエンドはクラッシュせず `EmptyState` または `console.error` でエラーを通知しなければならない
- NFR-102: `appendJournalDiff` が 3 回連続でエラーを返した場合、`FsapiPoller` は自動的にポーリングを停止しなければならない（既存 `MAX_ERROR_COUNT` ロジックで保証済み）

### スタイル制約

- NFR-201: 新規追加フロントエンドコンポーネントはすべて Tailwind CSS を使用してはならない（インラインスタイルまたは CSS 変数のみ）

---

## Edge ケース

### REQ-101 (filterByRanges)

- EDGE-101-A: `ranges_json` が `{}` の場合（フィルタなし）、全試行インデックスを返さなければならない
- EDGE-101-B: すべての試行が条件を外れる場合、空の `Uint32Array` を返しチャートは `EmptyState` を表示しなければならない
- EDGE-101-C: 軸名が DataFrame の列名と一致しない場合、その軸のフィルタ条件を無視して他の条件のみ適用しなければならない

### REQ-102 (serializeCsv)

- EDGE-102-A: `indices` が空の場合、ヘッダー行のみの CSV を返さなければならない
- EDGE-102-B: `columns_json` が `"[]"` の場合、全列を含む CSV を返さなければならない
- EDGE-102-C: パラメータ値にカンマ・ダブルクォートを含む場合、RFC 4180 に従って正しくエスケープされなければならない

### REQ-103 (computeHvHistory)

- EDGE-103-A: Pareto 点が 1 点のみの場合、HV 値 1 点の配列を返さなければならない
- EDGE-103-B: 全試行が同一の目的関数値を持つ場合、HV = 0 を返しチャートは平坦な線を表示しなければならない
- EDGE-103-C: `is_minimize` の長さと目的関数の次元数が一致しない場合、エラーを返し `EmptyState` を表示しなければならない

### REQ-104 (appendJournalDiff / ライブ更新 UI)

- EDGE-104-A: `isSupported === false` のブラウザ（Firefox, Safari）でライブ更新ボタンを押した場合、エラーメッセージを表示しなければならない
- EDGE-104-B: `appendJournalDiff` で `new_completed === 0` の場合（新規完了試行なし）、UI を更新しなくてよい
- EDGE-104-C: ライブ更新中にユーザーが別の Study を選択した場合、ポーリングは停止されなければならない

### REQ-105 (computeReportStats)

- EDGE-105-A: アクティブ Study が未選択の場合、`compute_report_stats()` は空 JSON `{}` を返し HTML レポートに「データなし」と表示しなければならない

---

## 受け入れ基準

### 機能テスト

- [ ] TC-101: LeftPanel スライダー操作で散布図のポイントが絞り込まれる（`filterByRanges` 正常呼び出し）
- [ ] TC-102: ExportPanel「エクスポート」で `.csv` ファイルがダウンロードされる
- [ ] TC-103: 多目的 Study で `hypervolume` チャートが HV 推移折れ線グラフを表示する
- [ ] TC-104: ToolBar にライブ更新ボタンが表示され、クリックで `liveUpdateStore.startLive()` が呼ばれる
- [ ] TC-105: ExportPanel「レポート生成」で `.html` ファイルがダウンロードされる

### Edge ケーステスト

- [ ] TC-101-E: フィルタ条件が全試行を除外するとき散布図に EmptyState が表示される
- [ ] TC-103-E: 単目的 Study で `hypervolume` チャートが `EmptyState` を表示する
- [ ] TC-104-E: Chrome/Edge 以外のブラウザでライブ更新ボタンが disabled になる

### 非機能テスト

- [ ] TC-NFR-001: `filterByRanges` を 50,000 試行で呼び出して 5 ms 以内に完了する（Rust 単体テストで確認済み）
- [ ] TC-NFR-201: 新規 UI に Tailwind クラスが使用されていない（コードレビュー）
