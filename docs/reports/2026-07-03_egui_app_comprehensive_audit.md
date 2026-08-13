# egui-app 網羅的監査レポート

- 日付: 2026-07-03
- 対象: `egui-app` クレート（tunny-desktop, UI 層, 約 36,000 行 / 95 ファイル）
- 観点: 速度・保守性・重複・脆弱性・責務
- 方法: 4 系統の並列調査（責務 / 重複 / 性能 / 安全性）を統合し、file:line で裏取り

## 総評

全体としてアーキテクチャは健全。重い計算は `spawn_task` + mpsc チャネルでバックグラウンドに逃がす規律が徹底されており（`poll_chart.rs` / `study_worker.rs`）、パス検証・HTML エスケープ・並行処理まわりも堅牢に書かれている。問題は次の 4 点に集約される。

1. **[速度・重大]** ストリーミングロード時のチャンク適用が UI スレッドで DataFrame を毎回全再構築しており、全体で O(n²)。大規模ジャーナルでの読み込み中フリーズの直接原因。
2. **[脆弱性・中]** CSV エクスポート全経路（約 20 箇所）にフィールドエスケープが無く、CSV 構造破壊（機能バグ）と Excel 数式インジェクションの両方が成立する。
3. **[責務]** 純粋な数値処理・パース処理が UI 層に数箇所残っている（等高線密度グリッド、CSV シリアライズ、行フィルタ、artifact メタデータパース）。rust_core に同等機能が既にあるものが多い。
4. **[重複]** 3D チャート族のオーケストレーション、normalize/range 計算、カラーバー描画に横断的なコピペがある（合計 400 行超）。

---

## 1. 速度

### 1-1. 【重大】チャンク適用ごとの DataFrame 全再構築（UI スレッド, O(n²)）

`state/message_handler.rs:356-414` (`core_rows_from_df`), `:437-468` (`handle_study_chunk`)

`StudyChunkLoaded`（約 1000 trial ごと）を受けるたびに、**ロード済み全行**を `core_rows_from_df` で行指向 `Vec<CoreTrialRow>` に復元（行×パラメータごとに HashMap と String クローンを生成）し、新チャンクを連結して `DataFrame::from_trials` で全体を作り直す。チャンクあたり O(ロード済み行数)、ロード全体で O(n²/chunk_size)。しかもこれは UI スレッド上で実行される（`app.rs` のメッセージポンプは `StudyChunkLoaded` を 1 フレーム 1 件に絞ってはいるが、1 件の再構築コスト自体が行数に比例して増大する）。

**修正方針**: DataFrame に追記 API（列を in-place で伸長）を rust_core 側に追加し、行指向への往復をなくす。Pareto 計算は既に `is_final` のみに遅延済み（`:471-483`）で正しい。

### 1-2. 【高】Parallel Coordinates が毎フレーム全 trial を描画・全 trial 分アロケート

`ui/widgets/scatter/parallel_coords.rs:356-424`

trial ごとに `Vec<Pos2>` を新規確保し、ブラシ有効時は trial×軸の全判定を毎フレーム再実行。`scatter_matrix.rs` にある `MAX_SCATTER_POINTS = 1500` + `downsample_indices_to_cap` のキャップ機構が無い。50k trial なら毎リペイント 50k 本のポリライン。

**修正方針**: scatter_matrix と同じダウンサンプルキャップの適用、およびポリライン用スクラッチ Vec の再利用。

### 1-3. 【高】Trial Table の可視行インデックスを毎フレーム再計算

`ui/widgets/common/trial_table.rs:118-131`

選択・ピン留めが変わらなくても毎フレーム HashSet 構築 + 全行スキャン。行描画自体は `TableBuilder::body().rows()` の仮想化で正しい。

**修正方針**: `(selected_indices, pinned)` のバージョン/ハッシュをキーに `visible` をキャッシュ（`ParallelCoordsChart::col_ranges_cache` と同型）。

