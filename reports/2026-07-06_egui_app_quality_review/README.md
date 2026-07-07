# egui-app リリース前品質レビュー(脆弱性・保守性・重複・速度)

- **実施日**: 2026-07-06
- **対象**: `egui-app/` 全体(src 約 43,000 行・90 ファイル + build.rs)
- **目的**: リリース前の品質確認。UI 層を (1) 脆弱性・堅牢性、(2) 保守性、(3) 重複、(4) 速度の 4 観点で網羅的にレビューする。
- **方法**: モジュール単位で 8 担当(app/state、io、UI コア+テーマ、scatter 系、surrogate+PDP、decision/pareto/history/stats、common 系、横断重複専任)+ 補完 1 担当の計 9 レビューを並列実施し、重大度 high の指摘は統合時に実コードで再検証した。egui の immediate mode 特性(毎フレーム `update` 実行)を前提に速度問題を判定している。
- **報告基準**: 実害シナリオを説明できる欠陥のみ。スタイル上の好みは含まない。

## 修正対応状況(2026-07-07 完了)

本報告書の全項目を同ブランチ上で修正済み。要点:

- **High(H-1〜H-4)**: すべて修正。H-1/H-2 は `spawn_task` + 世代ガード付き `AppMessage::PollerReady` でバックグラウンド化、H-3 は 2D にも `ranked_hash` を導入、H-4 はセル統計を df 恒等性キーでキャッシュ化。
- **Medium(M-1〜M-21)**: すべて修正。設計変更を伴う2件は安全な緩和で対応 — M-7 はデータ実変更時のみ再計算(不変時は clone + O(N²) をスキップ)、M-8 は行指向再構築の廃止 + Pareto を最終バッチのみで確定計算。M-9 はドラッグ解放時のみ再計算するデバウンス + サンプル/仕様限界統計の分離。M-10/ABA はフィット採用時に単調増加する `fit_generation` をキーに採用。M-4 は `catch_unwind` + `AppMessage::TaskPanicked` で panic を可視化。
- **Low**: すべて修正(flat_csv のパス検証、csv_import_modal の添字防御、PROMETHEE の `.get` 化、report_modal の拡張子導出、アトミック書き込み(`io::file::write_atomic`)、ポーラー `Drop`、テクスチャ解放 ほか)。
- **重複(D-1〜D-12)**: すべて共通化。3D 描画骨格 `draw_depth_sorted_points`、ツールチップ行 + click/hover 解決(`trial_detail_modal`)、モーダル足場(`common/modal.rs`)、クラスタ制御(`common/cluster_controls.rs`)、`MODEL_CHOICES` 単一情報源化、`range_math::value_range`/`finite_value_range` への置換、`heatmap::draw_gradient_bar`、`ColorMap::sample_categorical`、`rgba_key`、`feature_matrix` ほか。意図的な残置は2件のみ: クラスタ実行状態の 3 メソッド(共通化すると差引で悪化)、radar_chart の行列化(欠損軸の部分描画という固有挙動を保存)。
- **M-20 分割**: `poll_chart_work` を 36 ヘルパーへ分割(本体 54 行)、`surrogate_opt.rs`(2,198行)を 7 ファイルへ、`artifact_gallery.rs`(1,185行)を 3 ファイルへ分割。いずれも外部 API・挙動不変。
- **検証**: `cargo clippy --workspace --all-targets --locked -- -D warnings` 警告ゼロ、3 クレートの `cargo fmt --check` 通過(CI の Linux lint ジョブと同条件)。テストのリンクは LightGBM ネイティブライブラリ不在のため本環境では不可(CI の Windows/macOS ジョブで実行される)。

## 総評

到達可能な panic・ゼロ除算・インジェクションはほぼ潰れており、基礎的な堅牢性は高い。
CSV 数式インジェクションは core 側 `sanitize_csv_text` が全 Text 列に適用済みで**該当なし**、
`unsafe` は Windows FFI(main.rs)のみ、min==max・空データ・NaN の退化ケースも大半のウィジェットで明示処理されている。

一方で、リリース前に対応すべき系統的な問題が 4 つある:

1. **UI スレッドの同期 I/O**: Live Update 起動経路で DB 接続とジャーナル全読込が UI スレッドで走り、環境次第でウィンドウがフリーズする(high ×2)。
2. **キャッシュキーの恒等性不足**: 「同一次元の別データ」に切り替えるとキャッシュが古いまま誤描画されるキー設計が複数ウィジェットに残っている(mcdm_scatter 2D は実バグ、parallel_coords / cluster_scatter / pca_biplot も同型)。
3. **ワーカー panic の無限スピナー化**: `spawn_task` が panic を捕捉しないため、ワーカー内の防御不足なインデックス参照が「永久に computing 表示」に化ける構造がある。
4. **毎フレーム再計算**: immediate mode 前提のキャッシュ漏れが十数箇所あり、数万 trial 規模でのフレーム落ち要因になっている(scatter_matrix が最重量)。

