# rust_core 外部ライブラリ高速化 アーキテクチャ設計

**作成日**: 2026-05-15
**関連要件定義**: [requirements.md](../../spec/rust-core-perf-libs/requirements.md)
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 要件定義書・コードベース調査・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: 要件定義書・コードベース調査から妥当な推測による設計
- 🔴 **赤信号**: 要件定義書・コードベース調査にない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *要件定義書・ユーザヒアリングより*

`rust_core` クレートの計算ヘビーパスにおいて、自前実装を外部ライブラリで置き換える。
主要な変更は以下の 5 領域:

1. **faer 活用拡大**: PCA 固有値分解、FITC Cholesky、Ridge 回帰を faer に統一
2. **argmin 導入**: 手作り L-BFGS を argmin の LBFGS solver に置き換え
3. **rand 導入**: 3 種の独自 PRNG を rand + rand_chacha に統一
4. **linfa-clustering 導入**: 重複 K-means を外部 crate に統合
5. **データレイアウト移行**: `Vec<Vec<f64>>` → `faer::Mat` への全局的移行

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存 Cargo Workspace 構成・要件定義より*

- **パターン**: 2 クレート Cargo Workspace + 内部モジュールレイヤード構成（変更なし）
- **選択理由**: クレート構成は維持し、内部実装のみを最適化。egui-app は API 変更に追従。

```
tunny-dashboard/
├── rust_core/             ← 本要件の対象
│   ├── Cargo.toml         ← 依存関係変更
│   └── src/               ← 内部実装変更
└── egui-app/              ← API 変更に追従
```

## 依存関係設計 🔵

**信頼性**: 🔵 *要件定義・crates.io 調査・ユーザヒアリングより*

### 変更前

```toml
[dependencies]
faer = "0.24.0"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rayon = "1"
```

### 変更後

```toml
[dependencies]
faer = "0.24.0"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rayon = "1"
argmin = "0.11"
argmin-math = { version = "0.5", features = ["vec"] }    # vec backend（faer競合回避）
rand = "0.9"
rand_chacha = "0.9"                                        # rand_chacha は 0.9 が最新; rand 0.10 は未対応
linfa-clustering = "0.8"                                   # ndarray が推移依存に追加
ndarray = "0.16"                                            # linfa-clustering + faer↔ndarray 変換用
```

### 依存関係追加の技術的判断

| Crate | Backend | 理由 |
|-------|---------|------|
| `argmin` + `argmin-math(vec)` | vec | L-BFGS はパラメータベクトルのみ扱う。argmin-math の faer バックエンドは v0.21 までのサポートで v0.24 と非互換のため vec backend を選択（調査済み）🔵 |
| `rand` + `rand_chacha` | なし | ChaCha8 による高品質 PRNG。決定論的シード対応 🔵 |
| `linfa-clustering` | ndarray | K-means の外部委譲。ndarray が推移依存に追加される 🔵 |
| `ndarray` | なし | faer::Mat ↔ ndarray::Array2 変換用の明示的依存 🔵 |

### 削除対象 🔵

- `core/random_forest/` モジュール全体（tree.rs, forest.rs, types.rs, rng.rs, tests.rs）

## コンポーネント構成

### モジュール構成（変更後）🔵

**信頼性**: 🔵 *既存 lib.rs + 要件定義より*

```
rust_core/src/
├── lib.rs                  ← RandomForest の re-export 削除
├── clustering/
│   ├── mod.rs              ← 公開 API 変更なし
│   ├── kmeans.rs           ← linfa-clustering バックエンド化
│   ├── pca.rs              ← faer 固有値分解化
│   └── types.rs            ← 変更なし
├── convergence.rs          ← 変更なし
├── core/
│   ├── mod.rs              ← random_forest モジュール削除
│   ├── math/
│   │   ├── mod.rs
│   │   ├── linear_algebra.rs  ← Vec<Vec<f64>> 変換関数削除
│   │   ├── stats.rs           ← 変更なし
│   │   ├── grid.rs            ← 変更なし
│   │   └── rng.rs             ← 新規: ChaCha8Rng ラッパー（rand 移行用）
│   ├── optimization/
│   │   ├── mod.rs          ← 公開 API 変更（argmin ベース）
│   │   ├── lbfgs.rs        ← argmin LBFGS に置き換え
│   │   └── line_search.rs  ← 削除（argmin 内蔵）
│   ├── kriging/
│   │   ├── kernel.rs       ← faer::Mat 化
│   │   ├── sparse_fitc.rs  ← faer Cholesky 化 + 重複 k-means 削除
│   │   └── gaussian_process/
│   │       ├── model.rs
│   │       ├── training.rs    ← argmin 呼び出し化
│   │       ├── inference.rs
│   │       ├── optimization.rs
│   │       ├── likelihood.rs
│   │       ├── kernel_ops.rs
│   │       ├── solvers.rs     ← faer::Mat 化
│   │       └── tests.rs
│   ├── lgbm.rs             ← 変更なし
│   └── [random_forest/]    ← 削除
├── data/                   ← 変更なし
├── io/                     ← 変更なし
├── mcdm/                   ← faer::Mat 化（内部計算）
├── multi_objective/        ← faer::Mat 化（Pareto、Hypervolume）
├── pdp/
│   ├── mod.rs
│   ├── api.rs
│   ├── ridge_core.rs       ← faer QR/Cholesky 化
│   ├── kriging_core.rs     ← faer::Mat 化
│   └── ...
├── sampling/
│   ├── common.rs           ← rand 化
│   └── context.rs          ← rand 化
└── sensitivity/            ← faer::Mat 化（Ridge 部分）
```