### 1-4. 【中】毎フレームの再計算・再確保（個別は小、件数が多い）

| 箇所 | 内容 |
|---|---|
| `parallel_coords.rs:309-312`, `scatter_matrix.rs:134-137` | 軸ラベルの galley（フォント整形）を毎フレーム再生成 |
| `scatter_matrix.rs:121-127` | feasibility 分割 + ダウンサンプルを毎フレーム再計算 |
| `scatter_matrix.rs:240-265` | 描画は ≤1500 点なのに全 trial 分の `Vec<Color32>` を毎フレーム確保 |
| `optimization_history.rs:149-172, 241-243, 270` | `to_vec` / feasibility 分割 / best 値 / 移動平均を毎フレーム再計算 |
| `pareto_2d.rs:109`, `pareto_3d.rs:82` | `downsample_cache.scatter.clone()` — `.as_deref()` 借用で足りる |

**修正方針**: いずれも既存の `col_ranges_cache` 型のキー付きメモ化パターンを適用。

### 1-5. 良い既存パターン（維持・展開すべき）

- `poll_chart.rs`: 重計算はすべて `pending_*.take()` ゲート + `spawn_task` でバックグラウンド実行。新規計算のテンプレートにすべき。
- `study_worker.rs`: パース・スキャンは専用スレッド + バイトキャッシュ。
- 列指向アクセス（`view.numeric_column`）の徹底（MEM-002/003 対応済み）。
- `app_state.rs` の `DownsampleCache` による共有ダウンサンプル。
- `trial_table.rs` の仮想化行描画。

### 1-6. 備考: 未配線の GPU バッファ

`render/gpu_buffer.rs` の `GpuBufferLayout`（50k 点キャパ）はテスト以外から参照されておらず、散布図系は全て CPU 描画。trial 数が数万規模になるなら、個別のアロケーション修正よりこちらへの配線が本筋になる。

---

## 2. 脆弱性・堅牢性

### 2-1. 【中】CSV エクスポートのエスケープ欠如 + 数式インジェクション

`io/csv_export.rs`（`build_*_csv` 約 20 関数）, `io/export.rs`（`build_csv_string`, `build_csv_string_from_view`）

全ビルダーが `format!("{},{}")` / `.join(",")` の素結合で、ジャーナルファイル由来のパラメータ名・目的関数名を無加工で埋め込む。

- カンマ・引用符を含む名前で CSV 構造が壊れる（機能バグ）。
- `=` `+` `-` `@` で始まる名前（例: `=HYPERLINK("http://evil/","x")`）が Excel で数式として実行される（CSV インジェクション）。信頼できないジャーナルを開いて CSV エクスポート → Excel で開く、で成立。

なお rust_core の `io/export/csv.rs` には `escape_csv_field` を備えた正しいシリアライザが既に存在する。HTML レポート側（`html_report.rs::escape_html`）は XSS 回帰テスト込みで正しく実装済み。

**修正方針**: 全フィールドをクオート（内部 `"` は二重化）し、`=+-@` 先頭には `'` 前置。rust_core 側に共通の CSV 行ビルダーを置き、egui-app の全ビルダーから使う（§4-2 と同一施策）。

### 2-2. 【低】その他

- `io/artifacts.rs:188`: `trial_id as u64 → as u32` のサイレント切り捨て。悪意あるジャーナルで artifact の紐付け先がずれるのみ（整合性エッジケース）。
- `parallel_coords.rs:520-521`: `partial_cmp().unwrap()` がコードベース唯一の fallback 無し箇所。現状 NaN 到達経路は無い（`n_visible < 2` の早期 return で保護）が、他所と同じ `.unwrap_or(Ordering::Equal)` に揃えるべき。

### 2-3. 確認済みで問題なし

