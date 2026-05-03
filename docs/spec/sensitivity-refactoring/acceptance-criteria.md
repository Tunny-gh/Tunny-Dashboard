# sensitivity-refactoring 受け入れ基準

**作成日**: 2026-05-04
**関連要件定義**: [requirements.md](requirements.md)
**関連ユーザストーリー**: [user-stories.md](user-stories.md)
**ヒアリング記録**: [interview-record.md](interview-record.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: コード分析・ユーザヒアリングを参考にした確実な基準
- 🟡 **黄信号**: コード分析・ユーザヒアリングから妥当な推測による基準
- 🔴 **赤信号**: コード分析・ユーザヒアリングにない推測による基準

---

## REQ-001 / REQ-002: core::math::stats の新規作成と標準化統一 🔵

**信頼性**: 🔵 *コード分析・ユーザヒアリングより*

### Given（前提条件）
- `rust_core/src/core/math/` ディレクトリが存在する
- `sensitivity/ridge.rs`、`sensitivity/analysis/common.rs`、`sensitivity/sobol.rs`、`pdp/utils.rs` それぞれに個別の平均・標準偏差計算ロジックが存在する

### When（実行条件）
- `core/math/stats.rs` を新規作成し、`column_mean_std` を実装する
- 上記4ファイルのローカル実装を削除して `core::math::stats::column_mean_std` に置き換える

### Then（期待結果）
- `cargo build --release` がエラーなく完了する
- `pdp/utils.rs` に `col_mean_std` のローカル定義が残っていない
- `sensitivity/sobol.rs` に `column_mean_std` のローカル定義が残っていない
- `sensitivity/ridge.rs` に `transpose_and_standardize` 内のインライン計算が残っていない
- `sensitivity/analysis/common.rs` の `build_standardized_param_columns` が共通関数を使用している

### テストケース

#### 正常系

- [ ] **TC-REF-001-01**: 通常データで平均・標準偏差を正しく計算する 🔵
  - **入力**: `[1.0, 2.0, 3.0, 4.0, 5.0]`
  - **期待結果**: mean = 3.0、std_dev = sqrt(2.0) ≈ 1.414
  - **信頼性**: 🔵 *既存実装の動作より*

- [ ] **TC-REF-001-02**: 全要素が同値の場合、std_dev に 1.0 を返す 🔵
  - **入力**: `[5.0, 5.0, 5.0]`
  - **期待結果**: `(5.0, 1.0)`（ゼロ除算防止）
  - **信頼性**: 🔵 *既存実装の共通動作より*

#### 異常系

- [ ] **TC-REF-001-E01**: 空スライスで `(0.0, 1.0)` を返す 🔵
  - **入力**: `[]`
  - **期待結果**: `(0.0, 1.0)`
  - **信頼性**: 🔵 *pdp/utils.rs の既存動作より*

- [ ] **TC-REF-001-E02**: 要素が1つの場合、std_dev に 1.0 を返す 🟡
  - **入力**: `[3.0]`
  - **期待結果**: `(3.0, 1.0)`
  - **信頼性**: 🟡 *ゼロ除算防止の挙動から推測*

#### 回帰テスト

- [ ] **TC-REF-001-R01**: 既存の Spearman テスト（TC-801-01〜05）が全てパス 🔵
- [ ] **TC-REF-001-R02**: 既存の Ridge テスト（TC-801-06〜09）が全てパス 🔵
- [ ] **TC-REF-001-R03**: 既存の統合テスト（TC-801-10〜14）が全てパス 🔵
- [ ] **TC-REF-001-R04**: 既存の Sobol テスト（tc_1610_*）が全てパス 🔵

---

## REQ-003: Newtypeパターンへの移行 🔵

**信頼性**: 🔵 *ユーザヒアリングより*

### Given（前提条件）
- `types.rs` に `type RfAnovaResult = TreeImportanceResult;` 等の型エイリアスが存在する

### When（実行条件）
- 型エイリアスを `pub struct RfAnovaResult(pub TreeImportanceResult);` 形式に変更する
- 各メトリクスの戻り値でNewtypeでラップする

### Then（期待結果）
- `cargo build --release` がエラーなく完了する
- `RfAnovaResult` を期待する引数に `MdiResult` を渡した場合、コンパイルエラーが発生する
- `pub use types::{RfAnovaResult, MdiResult, ShapResult, PermutationResult}` が維持されている
- `sensitivity.rs` を `use` する呼び出し側コードが変更なしでコンパイルできる

### テストケース

#### 正常系

- [ ] **TC-REF-003-01**: 各メトリクスが正しい Newtype で結果を返す 🔵
  - **確認方法**: `compute_mdi_importances` の戻り値の型が `MdiResult` であることをコンパイラが確認
  - **信頼性**: 🔵 *型安全性要件より*

- [ ] **TC-REF-003-02**: `.0` を通じて内部の `TreeImportanceResult` にアクセスできる 🔵
  - **入力**: `let result: MdiResult = compute_mdi_importances(...)?;`
  - **期待結果**: `result.0.importances` でアクセス可能
  - **信頼性**: 🔵 *Newtypeパターンの標準動作より*

#### 回帰テスト

- [ ] **TC-REF-003-R01**: 既存の全29テストケースがパス 🔵
- [ ] **TC-REF-003-R02**: `SensitivityResult` の各フィールドが正しい値を持つ（TC-801-10） 🔵

---

## REQ-004: TreeMetric トレイトの実装 🔵

**信頼性**: 🔵 *ユーザヒアリングより*

### Given（前提条件）
- MDI/SHAP/RF-ANOVA/PFI が独立した関数として実装されている

### When（実行条件）
- `TreeMetric` トレイトを定義し、各メトリクス型に実装する

### Then（期待結果）
- `analysis/full.rs` がトレイトを通じてメトリクスを呼び出せる
- 新しいメトリクス型を `TreeMetric` を実装するだけで追加できる（`analysis/full.rs` の変更不要）
- 既存のメトリクス関数（`compute_mdi_importances` 等）は維持され、`pub use` でエクスポートされる

### テストケース

#### 正常系

- [ ] **TC-REF-004-01**: `SensitivityMetric::Mdi` 指定で MDI を計算できる 🔵
  - **確認**: `compute_sensitivity_single_obj(df, &SensitivityMetric::Mdi, 0)` が以前と同じ結果を返す
  - **信頼性**: 🔵 *既存テスト TC-801-10 より*

- [ ] **TC-REF-004-02**: トレイトを通じた計算と旧実装の計算結果が一致する 🔵
  - **確認**: 固定シード・固定データで計算値が変化しないこと
  - **信頼性**: 🔵 *回帰テスト要件より*

#### 回帰テスト

- [ ] **TC-REF-004-R01**: PFI テスト（tc_pfi_*）が全てパス 🔵
- [ ] **TC-REF-004-R02**: RF-ANOVA を含む統合テストがパス 🔵

---

## REQ-005: 定数の集約 🔵

**信頼性**: 🔵 *ユーザヒアリングより*

### Given（前提条件）
- `MAX_ROWS` 等の定数が各メトリクスファイルに個別に定義されている

### When（実行条件）
- 全定数を `tree_common.rs` または `constants.rs` に集約する

### Then（期待結果）
- `MDI_MAX_ROWS`・`SHAP_MAX_ROWS`・`RF_ANOVA_MAX_ROWS`・`PFI_MAX_ROWS` が1箇所で定義されている
- 各メトリクスファイルにローカルの `const MAX_ROWS` が残っていない
- 定数ごとに根拠コメントが記載されている

### テストケース

#### 回帰テスト

- [ ] **TC-REF-005-R01**: 大規模データでのパフォーマンステスト（TC-801-P01〜P03）がパス 🔵
  - `MAX_ROWS` 変更なしのため、パフォーマンス特性は変化しない
  - **信頼性**: 🔵 *既存テストより*

---

## REQ-006 / REQ-007: テスト維持・パブリックAPI維持 🔵

**信頼性**: 🔵 *品質要件・WASM API維持要件より*

### テストケース

- [ ] **TC-REF-006-01**: `cargo test` で29件全てパス 🔵
- [ ] **TC-REF-006-02**: release ビルドで `cargo test --release` がパス 🔵
- [ ] **TC-REF-006-03**: パフォーマンステスト（P01-P03）が制約値内 🔵
  - Spearman: 50k×30×4 ≤500ms
  - Ridge: 50k×30×4 ≤300ms
  - Selected: 50k ≤50ms
- [ ] **TC-REF-007-01**: `sensitivity/mod.rs` の `pub use` シンボル一覧が変更前と一致する 🔵
- [ ] **TC-REF-007-02**: `pdp/mod.rs` の `pub use` シンボル一覧が変更前と一致する 🔵

---

## テストケースサマリー

### カテゴリ別件数

| カテゴリ | 正常系 | 異常系 | 回帰テスト | 合計 |
|---------|--------|--------|-----------|------|
| 標準化統一（REQ-001/002） | 2 | 2 | 4 | 8 |
| Newtype移行（REQ-003） | 2 | 0 | 2 | 4 |
| Trait抽象化（REQ-004） | 2 | 0 | 2 | 4 |
| 定数集約（REQ-005） | 0 | 0 | 1 | 1 |
| テスト・API維持（REQ-006/007） | 0 | 0 | 5 | 5 |
| **合計** | **6** | **2** | **14** | **22** |

### 信頼性レベル分布

- 🔵 青信号: 20件 (91%)
- 🟡 黄信号: 2件 (9%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質

### 優先度別テストケース

- **Must Have**: 22件
- **Should Have**: 0件
- **Could Have**: 0件
