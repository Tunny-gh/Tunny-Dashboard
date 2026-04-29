# PROMETHEE Ranking 要件定義書

## 概要

Tunny Dashboard の MCDM（多基準意思決定）機能に **PROMETHEE I（部分順位付け）** と **PROMETHEE II（完全順位付け）** を追加する。既存の TOPSIS / VIKOR に並ぶ第3・第4の選択肢として `McdmMethod` コンボボックスに統合し、同一の計算結果から 2 種類のランキングを提供する。

Linear 選好関数（閾値 q=0、p=範囲の 20% 自動設定）のみをサポートし、実装・UI を最小限に抑える。

## 関連文書

- **ヒアリング記録**: [💬 interview-record.md](interview-record.md)
- **ユーザストーリー**: [📖 user-stories.md](user-stories.md)
- **受け入れ基準**: [✅ acceptance-criteria.md](acceptance-criteria.md)
- **コンテキストノート**: [📝 note.md](note.md)
- **既存 MCDM 設計**: [theory/mcdm/topsis.md](../../theory/mcdm/topsis.md)
- **既存 MCDM 設計**: [theory/mcdm/vikor.md](../../theory/mcdm/vikor.md)

## 機能要件（EARS 記法）

**【信頼性レベル凡例】**:
- 🔵 **青信号**: PRD・EARS 要件定義書・設計文書・ユーザヒアリングを参考にした確実な要件
- 🟡 **黄信号**: PRD・EARS 要件定義書・設計文書・ユーザヒアリングから妥当な推測による要件
- 🔴 **赤信号**: PRD・EARS 要件定義書・設計文書・ユーザヒアリングにない推測による要件

---

### REQ-PR-001〜008: Rust アルゴリズム層 (`rust_core/src/mcdm/promethee.rs`)

#### REQ-PR-001: `compute_promethee` 関数の追加 🔵

*ユーザヒアリング・既存 topsis.rs / vikor.rs パターンより*

- REQ-PR-001-A: `rust_core/src/mcdm/promethee.rs` に `compute_promethee` 関数を実装しなければならない
- REQ-PR-001-B: `compute_promethee` のシグネチャは以下でなければならない:
  ```rust
  pub fn compute_promethee(
      values: &[f64],
      n_trials: usize,
      n_objectives: usize,
      weights: &[f64],
      is_minimize: &[bool],
  ) -> Result<PrometheeResult, String>
  ```
- REQ-PR-001-C: `rust_core/src/mcdm/mod.rs` に `pub mod promethee;` を追加しなければならない
- REQ-PR-001-D: `rust_core/src/lib.rs` に `pub use mcdm::promethee;` を追加しなければならない

#### REQ-PR-002: PrometheeResult 型の定義 🔵

*ユーザヒアリング・既存 TopsisResult / VikorResult パターンより*

- REQ-PR-002-A: `rust_core/src/mcdm/promethee.rs` に以下の構造体を定義しなければならない:
  ```rust
  #[derive(Debug, Clone, serde::Serialize)]
  pub struct PrometheeResult {
      pub phi_plus: Vec<f64>,        // 正フロー Φ+  [n_trials]
      pub phi_minus: Vec<f64>,       // 負フロー Φ-  [n_trials]
      pub phi_net: Vec<f64>,         // 純フロー Φ   [n_trials]
      pub ranked_indices_i: Vec<u32>,  // PROMETHEE I  ランキング
      pub ranked_indices_ii: Vec<u32>, // PROMETHEE II ランキング
      pub duration_ms: f64,
  }
  ```

#### REQ-PR-003: 入力バリデーション 🔵

*既存 `mcdm::validate_inputs` パターンより*

- REQ-PR-003-A: `compute_promethee` は `super::validate_inputs(values, n_trials, n_objectives, weights, is_minimize)?` を呼び出さなければならない
- REQ-PR-003-B: バリデーション失敗時は `Err(String)` を返さなければならない

#### REQ-PR-004: NaN 試行の除外 🔵

*既存 `mcdm::filter_valid_indices` パターンより*

