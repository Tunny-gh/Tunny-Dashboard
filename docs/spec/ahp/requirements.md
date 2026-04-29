# AHP (Analytic Hierarchy Process) 要件定義書

**作成日**: 2026-04-29
**ブランチ**: featura/egui

## 概要

Tunny Dashboard の MCDM（多基準意思決定）機能に **AHP（階層分析法）** を追加する。既存の TOPSIS / VIKOR / PROMETHEE とは独立した新規チャート（`ChartId::AhpRankChart` / `ChartId::AhpTable`）として実装する。

ユーザーは目的関数間の一対比較行列（Saaty 1-9スケール）を入力し、AHPが固有ベクトル近似法で優先度ベクトル（重み）を導出する。整合性比率（CR）を計算・表示し、CR > 0.1 の場合に警告を出す。加重和法でトライアルをスコアリングしてランキングを提供する。

## 関連文書

- **ヒアリング記録**: [💬 interview-record.md](interview-record.md)
- **ユーザストーリー**: [📖 user-stories.md](user-stories.md)
- **受け入れ基準**: [✅ acceptance-criteria.md](acceptance-criteria.md)
- **コンテキストノート**: [📝 note.md](note.md)
- **既存 MCDM 設計**: [theory/mcdm/topsis.md](../../theory/mcdm/topsis.md)
- **既存 MCDM 設計**: [theory/mcdm/vikor.md](../../theory/mcdm/vikor.md)
- **既存 MCDM 設計**: [theory/mcdm/promethee.md](../../theory/mcdm/promethee.md)

## 機能要件（EARS 記法）

**【信頼性レベル凡例】**:

- 🔵 **青信号**: PRD・EARS 要件定義書・設計文書・ユーザヒアリングを参考にした確実な要件
- 🟡 **黄信号**: PRD・EARS 要件定義書・設計文書・ユーザヒアリングから妥当な推測による要件
- 🔴 **赤信号**: PRD・EARS 要件定義書・設計文書・ユーザヒアリングにない推測による要件

---

### REQ-AHP-001〜008: Rust アルゴリズム層 (`rust_core/src/mcdm/ahp.rs`)

#### REQ-AHP-001: `compute_ahp` 関数の追加 🔵

_ユーザヒアリング・既存 topsis.rs / vikor.rs / promethee.rs パターンより_

- REQ-AHP-001-A: `rust_core/src/mcdm/ahp.rs` に `compute_ahp` 関数を実装しなければならない
- REQ-AHP-001-B: `compute_ahp` のシグネチャは以下でなければならない:
  ```rust
  pub fn compute_ahp(
      values: &[f64],
      n_trials: usize,
      n_objectives: usize,
      pairwise_matrix: &[f64],   // n_objectives × n_objectives の一対比較行列（行優先）
      is_minimize: &[bool],
  ) -> Result<AhpResult, String>
  ```
- REQ-AHP-001-C: `rust_core/src/mcdm/mod.rs` に `pub mod ahp;` を追加しなければならない
- REQ-AHP-001-D: `rust_core/src/lib.rs` に `pub use mcdm::ahp;` を追加しなければならない

#### REQ-AHP-002: AhpResult 型の定義 🔵

_ユーザヒアリング・既存 TopsisResult / VikorResult / PrometheeResult パターンより_

- REQ-AHP-002-A: `rust_core/src/mcdm/ahp.rs` に以下の構造体を定義しなければならない:
  ```rust
  #[derive(Debug, Clone, serde::Serialize)]
  pub struct AhpResult {
      pub priority_vector: Vec<f64>,     // 優先度ベクトル w [n_objectives]
      pub scores: Vec<f64>,              // 各トライアルの AHP スコア [n_trials]
      pub ranked_indices: Vec<u32>,      // スコア降順のランキング [n_trials]
      pub lambda_max: f64,               // 最大固有値 λmax
      pub ci: f64,                       // 整合性指標 CI
      pub ri: f64,                       // ランダム整合性指標 RI
      pub cr: f64,                       // 整合性比率 CR
      pub is_consistent: bool,           // CR ≤ 0.10 かどうか
      pub duration_ms: f64,
  }
  ```

