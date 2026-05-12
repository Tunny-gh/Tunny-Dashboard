# ダッシュボード不足機能 アーキテクチャ設計

**作成日**: 2026-05-12
**関連要件定義**: [requirements.md](../../spec/dashboard-missing-features/requirements.md)
**ヒアリング記録**: [design-interview.md](design-interview.md)
**関連詳細設計**:
- [pdp-observed-overlay/architecture.md](../pdp-observed-overlay/architecture.md)
- [lightgbm-surface-plot/architecture.md](../lightgbm-surface-plot/architecture.md)
- [surface-plot-surrogate-models/architecture.md](../surface-plot-surrogate-models/architecture.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 現行コード・要件定義書・既存設計文書を根拠にした確実な設計
- 🟡 **黄信号**: 現行コード・要件定義書から妥当と判断した推測を含む設計
- 🔴 **赤信号**: 現行コード・要件定義書に根拠が薄く追加検証が必要な設計

---

## システム概要 🔵

**信頼性**: 🔵 *requirements.md・toolbar.rs・layout_state.rs・comparison_panel.rs・trial_table.rs より*

本設計は、Tunny Dashboard に残っている 8 つの不足機能を、既存の Rust + egui アーキテクチャを崩さずに統合するための横断設計である。

対象機能:

1. CSV Export UI
2. Comparison Study 追加 UI
3. Trial ピン留め UI
4. PDP Observed Data Overlay の仕上げ
5. Surface Plot ウィジェット
6. Brushing & Linking
7. Comparison Diff タブ
8. チャート PNG 保存

本件の中心方針は次の 3 点である。

- **UI は `egui-app` に閉じ込める**: ファイルダイアログ、ポップアップ、セルヘッダーメニュー、比較タブ追加は `egui-app` の責務とする。
- **計算・重い処理は既存の `spawn_task` + `AppMessage` に乗せる**: Surface Plot と Comparison Study ロードは既存の非同期パターンを再利用する。
- **選択状態は `AppState` に一本化する**: Brushing、ピン留め、PDP overlay、Trial Table 表示条件を単一の可視性ポリシーで接続する。

---

## 現行ベースラインと設計上の含意 🔵

**信頼性**: 🔵 *現行ワークスペースのコード読み取りより*

| 機能 | 現行コードの状態 | 設計上の扱い |
|---|---|---|
| F-001 CSV Export UI | `io/export.rs` に `ExportTarget` とフィルタ関数あり、ツールバー UI は未接続 | UI 追加と保存ヘルパー追加で対応 |
| F-002 Comparison Study 追加 UI | `AppMessage::ComparisonStudyLoaded` と `app_state.comparison_studies` は存在 | ツールバー起点のロード入口と削除・リセット規約を追加 |
| F-003 ピン留め UI | `session.rs` に `pinned_trials` あり、`AppState` と Trial Table に反映なし | `AppState` へ昇格し、表示ポリシーに統合 |
| F-004 PDP Overlay | `pdp_chart.rs` に `show_observed` と散布点描画はすでに存在 | 新規描画実装は不要。`selected_indices` / `pinned_trials` 連動だけ追加 |
| F-005 Surface Plot | `ChartId`、`WidgetStates`、`render_chart`、`poll_chart` に未登録 | 新規ウィジェットとして統合 |
| F-006 Brushing & Linking | `ParetoScatter2D` は `selected_indices` を読むが書かない。`ParallelCoordsChart` は `brush_ranges` を持つが未接続 | 選択の書き込み経路を追加し、既存 state を活かして仕上げる |
| F-007 Comparison Diff | `ComparisonView` に `Diff` がない | 純関数ベースの比較メトリクス層を追加 |
| F-008 PNG 保存 | `grid_canvas.rs` のセルヘッダーは `Move` / `?` / `x` のみ | セル単位キャプチャパイプラインを追加 |

この差分により、F-004 と F-006 は「完全新規」ではなく、**既存実装の仕上げ**として扱う。

---

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *app.rs・render_chart.rs・poll_chart.rs・widget_states.rs より*

- **データ状態**: `AppState`
  - Study、選択、比較対象、アーティファクト、MCDM 結果など永続的な状態を保持する。
- **UI 状態**: `WidgetStates`
  - 各チャート固有の一時状態、進行中フラグ、キャッシュ、ヘルプモーダル状態を保持する。
- **描画**: `render_chart.rs`
  - `ChartId` ごとの描画ルーティングを担当する。
- **非同期実行**: `poll_chart.rs` + `AppMessage`
  - `pending_*` フラグを検出し、`spawn_task()` でバックグラウンド処理を起動する。
- **ツールバー入口**: `toolbar.rs` -> `app.rs::apply_toolbar_actions()`
  - ツールバー由来のユーザー操作をアプリケーション動作に変換する。

今回の設計ではこの分離を維持し、新しいマネージャ層は作らない。代わりに純関数ヘルパーと状態フィールドを追加して拡張する。

---

## 横断設計

### 1. Toolbar コマンド層 🔵

**信頼性**: 🔵 *toolbar.rs・app.rs より*

`ToolbarAction` に以下を追加する。

- `ExportCsv(ExportTarget)`
- `AddComparisonStudy`
- `RemoveComparisonStudy(usize)`

設計方針:

- CSV Export は `menu_button` で 3 種の `ExportTarget` を選択させ、その場で `ToolbarAction::ExportCsv(target)` を発火する。
- Comparison Study 追加は `ToolbarAction::AddComparisonStudy` を発火し、`app.rs` 側でファイル選択ダイアログを開いて worker に渡す。
- Comparison Study 削除はツールバー上の chip または Comparison パネル内の削除ボタンから `RemoveComparisonStudy(index)` を発火する。

### 2. 選択・可視性ポリシー層 🔵

**信頼性**: 🔵 *app_state.rs・trial_table.rs・pdp_chart.rs・pareto_2d.rs より*

`selected_indices` と `pinned_trials` を別々に持ちながら、描画時には次のルールで統合する。

- **選択の意味**: ユーザーが現在注目している集合
- **ピン留めの意味**: フィルターやブラッシング後でも消してはいけない集合
- **実効可視集合**: `selected_indices ∪ pinned_trials`

この実効可視集合を以下で共通利用する。

- Trial Table 表示行
- PDP observed overlay の点群
- Pareto Scatter 2D の dim/strong 判定
- 将来的な Scatter Matrix や Surface Plot のハイライト判定

`AppState` に `pinned_trials: Vec<u32>` を追加し、共通ヘルパーで `Vec<u32>` を生成する。

### 3. 非同期 worker 層 🔵

**信頼性**: 🔵 *app.rs・message_handler.rs・poll_chart.rs より*

重い処理は次の既存パターンに揃える。

1. UI が `pending_*` または `ToolbarAction` を立てる
2. `app.rs` または `poll_chart.rs` が `spawn_task()` を呼ぶ
3. バックグラウンドでパース/計算を実行する
4. 完了時に `AppMessage` を送る
5. `message_handler.rs` が `AppState` / `WidgetStates` を更新する

対象:

- Comparison Study ロード
- Surface Plot 計算
- PNG 保存用スクリーンショット後処理

### 4. セルヘッダー操作層 🟡

**信頼性**: 🟡 *grid_canvas.rs の既存ヘッダー設計と egui のスクリーンショット API 想定より*

`show_cell_toolbar()` にオーバーフローメニューを追加し、各セル単位で次を提供する。

- `Help`
- `Save as PNG`
- `Close`

実装は `CellToolbarAction` に `SavePng { row, col, item }` を追加し、`show_grid_canvas()` が各セルの `Rect` を記録する。PNG 保存時は「最後に描画されたセル矩形」を使ってビュー全体のスクリーンショットを切り出す。

---

## 機能別アーキテクチャ

### F-001 CSV Export UI 🔵

**信頼性**: 🔵 *toolbar.rs・io/export.rs・requirements.md より*

変更方針:

- `toolbar.rs` に `Export CSV` メニューを追加する
- `app.rs` で `ToolbarAction::ExportCsv(target)` を受ける
- `io/export.rs` に「行選択 + CSV エンコード + 保存ダイアログ」まで含む UI ヘルパーを追加する

責務分担:

- `toolbar.rs`: ExportTarget 選択 UI
- `app.rs`: 現在の Study / selected / pareto 情報を収集して export 関数を呼ぶ
- `io/export.rs`: CSV 文字列生成とファイル保存

本機能は同期処理でよい。50,000 行規模でも Rust の文字列生成と単発ファイル書き込みで十分軽量である。

### F-002 Comparison Study 追加 UI 🔵

**信頼性**: 🔵 *app_state.rs・messages.rs・message_handler.rs・comparison_panel.rs より*

設計方針:

- Comparison Study ロードは `study_worker` 系の非同期処理として追加する
- `comparison_base_study: Option<u32>` を `AppState` に追加し、どのメイン Study に対する比較セッションかを明示する
- メイン Study が変わった場合は `comparison_studies` と `comparison_colors` をクリアする

追加コンポーネント:

- `dispatch_load_comparison_study(path, base_study_meta, sender)`
- `AppMessage::ComparisonStudyLoadFailed(String)`
- `ToolbarAction::RemoveComparisonStudy(usize)`

比較 Study の選定規約:

- 同一 Journal 内に複数 Study がある場合は、まずメイン Study と同名の Study を探す
- 同名がない場合は最初の Study を採用し、その旨を UI に軽く通知する

### F-003 ピン留め UI 🔵

**信頼性**: 🔵 *trial_table.rs・session.rs・requirements.md より*

設計方針:

- `AppState` に `pinned_trials` を追加する
- Trial Table に `Pin` 列を追加し、行ごとにトグルする
- `SessionSnapshot::new()` と Load/Save 適用処理に `pinned_trials` を接続する

可視性ルール:

- `selected_indices` が空のときは全件表示
- `selected_indices` が非空のときは `selected ∪ pinned` の行を表示
- ピン留め行は背景色またはアイコン色で選択行と区別する

制約:

- 最大 20 件まで
- 21 件目は追加拒否しエラー表示する

### F-004 PDP Observed Overlay の仕上げ 🔵

**信頼性**: 🔵 *pdp_chart.rs・render_chart.rs より*

現行コードでは以下がすでに実装済みである。

- `PdpChart.show_observed`
- `Show data` トグル
- 観測点抽出と `egui_plot::Points` 描画

残っている設計対象は **連動条件の修正** のみである。

- `render_chart.rs` は現在 `trial_rows` 全件を `pdp_chart.show()` に渡している
- これを「実効可視集合」で絞り込んだ `display_rows` に置き換える
- その結果、PDP overlay は Brushing / Pinning と連動する

つまり F-004 は新規チャート設計ではなく、F-003 / F-006 の統合先として扱う。

### F-005 Surface Plot ウィジェット 🟡

**信頼性**: 🟡 *layout_state.rs・render_chart.rs・poll_chart.rs・既存 surface plot 設計文書より*

設計方針:

- `ChartId::SurfacePlot` を新設する
- `WidgetStates` に `surface_plot: SurfacePlotState` を追加する
- `render_chart.rs` と `poll_chart.rs` に Surface Plot の描画・計算起動を追加する
- 既存の PDP 2D / surrogate 設計を再利用しつつ、UI 上は独立チャートとして扱う

描画戦略:

- **Phase 1**: `Heatmap` / `Contour` の 2 モードで実装する
- **Phase 2**: 必要なら wgpu ベースの 3D サーフェス表示へ拡張する

Phase 1 を採用する理由:

- `egui_plot` は 3D を直接サポートしていない
- 現行コードベースでは 2D グリッド描画の再利用余地が大きい
- 要件上も「3D または等高線ヒートマップ」で充足可能

### F-006 Brushing & Linking 🔵

**信頼性**: 🔵 *pareto_2d.rs・parallel_coords.rs・app_state.rs より*

設計方針:

- **Pareto Scatter 2D**: plot 座標上でドラッグ矩形を取り、矩形内の `trial_id` を `selected_indices` に書き戻す
- **Parallel Coordinates**: 既存の `brush_ranges` / `drag_start` を使って軸ブラッシングを完成させる
- 選択結果はすべて `AppState.selected_indices` に集約する

補足:

- `ParetoScatter2D` はすでに `selected_indices` を dim 表示に使っているため、選択の書き込みが追加されれば即座に UI 連動する
- `ParallelCoordsChart` は軸ブラシ用 state を既に保持しているため、UI 操作と filter 反映の接続が主な作業になる

### F-007 Comparison Diff タブ 🔵

**信頼性**: 🔵 *comparison_panel.rs・app_state.rs・requirements.md より*

設計方針:

- `ComparisonView::Diff` を追加する
- 比較メトリクス計算は `ui/comparison_metrics.rs` の純関数群に切り出す
- 互換性判定は「目的数」と「方向ベクトル」の一致で行う

Diff タブで表示する指標:

- Trial 数差分
- Best 値差分
- Hypervolume 差分
- Pareto 支配率

計算は同期純関数でよい。対象は最大 4 Study であり、毎フレーム再計算せず `show_diff()` 内で軽量に集計できる。

### F-008 チャート PNG 保存 🟡

**信頼性**: 🟡 *grid_canvas.rs のセル矩形管理追加案と egui スクリーンショット API 想定より*

設計方針:

- PNG 保存は「各チャート自身」ではなく「各セル」を単位に行う
- セルヘッダーから `Save as PNG` を選ぶ
- `show_grid_canvas()` が記録したセル矩形を利用して、アプリ全体のスクリーンショットから該当セル領域だけ切り出す

この設計の利点:

- Plot 系、Table 系、将来の custom renderer 系を同じ仕組みで保存できる
- チャートごとに別々の PNG エンコーダ実装を持たなくてよい
- `Move` / `Help` / `Close` と同じセルヘッダー UX を維持できる

---

## 変更対象ディレクトリ（設計上） 🔵

**信頼性**: 🔵 *現行コード構造より*

```text
egui-app/src/
├── app.rs                          ← ToolbarAction 追加適用
├── io/
│   ├── export.rs                   ← CSV 保存ヘルパー追加
│   ├── session.rs                  ← pinned_trials 復元接続
│   ├── study_worker.rs             ← Comparison Study ロード worker 追加
│   └── chart_capture.rs            ← PNG 保存後処理（新規）
├── state/
│   ├── app_state.rs                ← pinned_trials / comparison_base_study 追加
│   ├── layout_state.rs             ← ChartId::SurfacePlot 追加
│   ├── message_handler.rs          ← 新規 AppMessage の適用
│   └── messages.rs                 ← SurfacePlot / Comparison error / capture 系追加
└── ui/
    ├── toolbar.rs                  ← Export / AddComparisonStudy UI 追加
    ├── comparison_panel.rs         ← Diff タブ追加
    ├── comparison_metrics.rs       ← Diff 計算純関数（新規）
    ├── grid_canvas.rs              ← Save as PNG メニュー追加
    ├── right_panel.rs              ← Surface Plot を Variable Analysis に追加
    ├── render_chart.rs             ← Surface Plot ルーティング追加
    ├── poll_chart.rs               ← Surface Plot 非同期起動追加
    ├── widget_states.rs            ← SurfacePlotState / capture state 追加
    └── widgets/
        ├── trial_table.rs          ← pin 列追加
        ├── pdp_chart.rs            ← display_rows 入力へ変更
        ├── pareto_2d.rs            ← brush selection 追加
        ├── parallel_coords.rs      ← axis brushing 仕上げ
        └── surface_plot.rs         ← 新規 Surface Plot widget
```

---

## 実装順序の推奨 🔵

**信頼性**: 🔵 *依存関係の少なさと既存コードの近接性より*

1. F-001 / F-002 / F-003
   - 既存ロジックを UI に接続するだけなので最も着手しやすい
2. F-006 -> F-004
   - 選択状態の書き込みを先に整え、その後 PDP overlay 連動を仕上げる
3. F-007
   - Comparison Study ロード後にすぐ着手できる
4. F-005
   - 新規 widget / message / rendering 追加が必要
5. F-008
   - もっとも横断的で、セルヘッダー・スクリーンショット・保存をまたぐため最後にまとめて入れる

---

## 技術的リスクと対策

### PNG 保存の API 依存 🟡

**信頼性**: 🟡 *egui/eframe のスクリーンショット API 利用想定より*

- リスク: 利用中の eframe/egui バージョンで期待するスクリーンショット API が使えない可能性がある
- 対策: `chart_capture.rs` に API 依存を隔離し、利用不能時は「この環境では PNG Export を利用できません」と明示する

### Surface Plot の描画コスト 🟡

**信頼性**: 🟡 *50,000 試行・2D グリッド描画の負荷予測より*

- リスク: 高密度グリッドを毎フレーム再生成すると UI が重くなる
- 対策: `SurfacePlotResult` を widget state に保持し、再計算は Run ボタン起点のみに限定する

### Brushing の責務分散 🔵

**信頼性**: 🔵 *現行 `render_chart.rs` の `&mut AppState` ルーティングより*

- リスク: 各 widget が独自に selected state を持つと Linking が壊れる
- 対策: 書き込み先を `AppState.selected_indices` のみに限定する

---

## 信頼性レベルサマリー

- 🔵 青信号: 18件 (82%)
- 🟡 黄信号: 4件 (18%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
