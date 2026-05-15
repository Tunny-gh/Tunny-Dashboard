# TASK-2259: TDD開発コンテキストノート

**作成日**: 2026-05-15
**タスクID**: TASK-2259
**機能名**: rust-core-refactoring
**要件名**: rust-core-refactoring
**タスクタイプ**: TDD

---

## 1. 技術スタック

### 使用技術・フレームワーク

- **言語**: Rust 2021 edition
- **クレート名**: tunny-core (rust_core)
- **主要依存**:
  - `faer 0.24` - 行列演算ライブラリ（Ridge 回帰で使用）
  - `serde 1` / `serde_json 1` - シリアライゼーション
  - `rayon 1` - 並列処理
- **開発ツール**:
  - `cargo` - Rust ビルドシステム
  - `criterion 0.5` - ベンチマークツール
- **参照元**: rust_core/Cargo.toml

### アーキテクチャパターン

- **パターン**: トレイトオブジェクトによるポリモーフィズム
- **設計方針**: 既存関数をラップし、トレイトインターフェースを提供
- **参照元**: docs/design/rust-core-refactoring/architecture.md

---

## 2. 開発ルール

### プロジェクト固有のルール

1. **後方互換性維持**
   - 既存の計算関数（`compute_spearman`, `compute_ridge`）はそのまま維持
   - 新しい構造体（`SpearmanMetric`, `RidgeMetric`）は既存関数をラップするのみ
   - 内部ロジックの変更は行わない

2. **エラーハンドリング**
   - `SensitivityMetric::compute()` は失敗時に `None` を返す
   - データ不足や計算失敗時にパニックしない設計を保証
   - 参照元: docs/spec/rust-core-refactoring/requirements.md (REQ-A01, REQ-A02)

3. **数値計算精度**
   - 既存関数との結果一致要件: 浮動小数点許容誤差 `1e-10` 以内
   - 参照元: docs/spec/rust-core-refactoring/requirements.md (NFR-102)

### コーディング規約

- **トレイト実装**: `Send + Sync` スーパートレイトを必須とする
- **命名規則**:
  - 構造体: `SpearmanMetric`, `RidgeMetric`（Suffix: `Metric`）
  - メソッド: `compute()`, `name()`
- **ドキュメントコメント**: `///` 形式で記述
- **参照元**: docs/design/rust-core-refactoring/interfaces.rs

### テスト規約

- **テスト命名**: `tc_2259_*` プレフィックスを使用
- **アサーション**: 数値比較には許容誤差 `1e-10` を使用
- **テスト構造**: Given-When-Then パターン
- **参照元**: rust_core/src/sensitivity/tests.rs

---

## 3. 関連実装

### 類似機能の実装例

#### 既存の SensitivityMetric トレイト定義

**ファイル**: rust_core/src/sensitivity/metric_trait.rs

```rust
pub trait SensitivityMetric: Send + Sync {
    /// Compute sensitivity for a single objective identified by `obj_idx`.
    ///
    /// Returns `None` when the computation cannot be performed (e.g.
    /// insufficient data), never panics.
    fn compute(&self, df: &DataFrame, obj_idx: usize) -> Option<SensitivityResult>;

    /// Human-readable identifier for the metric (used in logging / debugging).
    fn name(&self) -> &'static str;
}
```

#### 既存の Spearman 計算関数

**ファイル**: rust_core/src/sensitivity/spearman.rs

```rust
pub fn compute_spearman(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    if n < 2 {
        return 0.0;
    }

    let rx = rank(&x[..n]);
    let ry = rank(&y[..n]);

    pearson_correlation(&rx, &ry)
}
```

#### 既存の Ridge 計算関数

**ファイル**: rust_core/src/sensitivity/ridge.rs

```rust
pub fn compute_ridge(x_matrix: &[Vec<f64>], y: &[f64], alpha: f64) -> RidgeResult {
    let n = y.len();
    let empty = RidgeResult {
        beta: vec![],
        r_squared: 0.0,
    };

    if n < 2 || x_matrix.len() != n {
        return empty;
    }
    let p = x_matrix[0].len();
    if p == 0 {
        return empty;
    }

    let x_cols = transpose_and_standardize(x_matrix, n, p);
    compute_ridge_from_standardized_columns(&x_cols, n, y, alpha)
}
```

#### 既存のテストパターン

**ファイル**: rust_core/src/sensitivity/tests.rs

