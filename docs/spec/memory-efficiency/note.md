# memory-efficiency コンテキストノート

**作成日**: 2026-05-29
**要件名**: memory-efficiency（メモリ効率化）
**PRD**: [memory-efficiency-requirements.md](../../../memory-efficiency-requirements.md)（プロジェクトルート）

## 技術スタック

| レイヤ | 技術 |
|---|---|
| コア処理 | Rust + ndarray（`rust_core` / クレート名 `tunny-core`）|
| UI | eframe + egui（`egui-app` / クレート名 `tunny-desktop`）|
| GPU描画 | wgpu + egui-wgpu |
| チャート | egui_plot |
| ML | linfa |

- ワークスペース構成: `rust_core`（パーサ・DataFrame・分析）と `egui-app`（デスクトップUI）の2クレート。
- WASM/TS対応は不要（ネイティブ専用）。後方互換性の維持は不要。

## 開発ルール（CLAUDE.md より）

- ビルド: `cargo build --workspace` / `cargo build --workspace --release`
- テスト: `cargo test --workspace` / `cargo test -p tunny-core` / `cargo test -p tunny-desktop`
- 実行: `cargo run -p tunny-desktop`
- ベンチ: `cargo bench -p tunny-core`

## 関連実装（コード調査で確認済み）

### データ表現の二重化（MEM-001 / 007 の中核）
- `egui-app/src/state/types.rs:59` `StudyContext { meta, trial_rows: Vec<TrialRow>, gpu_data, pareto_indices }`
- `egui-app/src/state/types.rs:37` `TrialRow { params: HashMap<String,f64>, objectives: Vec<f64>, user_attrs: HashMap<String,String>, ... }`（行指向・per-row HashMap）
- `rust_core/src/data/dataframe/model.rs:7` `DataFrame { numeric_cols: Vec<(String, Vec<f64>)>, string_cols: Vec<(String, Vec<String>)>, ... }`（列指向）
- `rust_core/src/data/dataframe/state.rs:12-21` DataFrame は **thread_local の GLOBAL_STATE** に格納される。
- `egui-app/src/io/study.rs:46-94` `extract_trial_rows()` が `with_active_df` 経由で列データを取り出し、各行ごとに `HashMap` を再構築して `Vec<TrialRow>` を生成 → `AppMessage::StudySelected` で UI スレッドへ送る。

### 重要な制約（要件方針を左右）
- **DataFrame は rust_core 内の thread_local（ワーカースレッド上）に存在し、UI スレッドからは直接借用できない。** これが `trial_rows` 複製の根本原因。
- ユーザー決定: MEM-001 は **共有ストア化（`Arc` 等で thread_local を廃止し UI/ワーカー両スレッドから安全に読めるようにする）** を方針とする。

### ウィジェットキャッシュ（MEM-002 / 003）
- `egui-app/src/ui/widgets/pareto_2d.rs:38` `display_rows_cache: Option<Vec<TrialRow>>`
- `egui-app/src/ui/widgets/pareto_3d.rs:99` `display_rows_cache: Option<Vec<TrialRow>>`
- `egui-app/src/ui/widgets/parallel_coords.rs:71-72` `col_data_cache: Option<Vec<Vec<f64>>>` + `col_ranges_cache`
- `egui-app/src/ui/widgets/scatter_matrix.rs:31` `col_data_cache: Option<Vec<Vec<f64>>>`
- いずれも `cache_key`（trial_count 等）でキャッシュ無効化している。

### 分析パイプライン（MEM-004）
- `egui-app/src/ui/poll_chart.rs:16-22` `r.params.get(p)` の HashMap 参照から `Vec<Vec<f64>>` 行列を実行ごとに再構築。PDP / Surface Plot / Sensitivity が対象。
- PRD 参照先: `rust_core/src/pdp/utils.rs`, `rust_core/src/sensitivity/analysis/full.rs`

### 比較モード（MEM-005）
- `egui-app/src/state/app_state.rs:43` `comparison_studies: Vec<StudyContext>`（最大4件、各々フル StudyContext）
- `app_state.rs:128` `reset_comparison_session()` で `comparison_studies.clear()`
- 関連: `egui-app/src/state/message_handler.rs`, `egui-app/src/io/study_worker.rs`

### ジャーナルパース（MEM-006）
- `rust_core/src/io/journal/parser/state.rs:9-13` `ParserState { trial_builders: HashMap<u32, TrialBuilder>, ... }`
- `rust_core/src/io/journal/parser/finalize.rs:20` `per_study_rows: Vec<Vec<TrialRow>>` を全試行ぶん構築 → `finalize.rs:104` `DataFrame::from_trials(&per_study_rows[index], ...)` で DataFrame 構築。**per_study_rows と DataFrame がピーク時に共存。**

### GPU バッファ（MEM-007）
- `egui-app/src/state/types.rs:50` `GpuBufferData { positions, positions3d, colors, sizes, trial_count }`
- `.gpu_data` への書き込み: `egui-app/src/state/message_handler.rs:254`
- 描画側からの `.gpu_data` 読み出しは grep で発見されず（テスト `egui-app/tests/poller_integration.rs:147` のみ）。UI/描画で未使用の疑いが濃厚。

## 設計文書

- 既存設計: `docs/design/tunny-dashboard/architecture.md`, `dataflow.md`
- 本要件の設計は今後 `/tsumiki:kairo-design memory-efficiency` で `docs/design/memory-efficiency/` に作成予定。

## 注意事項

- 既存の `TrialRow` には egui-app 版（`state/types.rs`）と rust_core 版（`dataframe/types.rs`）の2種類があり、フィールド構成が異なる。混同しないこと。
- 等価性検証は **既存テストスイート（`cargo test --workspace`）のグリーン維持** を主証拠とする（ユーザー決定）。
- メモリ削減効果は **100k trials × 約22列の代表データセットで定量ベンチマーク必須**（ユーザー決定）。
