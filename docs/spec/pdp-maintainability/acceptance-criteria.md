# pdp-maintainability 受け入れ基準

**作成日**: 2026-05-04
**関連要件定義**: [requirements.md](requirements.md)
**関連ユーザストーリー**: [user-stories.md](user-stories.md)
**ヒアリング記録**: [interview-record.md](interview-record.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: PRD・EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な基準
- 🟡 **黄信号**: PRD・EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による基準
- 🔴 **赤信号**: PRD・EARS要件定義書・設計文書・ユーザヒアリングにない推測による基準

---

## REQ-101/102/103: 正規化ヘルパー（normalize_x_minmax / normalize_y） 🔵

**信頼性**: 🔵 *コード直接分析・ユーザヒアリングより*

### Given（前提条件）
- `rust_core/src/pdp/utils.rs` が存在する
- テストデータ: `x_matrix = [[0.0, 10.0], [0.5, 20.0], [1.0, 30.0]]`

### When（実行条件）
- `normalize_x_minmax(x_matrix)` を呼び出す

### Then（期待結果）
- `col_stats[0] == (0.0, 1.0)` （min=0, range=1）
- `col_stats[1] == (10.0, 20.0)` （min=10, range=20）
- `x_norm[0] == [0.0, 0.0]`
- `x_norm[1] == [0.5, 0.5]`
- `x_norm[2] == [1.0, 1.0]`

### テストケース

#### 正常系

- [ ] **TC-101-01**: 通常データの正規化 🔵
  - **入力**: `x_matrix = [[0.0, 10.0], [0.5, 20.0], [1.0, 30.0]]`
  - **期待結果**: `x_norm` の各要素が `[0, 1]` に収まる

- [ ] **TC-102-01**: y 正規化の正確性 🔵
  - **入力**: `y = [1.0, 2.0, 3.0]`
  - **期待結果**: `y_mean ≈ 2.0`, `y_std > 0`, `y_norm.iter().sum() ≈ 0.0`

- [ ] **TC-103-01**: 置き換え後の既存テストが全件パス 🔵
  - **入力**: `cargo test` 実行
  - **期待結果**: 全テスト PASS（`tc_803_*` シリーズ含む）

#### 異常系

- [ ] **TC-101-E01**: 定数列（range=0）のクランプ 🔵
  - **入力**: `x_matrix = [[5.0], [5.0], [5.0]]`
  - **期待結果**: `col_stats[0] == (5.0, f64::EPSILON)`、パニックなし

- [ ] **TC-102-E01**: 空スライス 🟡
  - **入力**: `y = []`
  - **期待結果**: `y_mean = 0.0`, `y_std = f64::EPSILON`, パニックなし

#### 境界値

- [ ] **TC-101-B01**: 単一行データ 🟡
  - **入力**: `x_matrix = [[3.0, 7.0]]`
  - **期待結果**: `x_norm[0] = [0.0, 0.0]`（range=0 なので全列クランプ）

---

## REQ-201/202: R² 計算ヘルパー（r_squared） 🔵

**信頼性**: 🔵 *コード直接分析・ユーザヒアリングより*

### Given（前提条件）
- `utils.rs` に `r_squared(y_actual, y_pred)` が実装されている

### When（実行条件）
- 完全一致する予測値を渡す

### Then（期待結果）
- `r_squared(y, y) == 1.0`

### テストケース

#### 正常系

- [ ] **TC-201-01**: 完全予測 🔵
  - **入力**: `y_actual = y_pred = [1.0, 2.0, 3.0]`
  - **期待結果**: `r_squared ≈ 1.0`

- [ ] **TC-201-02**: 定数予測（y_mean を常に返す） 🔵
  - **入力**: `y_actual = [1.0, 2.0, 3.0]`, `y_pred = [2.0, 2.0, 2.0]`
  - **期待結果**: `r_squared ≈ 0.0`

- [ ] **TC-202-01**: 置き換え後の kriging 関数が同じ R² を返す 🔵
  - **入力**: 既存の `tc_803_04_pdp_r_squared_high_for_linear` テストデータ
  - **期待結果**: リファクタリング前後で同一の `r_squared` 値

#### 異常系

- [ ] **TC-201-E01**: 定数 y（ss_tot ≈ 0） 🔵
  - **入力**: `y_actual = [5.0, 5.0, 5.0]`, `y_pred = [5.0, 5.0, 5.0]`
  - **期待結果**: `r_squared == 1.0`（ゼロ除算ガード発動）

---

## REQ-301/302: DataFrame 抽出の共通化（extract_xy） 🔵

**信頼性**: 🔵 *コード直接分析・ユーザヒアリングより*

### Given（前提条件）
- `api.rs` に `extract_xy` ヘルパーが実装されている
- DataFrame に `param_names` と `objective_name` の列が存在する

### When（実行条件）
- `extract_xy(df, param_names, objective_name)` を呼び出す

### Then（期待結果）
- `x_matrix[i][j]` が `df.get_numeric_column(param_names[j]).get(i)` と一致する
- `y[i]` が `df.get_numeric_column(objective_name).get(i)` と一致する

### テストケース

#### 正常系

- [ ] **TC-301-01**: compute_pdp の動作が変わらない 🔵
  - **入力**: リファクタリング前後で同一の DataFrame
  - **期待結果**: 両方の `compute_pdp` が同一の `PdpResult1d` を返す

- [ ] **TC-302-01**: compute_pdp_2d の動作が変わらない 🔵
  - **入力**: リファクタリング前後で同一の DataFrame
  - **期待結果**: 両方の `compute_pdp_2d` が同一の `PdpResult2d` を返す

#### 異常系

- [ ] **TC-301-E01**: 欠損値（NaN 相当）は 0.0 にフォールバック 🔵
  - **入力**: DataFrame の一部列に値がない行
  - **期待結果**: `unwrap_or(0.0)` により 0.0 が補完される

---

## REQ-601: min/max スタイル統一 🔵

**信頼性**: 🔵 *コード直接分析より*

### テストケース

- [ ] **TC-601-01**: `cargo clippy` が警告なしでパス 🔵
  - **入力**: `cargo clippy -- -D warnings`
  - **期待結果**: EXIT CODE 0

- [ ] **TC-601-02**: `tc_803_*` テストが全件パス 🔵
  - **入力**: `cargo test`
  - **期待結果**: 全 PASS

---

## REQ-501/502/503: rayon 並列化 🔵

**信頼性**: 🔵 *ユーザヒアリング・rayon の公開 API より*

### Given（前提条件）
- `Cargo.toml` に `rayon = "1"` が追加されている
- `kriging_core.rs` で `use rayon::prelude::*;` がインポートされている

### When（実行条件）
- `compute_pdp_1d_sparse_kriging_raw` を N=1000 データで呼び出す

### Then（期待結果）
- 結果が並列化前と同等（浮動小数点の加算順序の差は許容）
- 全テストが PASS

### テストケース

#### 正常系

- [ ] **TC-501-01**: Cargo.toml に rayon が追加されている 🔵
  - **入力**: `cat rust_core/Cargo.toml`
  - **期待結果**: `rayon = "1"` が `[dependencies]` セクションに含まれる

- [ ] **TC-502-01**: Sparse Kriging PDP ループが並列化されている 🔵
  - **入力**: `kriging_core.rs` の `compute_pdp_1d_sparse_kriging_raw` のコードレビュー
  - **期待結果**: `grid.par_iter()` または同等の rayon API が使われている

- [ ] **TC-502-02**: 並列化後の結果が正確 🔵
  - **入力**: 既存の `tc_1652_tc_005_02_sparse_kriging_n100_grid_shape` テスト
  - **期待結果**: PASS（グリッド形状が正しい）

- [ ] **TC-503-01**: Standard Kriging の mean ループが並列化されている 🔵
  - **入力**: `kriging_core.rs` の `compute_pdp_1d_kriging_raw` のコードレビュー
  - **期待結果**: `x_norm.par_iter()` または同等の rayon API が使われている

- [ ] **TC-503-02**: 並列化後の Standard Kriging 結果が正確 🔵
  - **入力**: 既存の `tc_1645_01_kriging_raw_grid_shape` テスト
  - **期待結果**: PASS

#### 異常系

- [ ] **TC-502-E01**: N=3（最小データ数）でも正常動作 🟡
  - **入力**: N=3 のデータで Sparse Kriging PDP 呼び出し
  - **期待結果**: None or Some（既存の Guard に準ずる）、パニックなし

#### パフォーマンス

- [ ] **TC-NFR-001-01**: Sparse Kriging が N=1000 で 5s 以内 🟡
  - **測定項目**: `tc_nfr_002_01_sparse_kriging_n5000_under_5s`（#[ignore] 解除後）
  - **目標値**: Release ビルドで 5,000ms 以内
  - **測定条件**: `cargo test --release -- --ignored`
  - **信頼性**: 🟡 *rayon の一般的な性能特性から妥当な推測*

---

## 非機能要件テスト

### NFR-002: 既存テストのリグレッションなし 🔵

**信頼性**: 🔵 *ユーザヒアリングより*

- [ ] **TC-NFR-002-01**: `cargo test` が全件パス
  - **検証内容**: `tc_803_*`、`tc_1645_*`、`tc_1652_*`、`tc_1653_*` が全て PASS

### NFR-003: パフォーマンステスト 🔵

- [ ] **TC-NFR-003-01**: `tc_803_p01_pdp_1d_performance` (20ms)
  - **検証内容**: リファクタリング後も Ridge 1D PDP が 20ms 以内に完了

- [ ] **TC-NFR-003-02**: `tc_803_p02_pdp_2d_performance` (100ms)
  - **検証内容**: リファクタリング後も Ridge 2D PDP が 100ms 以内に完了

---

## テストケースサマリー

### カテゴリ別件数

| カテゴリ | 正常系 | 異常系 | 境界値 | 合計 |
|---------|--------|--------|--------|------|
| REF-1 正規化 | 3 | 2 | 1 | 6 |
| REF-2 R² | 3 | 1 | 0 | 4 |
| REF-3 抽出 | 2 | 1 | 0 | 3 |
| REF-5 スタイル | 2 | 0 | 0 | 2 |
| PERF-1 rayon | 5 | 1 | 1 | 7 |
| 非機能要件 | 3 | 0 | 0 | 3 |
| **合計** | **18** | **5** | **2** | **25** |

### 信頼性レベル分布

- 🔵 青信号: 22件 (88%)
- 🟡 黄信号: 3件 (12%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質

### 優先度別テストケース

- **Must Have**: 22件
- **Should Have**: 3件
- **Could Have**: 0件

---

## テスト実施計画

### Phase 1: リファクタリング検証
- REF-1〜5 のテストケース
- 優先度: Must Have / Should Have
- 実施: `cargo test` で自動検証

### Phase 2: rayon 導入後の検証
- PERF-1 のテストケース
- 優先度: Must Have
- 実施: `cargo test && cargo test --release -- --ignored`