### データレイアウト移行 🔵

**信頼性**: 🔵 *要件定義 REQ-104・ユーザヒアリング（全局的移行）より*

**移行方針**: `Vec<Vec<f64>>` → `faer::Mat` を全モジュールに適用

```rust
// 変更前
fn compute(x_matrix: &Vec<Vec<f64>>, y: &Vec<f64>) -> Result<PcaResult> { ... }

// 変更後
fn compute(x_matrix: &faer::Mat<f64>, y: &Vec<f64>) -> Result<PcaResult> { ... }
```

**境界変換** (faer ↔ ndarray):

```rust
/// faer::Mat → ndarray::Array2 変換（linfa-clustering 用）
///
/// 注意: faer::Mat は column-major だが ndarray::Array2 デフォルトは row-major。
/// as_slice() による直接変換は転置が発生するため element-wise コピーを使用する。
fn faer_to_ndarray(mat: &faer::Mat<f64>) -> ndarray::Array2<f64> {
    ndarray::Array2::from_shape_fn(
        (mat.nrows(), mat.ncols()),
        |(i, j)| mat[(i, j)],
    )
}

/// ndarray::Array2 → faer::Mat 変換（linfa 結果取り出し用）
fn ndarray_to_faer(arr: &ndarray::Array2<f64>) -> faer::Mat<f64> {
    faer::Mat::from_fn(arr.nrows(), arr.ncols(), |i, j| arr[[i, j]])
}
```

## 移行フェーズ構成 🔵

**信頼性**: 🔵 *ユーザヒアリング（ボトムアップ移行）より*

### Phase 1: 基盤整備（デッドコード削除 + rand 移行）

- `core/random_forest/` ディレクトリ削除
- `lib.rs` の `pub use RandomForest` 削除
- `core/math/rng.rs` 新規作成（ChaCha8Rng ラッパー）
- `sampling/common.rs` の LCG → rand 化
- `clustering/kmeans.rs` の xorshift64 → rand 化は**省略可**（Phase 4 の linfa 移行で一括削除されるため、一時対応の投資が不要）

### Phase 2: faer 活用拡大（データレイアウト移行含む）

- `core/math/linear_algebra.rs` の変換関数を整理
- 各モジュールの `Vec<Vec<f64>>` → `faer::Mat` 移行
- `clustering/pca.rs` Jacobi → faer 固有値分解
- `core/kriging/sparse_fitc.rs` 手作り Cholesky → faer Cholesky
- `pdp/ridge_core.rs` ガウス消去 → faer QR
- `sensitivity/` 内の Ridge 回帰 → faer 化

### Phase 3: argmin 導入

- `core/optimization/lbfgs.rs` → argmin LBFGS
- `core/optimization/line_search.rs` → 削除
- `core/kriging/gaussian_process/training.rs` → argmin 呼び出し

### Phase 4: linfa-clustering 導入

- `clustering/kmeans.rs` → linfa-clustering バックエンド
- `core/kriging/sparse_fitc.rs` 内の重複 k-means → clustering モジュール呼び出し

### Phase 5: 全体検証

- 全テストスイート実行
- ベンチマークによる性能比較
- egui-app 側のコンパイル確認

## 各モジュールの設計詳細

### 1. core/math/rng.rs（新規）🔵

**信頼性**: 🔵 *要件定義 REQ-301 より*

```rust
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

/// 決定論的シード対応の ChaCha8 RNG ラッパー
pub struct SeededRng {
    rng: ChaCha8Rng,
}

impl SeededRng {
    pub fn from_seed(seed: u64) -> Self {
        Self {
            rng: ChaCha8Rng::seed_from_u64(seed),
        }
    }

    /// [0, 1) の一様乱数を生成（&mut self: interior mutability 不要）
    pub fn next_f64(&mut self) -> f64 {
        use rand::Rng;
        self.rng.r#gen()   // rand 0.9 / edition 2024 では gen がキーワード: r# でエスケープ
    }

    pub fn next_usize(&mut self, bound: usize) -> usize {
        use rand::Rng;
        self.rng.gen_range(0..bound)
    }
}
```

### 2. core/optimization/lbfgs.rs（argmin 化）🔵

**信頼性**: 🔵 *要件定義 REQ-201 より*

