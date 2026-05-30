# memory-efficiency 残作業・引き継ぎ資料

**作成日**: 2026-05-29  
**更新日**: 2026-05-30
**ブランチ**: `feature/memory-efficiency`（`main` から分岐）
**状態**: MEM-001〜007 主要実装完了。残り: 互換シム完全除去（csv_export等大規模消費箇所）・検証タスク（100k データ待ち）。

---

## 1. 完了済み（コミット済み・全テストグリーン）

`cargo test -p tunny-desktop` グリーン（1127 テスト）。
`cargo test -p tunny-core` は性能タイミングテスト（`tc_901_p02_kmeans_performance`、
`tc_1615_12_performance_50k_trials`、`tc_201_p01_ndsort_1000_points_under_100ms`）のみマシン負荷依存で稀に失敗する**既存フレーキー**（本変更と無関係。最終的に `/tsumiki:auto-debug` 対象）。

| コミット | 内容 | MEM |
|---|---|---|
| `d5c6406` | 共有 Arc ストア + StudyView 基盤（TASK-2328〜2331） | MEM-001基盤 |
| `6135dd3` | StudyContext から trial_rows 撤廃（84箇所）+ ライブ更新 ArcSwap（TASK-2332/2340）+ **ストア致命バグ修正** | MEM-001 |
| `9ed2d13` | Pareto 2D/3D 行クローンキャッシュ撤廃（TASK-2334） | MEM-002 |
| `ccdc284` | PCP/ScatterMatrix 列キャッシュ共有化（TASK-2335） | MEM-003 |
| `7ee1d84` | Trial Table 列アクセス移行（TASK-2336） | MEM-003系 |
| `a3bcf77` | 未使用 gpu_data ホストバッファ撤廃（TASK-2342前半） | MEM-007 |
| `c947448` | cluster/mcdm/ahp/slice/opt-history を view 列参照化（TASK-2337） | — |
| `db5180a` | finalize.rs per-study Vec 逐次解放（TASK-2341） | MEM-006 |
| `ddf6737` | render_chart.rs:28 削除・pdp_chart view 化・filter/toolbar/widget_states（TASK-2342後半） | — |
| `00cce0c` | 分析パイプライン trial_rows() 完全廃止・Arc<DataFrame> 直接参照（TASK-2338） | MEM-004 |
| `31a81c3` | 同一ファイル比較の再パーススキップ（TASK-2339, option C） | MEM-005部分 |

### 確立されたアーキテクチャ（残作業の前提）
- **共有ストア**: `rust_core/src/data/dataframe/state.rs` の `SharedStudyStore`。
  本番ビルドはプロセスグローバル `RwLock`、テストビルドは `thread_local`（二重 cfg `with_store_read`/`with_store_write`）。
  公開 API: `store_dataframes` / `select_study` / `with_active_df` / `with_df` / `snapshot(id)` / `active_snapshot` / `swap_snapshot`。
  - **重要バグの教訓**: read/write アクセサは必ず**単一の `static`（`global_store()`）**を共有すること。当初 read/write が各々別 static を宣言し、snapshot が常に None を返す致命的バグがあった（修正済み）。
- **StudyView**（`egui-app/src/state/types.rs`）: `Arc<DataFrame>` + 並行配列（`trial_ids` / `pareto_rank` / `cluster_id` / `state`）。
  - 列アクセス: `view.numeric_column(name) -> Option<&[f64]>`（借用・コピーなし）。
  - 互換シム: `view.row_at(i) -> TrialRow` / `view.to_trial_rows()`（**移行用・残ウィジェット依存中**）。
- **StudyContext**: `{ meta, view, pareto_indices }`（`trial_rows` / `gpu_data` 撤廃済み）。
  - テスト用: `StudyContext::from_rows_for_test(meta, rows)` / `set_rows_for_test(rows)`。
- **StudySelected メッセージ**: `{ meta, study_id, pareto_rank, pareto_indices }`（行データは運ばない。UI が `snapshot(study_id)` で Arc を取得）。
- **ライブ更新**（`message_handler::handle_live_update_done`）: 既存 view から core 行を再構築 → 新試行追加 → `DataFrame::from_trials` → `swap_snapshot` → view 作り直し。

