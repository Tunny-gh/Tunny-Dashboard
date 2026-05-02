# Permutation Feature Importance 要件定義書

## 概要

Importance Chart に新しい指標 **Permutation Feature Importance (PFI)** を追加する。
LightGBM Random Forest で学習したモデルを用いて、各パラメータを n_repeats=5 回シャッフルし、
MSE増加量の平均値を重要度スコアとして返す。

既存の RF-Anova（1回シャッフル）に対して、繰り返しシャッフルによって推定の分散を低減した
統計的に安定した実装を提供する。

## 関連文書

- **ヒアリング記録**: [💬 interview-record.md](interview-record.md)
- **ユーザストーリー**: [📖 user-stories.md](user-stories.md)
- **受け入れ基準**: [✅ acceptance-criteria.md](acceptance-criteria.md)
- **コンテキストノート**: [📝 note.md](note.md)

## 機能要件（EARS記法）

**【信頼性レベル凡例】**:
- 🔵 **青信号**: PRD・EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な要件
- 🟡 **黄信号**: PRD・EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による要件
- 🔴 **赤信号**: PRD・EARS要件定義書・設計文書・ユーザヒアリングにない推測による要件

---

### REQ-PFI-001: rust_core — SensitivityMetric への追加

- REQ-PFI-001-A: `rust_core/src/sensitivity/types.rs` の `SensitivityMetric` 列挙体に `Permutation` バリアントを追加しなければならない 🔵 *既存 SensitivityMetric パターンおよびユーザヒアリングより*
- REQ-PFI-001-B: `rust_core/src/sensitivity/types.rs` に `PermutationResult` 構造体を追加しなければならない。構造体は以下のフィールドを持つ:
  ```rust
  pub struct PermutationResult {
      pub importances: Vec<Vec<f64>>, // [param][objective]
      pub r_squared: Vec<f64>,        // [objective]
  }
  ```
  🔵 *既存 RfAnovaResult / MdiResult / ShapResult と同一パターン*
- REQ-PFI-001-C: `rust_core/src/sensitivity/types.rs` の `SensitivityResult` 構造体に `pub permutation: Option<PermutationResult>` フィールドを追加しなければならない 🔵 *既存フィールドパターンより*

---

### REQ-PFI-002: rust_core — permutation.rs 実装

- REQ-PFI-002-A: `rust_core/src/sensitivity/permutation.rs` ファイルを新規作成しなければならない 🔵 *ユーザヒアリングより*
- REQ-PFI-002-B: `compute_permutation_importances(x_matrix: &[Vec<f64>], y: &[f64]) -> (Vec<f64>, f64)` 関数を実装しなければならない 🔵 *既存 rf_anova.rs / shap.rs のシグネチャパターンより*
- REQ-PFI-002-C: NaN/Inf を含む行を計算前にフィルタリングしなければならない。有効行が 2 未満の場合は `(vec![0.0; p], 0.0)` を返さなければならない 🔵 *既存すべての LightGBM メトリクスの共通パターンより*
- REQ-PFI-002-D: 最大行数を 2,000 行（`PFI_MAX_ROWS`）として、超過する場合はシード=42 の LCG で決定論的にダウンサンプリングしなければならない 🔵 *既存 rf_anova.rs の max_rows=2,000 パターンより*
- REQ-PFI-002-E: 80/20 holdout 分割を実施しなければならない（最低 train=2, eval=2 を保証） 🔵 *既存 rf_anova.rs / shap.rs パターンより*
- REQ-PFI-002-F: 以下の設定で LightGBM RF を学習しなければならない:
  - `num_iterations: 100`
  - `max_depth: 10`
  - `min_data_in_leaf: 2`
  - `seed: 42`
  🔵 *ユーザヒアリング + 既存 rf_anova.rs パターンより*
