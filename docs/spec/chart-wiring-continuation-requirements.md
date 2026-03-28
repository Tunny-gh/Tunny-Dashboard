# チャート配線 継続 要件定義書

## 概要

`ui-feature-gap-requirements.md` のPhase 1実装（最終コミット `c36f341`）で `scatter-matrix`・`edf`・Study セレクタ・BottomPanelデータバインディングが完了した。
本書はその後に残る **チャート配線ギャップ** と、それに必要な **新規WASMメソッド** を定義する。

既存の `ui-feature-gap-requirements.md` の REQ-101〜REQ-104（Phase 2: filterByRanges / serializeCsv / computeHvHistory / ライブ更新）は本書の対象外とし、そちらを参照すること。

---

## ギャップ分析サマリー（本書スコープ）

| チャートID | コンポーネント | 必要データ | 現状 | 本書タスク |
|---|---|---|---|---|
| `objective-pair-matrix` | ObjectivePairMatrix | gpuBuffer + currentStudy | **未配線** | Tier 1 |
| `importance` | 暫定バーチャート | currentStudy.paramNames | **未配線** | Tier 1 |
| `slice` | SlicePlot | per-trial params + values | **WASMなし** | Tier 2 |
| `contour` | ContourPlot | per-trial params + values | **WASMなし** | Tier 2 |
| `hypervolume` | HypervolumeHistory | computeHvHistory() | **Tier 3** | 既存REQ-103参照 |

---

## ユーザーストーリー

### ストーリー 1: 目的ペア行列を見る

- **である** 多目的最適化結果を分析するユーザー **として**
- **私は** Objective Pair Matrix チャートを FreeLayoutCanvas に配置 **したい**
- **そうすることで** 目的関数間の相関を直感的に把握できる

### ストーリー 2: パラメータと目的関数の関係を確認する

- **である** パラメータ感度を調べたいユーザー **として**
- **私は** Slice Plot・Contour Plot で各トライアルのパラメータ値と目的関数値を確認 **したい**
- **そうすることで** 重要なパラメータ範囲を特定できる

### ストーリー 3: パラメータ重要度を大まかに把握する

- **である** パラメータの重要度を素早く知りたいユーザー **として**
- **私は** Importance チャートで各パラメータの一覧を確認 **したい**
- **そうすることで** WASM 感度計算が利用可能になる前でも、どのパラメータが存在するかを把握できる

---

## 機能要件（EARS 記法）

### Tier 1: 既存データで配線可能（WASM 変更不要）

#### REQ-C01: `objective-pair-matrix` チャートの配線

- REQ-C01-A: `ChartContent` は `chartId === 'objective-pair-matrix'` のとき `ObjectivePairMatrix` コンポーネントを `gpuBuffer` と `currentStudy` を渡してレンダリングしなければならない
- REQ-C01-B: `currentStudy.objectiveNames.length <= 1` の場合、システムは `EmptyState` を表示しなければならない（ObjectivePairMatrix 自体が null を返すため、ラッパーで対処）
- REQ-C01-C: `gpuBuffer === null` の場合、ObjectivePairMatrix は `—` プレースホルダーセルを表示し、クラッシュしてはならない

**制約**:
- REQ-C01-X1: 現在の ObjectivePairMatrix は `positions[i*2]` / `positions[i*2+1]` を全ペアに使用する（2目的のみ正確）。N>2 目的の場合の正確な多軸対応は本要件の対象外とし、別途 `getTrials()` 実装後に対応する

#### REQ-C02: `importance` チャートの暫定表示

- REQ-C02-A: `ChartContent` は `chartId === 'importance'` のとき、`currentStudy.paramNames` を X 軸ラベルとした ECharts バーチャートを表示しなければならない（各バーの高さは均等値 1.0 で可）
- REQ-C02-B: バーチャートのタイトルには「重要度（暫定・WASM未計算）」と表示しなければならない
- REQ-C02-C: `currentStudy.paramNames.length === 0` の場合、`EmptyState` を表示しなければならない
- REQ-C02-D: WASM 感度計算（`computeSensitivity` 等）が実装された後、本暫定表示は実データに差し替えられなければならない（本要件は暫定版のみを対象とする）