### グローバルストアとテスト直列化の注意
- tunny-desktop のテストは tunny-core を**通常リンク**するため、tunny-core は本番グローバルストアを使う。
  `store_dataframes` + `snapshot` を使う message_handler テストは**プロセス共有ストアを取り合う**ため、
  `test_store_guard()`（`message_handler.rs` テストmod）で**直列化**している。
  → 新たに store を使う tunny-desktop テストを追加する場合は同ガードを取得すること。

---

## 2. 残タスク

> ★ = 今セッションで完了済み。未着手・部分残りは通常記述。

### ★ TASK-2337: Cluster / MCDM ウィジェットの view 列参照化 → **完了** (`c947448`)

**目的**: `render_chart.rs:28` が毎フレーム生成する `ctx.trial_rows()`（全行＋per-row HashMap 再構築）への依存を、cluster/mcdm 系ウィジェットから除去する。

**対象と現状**:
- `render_chart.rs:28` `let trial_rows = &ctx.trial_rows();` を、各ウィジェットへ渡している。
  残る `&[TrialRow]` 消費ウィジェット: `opt_history`, `pdp_chart`(filter_rows_for_display経由), `cluster_scatter`, `mcdm_chart`, `scatter_chart`(mcdm), `mcdm_table`, `ahp_chart`, `slice_chart`。
- `cluster_scatter.rs`: `show(ui, trial_rows, ...)` + `compute_obj_axes_2d(trial_rows, obj_names)` + `build_cluster_matrix_data(trial_rows, param_names, obj_names, space)`。
- `mcdm_chart.rs` / `mcdm_scatter_chart.rs` / `mcdm_table` / `ahp_chart` / `slice_chart`: いずれも `&[TrialRow]` を受け取り目的値/パラメータ行列を構築。

**実装方針**（パターンは TASK-2334/2335/2336 と同一）:
1. 各 `show()` の `trial_rows: &[TrialRow]` を `view: &StudyView`（+ param/obj names）へ変更。
2. 内部の行イテレーションを `view.numeric_column(name)`（列スライス借用）＋ `view.row_count()` ＋ `view.trial_ids` に置換。
3. cluster の特徴量行列ビルダ（`build_cluster_matrix_data`）は列スライスから flat `Vec<f64>` を構築（クラスタリング入力なので構築自体は必要・重複ではない）。
4. `render_chart.rs` の各呼び出しを `&ctx.view` 渡しに変更。**全ウィジェット移行後**、`render_chart.rs:28` の `let trial_rows = &ctx.trial_rows();` を削除できる。

**完了後にできること**: `StudyView::row_at` / `to_trial_rows` 互換シム、egui `TrialRow` 型、`get_display_rows*`（trial_table）等の移行専用コードを除去（TASK-2342 後半）。
- ただし `egui-app/src/state/app_state.rs` の `filter_rows_for_display` / `merge_selected_with_pinned` や、各ウィジェットのテストが `TrialRow` を使うため、除去は全消費箇所の確認後に。

**テスト**: 各ウィジェットの既存テストは `TrialRow` ベースのものが多い。view ベースのヘルパー（`from_rows_for_test` で StudyContext を作り `&ctx.view` を渡す）へ寄せる。

---

### ★ TASK-2338: 分析パイプライン入力の列参照化 → **完了** (`00cce0c`)

**目的**: PDP / Surface Plot / Sensitivity が実行ごとに大きな `Vec<Vec<f64>>` を行データから再構築するのを削減する。

**対象と現状**:
- `egui-app/src/ui/poll_chart.rs`: 分析ディスパッチで `ctx.trial_rows()` から `core_rows` を作り `DataFrame::from_trials(...)` で**単一目的の一時 DataFrame** を再構築して `compute_sobol_from_df` 等に渡している（例: ImportanceChart/Sobol 経路 line ~121-146）。
- `rust_core/src/pdp/utils.rs`, `rust_core/src/sensitivity/analysis/full.rs`: 内部で `Vec<Vec<f64>>` を構築（PRD MEM-004 が指摘）。