```rust
#[test]
fn tc_801_01_spearman_perfect_positive() {
    let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];

    let r = compute_spearman(&x, &y);

    assert!(
        (r - 1.0).abs() < 1e-9,
        "Spearman should be 1.0: {}",
        r
    );
}

#[test]
fn tc_801_06_ridge_perfect_linear_r_squared_near_1() {
    let n = 50;
    let x_matrix: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64]).collect();
    let y: Vec<f64> = (0..n).map(|i| 2.0 * i as f64 + 1.0).collect();

    let result = compute_ridge(&x_matrix, &y, 0.001);

    assert!(
        result.r_squared > 0.99,
        "R² should be close to 1.0: {}",
        result.r_squared
    );
}
```

### 参照元: rust_core/src/sensitivity/spearman.rs, rust_core/src/sensitivity/ridge.rs, rust_core/src/sensitivity/tests.rs

---

## 4. 設計文書

### アーキテクチャ・API仕様

#### SensitivityMetric トレイトの実装計画

**参照元**: docs/design/rust-core-refactoring/architecture.md

| 構造体 | ファイル | 役割 |
|--------|----------|------|
| `SpearmanMetric` | `sensitivity/spearman.rs` | 既存 `compute_spearman` をラップ |
| `RidgeMetric` | `sensitivity/ridge.rs` | 既存 `compute_ridge` をラップ |

#### 実装要件

**SpearmanMetric 構造体**:
```rust
pub struct SpearmanMetric;

impl SensitivityMetric for SpearmanMetric {
    fn compute(&self, df: &DataFrame, obj_idx: usize) -> Option<SensitivityResult> {
        // 既存の spearman 計算関数をラップ
        // データ不足・エラー時は None を返す
    }

    fn name(&self) -> &'static str {
        "Spearman"
    }
}
```

**RidgeMetric 構造体**:
```rust
pub struct RidgeMetric;

impl SensitivityMetric for RidgeMetric {
    fn compute(&self, df: &DataFrame, obj_idx: usize) -> Option<SensitivityResult> {
        // 既存の ridge 計算関数をラップ
        // データ不足・エラー時は None を返す
    }

    fn name(&self) -> &'static str {
        "Ridge"
    }
}
```

### データモデル

#### SensitivityResult 型（既存）

**ファイル**: rust_core/src/sensitivity/types.rs

```rust
#[derive(Debug, Clone)]
pub struct SensitivityResult {
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub spearman: Vec<Vec<f64>>,
    pub ridge: Vec<RidgeResult>,
    pub rf_anova: Option<RfAnovaResult>,
    pub mdi: Option<MdiResult>,
    pub shap: Option<ShapResult>,
    pub permutation: Option<PermutationResult>,
}

#[derive(Debug, Clone)]
pub struct RidgeResult {
    pub beta: Vec<f64>,
    pub r_squared: f64,
}
```

#### DataFrame 型（既存）

**ファイル**: rust_core/src/data/dataframe.rs

```rust
pub struct DataFrame {
    // パラメータ・目的関数データを保持する構造体
    // 詳細は rust_core/src/data/dataframe/model.rs
}
```

### 参照元:
- docs/design/rust-core-refactoring/architecture.md
- docs/design/rust-core-refactoring/interfaces.rs
- rust_core/src/sensitivity/types.rs
- rust_core/src/data/dataframe.rs

---

## 5. テスト関連情報

### テストフレームワーク・設定ファイル

#### テスト設定

**ファイル**: rust_core/Cargo.toml

```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "sampling_bench"
harness = false

[[bench]]
name = "sensitivity_bench"
harness = false
```

#### テスト実行コマンド

```bash
# すべてのテストを実行
cargo test -p tunny-core

# 特定のテストモジュールを実行
cargo test -p tunny-core -- sensitivity::tests

# 特定のテストケースを実行
cargo test -p tunny-core -- tc_801_01_spearman_perfect_positive
```

### 既存テストのディレクトリ構成・命名パターン

#### ディレクトリ構成

```
rust_core/src/
├── sensitivity/
│   ├── tests.rs              # 感度分析の単体テスト
│   ├── spearman.rs           # Spearman 実装 + テスト
│   └── ridge.rs              # Ridge 実装 + テスト
└── data/
    └── dataframe/
        └── tests.rs          # DataFrame テスト
```

#### テスト命名パターン

- **機能テスト**: `tc_801_*` (既存の感度分析テスト)
- **パフォーマンステスト**: `tc_801_p*` (パフォーマンス要件)
- **統合テスト**: `tc_*_int_*` (統合テスト)
- **TASK-2259 用**: `tc_2259_*` プレフィックスを使用

### テストユーティリティ・モック設定

#### テストユーティリティ関数

**ファイル**: rust_core/src/sensitivity/tests.rs