#### REQ-C03: ChartContent デフォルトケースの EmptyState 修正

- REQ-C03-A: `ChartContent` の `default` ケース（未実装チャート）は `EmptyState` に `message="このチャートは準備中です"` を渡してレンダリングしなければならない
- REQ-C03-B: データ未ロード時（`currentStudy === null`）の EmptyState は `message="データを読み込んでください"` を表示しなければならない

---

### Tier 2: 新規 WASM メソッドが必要

#### REQ-C04: `getTrials` WASM メソッドの追加

Rust の `dataframe.rs` はすでにパラメータ列・目的関数列をメモリ内に保持している（`get_numeric_column()` / `get_string_column()` で列アクセス可能）。これを JS に公開する新規 WASM メソッドを追加する。

- REQ-C04-A: `rust_core/src/lib.rs` に `#[wasm_bindgen(js_name = "getTrials")]` 関数を追加しなければならない
- REQ-C04-B: `getTrials()` は引数なしで現在アクティブな Study のトライアルデータを返さなければならない
- REQ-C04-C: 戻り値の型は以下の JSON 配列でなければならない:
  ```
  TrialData[]:
    trialId: number
    params: Record<string, number>    // 数値パラメータのみ（カテゴリは文字列キー→NaN相当で除外可）
    values: number[]                   // 目的関数値（COMPLETE のみ）
    paretoRank: number | null
  ```
- REQ-C04-D: `getTrials()` は `with_active_df()` を使用し、`param_col_names()` と `get_numeric_column()` でデータを読み取らなければならない
- REQ-C04-E: アクティブ Study が選択されていない場合、空配列 `[]` を返さなければならない
- REQ-C04-F: `frontend/src/wasm/pkg/tunny_core.d.ts` に `getTrials(): any` の型宣言を追加しなければならない
- REQ-C04-G: `frontend/src/wasm/wasmLoader.ts` の `WasmModule` インターフェースに `getTrials(): TrialData[]` を追加しなければならない

**性能要件**:
- REQ-C04-P1: 10,000 トライアルの `getTrials()` 呼び出しは 200ms 以内に完了しなければならない

#### REQ-C05: `slice` チャートの配線

- REQ-C05-A: `getTrials()` WASM 実装後、`ChartContent` は `chartId === 'slice'` のとき `SlicePlot` コンポーネントを表示しなければならない
- REQ-C05-B: `SlicePlot` に渡す `trials` は `getTrials()` の戻り値を `SliceTrial[]` 型に変換したものでなければならない
- REQ-C05-C: `SlicePlot` に渡す `paramNames` は `currentStudy.paramNames`、`objectiveNames` は `currentStudy.objectiveNames` でなければならない
- REQ-C05-D: `getTrials()` が空配列を返す場合、`SlicePlot` は `EmptyState` を表示しなければならない

#### REQ-C06: `contour` チャートの配線

- REQ-C06-A: `getTrials()` WASM 実装後、`ChartContent` は `chartId === 'contour'` のとき `ContourPlot` コンポーネントを表示しなければならない
- REQ-C06-B: `ContourPlot` に渡す `trials` は `getTrials()` の戻り値を `ContourTrial[]` 型に変換したものでなければならない
- REQ-C06-C: `ContourPlot` に渡す `paramNames` は `currentStudy.paramNames`、`objectiveNames` は `currentStudy.objectiveNames` でなければならない
- REQ-C06-D: `getTrials()` が空配列を返す場合、`ContourPlot` は `EmptyState` を表示しなければならない

---

### 状態要件