#### REQ-AHP-003: 一対比較行列バリデーション 🔵

_ユーザヒアリング・AHP アルゴリズム仕様より_

- REQ-AHP-003-A: `compute_ahp` は `pairwise_matrix.len() != n_objectives * n_objectives` の場合 `Err` を返さなければならない
- REQ-AHP-003-B: `compute_ahp` は `n_objectives == 0` の場合 `Err` を返さなければならない
- REQ-AHP-003-C: `compute_ahp` は既存 `super::validate_inputs` を呼び出す際に `weights` の代わりに一様重み（`vec![1.0 / n_objectives as f64; n_objectives]`）を使用してよい 🟡 _validate_inputs の weights 引数を流用する設計から妥当な推測_

#### REQ-AHP-004: 一対比較行列の前処理 🔵

_AHP アルゴリズム標準定義より_

- REQ-AHP-004-A: 対角成分 `A[i][i]` は常に 1.0 として扱わなければならない（入力値を無視して上書き）
- REQ-AHP-004-B: `A[j][i]` は `1.0 / A[i][j]` として導出しなければならない（上三角入力から下三角を自動補完） 🟡 _UIは上三角のみ入力する設計から妥当な推測_

#### REQ-AHP-005: 優先度ベクトル導出（固有ベクトル近似法） 🔵

_ユーザヒアリング（固有ベクトル法）・AHP アルゴリズム標準定義より_

- REQ-AHP-005-A: 各列の合計 `col_sum[j] = Σ_i A[i][j]` を計算しなければならない
- REQ-AHP-005-B: 正規化行列 `B[i][j] = A[i][j] / col_sum[j]` を計算しなければならない
- REQ-AHP-005-C: 優先度ベクトル `w[i] = Σ_j B[i][j] / n_objectives` を計算しなければならない（行平均）
- REQ-AHP-005-D: `col_sum[j] == 0` の場合は `Err` を返さなければならない 🟡 _入力値がすべて0の場合の除算回避_

#### REQ-AHP-006: 整合性チェック 🔵

_ユーザヒアリング（CR チェック実装）・Saaty 整合性指標定義より_

- REQ-AHP-006-A: `λmax = Σ_j (col_sum[j] × w[j])` を計算しなければならない
- REQ-AHP-006-B: `CI = (λmax - n_objectives) / (n_objectives - 1)` を計算しなければならない（`n_objectives == 1` の場合は `CI = 0.0`）
- REQ-AHP-006-C: RI は以下のテーブルから取得しなければならない:
  - n=1: 0.00, n=2: 0.00, n=3: 0.58, n=4: 0.90, n=5: 1.12, n≥6: 1.24 🟡 _Saaty テーブル標準値。n≥6 は追加検討_
- REQ-AHP-006-D: `CR = CI / RI` を計算しなければならない（`RI == 0.0` の場合は `CR = 0.0`、すなわち n≤2 は常に一貫）
- REQ-AHP-006-E: `is_consistent = CR ≤ 0.10` を設定しなければならない

#### REQ-AHP-007: スコア計算（加重和法） 🔵

_ユーザヒアリング（加重和法・Min-Max正規化）より_

- REQ-AHP-007-A: NaN 含有試行を `super::filter_valid_indices` で除外しなければならない
- REQ-AHP-007-B: 有効試行のみから `min_j` / `max_j` を計算しなければならない
- REQ-AHP-007-C: Min-Max 正規化を以下で行わなければならない:
  - `is_minimize[j] = true`: `normalized = (max_j - v) / (max_j - min_j)` （小さいほど高スコア）
  - `is_minimize[j] = false`: `normalized = (v - min_j) / (max_j - min_j)` （大きいほど高スコア）
  - `max_j == min_j` の場合: `normalized = 0.0`（全値同一）