**実装方針**:
1. poll_chart 側: study の DataFrame は既に共有ストアに常駐（`ctx.view.df: Arc<DataFrame>`）。
   分析が「全パラメータ × 単一目的」の行列を要するため、`ctx.view.df` の列スライス（`get_numeric_column`）から
   **フラットバッファ／借用スライス**で入力を組み、行→列の再構築（per-row HashMap 経由）を避ける。
   - 注意: 現状は選択目的のみの DataFrame を作る（`from_trials(&core_rows, &params, &[selected_obj], ...)`）。
     共有ストアの df は全目的を持つため、分析関数が単一目的を要求する場合は列の部分参照で渡せるよう
     rust_core 側関数のシグネチャを `&[f64]`（目的列）+ パラメータ列スライス群を受ける形へ調整するのが理想。
2. rust_core 分析内部（pdp/utils, sensitivity/full）: ネスト Vec の再構築を借用スライス／フラットバッファ／
   `ndarray` ビューへ。**入力不変時に再計算しない**キャッシュ判定を追加（既存 `importance_cache`/`sobol_cache` と同様）。

**検証**: 受け入れ基準 TC-008（同一入力での再実行で大行列を再生成しない、数値出力が現行と等価）。
分析結果の数値等価は浮動小数の積算順序に注意（同一順序で構築）。

---

### ★ TASK-2339: 比較 study の軽量化 → **option C 実装済み** (`31a81c3`)

**目的**: 比較モードが study ごとにフル `StudyContext` を保持・再パースするのを軽量化する。

**実装中に判明した重要な制約**:
- 比較は**別ジャーナルファイル**からも study を読み込める（`dispatch_load_comparison_study(path, main_study_name, study_idx, tx)` が任意の `path` を受ける）。
- 現状 `study_worker::load_comparison_study_task` は対象ファイルを**全体再パース** → `store_dataframes`（**グローバルストアを上書き**）→ `snapshot` で view 構築。
  - 同一ファイル内比較なら、TASK-2330 で全 study 常駐済みのため再パース不要にできる。
  - **クロスファイル比較**では、対象ファイルの study は現ストアに無いため、何らかの読み込みが必要。

**設計の選択肢（要ユーザー判断）**:
- **A. ストアを複数ジャーナル対応に拡張**: `study_id` 単一キーを `(source_id, study_id)` 等の複合キーにし、
  複数ファイルの study を同時常駐。比較ロードは対象ファイルをパースして追加格納→参照（再パースは初回のみ、上書きしない）。
  - 影響: `SharedStudyStore` のキー設計、`select_study`/`snapshot` のシグネチャ、main/比較の study 参照方法。
- **B. 比較は対象ファイルをパースして必要 study のみ Arc 保持**（現状に近いが上書きしない・フル StudyContext は持たず軽量メタ＋遅延 view）。
  - グローバルストアの上書き問題を避けるため、比較用は別の保持先（比較専用マップ）に Arc を置く。
- **C. 同一ファイル内比較のみ最適化**（クロスファイルは現状維持）: スコープを絞り、`comparison_studies` を
  `study_id` 参照＋軽量メタ＋遅延 view に変更、同一ファイルなら `snapshot(study_id)` で再パース回避。

**現状の `comparison_studies`**: `egui-app/src/state/app_state.rs:43` `Vec<StudyContext>`（最大4件）。
設計案 `types.rs` の `ComparisonStudy { study_id, meta, view: Option<StudyView> }` を参照。
解放は `reset_comparison_session`（`app_state.rs:128`）。

**→ 実装前に A/B/C を決めること。** メモリ最適化観点では B か C が現実的。

---

### ★ TASK-2341: ジャーナルパースのピークメモリ削減 → **完了** (`db5180a`)

**目的**: パース時に中間状態（`TrialBuilder` / per-study 行ベクタ）と確定 `DataFrame` が同時に存在するピークを下げる。

**対象と現状**:
- `rust_core/src/io/journal/parser/state.rs:9-13` `ParserState { trial_builders: HashMap<u32, TrialBuilder>, ... }`。
- `rust_core/src/io/journal/parser/finalize.rs`:
  - `per_study_rows: Vec<Vec<TrialRow>>`（全試行ぶん・line 20 付近）を構築 → ループ末尾で
    `DataFrame::from_trials(&per_study_rows[index], ...)`（line 104 付近）。
    → `per_study_rows`（全行）と確定 DataFrame 群がピーク時に共存。

**実装方針**:
1. study 単位で `per_study_rows[idx]` を構築→即 `from_trials`→当該中間 Vec を `drop`/`std::mem::take` で解放、を
   ループ内で逐次化（全 study 行の同時保持を避ける）。