重複は「3D 描画骨格」「ツールチップ行組み立て」「モーダル足場」「クラスタ制御 UI」など約 1,000 行規模の共通化余地がある。

---

## 重大度 High(コード再検証済み)

### H-1 [vuln/perf] app.rs:741 — RDB フィンガープリント取得が UI スレッドで同期実行

`restart_poller` の RDB 分岐が `tunny_core::rdb::study_fingerprint_url`(DB 接続 + クエリ)を UI スレッドで同期実行する。Live Update 有効時は Study 選択・トグルのたびに走り、DB が低速・到達不能だと接続タイムアウトまでウィンドウが完全にフリーズする。
**対応**: フィンガープリント取得をワーカースレッドへ移し、結果を `AppMessage` 経由でポーラー起動へ渡す。

### H-2 [perf] app.rs:761-779 — ジャーナル全読込 + trial 数カウントが UI スレッドで同期実行

Journal 分岐が `std::fs::read(file_path)` で全体をメモリへ読み、`count_created_trials(_per_study)` を UI スレッドで実行する。数百 MB 級ジャーナルで Live Update を有効化するたび数百 ms〜秒単位のフレーム停止。
**対応**: H-1 と同じくワーカーへ退避。

### H-3 [vuln(実バグ)] mcdm_scatter_chart.rs:126,140 — キャッシュキーが重み変更を検出できない

2D 散布図のキャッシュキーが `primary_scores().first()` の bit + 件数 + 手法のみ。行 0 が Pareto フロント外(スコア 0.0 のまま)の場合、重みを変えて Run してもキーが一致し、**古いランク色が無言で表示され続ける**。3D 版は `ranked_hash`(mcdm_scatter_chart_3d.rs:99)で修正済みなのに 2D へ未反映。
**対応**: 3D と同じ `ranked_indices()` ハッシュをキーに含める(→ D-6 の共通化と同時実施を推奨)。

### H-4 [perf] scatter_matrix.rs:293-336 — 毎フレーム全セル統計再計算

セル描画ループが毎フレーム、ヒストグラム・相関係数・min/max を**全列・全 trial** に対して再計算する(O(n_axes² × trial_count))。間引きは描画点数のみで統計計算には効かず、10 パラメータ × 数万 trial でフレーム落ちする。`point_colors` も毎フレーム再構築(scatter_matrix.rs:265)。
**対応**: セル統計・色を (df の Arc 恒等性, mode, color_objective) キーでキャッシュ。

---

## 重大度 Medium

### 脆弱性・堅牢性

| # | 箇所 | 内容 |
|---|---|---|
| M-1 | io/live_update_poller.rs:128 | journal がローテーション/切り詰めされると `byte_offset > file_size` のまま**無言で永久に更新停止**し、無変化タイムアウトで「最適化完了」と誤認させる。サイズ縮小検出時に offset を 0 リセットして再読込する。 |
| M-2 | io/session.rs:152(同型: report_export.rs:110, export.rs:94, chart_capture.rs:87) | `std::fs::write` の truncate→書込みが非アトミック。上書き保存中のディスク満杯/クラッシュで**既存セッションファイルが破損・消失**する。一時ファイル + `rename` に変更。 |
| M-3 | ui/chart/poll_chart.rs:36-91 | フィット用データ構築が `unwrap_or(0.0)` で欠損を埋めるが**非有限値を除去しない**。pruned/failed trial の NaN が GP/回帰の学習行列へ流れ、全 NaN 予測またはワーカー panic を起こす(observed_contour は `is_finite` フィルタ済み)。 |
| M-4 | app.rs:889 `spawn_task` | ワーカーの panic を捕捉しないため、panic するとメッセージが届かず `computing`/`fitting` フラグが立ちっぱなしで**スピナーが永久に回る**。`catch_unwind` してエラーメッセージを送るラッパーにする。M-3・poll_chart.rs:495(`spearman[pi][0]` 直接添字)・poll_chart.rs:1242(`x_matrix[best_row]`)が引き金候補。 |
| M-5 | scatter/parallel_coords.rs:200-216 | 軸レンジキャッシュのキーが (trial_count, n_params, n_objs) のみ。**同一次元の別 Study に切り替えると古いレンジで誤った位置に描画**される(rank_plot は `Arc::as_ptr` で対策済み)。 |
| M-6 | scatter/cluster_scatter.rs:287-292 | 同型。キーが (n_trials, n_clusters) のみで、別 Study 切替後も前 Study の座標を描画し続ける。M-5 と合わせて「df の Arc 恒等性(または世代 ID)をキーに含める」を全ウィジェットで統一する。 |