- REQ-PR-004-A: `compute_promethee` は `super::filter_valid_indices(values, n_trials, n_objectives)` を用いて NaN 含有試行を除外しなければならない
- REQ-PR-004-B: NaN 試行の `phi_plus` / `phi_minus` / `phi_net` は `0.0` にしなければならない
- REQ-PR-004-C: NaN 試行は `ranked_indices_i` および `ranked_indices_ii` の末尾に配置しなければならない

#### REQ-PR-005: Linear 選好関数 🔵

*ユーザヒアリング（Linear のみ・自動閾値）より*

- REQ-PR-005-A: 各目的関数 j の閾値を自動計算しなければならない:
  - `q_j = 0.0`
  - `p_j = 0.2 × (max_j - min_j)` ただし `max_j` / `min_j` は有効試行のみから算出
  - `range_j = 0` の場合は `p_j = 0.0`（全 P = 0 にフォールバック）
- REQ-PR-005-B: 差分 d を以下で定義しなければならない:
  - `is_minimize[j] = true`  の場合: `d = f_j(a) - f_j(b)`
  - `is_minimize[j] = false` の場合: `d = f_j(b) - f_j(a)`
- REQ-PR-005-C: Linear 選好関数の値は以下でなければならない:
  - `d ≤ q_j`:        `P = 0.0`
  - `q_j < d ≤ p_j`:  `P = (d - q_j) / (p_j - q_j)`
  - `d > p_j`:        `P = 1.0`
  - `p_j = q_j`（範囲ゼロ）: `P = 0.0`

#### REQ-PR-006: 集約選好指数とフロー計算 🔵

*PROMETHEE アルゴリズム標準定義・ユーザヒアリングより*

- REQ-PR-006-A: 集約選好指数を `π(a,b) = Σ_j weights[j] × P_j(a,b)` で計算しなければならない
- REQ-PR-006-B: 正フロー `Φ+(i) = 1/(n_valid-1) × Σ_{k≠i} π(i,k)` で計算しなければならない
- REQ-PR-006-C: 負フロー `Φ-(i) = 1/(n_valid-1) × Σ_{k≠i} π(k,i)` で計算しなければならない
- REQ-PR-006-D: 純フロー `Φ(i) = Φ+(i) - Φ-(i)` で計算しなければならない
- REQ-PR-006-E: `n_valid = 1` の場合は `Φ+(i) = Φ-(i) = Φ(i) = 0.0` としなければならない（除算回避）
- REQ-PR-006-F: `n_valid = 0` の場合（全試行が NaN）は全フローを `0.0` として正常終了しなければならない

#### REQ-PR-007: PROMETHEE I ランキング 🔵

*ユーザヒアリング（Φ+降順・Φ-昇順タイブレーク）より*

- REQ-PR-007-A: `ranked_indices_i` は `Φ+` 降順でソートしなければならない
- REQ-PR-007-B: `Φ+` が同値の場合は `Φ-` 昇順でタイブレークしなければならない
- REQ-PR-007-C: NaN 試行は末尾に配置しなければならない

#### REQ-PR-008: PROMETHEE II ランキング 🔵

*ユーザヒアリング（Φnet 降順）より*

- REQ-PR-008-A: `ranked_indices_ii` は `Φnet` 降順でソートしなければならない
- REQ-PR-008-B: NaN 試行は末尾に配置しなければならない

---

### REQ-PR-010〜016: egui-app 状態管理層 (`egui-app/src/state/results.rs`)

#### REQ-PR-010: PrometheeResult 型の追加 🔵

*既存 TopsisResult / VikorResult パターンより*

- REQ-PR-010-A: `egui-app/src/state/results.rs` に以下を追加しなければならない:
  ```rust
  #[derive(Debug, Clone)]
  pub struct PrometheeResult {
      pub phi_plus: Vec<f64>,
      pub phi_minus: Vec<f64>,
      pub phi_net: Vec<f64>,
      pub ranked_indices_i: Vec<u32>,
      pub ranked_indices_ii: Vec<u32>,
      pub duration_ms: f64,
  }
  ```

#### REQ-PR-011: McdmMethod への追加 🔵

