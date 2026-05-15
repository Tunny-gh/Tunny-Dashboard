# TDD 要件定義書: SpearmanMetric・RidgeMetric の SensitivityMetric トレイト実装

**作成日**: 2026-05-15
**タスクID**: TASK-2259
**機能名**: rust-core-refactoring
**要件名**: rust-core-refactoring
**出力ファイル**: `docs/implements/rust-core-refactoring/TASK-2259/spearman-ridge-metric-impl-requirements.md`

---

## 1. 機能の概要（EARS要件定義書・設計文書ベース）

### ユーザーストーリー

**私は** rust_core の開発者として、
**新しい感度指標を追加したい**、
**そうすることで** `SensitivityMetric` トレイトを実装するだけで、ディスパッチ・結果収集・ベンチマークが自動的に対応される。

### 何をする機能か 🔵

TASK-2258 で定義された `SensitivityMetric` トレイトを、既存の Spearman 感度分析および Ridge 感度分析に対して実装する。具体的には:

- `SpearmanMetric` 構造体を `sensitivity/spearman.rs` に追加し、`SensitivityMetric` トレイトを実装する
- `RidgeMetric` 構造体を `sensitivity/ridge.rs` に追加し、`SensitivityMetric` トレイトを実装する
- いずれも既存の計算関数をラップし、内部ロジックの変更は行わない

**参照したEARS要件**: REQ-A01, REQ-A02
**参照した設計文書**: architecture.md A-1「SensitivityMetric トレイト導入」

### どのような問題を解決するか 🔵

既存の `compute_sensitivity_single_obj`（full.rs）では、Spearman・Ridge の計算が match アーム内にハードコードされている。これにより:

- 新しい指標の追加時にディスパッチ側のコード修正が必要
- 各指標の計算ロジックと結果構築が密結合
- 共通のエラーハンドリングパターンが重複

トレイト実装により、各指標を独立した構造体としてカプセル化し、ディスパッチ側は `Vec<Box<dyn SensitivityMetric>>` のイテレーションで統一的に処理可能になる。

**参照したEARS要件**: REQ-A03
**参照したユーザーストーリー**: ストーリー A-1「木ベース感度指標の共通インターフェース」

### 想定されるユーザー 🔵

rust_core クレートの開発者。具体的には:

- 新しい感度指標を追加する開発者
- `compute_sensitivity_single_obj` のディスパッチロジックを簡略化する開発者
- egui-app から感度分析を呼び出す開発者

**参照したユーザーストーリー**: ストーリー A-1

### システム内での位置づけ 🔵

- **レイヤー**: `rust_core/src/sensitivity/` モジュール
- **位置づけ**: 感度分析の統一インターフェース層
- **依存関係**:
  - 前提: TASK-2258 で定義された `SensitivityMetric` トレイト（`metric_trait.rs`）
  - 後続: TASK-2263（`compute_sensitivity_single_obj` の簡略化）

```
SensitivityMetric トレイト (metric_trait.rs) ← TASK-2258
    ├── SpearmanMetric (spearman.rs)        ← TASK-2259 (本タスク)
    ├── RidgeMetric (ridge.rs)              ← TASK-2259 (本タスク)
    ├── RfAnovaMetric (metrics.rs)          ← TASK-2260
    ├── MdiMetric (metrics.rs)              ← TASK-2260
    ├── ShapMetric (metrics.rs)             ← TASK-2260
    └── PermutationMetric (metrics.rs)      ← TASK-2260
```

**参照した設計文書**: architecture.md「ディレクトリ構造」、interfaces.rs「A-1. SensitivityMetric トレイト」

---

## 2. 入力・出力の仕様（EARS機能要件・TypeScript型定義ベース）

### 入力パラメータ 🔵

#### SpearmanMetric::compute()

| パラメータ | 型 | 制約 | 説明 |
|-----------|-----|------|------|
| `&self` | `&SpearmanMetric` | - | ゼロサイズ構造体の参照 |
| `df` | `&DataFrame` | `row_count() >= 2` 必要 | パラメータ・目的関数データ |
| `obj_idx` | `usize` | `obj_idx < objective_col_names().len()` | 対象目的関数のインデックス |