```rust
fn make_row_multi(trial_id: u32, params: &[(&str, f64)], objectives: Vec<f64>) -> TrialRow {
    TrialRow {
        trial_id,
        param_display: params.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        param_category_label: HashMap::new(),
        objective_values: objectives,
        user_attrs_numeric: HashMap::new(),
        user_attrs_string: HashMap::new(),
        constraint_values: vec![],
    }
}

fn setup_df(rows: Vec<TrialRow>, params: &[&str], objs: &[&str]) -> DataFrame {
    let param_names: Vec<String> = params.iter().map(|s| s.to_string()).collect();
    let obj_names: Vec<String> = objs.iter().map(|s| s.to_string()).collect();
    let df = DataFrame::from_trials(&rows, &param_names, &obj_names, &[], &[], 0);
    store_dataframes(vec![df.clone()]);
    select_study(0).expect("study 0 exists");
    df
}
```

### E2Eテスト設定

該当なし（本タスクは core ライブラリの単体テストのみ）

### 参照元:
- rust_core/Cargo.toml
- rust_core/src/sensitivity/tests.rs
- rust_core/CLAUDE.md

---

## 6. 注意事項

### 技術的制約

1. **WASM 互換性不要**
   - WASM ビルド対応は不要
   - ネイティブ API を自由に使用してよい
   - 参照元: docs/spec/rust-core-refactoring/note.md

2. **既存関数のラップのみ**
   - `compute_spearman` と `compute_ridge` の内部ロジックは変更しない
   - 既存のテストを壊さないように注意
   - 参照元: docs/tasks/rust-core-refactoring/TASK-2259.md

3. **後方互換性**
   - 既存の `compute_sensitivity_all` 関数は変更なし
   - 新しいトレイト実装は追加のAPIとして提供
   - 参照元: docs/design/rust-core-refactoring/architecture.md

### セキュリティ・パフォーマンス要件

1. **パフォーマンス要件**
   - 既存のベンチマーク（`sensitivity_bench`）は同等以上のスコアを維持
   - 参照元: docs/spec/rust-core-refactoring/requirements.md (NFR-001)

2. **正確性要件**
   - 全ての既存テスト（`cargo test -p tunny-core`）はパス
   - 数値計算結果は浮動小数点許容誤差 `1e-10` 以内で一致
   - 参照元: docs/spec/rust-core-refactoring/requirements.md (NFR-101, NFR-102)

3. **保守性要件**
   - 各関数は 50 行以内とする（今回のラッパーは当然満たす）
   - ドキュメントコメントを記述する
   - 参照元: docs/spec/rust-core-refactoring/requirements.md (NFR-201, NFR-202)

### 参照元:
- docs/spec/rust-core-refactoring/requirements.md
- docs/spec/rust-core-refactoring/note.md
- docs/tasks/rust-core-refactoring/TASK-2259.md

---

## 7. 完了条件チェックリスト

- [ ] `rust_core/src/sensitivity/spearman.rs` に `SpearmanMetric` 構造体が追加されている
- [ ] `SpearmanMetric` が `SensitivityMetric` トレイトを実装している
- [ ] `rust_core/src/sensitivity/ridge.rs` に `RidgeMetric` 構造体が追加されている
- [ ] `RidgeMetric` が `SensitivityMetric` トレイトを実装している
- [ ] `SpearmanMetric::compute()` が既存の spearman 計算と同一結果を返す（差 < 1e-10）
- [ ] `RidgeMetric::compute()` が既存の ridge 計算と同一結果を返す（差 < 1e-10）
- [ ] データ不足時に `None` を返しパニックしないことが確認されている
- [ ] `cargo test -p tunny-core` が全て通る

**参照元**: docs/tasks/rust-core-refactoring/TASK-2259.md

---

## 8. 依存関係

### 前提タスク

- **TASK-2258**: SensitivityMetric トレイト定義と SensitivityKind リネーム ✅ 完了
  - 参照元: docs/tasks/rust-core-refactoring/TASK-2258.md
  - 検証レポート: docs/implements/rust-core-refactoring/TASK-2258/verify-report.md

### 後続タスク

- **TASK-2260**: RfAnovaMetric・MdiMetric・ShapMetric・PermutationMetric の SensitivityMetric 実装
  - 参照元: docs/tasks/rust-core-refactoring/TASK-2260.md
- **TASK-2263**: compute_sensitivity_single_obj の簡略化（TASK-2259 と TASK-2260 が完了後に実行）

### 参照元:
- docs/tasks/rust-core-refactoring/overview.md
- docs/tasks/rust-core-refactoring/TASK-2259.md