*ユーザヒアリング（コンボボックス統合）より*

- REQ-PR-011-A: `McdmMethod` enum に `PrometheeI` と `PrometheeII` を追加しなければならない
- REQ-PR-011-B: `McdmMethod::label()` で以下を返さなければならない:
  - `PrometheeI`  → `"PROMETHEE I"`
  - `PrometheeII` → `"PROMETHEE II"`
- REQ-PR-011-C: `McdmMethod::all()` は `[Topsis, Vikor, PrometheeI, PrometheeII]` を返さなければならない

#### REQ-PR-012: McdmResult enum への追加 🔵

*ユーザヒアリング（既存キャッシュ機構流用）より*

- REQ-PR-012-A: `McdmResult` enum に `Promethee(PrometheeResult)` バリアントを追加しなければならない

#### REQ-PR-013: McdmResult メソッドの更新 🔵

*既存 primary_scores / ranked_indices パターンより*

- REQ-PR-013-A: `McdmResult::primary_scores()` は `Promethee` バリアントに対して以下を返さなければならない:
  - `method = PrometheeI`  → `&r.phi_plus`
  - `method = PrometheeII` → `&r.phi_net`
  - **注**: `Promethee` バリアントは `method` フィールドを保持するか、`McdmResult` に `method` 情報を含める 🟡
- REQ-PR-013-B: `McdmResult::ranked_indices()` は `Promethee` バリアントに対して以下を返さなければならない:
  - `method = PrometheeI`  → `&r.ranked_indices_i`
  - `method = PrometheeII` → `&r.ranked_indices_ii`
- REQ-PR-013-C: `McdmResult::duration_ms()` は `Promethee(r) => r.duration_ms` を返さなければならない
- REQ-PR-013-D: `McdmResult::method_label()` は `PrometheeI => "PROMETHEE I"` / `PrometheeII => "PROMETHEE II"` を返さなければならない

#### REQ-PR-014: McdmResult に method フィールドを持つ方針 🟡

*REQ-PR-013 から妥当な推測: Promethee バリアントは I/II どちらで計算されたかを区別する必要がある*

- REQ-PR-014-A: `McdmResult::Promethee` バリアントは `method: McdmMethod` フィールドを含むか、あるいは `McdmResult::PrometheeI(PrometheeResult)` / `McdmResult::PrometheeII(PrometheeResult)` の 2 バリアントに分けるか、どちらかで実装しなければならない 🟡

---

### REQ-PR-020〜024: UI 層 (`egui-app/src/ui/widgets/mcdm_chart.rs`)

#### REQ-PR-020: McdmRankChart にキャッシュフィールドを追加 🔵

*既存 cached_topsis / cached_vikor パターンより*

- REQ-PR-020-A: `McdmRankChart` に `cached_promethee: Option<PrometheeResult>` フィールドを追加しなければならない

#### REQ-PR-021: メソッド切替時のキャッシュ復元 🔵

*既存 pending_restore パターンより*

- REQ-PR-021-A: メソッドコンボボックスで `PrometheeI` / `PrometheeII` に切り替えた時、`cached_promethee` が存在すれば `pending_restore` に `McdmResult::Promethee(...)` を設定しなければならない

#### REQ-PR-022: PROMETHEE I バーチャート表示 🔵

*ユーザヒアリング（Φ+/Φ- 2 本バー）より*

- REQ-PR-022-A: `McdmRankChart::show` 内で `method == McdmMethod::PrometheeI` の場合は `Φ+` バー（青: `#0c6ac0`）と `Φ-` バー（赤: `#c02020`）を横並びで表示しなければならない
- REQ-PR-022-B: `Φ+` バーは既存バーチャートと同様に最大値正規化した幅で描画しなければならない
- REQ-PR-022-C: `Φ-` バーは最大値正規化した幅で描画し、`Φ-` が小さいほど良いことをラベルで明示しなければならない (`"Φ-" label`)
- REQ-PR-022-D: `ranked_indices` は `ranked_indices_i` を使用しなければならない

#### REQ-PR-023: PROMETHEE II バーチャート表示 🔵