**データ抽出方法**（full.rs の Spearman ブロックを参考）:
- `df.param_col_names()` からパラメータ名リストを取得 🔵
- `get_param_numeric_values(df, name, n)` からパラメータ値を取得 🔵
- `df.get_numeric_column(&objective_name)` から目的関数値を取得 🔵
- 最小行数チェック: `n < 2` の場合は `None` を返す 🔵
- `param_names.is_empty()` の場合は `None` を返す 🔵

**参照したEARS要件**: REQ-A01
**参照した設計文書**: interfaces.rs `SensitivityMetric` トレイト定義
**参照した既存実装**: `sensitivity/analysis/full.rs` L57-76 (Spearman ブロック)

#### RidgeMetric::compute()

| パラメータ | 型 | 制約 | 説明 |
|-----------|-----|------|------|
| `&self` | `&RidgeMetric` | - | ゼロサイズ構造体の参照 |
| `df` | `&DataFrame` | `row_count() >= 2` 必要 | パラメータ・目的関数データ |
| `obj_idx` | `usize` | `obj_idx < objective_col_names().len()` | 対象目的関数のインデックス |

**データ抽出方法**（full.rs の Ridge ブロックを参考）:
- `df.param_col_names()` からパラメータ名リストを取得 🔵
- `build_standardized_param_columns(df, &param_names, n)` から標準化パラメータ列を構築 🔵
- `df.get_numeric_column(&objective_name)` から目的関数値を取得 🔵
- 最小行数チェック: `n < 2` の場合は `None` を返す 🔵
- `param_names.is_empty()` の場合は `None` を返す 🔵

**参照したEARS要件**: REQ-A02
**参照した設計文書**: interfaces.rs `SensitivityMetric` トレイト定義
**参照した既存実装**: `sensitivity/analysis/full.rs` L77-89 (Ridge ブロック)

### 出力値 🔵

#### compute() の戻り値

| 戻り値 | 型 | 説明 |
|--------|-----|------|
| 成功時 | `Some(SensitivityResult)` | 計算結果 |
| データ不足・エラー時 | `None` | パニックしない |

#### SensitivityResult の内容（SpearmanMetric）

| フィールド | 値 | 型 |
|-----------|-----|-----|
| `param_names` | `df.param_col_names()` | `Vec<String>` |
| `objective_names` | `vec![objective_name]` | `Vec<String>` (1要素) |
| `spearman` | `Vec<Vec<f64>>` (各パラメータの感度値) | `Vec<Vec<f64>>` |
| `ridge` | `vec![]` | `Vec<RidgeResult>` (空) |
| `rf_anova` | `None` | `Option<RfAnovaResult>` |
| `mdi` | `None` | `Option<MdiResult>` |
| `shap` | `None` | `Option<ShapResult>` |
| `permutation` | `None` | `Option<PermutationResult>` |

#### SensitivityResult の内容（RidgeMetric）

| フィールド | 値 | 型 |
|-----------|-----|-----|
| `param_names` | `df.param_col_names()` | `Vec<String>` |
| `objective_names` | `vec![objective_name]` | `Vec<String>` (1要素) |
| `spearman` | `vec![]` | `Vec<Vec<f64>>` (空) |
| `ridge` | `vec![RidgeResult { beta, r_squared }]` | `Vec<RidgeResult>` (1要素) |
| `rf_anova` | `None` | `Option<RfAnovaResult>` |
| `mdi` | `None` | `Option<MdiResult>` |
| `shap` | `None` | `Option<ShapResult>` |
| `permutation` | `None` | `Option<PermutationResult>` |

#### name() の戻り値

| 実装者 | 戻り値 |
|--------|--------|
| `SpearmanMetric` | `"Spearman"` |
| `RidgeMetric` | `"Ridge"` |

**参照したEARS要件**: REQ-A01, REQ-A02
**参照した設計文書**: interfaces.rs `SensitivityMetric` トレイト定義、architecture.md A-1 実装者テーブル

### 入出力の関係性 🔵