### 速度

| # | 箇所 | 内容 |
|---|---|---|
| M-7 | state/message_handler.rs:594 | Live Update の毎ポーリング(既定 2 秒)ごとに DataFrame 全 `clone()` + 多目的 O(N²) Pareto ランク再計算を UI スレッドで実行。大規模 Study で UI が継続的に重くなる。 |
| M-8 | state/message_handler.rs:418 | ストリーミングロードで各バッチが既存 DataFrame 全体を `clone()`→`append_trials`。K バッチで列コピー総量 O(N²)、後半バッチほど遅くなる。 |
| M-9 | surrogate/robustness.rs:224,263-281 | `noise_pct` スライダー/LSL・USL の DragValue がドラッグ中毎フレームキャッシュミスし、最大 4096 サンプルのロバスト解析(GP なら不確かさ予測込み)を UI スレッドで同期再実行。デバウンス or バックグラウンド化。仕様限界はサンプル分布に影響しないため spec 変更時のサンプル再利用も有効。 |
| M-10 | surrogate/response_surface.rs:295-314 | `surface_slice_at`(Grid=50 → GP 2500 点予測)を描画パスで同期実行。軸/アンカー変更時に UI ブロック。 |
| M-11 | ui/chart/render_chart.rs:61-93,312-331,466-489 | OptimizationHistory/EdfPlot/SurrogateOpt が毎フレーム目的列を `to_vec()` で全複製・overlay 再構築。選択変更時のみ再構築するようキャッシュ。 |
| M-12 | ui/canvas/canvas_view.rs:118-133 | ドット格子を毎フレーム `circle_filled` ×約 3 万個で描画(最小間隔 8px)。メッシュ化 or 間隔引き上げ。 |
| M-13 | scatter/cluster_scatter_3d.rs:195-238 | 2D 系(1500 点上限)と異なり全 trial を毎フレーム 3 回深度ソートして描画。`downsample_indices_to_cap` を適用。 |
| M-14 | scatter/parallel_coords.rs:373-390 | ブラシ表示中、毎フレーム全 trial × 全軸の brush 判定 + HashSet 確保。ブラシ範囲変更時のみ再計算。 |
| M-15 | common/artifact_gallery.rs:442-478 | Cluster モードが非仮想化で全クラスタ・全メンバー画像を毎フレーム add。数百枚で表示切替時の大ハング + VRAM 常駐(All モードは PAGE_SIZE=12 で緩和済み)。ページ分割 or 可視行のみ描画。 |
| M-16 | theme/color_compute.rs:28(`compute_point_alpha`) | 呼び出し側(mcdm_scatter/pareto_2d/pareto_3d)で点ごとに `selected_indices.contains()` の線形走査 → O(n·s)/フレーム。HashSet を 1 回構築して渡す。 |
| M-17 | pareto/pareto_2d.rs:162-192、decision/mcdm_scatter_chart.rs:387-409、history/convergence.rs:272-280、history/intermediate_values.rs:46-102、scatter/pca_biplot.rs:154,209-216 | 点集合分割 / 色グループ化+ソート / hit_points(O(m·n)) / 曲線 2000 本 / 色グループの毎フレーム再構築。いずれもデータ恒等性キーのキャッシュ追加で解消。 |

### 保守性

| # | 箇所 | 内容 |
|---|---|---|
| M-18 | ui/chart/poll_chart.rs:248-1340 | `poll_chart_work` が約 1,100 行の単一 match。特に SurrogateOpt アームは 6 段の `else if let ...pending.take()` 連鎖(約 300 行)でテスト不能。ChartId アームごとに関数分割。 |
| M-19 | io/study_worker.rs:191-256 | 比較 Study ロードが 4 段ネスト match + 同一 8 行ブロック ×3(journal/sqlite/rdb)。さらに `Err(_) => None` で実エラーを握り潰し、利用者には常に汎用文言のみ(210,229,249)。パース部のみクロージャ化 + エラー文字列を `ComparisonStudyLoadFailed` へ。 |
| M-20 | surrogate/surrogate_opt.rs(2,209 行)、common/artifact_gallery.rs(1,161 行) | フィット/最適化/Suggest/2D・3D 散布図/テーブル、および 3 表示モード+制御 UI+データ整形が単一ファイルに同居。submodule 分割。 |
| M-21 | state/results.rs:279 | `LiveUpdateState` の `file_path`/`last_byte_offset`/`consecutive_errors` が実質 dead field(ポーラーが内部管理)。実体と乖離し読み手を誤らせるため削除。 |

