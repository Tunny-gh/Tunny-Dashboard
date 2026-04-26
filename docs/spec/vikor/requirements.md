# VIKOR 要件定義書

**作成日**: 2026-04-24
**ブランチ**: featura/egui

## 概要

VIKOR (VIseKriterijumska Optimizacija I Kompromisno Resenje) 多基準意思決定アルゴリズムを実装する。既存のTOPSIS実装と同じアーキテクチャ（`McdmMethod` enum拡張・`McdmResult` enum拡張）を踏襲し、`rust_core` に純粋Rust計算実装を追加、`egui-app` に状態型・UIウィジェット拡張を行う。

## 関連文書

- **ヒアリング記録**: [💬 interview-record.md](interview-record.md)
- **ユーザストーリー**: [📖 user-stories.md](user-stories.md)
- **受け入れ基準**: [✅ acceptance-criteria.md](acceptance-criteria.md)
- **コンテキストノート**: [📝 note.md](note.md)

## 機能要件（EARS記法）

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 既存実装・ユーザヒアリングを参考にした確実な要件
- 🟡 **黄信号**: 既存実装・アルゴリズム仕様から妥当な推測による要件
- 🔴 **赤信号**: ヒアリングにない推測による要件

---

### 通常要件

- REQ-001: システムは `compute_vikor(values, n_trials, n_objectives, weights, is_minimize, v)` を呼び出して全試行のS値・R値・Q値を計算しなければならない 🔵 *ユーザヒアリング・TOPSIS実装パターンより*

- REQ-002: システムはQ値に基づいて試行を昇順にランキングしなければならない（Q低い = 良い） 🔵 *VIKORアルゴリズム仕様より*

- REQ-003: システムはUIのバーチャート表示のために `primary_scores() = 1.0 - q_values` を提供しなければならない（高い = 良い の統一インターフェース） 🔵 *ユーザヒアリング・既存McdmResult.primary_scores()パターンより*

- REQ-004: システムは `McdmMethod` enum に `Vikor` バリアントを追加しなければならない 🔵 *既存architecture・ユーザヒアリングより*

- REQ-005: システムは `McdmResult` enum に `Vikor(VikorResult)` バリアントを追加しなければならない 🔵 *既存architecture・ユーザヒアリングより*

- REQ-006: システムはUIの手法選択コンボボックスに "VIKOR" を追加しなければならない 🔵 *既存McdmRankChart実装パターンより*

- REQ-007: システムはVIKORを選択した場合に v パラメータのスライダーを表示しなければならない 🔵 *ユーザヒアリングより*

### 条件付き要件

- REQ-101: NaN値を含む試行がある場合、システムはその試行をQ値計算から除外し、当該試行のQ=1.0（最悪値）・S=0.0・R=0.0を設定し ranked_indices 末尾に配置しなければならない 🔵 *ユーザヒアリング（TOPSIS同方針）より*

- REQ-102: `f*_j == f-_j`（基準jの全値が同一）の場合、システムは当該基準の寄与分を0とし、ゼロ除算を回避しなければならない 🔵 *VIKORアルゴリズム仕様・TOPSIS実装パターンより*

- REQ-103: `S- == S*`（全試行のS値が同一）の場合、システムはQ計算の第1項を0としなければならない 🟡 *VIKORアルゴリズム仕様から妥当な推測*

- REQ-104: `R- == R*`（全試行のR値が同一）の場合、システムはQ計算の第2項を0としなければならない 🟡 *VIKORアルゴリズム仕様から妥当な推測*

- REQ-105: 全有効試行が同一目的関数値の場合、システムはすべての試行のQ=0.0を設定しなければならない 🟡 *TOPSIS同様のエッジケース処理から妥当な推測*

### 状態要件

- REQ-201: VIKORを選択している場合、システムは v パラメータ（0.0〜1.0、デフォルト 0.5）をMcdmRankChartのUI状態に保持しなければならない 🔵 *ユーザヒアリングより*

- REQ-202: 計算完了後、システムは `McdmResult::Vikor(VikorResult)` を app_state.mcdm_result に格納しなければならない 🔵 *既存AppMessage::McdmDone・message_handler実装パターンより*

### オプション要件

- REQ-301: システムはVikorResultのs_values・r_values・q_valuesを将来のテーブル拡張表示のために格納してもよい 🔵 *ユーザヒアリング（S/R/Q全出力）より*

### 制約要件

- REQ-401: システムは外部線形代数ライブラリ（nalgebra等）を使用せず手動実装しなければならない 🔵 *既存コードベース制約・TOPSIS実装パターンより*

