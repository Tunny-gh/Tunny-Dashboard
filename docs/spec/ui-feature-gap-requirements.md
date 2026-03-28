# UI機能ギャップ 要件定義書

## 概要

Tunny Dashboard のフロントエンドには、型定義・ストア・コンポーネントとして実装された機能が存在するが、UI から到達できない機能が多数ある。
本書はその差を明確化し、「UI から利用可能にする」ための要件を EARS 記法で定義する。

## ギャップ分析サマリー

| 機能 | Store | UI コンポーネント | WASM | 現状 |
|---|---|---|---|---|
| Journal 読み込み | ✓ | ✓ | ✓ | **動作** |
| Study 切り替え | ✓ | ✗ ドロップダウン未実装 | ✓ | **部分** |
| チャートレンダリング | ✓ | 3/15 のみ | - | **不完全** |
| パラメータフィルタ | ✓ | ✓ | ✗ stub | **BLOCKED** |
| Trial テーブル | ✓ | ✓ (仮) | ✗ stub | **不完全** |
| CSV エクスポート | ✓ | ✓ | ✗ stub | **BLOCKED** |
| クラスタリング | ✗ store 未実装 | ✓ | ✗ | **BROKEN** |
| 感度分析/PDP | ✗ store 未実装 | ✓ | ✗ | **BROKEN** |
| ライブ更新 | ✓ | ✗ UI 未実装 | ✗ stub | **BLOCKED** |
| Study 比較 | ✓ | ✓ | ✗ stub | **BROKEN** |

---

## ユーザーストーリー

### ストーリー 1: チャート切り替え

- **である** Tunny ユーザー **として**
- **私は** 15 種類のチャートをキャンバスに配置して表示 **したい**
- **そうすることで** 最適化結果を多角的に分析できる

### ストーリー 2: Study 切り替え

- **である** 複数 Study を含むジャーナルの分析者 **として**
- **私は** UI 上のドロップダウンで Study を切り替え **たい**
- **そうすることで** ジャーナルを再読み込みせずに複数 Study を比較できる

### ストーリー 3: Trial テーブルの確認

- **である** 選択された Trial を詳細確認したいユーザー **として**
- **私は** BottomPanel のテーブルで実際のパラメータ値と目的関数値を確認 **したい**
- **そうすることで** 良好な Trial のパラメータ設定を把握できる

---

## 機能要件（EARS 記法）

### Phase 1: UI から到達可能（WASM 不要）の修正

#### REQ-001: チャートコンポーネントの ChartContent への組み込み

- REQ-001-A: システムは `scatter-matrix` を `ChartContent` でレンダリングしなければならない（`ScatterMatrix` コンポーネント使用）
- REQ-001-B: システムは `hypervolume` を `ChartContent` でレンダリングしなければならない（`HypervolumeHistory` コンポーネント使用）
- REQ-001-C: システムは `slice` を `ChartContent` でレンダリングしなければならない（`SlicePlot` コンポーネント使用）
- REQ-001-D: システムは `edf` を `ChartContent` でレンダリングしなければならない（`EdfPlot` コンポーネント使用）
- REQ-001-E: システムは `contour` を `ChartContent` でレンダリングしなければならない（`ContourPlot` コンポーネント使用）
- REQ-001-F: システムは `importance` を `ChartContent` でレンダリングしなければならない（`ImportanceChart` または暫定 ECharts）
- REQ-001-G: システムは `objective-pair-matrix` を `ChartContent` でレンダリングしなければならない（`ObjectivePairMatrix` コンポーネント使用）
- REQ-001-H: データ未ロード時、システムは `EmptyState` を表示しなければならない（「チャートは準備中です」ではなく「データを読み込んでください」）

#### REQ-002: Study セレクタ

- REQ-002-A: ToolBar は allStudies が 2 件以上のとき、Study 切り替えドロップダウンを表示しなければならない
- REQ-002-B: ドロップダウン選択時、システムは `studyStore.selectStudy(id)` を呼び出さなければならない
- REQ-002-C: 現在選択中の Study はドロップダウンのデフォルト値として表示されなければならない

#### REQ-003: BottomPanel Trial テーブルのデータバインディング

- REQ-003-A: システムは `selectedIndices` の各 Trial に対し、`gpuBuffer.positions` から目的関数値を読み取り表示しなければならない
- REQ-003-B: システムは `currentStudy.objectiveNames` をテーブルヘッダとして表示しなければならない
- REQ-003-C: データが空の場合、テーブルは「データを読み込んでください」メッセージを表示しなければならない