- パストラバーサル: `artifacts.rs::validate_path` が canonicalize + `starts_with` で正しく防御（NFR-201 テストあり）。
- `unsafe`: `main.rs:37-41` の Windows DPI FFI 1 箇所のみ、コメント済み。
- 並行処理: `.lock().unwrap()` ゼロ。poller は `catch_unwind` + 3 ストライク自動停止 + `stop_signal`/`join()` のクリーンシャットダウン。
- テスト外の unwrap/expect は 21 箇所、全数レビュー済みで全てガード付きまたはライブラリ不変条件。
- `build.rs`（377 行）: ビルド時のみ、ネットワーク・インジェクション面なし。

---

## 3. 責務（rust_core へ移すべきもの）

| # | 箇所 | 内容 | 判定 |
|---|---|---|---|
| 1 | `ui/widgets/scatter/observed_contour.rs:554-613, 632-712` | `cell_density_grid`（2D ビニング）+ `box_blur_2d`（分離ボックスブラー）+ マーチングスクエア級の等高線抽出（`edge_cross`）。egui 依存ゼロの数値パイプライン。rust_core には既に `contour` モジュールがあり、同チャート用の第 2 の独立実装になっている | **MOVE** → `rust_core::contour` |
| 2 | `io/export.rs:34-124` | `build_csv_string` / `build_csv_string_from_view`。rust_core `io/export/csv.rs` の `serialize_csv*` と重複し、かつエスケープが無い分**弱い** | **MOVE/統合** |
| 3 | `state/filter.rs:60-84` | `apply_filters` が rust_core `data/filter.rs::filter_rows` と同一アルゴリズム。ただし未知カラムの扱いが異なる（rust_core: 空結果 / egui-app: 素通し）。統合前に仕様を決める必要あり | **DELEGATE**（挙動差の解決が前提） |
| 4 | `io/artifacts.rs:19-88, 133-171, 251-282` | artifact メタデータの JSON 行パース・レイアウト走査・パス検証。egui 依存ゼロ。ジャーナルパースは rust_core の管轄 | **MOVE**（`scan_artifacts_dir` の非同期ディスパッチのみ残す) |
| 5 | `ui/widgets/decision/mcdm_chart.rs:151-162` | `normalize_weights` が rust_core `mcdm/vikor.rs:22` の private `normalize_weights_defensive` と同一 | rust_core 側を公開 API 化して **DELEGATE** |
| 6 | `io/csv_export.rs` | ディスパッチ/キャッシュ参照は UI 固有なので構造は維持。CSV 文字列組み立てだけ rust_core の共通ビルダーへ | **部分 DELEGATE** |

**適切な薄いラッパーの好例**（変更不要）: `io/journal.rs`, `io/flat_csv.rs`, `scatter_matrix.rs`（`pearson_correlation` を委譲）, `surrogate_opt.rs`（2203 行だが中身はほぼ描画）, `theme/*`（純粋な表示用色計算）。

**別件フラグ**: `html_report.rs` の `TrialStatistics` が `vec![0.0; ...]` のハードコード（未完成機能のスタブ？）。

---

## 4. 重複（保守性）

### 4-1. 3D チャート族のオーケストレーション（~230–280 行）

低レベル 3D プリミティブは `scatter_3d.rs` に正しく共有済み。重複はその一段上:

- アイソメトリックカメラの四元数リテラル `[-0.2391, 0.3696, 0.0990, 0.8924]` が 4 箇所にハードコード（`cluster_scatter_3d.rs:45`, `pareto_3d.rs:33`, `mcdm_scatter_chart_3d.rs:72`, `pdp_2d.rs:56`）→ `ArcballCamera::isometric_default()` を追加。
- レンジキャッシュ再計算ブロックが `cluster_scatter_3d.rs:104-120` と `pareto_3d.rs:87-102` で逐語一致（~17 行）→ `Range3DCache` ヘルパー化。
- ホバーツールチップ + クリック→詳細モーダルの定型 ~55–68 行 × 3 ファイル → 行ビルダーをクロージャで差し替える共通ヘルパー化。

### 4-2. normalize / range 計算の多重実装（~100–130 行）