- REQ-AHP-007-D: AHP スコア `score[i] = Σ_j w[j] × normalized_j(i)` を計算しなければならない
- REQ-AHP-007-E: NaN 試行の `scores[i]` は `0.0` に設定しなければならない

#### REQ-AHP-008: ランキング生成 🔵

_ユーザヒアリング・既存 topsis.rs パターンより_

- REQ-AHP-008-A: `ranked_indices` は AHP スコア降順（高スコア = 良い）でソートしなければならない
- REQ-AHP-008-B: NaN 試行は `ranked_indices` の末尾に配置しなければならない

---

### REQ-AHP-010〜015: egui-app 状態管理層 (`egui-app/src/state/`)

#### REQ-AHP-010: AhpResult 型の追加 🔵

_既存 TopsisResult / VikorResult / PrometheeResult パターンより_

- REQ-AHP-010-A: `egui-app/src/state/results.rs` に以下を追加しなければならない:
  ```rust
  #[derive(Debug, Clone)]
  pub struct AhpResult {
      pub priority_vector: Vec<f64>,
      pub scores: Vec<f64>,
      pub ranked_indices: Vec<u32>,
      pub lambda_max: f64,
      pub ci: f64,
      pub ri: f64,
      pub cr: f64,
      pub is_consistent: bool,
      pub duration_ms: f64,
  }
  ```

#### REQ-AHP-011: AppMessage::AhpDone 追加 🔵

_ユーザヒアリング（McdmDone と独立したメッセージ）より_

- REQ-AHP-011-A: `egui-app/src/state/messages.rs` に `AppMessage::AhpDone(AhpResult)` を追加しなければならない
- REQ-AHP-011-B: AHP は既存 `AppMessage::McdmDone` とは独立したメッセージを使用しなければならない

#### REQ-AHP-012: メッセージハンドラの更新 🔵

_既存 AppMessage::McdmDone ハンドラパターンより_

- REQ-AHP-012-A: `MessageHandler::handle` に `AppMessage::AhpDone(result)` 分岐を追加し、`app_state.ahp_result = Some(result.clone())` / `widget_states.ahp_chart.computing = false` をセットしなければならない

#### REQ-AHP-013: AppState への追加 🔵

_既存 app_state.rs パターンより_

- REQ-AHP-013-A: `egui-app/src/state/app_state.rs` に `pub ahp_result: Option<AhpResult>` フィールドを追加しなければならない
- REQ-AHP-013-B: `AppState::clear()` 時に `ahp_result = None` をクリアしなければならない

#### REQ-AHP-014: ChartId への追加 🔵

_ユーザヒアリング（完全別チャート）より_

- REQ-AHP-014-A: `egui-app/src/state/layout_state.rs` の `ChartId` enum に `AhpRankChart` と `AhpTable` を追加しなければならない
- REQ-AHP-014-B: `ChartId::all()` に `AhpRankChart` と `AhpTable` を含めなければならない
- REQ-AHP-014-C: `ChartId::AhpRankChart.label()` は `"AHP Ranking"` を返さなければならない
- REQ-AHP-014-D: `ChartId::AhpTable.label()` は `"AHP Table"` を返さなければならない

#### REQ-AHP-015: WidgetStates への追加 🔵

_既存 widget_states.rs パターンより_

- REQ-AHP-015-A: `egui-app/src/ui/widget_states.rs` に `pub ahp_chart: AhpChart` フィールドを追加しなければならない

---

### REQ-AHP-020〜027: UI 層 (`egui-app/src/ui/widgets/ahp_chart.rs`)

#### REQ-AHP-020: AhpChart 構造体の定義 🔵

_ユーザヒアリング（一対比較行列入力UI + ランキング表示）より_