```rust
use argmin::core::{Executor, State};
use argmin::solver::quasinewton::LBFGS;

/// L-BFGS 最適化の argmin ベース実装
pub struct LbfgsOptimizer {
    max_iter: u64,
    tolerance: f64,
}

impl LbfgsOptimizer {
    pub fn new(max_iter: u64, tolerance: f64) -> Self {
        Self { max_iter, tolerance }
    }

    /// 目的関数（cost function）を最適化
    /// cost_fn: &dyn Fn(&[f64]) -> f64 （スカラー目的関数）
    /// grad_fn: &dyn Fn(&[f64]) -> Vec<f64> （勾配関数）
    /// init_params: 初期パラメータ
    pub fn optimize(
        &self,
        cost_fn: &dyn Fn(&[f64]) -> f64,
        grad_fn: &dyn Fn(&[f64]) -> Vec<f64>,
        init_params: Vec<f64>,
    ) -> Result<OptimizationResult> { ... }
}
```

### 3. clustering/kmeans.rs（linfa バックエンド）🔵

**信頼性**: 🔵 *要件定義 REQ-401 より*

```rust
use linfa_clustering::KMeans;
use ndarray::Array2;

/// linfa-clustering バックエンドの K-means
pub fn kmeans_clustering(
    data: &faer::Mat<f64>,
    k: usize,
    max_iter: usize,
    seed: u64,
) -> KmeansResult {
    // faer::Mat → ndarray::Array2 変換
    let ndarray_data = faer_to_ndarray(data);

    // linfa K-means 実行
    let model = KMeans::params(k)
        .max_n_iterations(max_iter)
        .rng_seed(seed)
        .fit(&ndarray_data)
        .unwrap();

    // 結果を既存の KmeansResult 型に変換
    let centroids = ndarray_to_faer(&model.centroids());
    let assignments = model.predict(&ndarray_data);
    ...
}
```

### 4. clustering/pca.rs（faer 固有値分解）🔵

**信頼性**: 🔵 *要件定義 REQ-101 より*

```rust
/// faer selfadjoint_eigenvalue_decomposition を使用する PCA
pub fn compute_pca(
    data: &faer::Mat<f64>,
    n_components: usize,
) -> PcaResult {
    // 1. 中心化
    let centered = center_rows(data);

    // 2. 共分散行列
    let cov = compute_covariance(&centered);

    // 3. 固有値分解（faer）
    let eig = cov.selfadjoint_eigenvalue_decomposition(faer::Side::Lower);

    // 4. 降順ソート + 上位 n_components 射影
    ...
}
```

## 非機能要件の実現方法

### パフォーマンス 🟡

**信頼性**: 🟡 *faer SIMD 効果の推測*

| モジュール | 従来手法 | 変更後手法 | 期待効果 |
|-----------|---------|-----------|---------|
| PCA | Jacobi 固有値分解（純スカラー） | faer SIMD 固有値分解 | 5-50x |
| FITC Cholesky | 手作りループ | faer Cholesky + 三角 solve | 3-10x |
| Ridge | ガウス消去 | faer Cholesky | 3-10x |
| L-BFGS | 手作り 2-loop recursion | argmin LBFGS | 収束改善 |
| K-means | 2 つの手作り実装 | linfa-clustering | 重複解消 |
| PRNG | 3 種の独自実装 | rand ChaCha8 | 品質向上 |

### 品質 🔵

**信頼性**: 🔵 *既存テストスイート要件より*

- 全既存テストが通過することが必須
- 数値精度: 相対誤差 1e-10 以内（faer の精度が既存実装以上である前提）
- ベンチマークによる性能回帰防止

## 技術的制約

### faer バージョン互換性 🔵

**信頼性**: 🔵 *crates.io 調査より*

- argmin-math の faer バックエンドは v0.21 まで対応。v0.24 は未対応。
- 解決策: argmin-math は `vec` バックエンドを使用し、LA 操作は自前で faer を直接呼び出す。

### ndarray 推移依存 🔵

**信頼性**: 🔵 *linfa-clustering 仕様より*

- linfa-clustering は ndarray に依存。コンパイル時間とバイナリサイズに影響。
- faer::Mat ↔ ndarray::Array2 の境界変換は O(N*M) コピーが発生。
- K-means の計算量 O(N*K*iter) に対してコピー O(N*M) は無視できる 🟡

### egui-app API 追従 🔵

**信頼性**: 🔵 *ユーザヒアリング（API 含むリファクタ）より*

- rust_core の公開 API 変更に伴い、egui-app 側も更新が必要
- 特に `Vec<Vec<f64>>` → `faer::Mat` の移行で egui-app の呼び出しコード変更が発生

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **要件定義**: [requirements.md](../../spec/rust-core-perf-libs/requirements.md)
- **関連既存設計**:
  - [kriging-performance-optimization](../kriging-performance-optimization/architecture.md)
  - [rayon-performance-optimization](../../spec/rayon-performance-optimization/requirements.md)
  - [egui-migration](../egui-migration/architecture.md)

## 信頼性レベルサマリー

- 🔵 青信号: 18 件 (86%)
- 🟡 黄信号: 3 件 (14%)
- 🔴 赤信号: 0 件 (0%)

**品質評価**: ✅ 高品質 — コードベース詳細調査とユーザヒアリングに基づく確実な設計。黄信号は性能期待値の推測のみ。