- REQ-C201: `currentStudy === null` のとき、すべての `ChartContent` は「データを読み込んでください」メッセージを持つ `EmptyState` を表示しなければならない
- REQ-C202: `gpuBuffer.trialCount === 0` のとき、チャートはクラッシュせず `EmptyState` を表示しなければならない

---

## 非機能要件

### パフォーマンス

- NFR-C01: `ChartContent` の再レンダリングは `getTrials()` 呼び出しをメモ化（`useMemo` / `useCallback`）し、Study 変更時のみ再計算しなければならない
- NFR-C02: `objective-pair-matrix` の初期表示は 500ms 以内に完了しなければならない

### テスト

- NFR-C03: Tier 1 実装後、既存の 314 テストがすべてパスしなければならない
- NFR-C04: `getTrials()` WASM メソッドは Rust 単体テストと TypeScript ユニットテストの両方でテストされなければならない

### 制約

- NFR-C05: Tailwind クラスは使用してはならない。すべてインラインスタイルまたは CSS 変数を使用しなければならない
- NFR-C06: ECharts グローバルモック（`__mocks__/echarts-for-react.tsx`）を破壊してはならない

---

## Edge ケース

### エラー処理

- EDGE-C01: `getTrials()` 呼び出し中に WASM が未初期化の場合、`ChartContent` はエラーをスローせず `EmptyState` を表示しなければならない
- EDGE-C02: `currentStudy.paramNames.length < 2` の場合、`ContourPlot` は「パラメータが2つ以上必要です」の `EmptyState` を表示しなければならない
- EDGE-C03: `objective-pair-matrix` で `currentStudy.objectiveNames.length === 1` の場合、`EmptyState`（「多目的 Study でのみ利用可能です」）を表示しなければならない

### 境界値

- EDGE-C101: `currentStudy.paramNames.length === 0` の場合、`importance` チャートは `EmptyState` を表示しなければならない
- EDGE-C102: `getTrials()` の戻り値が 0 件の場合、`slice` / `contour` チャートは `EmptyState` を表示しなければならない
- EDGE-C103: Pareto ランクが計算されていない場合、`SliceTrial.paretoRank` は `null` を返さなければならない

---

## 受け入れ基準

### Tier 1 機能テスト

- [ ] FreeLayoutCanvas の `objective-pair-matrix` カードに ObjectivePairMatrix が表示される（2目的 Study）
- [ ] 1目的 Study の場合、`objective-pair-matrix` カードは EmptyState を表示する
- [ ] FreeLayoutCanvas の `importance` カードに paramNames をラベルとするバーチャートが表示される
- [ ] データ未ロード時のチャートは「データを読み込んでください」EmptyState を表示する
- [ ] 全テストがパスする（既存 314 件 + 新規テスト）

### Tier 2 機能テスト

- [ ] `wasm.getTrials()` が `[{ trialId, params, values }]` 形式の配列を返す
- [ ] FreeLayoutCanvas の `slice` カードに SlicePlot が表示される（パラメータ選択ドロップダウンあり）
- [ ] FreeLayoutCanvas の `contour` カードに ContourPlot が表示される（X/Y 軸選択ドロップダウンあり）
- [ ] Study 切り替え時に slice / contour チャートのデータが更新される

### 非機能テスト

- [ ] 10,000 トライアルの Study で `getTrials()` が 200ms 以内に完了する
- [ ] Tier 1 実装後、既存 314 テストがすべてパスする

---

## 実装優先順位

1. **Tier 1-A**: REQ-C01（objective-pair-matrix）→ コード量が少なく即効果が高い
2. **Tier 1-B**: REQ-C02（importance 暫定）+ REQ-C03（EmptyState メッセージ修正）
3. **Tier 2-A**: REQ-C04（getTrials WASM）→ Rust + TS 型定義の追加
4. **Tier 2-B**: REQ-C05（slice）+ REQ-C06（contour）→ getTrials 完成後に並行実装可能