---

## 重大度 Low(要点のみ)

- **vuln**: io/flat_csv.rs:50 — CSV の `img` 列が絶対パス・`../` を検証せず join され、CSV ディレクトリ外のファイルを参照可能(ローカル完結だが `ParentDir`/`RootDir` 拒否を推奨)。common/csv_import_modal.rs:48 — `maximize[i]` 直接添字で、`directions` と `objective_names` の長さ不一致 CSV メタで panic。decision/mcdm_chart.rs:421-497 — PROMETHEE の `phi_*[idx]` 直接添字(隣の `incomparable_counts.get(idx)` と非対称)。surrogate response_surface.rs:130 / robustness.rs:409 — `Arc::as_ptr` キーの ABA で古い結果を誤表示しうる(フィット世代 ID へ)。pca_biplot.rs:123 — キャッシュキーが (study_name, row_count, space) で内容変更を検出しない。
- **堅牢性(挙動)**: app.rs:464 — CSV エクスポート失敗を `let _ =` で握り潰し(SaveSession は `load_error` 表示で不一致)。common/report_modal.rs:102 — 既定拡張子が常に `.html` で、JSON/Markdown のみ選択時に OS 上書き確認の不変条件が壊れる。io/live_update_poller.rs:16 — ポーラーに `Drop` 実装がなく `stop()` 規約頼み(スレッドリーク予防に `Drop { stop() }`)。
- **perf**: io/csv_export.rs:39 — CSV 構築 + rfd モーダルが UI スレッド同期(report_export は背景実行済みで不一致)。live_update_poller.rs:160 — 差分サイズ分を一括 `vec![0u8; ..]` 確保(チャンク読みへ)。history/edf_plot.rs:166 — ホバー時 O(全ステップ点) の手書きヒットテスト(共通 `hit_test_nearest` へ)。robustness.rs:495-513 — 結果不変でも毎フレームヒストグラム再計算。robustness.rs:324 / response_surface.rs:295 — `best_trial_row` の毎フレーム O(N) 走査。common/artifact_gallery.rs:779 / trial_detail_modal.rs:245 — `file://` URI の lossy 変換 + テクスチャ未解放蓄積(`ctx.forget_image`)。render_chart.rs:109-133 — ConvergenceIndicators の毎フレーム clone。
- **maint**: io/csv_export.rs:135 — `has_csv_data` と `build_chart_csv` の判定二重化(乖離リスク)。render_chart.rs:83,126,480 — 比較色 `[66,133,244,255]` のハードコード ×3(`comparison_color_at` へ)。app.rs:153 — `ComputeSyncKind::propagate` の同一 3 アーム。

---

## 重複と共通化ロードマップ(モジュール横断)

費用対効果順。上位 2 件で約 280 行、全体で約 1,000 行の削減余地。