*ユーザヒアリング・既存バーチャートパターンより*

- REQ-PR-023-A: `McdmRankChart::show` 内で `method == McdmMethod::PrometheeII` の場合は `Φnet` の単一バーを表示しなければならない
- REQ-PR-023-B: `Φnet` が負の値を取る場合も表示が崩れないよう対処しなければならない 🟡 *Φnet ∈ [-1, 1] であり既存の `max_score` 正規化では 0 以下が表示できない可能性*
- REQ-PR-023-C: `ranked_indices` は `ranked_indices_ii` を使用しなければならない

#### REQ-PR-024: メッセージハンドラの更新 🔵

*既存 AppMessage::McdmDone ハンドラパターンより*

- REQ-PR-024-A: `MessageHandler::handle` の `AppMessage::McdmDone(result)` ハンドラに `McdmResult::Promethee(r)` 分岐を追加し `cached_promethee = Some(r.clone())` をセットしなければならない

---

### REQ-PR-030: 理論ドキュメント追加 🟡

*既存 theory/mcdm/topsis.md / vikor.md パターンから妥当な推測*

- REQ-PR-030-A: `theory/mcdm/promethee.md` を作成しなければならない（アルゴリズム説明・Linear 選好関数の数式・フロー計算式を含む）

---

## 非機能要件

### パフォーマンス

- NFR-PR-001: `compute_promethee` は 50,000 試行 × 4 目的で 200 ms 以内に完了しなければならない 🟡 *O(n²) アルゴリズムのため TOPSIS/VIKOR より低い目標値。既存 50k テストパターンから妥当な推測*
- NFR-PR-002: `compute_promethee` は 10,000 試行 × 4 目的で 20 ms 以内に完了しなければならない 🟡 *実用的なユースケースでの推測*

### コード規約

- NFR-PR-010: `promethee.rs` は既存の `topsis.rs` / `vikor.rs` と同一コーディングスタイルを採用しなければならない 🔵 *既存 mcdm コードベースより*
- NFR-PR-011: `egui-app` 側の新規コードは Tailwind CSS を使用してはならない（インラインスタイルのみ） 🔵 *wasm-phase2-requirements.md NFR-201 ／ プロジェクトルールより*

### テスト

- NFR-PR-020: `promethee.rs` に正常系・異常系・境界値・パフォーマンスの単体テストを含めなければならない 🔵 *既存 topsis.rs / vikor.rs の テスト構成より*
- NFR-PR-021: `mcdm_chart.rs` の Promethee 関連 UI ロジック（キャッシュ、ランキング行列生成）に単体テストを追加しなければならない 🔵 *既存 mcdm_chart.rs テストパターンより*

---

## Edge ケース

### アルゴリズム

- EDGE-PR-001: n_trials = 1 の場合、全フローを 0.0 として正常終了しなければならない 🔵 *REQ-PR-006-E より確定*
- EDGE-PR-002: 全目的関数が同一値の場合（range_j = 0）、全 P = 0 → 全フロー = 0.0 で正常終了しなければならない 🔵 *REQ-PR-005-A フォールバックより確定*
- EDGE-PR-003: n_objectives = 1 の場合も計算が正常に動作しなければならない 🔵 *validate_inputs による 0 チェックのみで自然に動作*
- EDGE-PR-004: 全試行が NaN の場合（valid_indices が空）、全フロー = 0.0 で正常終了しなければならない 🔵 *REQ-PR-006-F より確定*
- EDGE-PR-005: Φnet が負の場合でも PROMETHEE II バーチャートがクラッシュしないようにしなければならない 🟡 *Φnet ∈ [-1, 1] の範囲から妥当なリスク*

### UI

- EDGE-PR-010: PROMETHEE I / II 選択後に別メソッドへ切り替え、再び戻った場合は `cached_promethee` から復元されなければならない 🔵 *既存キャッシュパターンより*
- EDGE-PR-011: Study 変更時に `cached_promethee` と `app_state.mcdm_result`（Promethee）はクリアされなければならない 🔵 *既存 StudySelected ハンドラパターンより*