```
入力: DataFrame + obj_idx
    │
    ├── 前提チェック
    │   ├── n < 2 || param_names.is_empty() → None
    │   └── obj_idx >= objective_names.len() → None
    │
    ├── [SpearmanMetric]
    │   ├── パラメータごとに get_param_numeric_values で x を取得
    │   ├── 目的関数値 y を取得
    │   ├── 各パラメータで compute_spearman(&x, &y) を呼び出し
    │   └── SensitivityResult を構築（spearman フィールドのみ設定）
    │
    └── [RidgeMetric]
        ├── build_standardized_param_columns で標準化パラメータ列を構築
        ├── 目的関数値 y を取得
        ├── compute_ridge_from_standardized_columns(&x_flat, n, &y) を呼び出し
        └── SensitivityResult を構築（ridge フィールドのみ設定）
```

**参照したデータフロー**: dataflow.md「2. SensitivityMetric トレイト呼び出しフロー」
**参照した既存実装**: `sensitivity/analysis/full.rs` L16-89

---

## 3. 制約条件（EARS非機能要件・アーキテクチャ設計ベース）

### パフォーマンス要件 🔵

- トレイト実装によるラッパーオーバーヘッドはゼロコスト抽象化として扱う（Rust の静的ディスパッチまたは `dyn` ディスパッチの最小コスト）
- 既存のベンチマーク（`sensitivity_bench`）は同等以上のスコアを維持する
- 追加のヒープアロケーションは発生しない（ゼロサイズ構造体）

**参照したEARS要件**: NFR-001
**参照した設計文書**: architecture.md「パフォーマンス」

### 正確性要件 🔵

- `SpearmanMetric::compute()` の結果は `compute_sensitivity_single_obj` 内の Spearman ブロックと同一結果を返す（浮動小数点許容誤差 `1e-10` 以内）
- `RidgeMetric::compute()` の結果は `compute_sensitivity_single_obj` 内の Ridge ブロックと同一結果を返す（浮動小数点許容誤差 `1e-10` 以内）
- `cargo test -p tunny-core` の全既存テストがパスする

**参照したEARS要件**: NFR-101, NFR-102
**参照した受け入れ基準**: TC-NFR-101-01, TC-NFR-102-01

### 互換性要件 🔵

- 既存の `compute_spearman` 関数の内部ロジックは変更しない
- 既存の `compute_ridge` / `compute_ridge_from_standardized_columns` 関数の内部ロジックは変更しない
- 既存の `compute_sensitivity_single_obj` 関数は本タスクでは変更しない（後続 TASK-2263 で対応）
- 既存の `compute_sensitivity_all` 関数は変更しない
- `SensitivityResult` 型のフィールド定義は変更しない

**参照したEARS要件**: REQ-A01, REQ-A02
**参照した設計文書**: architecture.md「既存機能の維持」

### アーキテクチャ制約 🔵

- `SpearmanMetric` は `Send + Sync` を満たす必要がある（スーパートレイト要件）
- `RidgeMetric` は `Send + Sync` を満たす必要がある（スーパートレイト要件）
- 両構造体はステートレス（フィールドなし）とする
- `common.rs` の `build_standardized_param_columns` を Ridge 実装で使用する
- `common.rs` の `get_param_numeric_values` を Spearman 実装で使用する

**参照した設計文書**: interfaces.rs `SensitivityMetric` トレイト定義（`Send + Sync` 制約）

### エラーハンドリング制約 🔵

- `compute()` は計算失敗時に `None` を返し、パニックしてはならない
- データ不足（`n < 2`、`param_names.is_empty()`）時に `None` を返す
- 無効な `obj_idx`（範囲外）時に `None` を返す
- `compute_spearman` 内部のエラー（分散ゼロ等）は既存のハンドリングに依存（`0.0` を返す）
- `compute_ridge_from_standardized_columns` 内部のエラー（特異行列等）は既存のハンドリングに依存

**参照したEARS要件**: EDGE-001
**参照した設計文書**: dataflow.md「2. SensitivityMetric トレイト呼び出しフロー」

### 保守性要件 🟡

- 各構造体の `impl` ブロックは 50 行以内とする
- `///` ドキュメントコメントを記述する
- 構造体命名規則: `{MetricName}Metric`（Suffix: `Metric`）

**参照したEARS要件**: NFR-201, NFR-202
**参照した設計文書**: architecture.md「保守性」

### WASM 互換性不要 🔵

- WASM ビルド対応は不要
- ネイティブ API を自由に使用してよい