---

### Phase 2: WASM スタブの実装が必要な機能（定義のみ・実装は別タスク）

#### REQ-101: パラメータフィルタ（`filterByRanges` 必須）

- REQ-101-A: `filterByRanges()` WASM 実装後、LeftPanel スライダーは選択インデックスを更新しなければならない

#### REQ-102: CSV エクスポート（`serializeCsv` 必須）

- REQ-102-A: `serializeCsv()` WASM 実装後、ExportPanel の「エクスポート」ボタンはファイルダウンロードを開始しなければならない

#### REQ-103: Hypervolume（`computeHvHistory` 必須）

- REQ-103-A: `computeHvHistory()` 実装後、`hypervolume` チャートは HV 推移グラフを表示しなければならない

#### REQ-104: ライブ更新 UI

- REQ-104-A: ToolBar に「ライブ更新」トグルボタンを追加しなければならない
- REQ-104-B: トグル ON 時、`liveUpdateStore.startLive()` を呼び出さなければならない
- REQ-104-C: `appendJournalDiff()` WASM 実装後、新規 Trial がポーリングで自動追加されなければならない

---

### 状態要件

- REQ-201: Study が未選択の場合、全チャートは `EmptyState`（ファイル読み込み促進）を表示しなければならない
- REQ-202: WASM メソッドが stub の場合、関連 UI は disabled 属性を付与しなければならない（現状はサイレントにエラーになる）

### オプション要件

- REQ-301: システムは `importance` チャートに暫定的なバー棒グラフ（パラメータ名を X 軸）を表示してもよい
- REQ-302: BottomPanel は `currentStudy.paramNames` をヘッダに追加してもよい

### 制約要件

- REQ-401: 新規チャートの追加は既存の 314 テストをすべてパスしなければならない
- REQ-402: ECharts のグローバルモック（`__mocks__/echarts-for-react.tsx`）を破壊してはならない
- REQ-403: Tailwind クラスは使用してはならない（設定されていないため）。すべてインラインスタイルまたは CSS 変数を使用しなければならない

---

## 非機能要件

### パフォーマンス

- NFR-001: チャートの初期レンダリングは 500ms 以内に完了しなければならない
- NFR-002: Study 切り替えは 200ms 以内に完了しなければならない

### ユーザビリティ

- NFR-201: WASM スタブで blocked されている機能は disabled（グレーアウト）で明示しなければならない
- NFR-202: データ未ロード時は EmptyState でファイル読み込みを促しなければならない

---

## Edge ケース

### エラー処理

- EDGE-001: `gpuBuffer.trialCount === 0` の場合、チャートはクラッシュせず EmptyState を表示すること
- EDGE-002: WASM stub が呼ばれた場合、コンソールエラーのみとし UI はクラッシュしないこと
- EDGE-003: ドロップダウン選択時に `selectStudy` が失敗した場合、現在の Study は維持されること

### 境界値

- EDGE-101: allStudies が 1 件の場合、Study セレクタは非表示
- EDGE-102: `selectedIndices` が空配列の場合、Trial テーブルは「選択されていません」を表示
- EDGE-103: `gpuBuffer.positions` が undefined の場合、散布図はクラッシュせず EmptyState を表示

---

## 受け入れ基準

### Phase 1 機能テスト

- [ ] FreeLayoutCanvas の `scatter-matrix` カードに ScatterMatrix コンポーネントが表示される
- [ ] FreeLayoutCanvas の `slice` カードに SlicePlot コンポーネントが表示される
- [ ] FreeLayoutCanvas の `edf` カードに EdfPlot コンポーネントが表示される
- [ ] FreeLayoutCanvas の `contour` カードに ContourPlot コンポーネントが表示される
- [ ] FreeLayoutCanvas の `objective-pair-matrix` カードに ObjectivePairMatrix が表示される
- [ ] allStudies が 2 件以上のとき ToolBar にドロップダウンが表示される
- [ ] ドロップダウン操作で currentStudy が変更される
- [ ] BottomPanel の Trial 行に目的関数値が表示される（"—" でなく実数値）
- [ ] データ未ロード時のチャートは EmptyState を表示する
- [ ] 全 314 テストがパスする

### 非機能テスト

- [ ] Study 切り替え後 200ms 以内に全チャートが更新される
- [ ] WASM stub 機能は disabled で表示される
