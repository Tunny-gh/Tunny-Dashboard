# memory-efficiency 設計ヒアリング記録

**作成日**: 2026-05-29
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

要件定義（[requirements.md](../../spec/memory-efficiency/requirements.md)）で MEM-001 の方針＝「共有ストア化（Arc）」は確定済み。本設計ヒアリングでは、追加コード調査（`message_handler.rs`, `study_worker.rs`, `render/gpu_buffer.rs`）で判明した統合上の論点について、アーキテクチャと型モデルを左右する3つの設計フォークを確定した。

## 質問と回答

### Q0: 設計作業規模

**質問日時**: 2026-05-29
**カテゴリ**: アーキテクチャ
**背景**: 出力文書セットを決定するため。本件は Rust デスクトップの内部リファクタリングで DB・REST API・TypeScript は存在しない。

**回答**: フル設計

**信頼性への影響**: architecture.md / dataflow.md / design-interview.md / Rust 型定義 `types.rs` を生成。DB スキーマ・API 仕様は対象外と確定。

---

### Q1: ライブ更新の書き込み戦略

**質問日時**: 2026-05-29
**カテゴリ**: アーキテクチャ
**背景**: `handle_live_update_done`（`message_handler.rs:209-268`）は `study.trial_rows` に追記し Pareto を再計算する一方、`DataFrame` は `from_trials` で一度だけ構築される不変構造（`model.rs:52`）。Arc 共有ストアでライブ更新の書き込みをどう成立させるかが、ストアの型（`ArcSwap` vs `Arc<RwLock>`）を決める。

**回答**: ArcSwap でスナップショット差替え（推奨）

**信頼性への影響**: 共有ストアを `study_id → ArcSwap<DataFrame>` と確定（🔵）。ロックフリー読み取り・原子的差替えのデータフロー（dataflow.md フロー3）が確定。

---

### Q2: 比較・複数 study の常駐スコープ

**質問日時**: 2026-05-29
**カテゴリ**: アーキテクチャ
**背景**: 現行の比較ロード `load_comparison_study_task`（`study_worker.rs:74-129`）は比較追加のたびに **ジャーナル全体を再パース** し、`trial_rows` を抽出してフル `StudyContext` を生成している（`app_state.rs:43` の `Vec<StudyContext>`）。共有ストアのスコープ（全 study 常駐 vs 選択分のみ）が、再パース排除と常駐メモリのトレードオフを決める。

**回答**: 全 study を初回パースで常駐化（推奨）

**信頼性への影響**: 共有ストアを「初回パースで全 study の DataFrame を `study_id` キーで常駐」と確定（🔵）。比較追加時の再パース・`trial_rows` 抽出を排除でき、MEM-005／MEM-001 の双方に整合。

---

### Q3: アプリ算出の行属性の配置

**質問日時**: 2026-05-29
**カテゴリ**: データモデル
**背景**: `TrialRow`（`state/types.rs:37-47`）には `DataFrame` にないアプリ層算出値（`pareto_rank`, `cluster_id`, `state`, `trial_number`）があり、ウィジェットが参照する。`Vec<TrialRow>` 撤廃後、これらをどこに保持するかが型モデル（`StudyView` の形）を決める。

**回答**: UI 側軽量 `StudyView` に並行配列（推奨）

**信頼性への影響**: `StudyView` を「`Arc<DataFrame>` ＋ 並行配列（`pareto_rank: Vec<u32>` 等）」と確定（🔵）。rust_core の `DataFrame` を純粋入力列に保ち、関心の分離（REQ-403）を維持。

---

## ヒアリング結果サマリー

### 確認できた事項
- `StudyContext.gpu_data` は描画側で未読（`render/gpu_buffer.rs` はレイアウト定数のみ）。MEM-007 の撤廃方針が確実化。
- ライブ更新・比較ロード・GPU 描画の統合経路を実コードで把握。

### 設計方針の決定事項
- 共有ストア: `study_id → ArcSwap<DataFrame>`、全 study 初回常駐（Q1, Q2）。
- UI 表現: `StudyContext { meta, view: StudyView, pareto_indices }`、`StudyView = Arc<DataFrame> + 並行配列`（Q3）。
- `trial_rows` と `gpu_data` を `StudyContext` から撤廃。
- 比較は `study_id` 参照＋遅延 `StudyView`、再パース廃止。

### 残課題
- `ArcSwap` 採用にあたり依存追加（`arc-swap` クレート）の可否は実装時に確認（Cargo.toml）。
- ライブ更新スナップショット再構築のコスト（全列再ビルド）が大規模 study で許容内かはベンチで確認（NFR-005）。
- 並行配列（pareto_rank 等）の再算出タイミングと `StudyView` 無効化の細部は実装フェーズで詳細化。

### 信頼性レベル分布

**ヒアリング前**（要件＋初期コード調査）:
- 🔵 青信号: 約 70%
- 🟡 黄信号: 約 30%
- 🔴 赤信号: 0%

**ヒアリング後**（3フォーク確定）:
- 🔵 青信号: 約 93%（+23pt）
- 🟡 黄信号: 約 7%
- 🔴 赤信号: 0%

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [types.rs](types.rs)
- **要件定義**: [requirements.md](../../spec/memory-efficiency/requirements.md)