- REQ-PFI-002-G: 学習後、eval セットで `baseline_mse` を計算しなければならない（`lgbm_mse` 使用） 🔵 *rf_anova.rs パターンより*
- REQ-PFI-002-H: 各特徴量について `n_repeats=5` 回シャッフルを繰り返し、各繰り返しで `permuted_mse` を計算し、`importance = mean(max(permuted_mse - baseline_mse, 0.0) over 5 repeats)` を算出しなければならない 🔵 *ユーザヒアリング（n_repeats=5、平均MSE増加量）より*
- REQ-PFI-002-I: 各繰り返しのシードは `seed_base + feature_idx * n_repeats + repeat_idx` で決定論的に決定しなければならない 🟡 *決定論的再現性要件から妥当な推測*
- REQ-PFI-002-J: 計算後、全特徴量の importance を合計で正規化（sum = 1.0）しなければならない。合計が epsilon 以下の場合はすべて 0.0 を返さなければならない 🔵 *既存 rf_anova.rs パターンより*
- REQ-PFI-002-K: R² = `mse_to_r_squared(baseline_mse, y_eval)` で計算し返さなければならない 🔵 *既存 LightGBM メトリクス共通パターンより*

---

### REQ-PFI-003: rust_core — mod.rs / full.rs への統合

- REQ-PFI-003-A: `rust_core/src/sensitivity/mod.rs` に `mod permutation;` と `pub use permutation::compute_permutation_importances;` を追加しなければならない 🔵 *既存 mod.rs パターンより*
- REQ-PFI-003-B: `rust_core/src/sensitivity/analysis/full.rs` の `compute_sensitivity_single_obj` 関数に `SensitivityMetric::Permutation` のケースを追加しなければならない 🔵 *既存 RfAnova/Shap ディスパッチパターンより*
- REQ-PFI-003-C: Permutation ケースは x_matrix を構築し、`compute_permutation_importances` を呼び出し、結果を `SensitivityResult.permutation` にセットしなければならない 🔵 *既存 rf_anova ケースと同一パターン*

---

### REQ-PFI-004: egui-app — results.rs への追加

- REQ-PFI-004-A: `egui-app/src/state/results.rs` に `PermutationResult` 構造体を追加しなければならない（`RfAnovaResult` と同一フィールド構成） 🔵 *既存 results.rs パターンより*
- REQ-PFI-004-B: `egui-app/src/state/results.rs` の `SensitivityResult` 構造体に `pub permutation: Option<PermutationResult>` フィールドを追加しなければならない 🔵 *既存パターンより*

---

### REQ-PFI-005: egui-app — importance_chart.rs への追加

- REQ-PFI-005-A: `egui-app/src/ui/widgets/importance_chart.rs` の `ImportanceMetric` 列挙体に `Permutation` バリアントを追加しなければならない 🔵 *ユーザヒアリングより*
- REQ-PFI-005-B: `ImportanceMetric::Permutation` の `label()` は `"Permutation"` を返さなければならない 🔵 *ユーザヒアリングより*
- REQ-PFI-005-C: `ImportanceMetric::Permutation` の `cache_id()` は `7` を返さなければならない 🔵 *既存 cache_id 連番パターンより*
- REQ-PFI-005-D: `ImportanceMetric::Permutation` は `is_sobol()` において `false` を返さなければならない 🔵 *Sobol以外は全て false のパターンより*
- REQ-PFI-005-E: コンボボックスの "── Tree-based ──" グループに `Permutation` を追加しなければならない（Spearman/Ridge/RfAnova/Mdi/Shap の既存順序を維持） 🔵 *ユーザヒアリング + 既存 UI グループパターンより*
- REQ-PFI-005-F: `compute_sorted_importance` 関数に `ImportanceMetric::Permutation` のケースを追加しなければならない（`result.permutation` から importances を取得） 🔵 *既存 rf_anova/mdi ケースと同一パターン*
- REQ-PFI-005-G: R² 表示の match 文に `ImportanceMetric::Permutation` のケースを追加し、`result.permutation.as_ref()?.r_squared.get(obj_idx)` から取得しなければならない 🔵 *既存パターンより*

---