**参照元**: docs/spec/rust-core-refactoring/note.md、CLAUDE.md フィードバック

---

## 4. 想定される使用例（EARS Edgeケース・データフローベース）

### 基本的な使用パターン 🔵

#### 使用例1: SpearmanMetric による感度計算

```
Given: 10パラメータ・100行の DataFrame と obj_idx = 0
When:  SpearmanMetric.compute(&df, 0) を呼び出す
Then:  10個のパラメータに対する Spearman 感度値が Some(SensitivityResult) として返る
       spearman フィールドに [[r1], [r2], ..., [r10]] が設定される
       他のフィールドは空/None
```

**参照したEARS要件**: REQ-A01
**参照した受け入れ基準**: TC-A01-01

#### 使用例2: RidgeMetric による感度計算

```
Given: 5パラメータ・50行の DataFrame と obj_idx = 0
When:  RidgeMetric.compute(&df, 0) を呼び出す
Then:  RidgeResult { beta: Vec<f64>, r_squared: f64 } が Some(SensitivityResult) として返る
       ridge フィールドに 1 つの RidgeResult が設定される
       他のフィールドは空/None
```

**参照したEARS要件**: REQ-A02
**参照した受け入れ基準**: TC-A01-01

### データフロー 🔵

```
SpearmanMetric::compute(df, obj_idx)
    │
    ├── 1. df.param_col_names() → param_names
    ├── 2. df.objective_col_names().get(obj_idx) → objective_name
    │       └── None → return None
    ├── 3. n = df.row_count()
    │       └── n < 2 || param_names.is_empty() → return None
    ├── 4. df.get_numeric_column(&objective_name) → y
    ├── 5. 各 param_name に対して:
    │       get_param_numeric_values(df, name, n) → x
    │       compute_spearman(&x, &y) → r
    ├── 6. SensitivityResult 構築
    │       spearman = [[r1], [r2], ...]
    │       他フィールド = 空/None
    └── 7. Some(result) を返す
```

```
RidgeMetric::compute(df, obj_idx)
    │
    ├── 1. df.param_col_names() → param_names
    ├── 2. df.objective_col_names().get(obj_idx) → objective_name
    │       └── None → return None
    ├── 3. n = df.row_count()
    │       └── n < 2 || param_names.is_empty() → return None
    ├── 4. build_standardized_param_columns(df, &param_names, n) → x_flat
    ├── 5. df.get_numeric_column(&objective_name) → y
    ├── 6. compute_ridge_from_standardized_columns(&x_flat, n, &y) → RidgeResult
    ├── 7. SensitivityResult 構築
    │       ridge = vec![RidgeResult]
    │       他フィールド = 空/None
    └── 8. Some(result) を返す
```

**参照したデータフロー**: dataflow.md「2. SensitivityMetric トレイト呼び出しフロー」

### エッジケース 🔵

#### EDGE-2259-01: データ不足時の None 返却

```
Given: 1行の DataFrame（n < 2）と obj_idx = 0
When:  SpearmanMetric.compute(&df, 0) を呼び出す
Then:  None が返り、パニックが発生しない
```

**参照したEARS要件**: EDGE-001
**参照したタスクファイル**: TASK-2259.md「完了条件」

#### EDGE-2259-02: パラメータなし時の None 返却

```
Given: param_names が空の DataFrame と obj_idx = 0
When:  RidgeMetric.compute(&df, 0) を呼び出す
Then:  None が返り、パニックが発生しない
```

**参照したEARS要件**: EDGE-001

#### EDGE-2259-03: 無効な obj_idx の None 返却 🟡

```
Given: 2目的関数の DataFrame と obj_idx = 5（範囲外）
When:  SpearmanMetric.compute(&df, 5) を呼び出す
Then:  None が返り、パニックが発生しない
```

**参照したEARS要件**: EDGE-001（妥当な推測）

### エラーケース 🔵

#### ERROR-2259-01: 目的関数カラムが存在しない場合 🟡

```
Given: objective_col_names[obj_idx] に対応する numeric_column が存在しない DataFrame
When:  SpearmanMetric.compute(&df, 0) を呼び出す
Then:  y = vec![0.0; n] のフォールバックで計算が継続する
       （既存の full.rs の動作に合わせる）
```