- REQ-AHP-020-A: `egui-app/src/ui/widgets/ahp_chart.rs` に以下のフィールドを持つ `AhpChart` 構造体を定義しなければならない:
  ```rust
  pub struct AhpChart {
      pub pairwise: Vec<f64>,        // 上三角の入力値 [n*(n-1)/2]（n=目的関数数）
      pub computing: bool,
      pub pending_compute: Option<AhpComputeRequest>,
      pub top_n: AhpTopN,
  }
  ```

#### REQ-AHP-021: 一対比較行列入力 UI 🔵

_ユーザヒアリング（一対比較行列入力グリッド）より_

- REQ-AHP-021-A: `AhpChart::show` で目的関数数 n に応じた n×n の行列グリッドを表示しなければならない
- REQ-AHP-021-B: 対角セルは "1" として読み取り専用表示しなければならない
- REQ-AHP-021-C: 上三角セル（i < j）は `egui::DragValue` で 1-9 の範囲で入力可能にしなければならない（デフォルト値: 1.0）
- REQ-AHP-021-D: 下三角セル（i > j）は対応する上三角の逆数を表示するだけで編集不可としなければならない
- REQ-AHP-021-E: 目的関数名を行・列ヘッダーとして表示しなければならない

#### REQ-AHP-022: CR 表示 🔵

_ユーザヒアリング（CR チェック・警告）より_

- REQ-AHP-022-A: 計算結果が存在する場合、CR 値を `"CR = 0.XXX"` 形式で表示しなければならない
- REQ-AHP-022-B: `CR ≤ 0.10` の場合は緑色（`egui::Color32::GREEN` 相当）で `"✓ Consistent"` を表示しなければならない
- REQ-AHP-022-C: `CR > 0.10` の場合は赤色（`egui::Color32::RED` 相当）で `"⚠ Inconsistent (CR > 0.10)"` を表示しなければならない
- REQ-AHP-022-D: CR が不整合でも計算（Run）は実行可能でなければならない 🔵 _ユーザヒアリングより_

#### REQ-AHP-023: Run ボタンと計算起動 🔵

_既存 mcdm_chart.rs Run ボタンパターンより_

- REQ-AHP-023-A: `"Run"` ボタンを表示し、押下時に `pending_compute: Some(AhpComputeRequest { ... })` をセットしなければならない
- REQ-AHP-023-B: 計算中は `"Computing..."` テキストを表示し Run ボタンを無効化しなければならない

#### REQ-AHP-024: 優先度ベクトルバーチャート 🔵

_ユーザヒアリング（優先度ベクトルバーチャート + ランキングテーブル）より_

- REQ-AHP-024-A: 計算結果が存在する場合、目的関数ごとの優先度ベクトル値をバーチャートで表示しなければならない
- REQ-AHP-024-B: バーの幅は `w[j] / max(w)` で正規化した相対幅で表示しなければならない
- REQ-AHP-024-C: 各バーに目的関数名と優先度値（例: `"f1: 0.500"`）を表示しなければならない

#### REQ-AHP-025: AhpTable ウィジェット（ランキングテーブル） 🔵

_ユーザヒアリング・既存 McdmTable パターンより_

- REQ-AHP-025-A: `AhpTable` ウィジェットを実装し、AHP スコア降順のランキングを表形式で表示しなければならない
- REQ-AHP-025-B: テーブルは「順位」「Trial ID」「AHP スコア」「各目的関数値」の列を持たなければならない
- REQ-AHP-025-C: 上位 N 件（Top5/Top10/Top20）表示切替コンボボックスを持たなければならない

#### REQ-AHP-026: chart_registry での spawn_task 🔵

_既存 chart_registry.rs MCDM spawn_task パターンより_

- REQ-AHP-026-A: `chart_registry.rs` の `ChartId::AhpRankChart` 分岐で `widgets.ahp_chart.pending_compute.take()` があれば `spawn_task` を呼び出さなければならない
- REQ-AHP-026-B: `spawn_task` 内で `tunny_core::ahp::compute_ahp(...)` を呼び出し、成功時は `AppMessage::AhpDone(result)` を送信しなければならない
- REQ-AHP-026-C: エラー時は `AppMessage::Error(...)` を送信しなければならない