| # | 対象 | 規模 | 共通化方針 |
|---|---|---|---|
| D-1 | 3D 点描画骨格(pareto_3d:147 / cluster_scatter_3d:195 / mcdm_scatter_chart_3d:356 / surrogate_opt:1555) | 約 160 行 | trial 走査→normalize→project→feasibility 分岐→深度ソート→circle_filled を `scatter_3d::draw_depth_sorted_points(painter, project, points, color_fn)` へ |
| D-2 | ホバー/クリック詳細行の組み立て(9 ファイルに `fmt` クロージャ + 軸値行 + Feasible 行) | 約 120 行 | `fmt_opt` / `push_feasible_row` / 軸値行ビルダを `common::trial_detail_modal` へ |
| D-3 | クラスタ制御 UI + compute キュー(cluster_scatter 2D/3D の `show_cluster_controls`/`try_queue_compute` 系、cluster_table:295-375、artifact_gallery:554-635) | 約 200 行 | 共有 ClusterControls ウィジェット + ランタイム状態 struct に集約(4 箇所) |
| D-4 | モーダル足場(csv_import / license / rdb_url / report / trial_detail の `Modal::new().show` + should_close) | 約 90 行 | `common::modal::scaffold(id, title, min_width, body_fn) -> ModalOutcome` |
| D-5 | click/hover 解決ボイラープレート(pareto_2d:257 / rank_plot:229 / cluster_scatter:362 ほか計 7 箇所) | 約 90 行 | `trial_detail_modal::resolve_click_hover(plot_ui, candidates) -> (clicked, hovered)` |
| D-6 | 2D/3D ペアの計算ロジック(mcdm_scatter_chart ⇔ _3d のランク色計算・キャッシュキー、pareto_2d ⇔ _3d の分類) | 約 150 行 | 点分類・色計算・キーハッシュを共有ヘルパー化。**H-3 はこの乖離が原因**であり最優先 |
| D-7 | study 更新後処理(message_handler の swap_snapshot→select→pareto→キャッシュ破棄 ×3) | 約 60 行 | 共通ヘルパー 1 つへ(コメント自体が「完全に同じ」と明記) |
| D-8 | `MODEL_CHOICES` ×3(surrogate_opt:33 / robustness:25 / response_surface:35)+ pdp_chart:83 の並行リスト | — | 単一の共有 const へ(モデル追加時の更新漏れ防止) |
| D-9 | min/max の手書き fold ×約 10 箇所(scatter_matrix:270 / parallel_coords:206 / box_plot:174 ほか) | 約 20 行 | 既存 `range_math::value_range` へ置換。有限値版 `finite_value_range` を追加し radar_comparison:78 / pca_biplot:236 / radar_chart:86 も置換 |
| D-10 | カラーバー縦グラデーション ×3(heatmap:64 / rank_plot:336 / mcdm_scatter_chart_3d:414) | 約 120 行 | `heatmap::draw_gradient_bar(painter, rect, cmap)` |
| D-11 | カテゴリ色の等間隔サンプリング ×4 / 色キーのバッチ描画 ×4 / feature 行列化 ×3 | 約 85 行 | `ColorMap::sample_categorical(idx, count)`、`color_compute::rgba_key`、`common::feature_matrix(view, features)` |
| D-12 | その他: csv_export の study 取得ガード ×21(`require_study` へ)、`val_range` 完全重複(mcdm_scatter_chart_3d:85 ⇔ scatter_3d:111)、trial CSV ヘッダ二重化(export.rs:14 ⇔ csv_export.rs:996)、ファイル種別 dispatch ×2(app.rs:224 ⇔ 612)、`minimize_flags` ×4+、45°回転ラベル配置 ×2、`dim`/`empty_state` ×2 | 約 150 行 | 各ヘルパー抽出 |

---

## 推奨対応順

**リリース前に必須(実バグ・フリーズ級)**
1. H-3: mcdm 2D のキャッシュキー修正(3D の `ranked_hash` 移植。最小 diff で直せる)
2. H-1 / H-2: Live Update 起動時の同期 I/O をワーカーへ退避
3. M-1: journal ローテーション時の無言停止
4. M-3 + M-4: NaN フィルタ + `spawn_task` の panic 捕捉(無限スピナーの根絶)
5. M-5 / M-6: キャッシュキーへのデータ恒等性追加(parallel_coords / cluster_scatter)
6. M-2: セッション保存のアトミック化

**リリース前に推奨(体感性能)**
7. H-4: scatter_matrix のセル統計キャッシュ
8. M-9 / M-10: robustness / response_surface の同期解析をデバウンス or 背景化
9. M-15: artifact_gallery Cluster モードのページ分割
10. M-12: キャンバス dot grid のメッシュ化

**リリース後の整理(保守性・重複)**
11. D-6 + D-3 + D-8(挙動乖離バグを生む重複から優先)
12. M-18 / M-19 / M-20 の分割・エラー伝搬
13. D-1 / D-2 / D-4 / D-5 / D-9〜D-12 の順で共通化
14. M-7 / M-8 の Live Update・ストリーミングロードのデータフロー見直し(設計変更を伴うため単独タスク化)

## 確認済みで問題なしと判定した事項

- CSV 数式インジェクション: core `CsvWriter::sanitize_csv_text` が `= + - @` に `'` 前置 + 構造クオートを全 Text 列へ適用済み
- `unsafe`: main.rs の Windows FFI のみ(妥当)
- rdb_url_modal の認証情報付き URL: 表示は入力欄内のみ、ログ・recent への永続化なし。report_export はパスワードマスク済み
- 退化ケース(空・全 NaN・1 点・min==max): scatter 系の正規化/座標変換、radar/heatmap/range_math、stats 系の quantile 等はいずれも明示処理済み
- repaint storm: 無条件 `request_repaint` なし(パネル開閉アニメーションの有界呼び出しのみ)