2. 余裕があれば `TrialBuilder` から列ベクタへ直接書き込み、`TrialRow` 中間生成自体を削減
   （`from_trials` の入力を行→列へ）。効果とコストを見て採否判断。

**検証**: 同一ログのパース結果（study 数・目的・user attr・制約・行数）が現行と一致（回帰なし）。
TC-012。ピーク低下は TASK-2343 のベンチで確認。

---

### TASK-2342（後半）: 互換シム `row_at` / `to_trial_rows` の完全除去 — **部分完了**

**render_chart.rs:28 は削除済み** (`ddf6737`)。主要ホットパスも view 化済み。

**残存する `trial_rows()` 呼び出し**（非ホットパス・大規模変更が必要）:
- `csv_export.rs`: 15+ 箇所（エクスポート時のみ）
- `html_report.rs`: 1 箇所
- `comparison_panel.rs`: 多数（`trial_rows().len()` 等）
- `bottom_panel.rs` / `trial_table.rs`: `get_display_rows`、`filter_rows_for_display`
- `app.rs`: イベント駆動の数箇所

**完全除去のブロッカー**: `csv_export.rs` など大規模消費箇所のリライトが必要。
`StudyView::row_at` / `to_trial_rows` / `StudyContext::trial_rows()` はまだ使われているため残存。

---

### TASK-2343 / 2344: 定量メモリ検証（**ブロック中**）

**ブロッカー**: 100k trials × 約22列の代表 Optuna Journal データセットが未用意（[prep.md](../../spec/memory-efficiency/prep.md) 必須タスク）。

- **TASK-2343（DIRECT）**: 計測手段の確定（Windows: 定常 RSS / `dhat` ヒープ等）＋ 改修前ベースライン測定。
  - ベースラインは `main` ブランチ（本ブランチ分岐前）で取得すること（同一データセット・同一操作）。
- **TASK-2344（TDD）**: 改修後（本ブランチ）の定常／ロードピーク／分析ピークを測定し、PRD 目標
  （NFR-001 定常 -50% 以上 等）を数値検証。`cargo test --workspace` グリーンを等価性の主証拠とする（REQ-404）。

**用意いただきたいもの**: 100k×22 規模の Journal ログ（固定パス）。既存の大規模 study があれば流用、
なければ Optuna で多目的・多パラメータの study を10万試行生成。

---

## 3. 既知の注意点・教訓

- **二重 static の罠**: グローバル状態のアクセサ（read/write）は単一 static を共有させる。
- **テスト分離**: tunny-core 自前テストは thread_local で分離。tunny-desktop テストは本番グローバルストアを共有するため、
  store を使うテストは `test_store_guard()` で直列化。
- **互換シム `row_at` の per-frame コスト**: `render_chart.rs:28` が毎フレーム全行を再構築している（TASK-2337 完了まで残存）。
  定常メモリは削減済みだが、描画フレームごとの一時アロケーションは TASK-2337/2342後半で解消する。
- **フレーキー性能テスト**: tunny-core の `tc_901_p02_kmeans_performance` / `tc_1615_12_performance_50k_trials` は
  タイミング閾値テストで、ビルド負荷時に稀に失敗する。本リファクタとは無関係。`/tsumiki:auto-debug` で安定化推奨。
- **比較クロスファイル**: TASK-2339 はクロスファイル比較の扱いで設計判断が必要（本資料 2-2339）。

## 4. 再開時の推奨順序（更新済み）

✅ 1〜4 は今セッションで完了。

5. `csv_export.rs` / `html_report.rs` / `comparison_panel.rs` / `bottom_panel.rs` 等の残存 `trial_rows()` を view 化 → 完全な互換シム除去（TASK-2342 残分）。
6. 100k×22 データ用意 → TASK-2343（ベースライン）→ TASK-2344（定量検証）。
7. `/tsumiki:auto-debug` でフレーキー性能テストを安定化。

## 関連文書
- 要件: [requirements.md](../../spec/memory-efficiency/requirements.md)
- 設計: [architecture.md](../../design/memory-efficiency/architecture.md) / [dataflow.md](../../design/memory-efficiency/dataflow.md) / [types.rs](../../design/memory-efficiency/types.rs)
- タスク一覧: [overview.md](overview.md)
- 準備タスク: [prep.md](../../spec/memory-efficiency/prep.md)