#### REQ-AHP-027: Study 変更時のクリア 🔵

_既存 StudySelected ハンドラパターンより_

- REQ-AHP-027-A: Study 変更（`AppMessage::StudySelected`）時に `app_state.ahp_result = None` をクリアしなければならない
- REQ-AHP-027-B: Study 変更時に `widget_states.ahp_chart` を `Default::default()` でリセットしなければならない 🟡 _既存パターンから妥当な推測_

---

### REQ-AHP-030: 理論ドキュメント追加 🟡

_既存 theory/mcdm/topsis.md / vikor.md / promethee.md パターンから妥当な推測_

- REQ-AHP-030-A: `theory/mcdm/ahp.md` を作成しなければならない（アルゴリズム説明・Saaty スケール・固有ベクトル近似法・CR 計算・加重和法の数式を含む）

---

## 非機能要件

### パフォーマンス

- NFR-AHP-001: `compute_ahp` は 50,000 試行 × 4 目的で 50 ms 以内に完了しなければならない 🟡 _O(n × m²) アルゴリズム（n=試行数、m=目的数）。TOPSIS より低い負荷から妥当な推測_
- NFR-AHP-002: `compute_ahp` は 10,000 試行 × 4 目的で 10 ms 以内に完了しなければならない 🟡 _実用的なユースケースでの推測_

### コード規約

- NFR-AHP-010: `ahp.rs` は既存の `topsis.rs` / `vikor.rs` / `promethee.rs` と同一コーディングスタイルを採用しなければならない 🔵 _既存 mcdm コードベースより_
- NFR-AHP-011: `egui-app` 側の新規コードはインラインスタイルのみを使用しなければならない（Tailwind CSS 不使用） 🔵 _プロジェクトルール・既存実装より_

### テスト

- NFR-AHP-020: `ahp.rs` に正常系・異常系・境界値・パフォーマンスの単体テストを含めなければならない 🔵 _既存 topsis.rs / vikor.rs / promethee.rs のテスト構成より_
- NFR-AHP-021: `ahp_chart.rs` の AHP 関連 UI ロジック（一対比較行列処理、CR 計算連携）に単体テストを追加しなければならない 🔵 _既存 mcdm_chart.rs テストパターンより_

---

## Edge ケース

### アルゴリズム

- EDGE-AHP-001: n_objectives = 1 の場合、優先度ベクトル = [1.0]、CI = 0、CR = 0、スコア = Min-Max 正規化値として正常終了しなければならない 🔵 _n=1 は常に一貫（RI=0）_
- EDGE-AHP-002: n_objectives = 2 の場合、RI = 0.0 なので CR = 0.0 として正常終了しなければならない 🔵 _2×2 行列は常に一貫_
- EDGE-AHP-003: 全試行が NaN の場合（valid_indices が空）、全スコア = 0.0 で正常終了しなければならない 🟡 _既存パターンから妥当な推測_
- EDGE-AHP-004: 目的関数の全値が同一（max_j == min_j）の場合、normalized = 0.0 として除算エラーを回避しなければならない 🔵 _REQ-AHP-007-C より確定_
- EDGE-AHP-005: 一対比較行列の入力値が 0 以下の場合、`Err` を返さなければならない 🔵 _Saaty スケールは正値のみ有効_
- EDGE-AHP-006: CR > 0.10 の場合でも計算は正常完了し、`is_consistent = false` で結果を返さなければならない 🔵 _ユーザヒアリングより（警告表示のみ）_

### UI

- EDGE-AHP-010: 目的関数が変わった場合（Study 変更）、一対比較行列の入力値はリセットされなければならない 🔵 _REQ-AHP-027-B より_
- EDGE-AHP-011: 目的関数が 1 つの場合、一対比較行列グリッドは 1×1 で行列入力 UI を省略してよい 🟡 _n=1 は比較不要のため合理的推測_