- `normalize`（クランプ付き線形正規化、退化レンジ→0.5）が 3 箇所で逐語一致: `heatmap.rs:186`, `pdp_2d.rs:632`, `parallel_coords.rs:24`。
- min/max + 退化フォールバックが **6 実装**（`scatter_3d.rs:105`, `pdp_2d.rs:577`, `:640`, `mcdm_scatter_chart_3d.rs:83`, `heatmap.rs:171`, `:84`）で、**退化レンジの扱いが実装間で不一致**（正しさの一貫性リスク）。
- `pdp_2d.rs:588-629` の `draw_colorbar` は `heatmap.rs:107-168` の `draw_colorbar_simple`（`observed_contour.rs` は既に再利用）の再実装。

→ 共通 `range_math` ヘルパー + `draw_colorbar_simple` への一本化。

### 4-3. CSV ビルダー ~20 関数の同型パターン

§2-1 と同根。`CsvRows` ビルダー 1 つでエスケープ欠如も同時解消。

### 4-4. 重複でないと確認したもの

`message_handler.rs`（1376 行）は長大だが well-separated なディスパッチャで、コピペ重複なし。`mcdm_chart.rs::normalize_weights` と `mcdm_scatter_chart.rs::normalize_values` はドメインが異なり非重複。

---

## 5. 修正計画（優先度順）

| 優先度 | 項目 | 種別 | 対応 |
|---|---|---|---|
| P0 | 1-1 チャンク適用の O(n²) 再構築 | 速度 | rust_core に DataFrame 追記 API を追加し UI 側の往復を排除 |
| P0 | 2-1 CSV エスケープ/インジェクション | 脆弱性 | rust_core に共通 CSV 行ビルダー（クオート + 数式ガード）、全ビルダーを移行 |
| P1 | 3-1 observed_contour の数値処理移行 | 責務 | `cell_density_grid`/`box_blur_2d`/等高線セグメント抽出を `rust_core::contour` へ |
| P1 | 3-2 io/export.rs → rust_core 統合 | 責務 | `serialize_csv*` へ委譲（P0 の CSV ビルダーと同時実施） |
| P1 | 1-2 PCP ダウンサンプルキャップ | 速度 | `downsample_indices_to_cap` 適用 + Vec 再利用 |
| P1 | 1-3 Trial Table 可視行キャッシュ | 速度 | バージョンキー付きメモ化 |
| P2 | 3-3 filter 統合 / 3-4 artifacts パース移行 / 3-5 normalize_weights 公開化 | 責務 | rust_core へ移行・委譲 |
| P2 | 4-1 3D 族の共通化 / 4-2 range_math 一本化 | 重複 | 共有ヘルパー抽出 |
| P2 | 1-4 毎フレーム再計算群 | 速度 | キー付きキャッシュ適用 |
| P3 | 2-2 trial_id 切り捨て / partial_cmp 統一 | 堅牢性 | ガード追加・スタイル統一 |
| P3 | 1-6 GPU バッファ配線 | 速度 | 将来課題として記録のみ |

---

## 6. 実施結果（2026-07-04, ブランチ refactor/egui-app-audit-fixes）

GPU バッファ配線（P3, 将来課題）を除く全項目を実施した。検証: `cargo test --workspace`
（tunny-core 652 / tunny-desktop 682 + 統合 5, all green）、
`cargo clippy --workspace --all-targets --locked -- -D warnings` clean、`cargo fmt --check` clean。

### 速度
- **1-1**: rust_core に `DataFrame::append_trials` を追加（`from_trials(全行)` との等価性テスト 7 件付き。
  途中出現カラムのバックフィル・カテゴリ型フリップ・制約途中出現も同一挙動）。
  `message_handler` のストリーミングチャンク適用とライブ更新差分適用を「列クローン + in-place 追記」に
  置き換え、行指向再構築（`core_rows_from_df`）を削除。チャンク適用コストは O(n) HashMap 群生成 →
  O(列数) memcpy + O(新規行) に低減。