- REQ-402: 計算関数は `Result<VikorResult, String>` を返しなければならない 🔵 *既存Rustエラーハンドリングパターンより*

- REQ-403: VikorResult は `#[derive(Debug, Clone, serde::Serialize)]` を付与しなければならない 🔵 *既存TopsisResult実装パターンより*

- REQ-404: UIコンポーネントはインラインスタイルまたはCSS変数のみ使用しなければならない（Tailwind禁止） 🔵 *既存アーキテクチャ制約より*

---

## 非機能要件

### パフォーマンス

- NFR-001: 50,000試行 × 4目的で **100ms 以内** に計算を完了しなければならない 🔵 *TOPSIS性能要件と同基準・既存タスク定義より*

### セキュリティ・堅牢性

- NFR-101: ゼロ除算が発生する可能性のある箇所はすべてガードしクラッシュしてはならない 🔵 *TOPSIS実装パターン・既存エラーハンドリング方針より*

- NFR-102: 入力検証エラー（n_trials=0, values長不一致等）は `Err(String)` を返さなければならない 🔵 *TOPSIS実装パターンより*

### ユーザビリティ

- NFR-201: v パラメータスライダーはデフォルト v=0.5 で表示し、変更してもRunボタンを押すまで再計算しなくてよい 🔵 *ユーザヒアリング・既存重みスライダーパターンより*

---

## Edgeケース

### エラー処理

- EDGE-001: `n_trials == 0` の場合 `Err("n_trials must be >= 1")` を返す 🔵 *TOPSIS実装パターンより*
- EDGE-002: `n_objectives == 0` の場合 `Err("n_objectives must be >= 1")` を返す 🔵 *TOPSIS実装パターンより*
- EDGE-003: `values.len() != n_trials * n_objectives` の場合 `Err("values length mismatch: ...")` を返す 🔵 *TOPSIS実装パターンより*
- EDGE-004: `weights.len() != n_objectives` の場合 `Err("weights length mismatch: ...")` を返す 🔵 *TOPSIS実装パターンより*
- EDGE-005: `is_minimize.len() != n_objectives` の場合 `Err("is_minimize length mismatch: ...")` を返す 🔵 *TOPSIS実装パターンより*

### 境界値

- EDGE-101: n_trials=1 の場合、S=R=Q=0.0、score=1.0 🟡 *VIKORアルゴリズムの自然な帰結から推測*
- EDGE-102: n_objectives=1 の場合、正常計算（1基準のVIKOR） 🟡 *TOPSIS同様のケース処理から推測*
- EDGE-103: v=0.0 の場合、Q = R方向のみ（最小遺憾重視） 🔵 *VIKORアルゴリズム仕様より*
- EDGE-104: v=1.0 の場合、Q = S方向のみ（最大多数合意重視） 🔵 *VIKORアルゴリズム仕様より*
- EDGE-105: 全試行がNaNの場合、全Q=1.0、scores=全0.0 🟡 *TOPSIS同様のNaN全除外パターンから推測*

---

## 変更ファイル一覧

| ファイル | 変更種別 | 内容 |
|---------|---------|------|
| `rust_core/src/mcdm/vikor.rs` | 新規作成 | VIKORアルゴリズム実装 |
| `rust_core/src/mcdm/mod.rs` | 修正 | `pub mod vikor;` 追加 |
| `egui-app/src/state/results.rs` | 修正 | `VikorResult` 型・`McdmMethod::Vikor`・`McdmResult::Vikor` 追加 |
| `egui-app/src/ui/chart_registry.rs` | 修正 | `McdmMethod::Vikor` dispatch 追加 |
| `egui-app/src/ui/widgets/mcdm_chart.rs` | 修正 | v スライダーUI追加・`pending_compute` に v 追加 |

---

## 品質評価

| 観点 | 状態 |
|-----|------|
| 要件の曖昧さ | ✅ なし（アルゴリズム手順・エッジケースを明確化） |
| 入出力定義の完全性 | ✅ 完全（型・制約・範囲すべて定義） |
| 制約条件の明確性 | ✅ 明確（パフォーマンス・NaN・ゼロ除算すべて記載） |
| 実装可能性 | ✅ 確実（TOPSIS実装パターンがテンプレートとして利用可能） |
| 信頼性レベル | 🔵 青信号: 27/32 (84%) / 🟡 黄信号: 5/32 (16%) / 🔴 赤信号: 0/32 (0%) |

**品質評価**: ✅ 高品質