**参照した既存実装**: full.rs L33-36（`unwrap_or_else(|| vec![0.0; n])`）

#### ERROR-2259-02: パラメータカラムが取得できない場合 🟡

```
Given: get_param_numeric_values が None を返すパラメータを含む DataFrame
When:  SpearmanMetric.compute(&df, 0) を呼び出す
Then:  x = vec![0.0; n] のフォールバックで計算が継続する
       （既存の full.rs の動作に合わせる）
```

**参照した既存実装**: full.rs L44（`unwrap_or_else(|| vec![0.0; n])`）

---

## 5. EARS要件・設計文書との対応関係

### 参照したユーザストーリー

- **ストーリー A-1**: 木ベース感度指標の共通インターフェース（🔵）

### 参照した機能要件

- **REQ-A01**: `SensitivityMetric` トレイトを定義し、各指標が共通インターフェース `compute()` を実装する（🔵）
- **REQ-A02**: `tree_common.rs` はボイラープレートを1箇所に集約し、各指標実装から再利用する（🔵） - ※本タスクでは SpearmanMetric・RidgeMetric の直接実装に相当

### 参照した非機能要件

- **NFR-001**: ベンチマーク同等以上のスコア維持（🔵）
- **NFR-101**: 全既存テストパス（🔵）
- **NFR-102**: 数値計算結果の浮動小数点許容誤差 `1e-10` 以内（🟡）
- **NFR-201**: 各関数 50 行以内（🟡）
- **NFR-202**: ドキュメントコメント記述（🔴）

### 参照したEdgeケース

- **EDGE-001**: `SensitivityMetric::compute()` は失敗時に `None` を返し、パニックしない（🟡）

### 参照した受け入れ基準

- **TC-A01-01**: トレイトを実装した指標が `compute()` を通じて呼び出せる（🔵）
- **TC-A01-02**: 新規指標追加時にディスパッチ側の変更なしで動作する（🔵）
- **TC-NFR-101-01**: `cargo test -p tunny-core` 全テストパス（🔵）
- **TC-NFR-102-01**: 感度指標の計算結果がリファクタリング前後で `1e-10` 以内に一致（🟡）

### 参照した設計文書

- **アーキテクチャ**: architecture.md「A-1. SensitivityMetric トレイト導入」 - 実装者テーブル（SpearmanMetric, RidgeMetric）
- **データフロー**: dataflow.md「2. SensitivityMetric トレイト呼び出しフロー」 - compute() 呼び出しシーケンス
- **型定義**: interfaces.rs「A-1. SensitivityMetric トレイト」 - トレイトシグネチャと実装者一覧
- **既存実装**: `sensitivity/analysis/full.rs` - Spearman ブロック (L57-76)、Ridge ブロック (L77-89)
- **既存トレイト**: `sensitivity/metric_trait.rs` - SensitivityMetric トレイト定義

---

## 信頼性レベルサマリー

| カテゴリ | 総項目 | 🔵 青 | 🟡 黄 | 🔴 赤 |
|---------|--------|-------|-------|-------|
| 機能概要 | 4 | 4 | 0 | 0 |
| 入出力仕様 | 8 | 8 | 0 | 0 |
| 制約条件 | 7 | 5 | 2 | 0 |
| 使用例 | 8 | 6 | 2 | 0 |
| **合計** | **27** | **23** | **4** | **0** |

- 🔵 **青信号**: 23項目 (85%)
- 🟡 **黄信号**: 4項目 (15%)
- 🔴 **赤信号**: 0項目 (0%)

---

## 品質判定結果

**品質評価**: **高品質**

- **要件の曖昧さ**: なし - 全ての要件は EARS要件定義書・設計文書・既存コードから明確に導出されている
- **入出力定義**: 完全 - `compute()` / `name()` の入出力が型レベルで定義され、既存実装（full.rs）との対応が明確
- **制約条件**: 明確 - `Send + Sync`、エラーハンドリング、精度要件、後方互換性が全て規定されている
- **実装可能性**: 確実 - 既存の `compute_spearman` / `compute_ridge_from_standardized_columns` をラップするのみで、新規の計算ロジックは不要
- **信頼性レベル**: 🔵（青信号）が 85% を占め、高信頼