### REQ-PFI-006: egui-app — chart_registry.rs への統合

- REQ-PFI-006-A: `egui-app/src/ui/chart_registry.rs` の `ImportanceMetric::Permutation` を `_`（unreachable）ではなく明示的なケースとして処理しなければならない 🔵 *既存ディスパッチパターンより*
- REQ-PFI-006-B: Permutation ケースは `tunny_core::sensitivity::SensitivityMetric::Permutation` にマッピングし、`compute_sensitivity_single_obj` を経由して計算しなければならない 🔵 *既存 RfAnova/Mdi/Shap と同一ディスパッチパターンより*
- REQ-PFI-006-C: `AppMessage::SensitivityDone` の result 変換部分で `permutation` フィールドを正しく変換しなければならない 🔵 *既存 rf_anova/mdi/shap の変換パターンより*

---

### REQ-PFI-007: テスト

- REQ-PFI-007-A: `rust_core/src/sensitivity/tests.rs` に `compute_permutation_importances` のユニットテストを追加しなければならない 🔵 *既存テストパターンより*
- REQ-PFI-007-B: テストは以下のケースをカバーしなければならない:
  - 最小ケース（n=2, p=1）: 空でない结果が返ること
  - 通常ケース（n=50, p=5）: importances の sum ≈ 1.0
  - NaN 混入ケース: フィルタリング後に正常に計算完了
  - p=1（単一特徴量）: importances = [1.0]
  🔵 *既存 rf_anova / shap テストパターンより*

---

## 非機能要件

### パフォーマンス

- NFR-PFI-001: `compute_permutation_importances` は 2,000 trials × 20 変数 × 1 目的で 5,000 ms 以内に完了しなければならない（n_repeats=5 分の追加コストを考慮） 🟡 *RF-Anova の既存パフォーマンステスト（TC-801系）からn_repeats=5倍を考慮した妥当な推測*
- NFR-PFI-002: 計算はバックグラウンドスレッドで行われるため、UI スレッドをブロックしてはならない 🔵 *既存 spawn_task パターンより*

### キャッシュ

- NFR-PFI-003: 同じ (metric=Permutation, obj_idx) の組み合わせに対して既にキャッシュが存在する場合は再計算を行わなければならない 🔵 *既存 importance_cache パターンより*
- NFR-PFI-004: Study 変更時に importance_cache がクリアされることにより、Permutation の結果も自動的に破棄される（既存動作） 🔵 *app_state.clear() 既存実装より*

### 決定論性

- NFR-PFI-005: 同じデータセット・同じ objective に対して常に同一の結果を返さなければならない（乱数シードを固定） 🔵 *既存 LightGBM メトリクス共通要件より*

## Edge ケース

### 入力データ

- EDGE-PFI-001: 有効行が 2 未満の場合、`(vec![0.0; p], 0.0)` を返さなければならない（クラッシュしない） 🔵 *既存 rf_anova.rs パターンより*
- EDGE-PFI-002: p=0（パラメータが0個）の場合、`(vec![], 0.0)` を返さなければならない 🔵 *既存パターンより*
- EDGE-PFI-003: すべての importance が 0（baseline_mse ≈ permuted_mse）の場合、正規化後もすべて 0.0 を返さなければならない 🔵 *正規化関数の既存動作より*
- EDGE-PFI-004: n=1 行の場合（80/20 分割不可）、全データを train と eval の両方に使用してフォールバックしなければならない 🔵 *既存 shap.rs / rf_anova.rs の holdout フォールバックパターンより*

### UI 側

- EDGE-PFI-010: `sensitivityResult.permutation` が `None` の場合（未計算時）、バーチャートには何も表示せず "No sensitivity data" を表示しなければならない 🔵 *既存 ImportanceChart.show() パターンより*
- EDGE-PFI-011: アクティブ Study が未選択の状態で "Run" ボタンを押した場合、`current_study.is_none()` チェックにより no-op になる（既存動作） 🔵 *chart_registry.rs の既存ガード条件より*