- **1-2**: PCP に `MAX_PCP_POLYLINES = 1500` の間引きキャップを導入（ブラシ通過 trial は間引き対象外の
  和集合として毎フレーム維持）。ポリライン用スクラッチ Vec の再利用化。
- **1-3**: Trial Table の可視行集合を（selected, pinned, row_count）内容比較キーでキャッシュ。
- **1-4**: 軸ラベル galley キャッシュ（PCP / scatter_matrix）、feasibility 分割 + 間引きのキャッシュ、
  色配列を間引き後サイズに縮小、optimization_history の O(n) ベクトル群を `HistoryCache` 化、
  pareto_2d/3d の `downsample_cache` クローンを借用に変更。
- **1-6**: GPU バッファ配線は未実施（別課題として残す）。

### 脆弱性
- **2-1**: rust_core `io::export` に `CsvWriter`/`CsvField` を新設（全テキストフィールドに構造クオート +
  先頭 `=+-@` への `'` 前置。数値フィールドはガード対象外なので負数に影響なし。非有限は空欄）。
  `io/export.rs` の trial エクスポート 2 関数と `io/csv_export.rs` の全 `build_*_csv`（約 20 関数）を移行。
  既存 `serialize_csv` のヘッダ・文字列セルにも数式ガードを適用。回帰テスト追加（カンマ名のクオート、
  `=` 先頭のガード、NaN 空欄化）。
- **2-2**: artifact メタデータの trial_id を `u32::try_from` 化（範囲外はスキップ、回帰テスト付き）。
  PCP の `partial_cmp().unwrap()` を `unwrap_or(Equal)` に統一。

### 責務
- **3-1**: `cell_density_grid` / `box_blur_2d` / marching-squares セグメント抽出
  （`contour_line_segments`）を `rust_core::contour` へ移行。widget は座標写像と描画のみ。
- **3-2**: trial CSV 生成を `CsvWriter` へ委譲（StudyView 由来の pareto_rank/cluster_id 合成は UI 状態
  なので egui-app 側に残置。シリアライズ・エスケープは rust_core に集約）。
- **3-3**: rust_core に `filter_rows_permissive` を追加（未知カラム素通し = 決定済み仕様）し、
  `AppState::apply_filters` を委譲化。厳格版 `filter_rows` は既存呼び出し向けに維持。
- **3-4**: artifact パース群（型・`validate_path`・`parse_artifact_metadata` ほか）を
  `rust_core::io::artifacts` へ移設（NFR-201 テスト含む）。egui 側は非同期ディスパッチのみ。
- **3-5**: `normalize_weights` を `tunny_core::mcdm` の公開 API に統合（vikor 内部実装と UI 側の
  防御挙動を和集合で統一）。

### 重複
- **4-1**: `ArcballCamera::isometric_default()` 新設（四元数リテラル 6 箇所を置換）。
  `Range3DCache` 共通化（cluster_scatter_3d / pareto_3d。mcdm_3d の `PointsCache` は他キャッシュと
  一体のため対象外と判断）。ホバー + クリック詳細モーダルの共通ヘルパー
  `show_hover_and_click_detail` を `scatter_3d.rs` に抽出し 3 widget を移行。
- **4-2**: `common/range_math.rs` 新設（`normalize01` / `value_range` / `expand_degenerate`、
  テスト 10 件）。normalize 3 実装・min/max 6 実装を集約（各呼び出し元の退化時挙動は維持。
  `pdp_2d::value_range_of` のみ意図的に退化拡張なしを維持しコメントで明示）。
  pdp_2d の `draw_colorbar` を削除し `draw_colorbar_simple` に一本化。
- **4-3**: CSV ビルダーの共通化は 2-1 で同時解消。

### 未実施（意図的）
- GPU バッファ配線（1-6）: 別課題。
- `html_report.rs` の `TrialStatistics` スタブ（§3 別件フラグ）: 未完成機能の仕様判断が必要なため
  本リファクタでは触れていない。
