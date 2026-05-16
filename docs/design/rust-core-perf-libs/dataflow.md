# rust_core 外部ライブラリ高速化 データフロー図

**作成日**: 2026-05-15
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連要件定義**: [requirements.md](../../spec/rust-core-perf-libs/requirements.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 要件定義書・コードベース調査を参考にした確実なフロー
- 🟡 **黄信号**: 要件定義書・コードベース調査から妥当な推測によるフロー
- 🔴 **赤信号**: 要件定義書・コードベース調査にない推測によるフロー

---

## モジュール間データフロー（全体）🔵

**信頼性**: 🔵 *既存コードベース + 要件定義より*

```mermaid
graph TD
    subgraph "rust_core (変更後)"
        IO[io/journal] --> DF[data/DataFrame]
        DF --> |faer::Mat| SENS[sensitivity/]
        DF --> |faer::Mat| MCDM[mcdm/]
        DF --> |faer::Mat| MO[multi_objective/]
        DF --> |faer::Mat| PDP[pdp/]
        DF --> |faer::Mat| CLUST[clustering/]

        CLUST --> |faer::Mat| PCA_KM[PCA: faer eigendecomp]
        CLUST --> |ndarray::Array2| LINFAC[linfa-clustering KMeans]
        LINFAC --> |変換| CLUST

        PDP --> KRIG[kriging/ GP 学習]
        PDP --> RIDGE[ridge: faer QR]

        KRIG --> OPT[optimization/ argmin LBFGS]
        KRIG --> FITC[sparse_fitc: faer Cholesky]
        FITC --> CLUST

        SENS --> RIDGE
        SENS --> LGBM[lgbm: 変更なし]
    end

    subgraph "egui-app"
        UI[UI コンポーネント] --> |faer::Mat| DF
        DF --> |分析結果| UI
    end

    RNG[core/math/rng: ChaCha8] -.-> CLUST
    RNG -.-> SENS
    RNG -.-> MO
```

---

## Phase 1: デッドコード削除 + rand 移行 🔵

**信頼性**: 🔵 *要件定義 REQ-501, REQ-301 より*

### 1.1 Random Forest 削除フロー

```mermaid
sequenceDiagram
    participant lib as lib.rs
    participant rf as core/random_forest/
    participant rng as core/math/rng.rs (新規)

    Note over lib: pub use RandomForest 削除
    Note over rf: tree.rs, forest.rs, types.rs, tests.rs 削除
    Note over rng: Lcg → ChaCha8Rng ラッパーを新規作成
    lib->>rng: Lcg 利用モジュール (sampling, sensitivity) が rng.rs 参照
    Note over lib: core/mod.rs から random_forest モジュール削除
```

### 1.2 PRNG 移行フロー

```mermaid
flowchart LR
    subgraph "変更前"
        LCG1[LCG<br>sampling/common.rs]
        XOR[xorshift64<br>clustering/kmeans.rs]
        LCG2[LCG<br>core/random_forest/rng.rs]
    end

    subgraph "変更後"
        CRNG[ChaCha8Rng<br>core/math/rng.rs]
    end

    LCG1 --> CRNG
    XOR --> CRNG
    LCG2 --> |削除| CRNG

    CRNG --> |シード指定| SAMPLING[sampling/]
    CRNG --> |シード指定| SENSITIVITY[sensitivity/]
    CRNG --> |シード指定| CLUSTERING[clustering/]
```

---

## Phase 2: faer 活用拡大（データレイアウト移行含む）🔵

**信頼性**: 🔵 *要件定義 REQ-101, REQ-102, REQ-103, REQ-104 より*

### 2.1 PCA データフロー（faer 固有値分解）

```mermaid
sequenceDiagram
    participant caller as 呼び出し元
    participant pca as clustering/pca.rs
    participant faer as faer::Mat

    caller->>pca: compute(data: &faer::Mat, n_components)
    pca->>pca: 中心化 (mean 減算)
    pca->>faer: 共分散行列構築
    pca->>faer: selfadjoint_eigenvalue_decomposition()
    faer-->>pca: (eigenvalues, eigenvectors)
    pca->>pca: 降順ソート
    pca->>pca: 上位 n_components 射影
    pca-->>caller: PcaResult { projected, eigenvalues, ... }
```

**従来との差分**:
- 従来: Jacobi 固有値分解（純スカラーループ、~90 行）
- 変更後: `faer::Mat::selfadjoint_eigenvalue_decomposition()`（SIMD 加速）

### 2.2 FITC Sparse GP データフロー（faer Cholesky）

```mermaid
sequenceDiagram
    participant gp as gaussian_process/
    participant fitc as sparse_fitc.rs
    participant faer as faer::Mat
    participant km as clustering/kmeans

    gp->>fitc: train(x, y, n_inducing)
    fitc->>km: 誘導点選択 (K-means)
    km-->>fitc: inducing_points (faer::Mat)

    fitc->>fitc: K_ZZ, K_XZ 構築
    fitc->>faer: cholesky(K_ZZ)
    faer-->>fitc: L (下三角)

    Note over fitc: Woodbury 恒等式で LML 計算
    fitc->>faer: triangular_solve(L, ...)
    faer-->>fitc: solve 結果

    fitc-->>gp: SparseGpModel { params, ... }

    gp->>fitc: predict(model, x_new)
    fitc->>faer: triangular_solve(L, K_XZ)
    faer-->>fitc: 予測値
    fitc-->>gp: (mean, variance)
```

**従来との差分**:
- 従来: `cholesky_flat`, `forward_sub_flat`, `backward_sub_flat`（純スカラーループ）
- 変更後: `faer::Mat::cholesky()`, `faer::Mat::solve_triangular()`（SIMD 加速）

### 2.3 Ridge 回帰データフロー（faer QR）

```mermaid
sequenceDiagram
    participant caller as 呼び出し元 (pdp/sensitivity)
    participant ridge as pdp/ridge_core.rs
    participant faer as faer::Mat

    caller->>ridge: ridge_regression(X: &faer::Mat, y: &[f64], alpha)
    ridge->>faer: X'X + alpha*I 構築
    ridge->>faer: cholesky(X'X + alpha*I)
    faer-->>ridge: L
    ridge->>faer: triangular_solve(L, X'y)
    faer-->>ridge: coefficients
    ridge->>ridge: R² 計算
    ridge-->>caller: RidgeResult { coefficients, r_squared }
```

**従来との差分**:
- 従来: 手作りガウス消去（~45 行）
- 変更後: `faer::Mat::cholesky()` + `solve_triangular()`（SIMD 加速）

### 2.4 データレイアウト移行フロー

```mermaid
flowchart TD
    subgraph "変更前: Vec<Vec<f64>>"
        V1[io/journal] -->|Vec<Vec<f64>>| V2[DataFrame]
        V2 -->|Vec<Vec<f64>>| V3[PCA]
        V2 -->|Vec<Vec<f64>>| V4[Kriging]
        V2 -->|Vec<Vec<f64>>| V5[Ridge]
        V2 -->|Vec<Vec<f64>>| V6[MCDM]
    end

    subgraph "変更後: faer::Mat"
        F1[io/journal] -->|faer::Mat| F2[DataFrame]
        F2 -->|&faer::Mat| F3[PCA]
        F2 -->|&faer::Mat| F4[Kriging]
        F2 -->|&faer::Mat| F5[Ridge]
        F2 -->|&faer::Mat| F6[MCDM]
    end

    V1 -->|移行| F1
    V2 -->|移行| F2
```

---

## Phase 3: argmin 導入 🔵

**信頼性**: 🔵 *要件定義 REQ-201 より*

### 3.1 GP 超パラメータ最適化フロー

```mermaid
sequenceDiagram
    participant train as training.rs
    participant argmin as argmin::LBFGS
    participant lml as likelihood.rs
    participant faer as faer::Mat

    train->>argmin: Executor::new(cost_fn, LBFGS::new())
    argmin->>lml: cost_fn(params) → (neg_lml, gradient)

    Note over lml: 目的関数評価（1 イテレーション）
    lml->>faer: カーネル行列 K 構築
    lml->>faer: cholesky(K)
    faer-->>lml: L
    lml->>faer: triangular_solve(L, y)
    faer-->>lml: alpha
    lml->>lml: LML + 勾配計算
    lml-->>argmin: (neg_lml, gradient)

    Note over argmin: L-BFGS 2-loop recursion + line search

    argmin-->>train: 最適 params
```

**従来との差分**:
- 従来: 手作り L-BFGS two-loop recursion + Armijo backtracking
- 変更後: `argmin::solver::quasinewton::LBFGS`（Wolfe 条件対応、収束性改善）

### 3.2 argmin 目的関数の設計 🔵

```rust
// argmin の CostFunction トレイト実装
struct GpCostFn<'a> {
    x: &'a faer::Mat<f64>,
    y: &'a [f64],
}

impl argmin::core::CostFunction for GpCostFn<'_> {
    type Param = Vec<f64>;
    type Output = f64;

    fn cost(&self, p: &Self::Param) -> Result<Self::Output, argmin::core::Error> {
        // faer で LML 計算
        Ok(neg_log_marginal_likelihood(self.x, self.y, p))
    }
}

impl argmin::core::Gradient for GpCostFn<'_> {
    type Param = Vec<f64>;
    type Gradient = Vec<f64>;

    fn gradient(&self, p: &Self::Param) -> Result<Self::Gradient, argmin::core::Error> {
        Ok(log_ml_gradient(self.x, self.y, p))
    }
}
```

---

## Phase 4: linfa-clustering 導入 🔵

**信頼性**: 🔵 *要件定義 REQ-401 より*

### 4.1 K-means データフロー

```mermaid
sequenceDiagram
    participant caller as 呼び出し元 (clustering/ or sparse_fitc)
    participant km as clustering/kmeans.rs
    participant conv as faer→ndarray 変換
    participant linfa as linfa-clustering
    participant conv2 as ndarray→faer 変換

    caller->>km: kmeans(data: &faer::Mat, k, max_iter, seed)
    km->>conv: faer_to_ndarray(data)
    conv-->>km: ndarray::Array2

    km->>linfa: KMeans::params(k).fit(&ndarray_data)
    linfa-->>km: KMeansModel

    km->>linfa: model.centroids()
    linfa-->>km: ndarray::Array2 (centroids)
    km->>conv2: ndarray_to_faer(centroids)
    conv2-->>km: faer::Mat

    km->>linfa: model.predict(&ndarray_data)
    linfa-->>km: assignments: Array1<usize>

    km->>km: WCSS 計算
    km-->>caller: KmeansResult { centroids, assignments, wcss }
```

### 4.2 エルボー法フロー

```mermaid
flowchart TD
    A[入力: data, max_k] --> B[k = 2]
    B --> C[kmeans_clustering(data, k)]
    C --> D[WCSS 保存]
    D --> E{k < max_k?}
    E -->|Yes| F[k += 1]
    F --> C
    E -->|No| G[エルボー点検出]
    G --> H[最適 k で最終クラスタリング]
    H --> I[KmeansResult 返却]
```

---

## エラーハンドリングフロー 🟡

**信頼性**: 🟡 *既存実装パターンから妥当な推測*

```mermaid
flowchart TD
    A[外部 crate 呼び出し] --> B{結果}
    B -->|Ok| C[結果を既存型に変換して返却]
    B -->|Err| D{エラー種別}

    D -->|Cholesky 失敗| E[非正定値エラー]
    D -->|argmin 収束失敗| F[最良パラメータ返却]
    D -->|linfa KMeans 失敗| G[フォールバック: 再初期化]

    E --> H[呼び出し元にエラー伝播]
    F --> I[警告ログ + 最良値返却]
    G --> J[リトライ or エラー]
```

## 関連文書

- **アーキテクチャ**: [architecture.md](architecture.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **要件定義**: [requirements.md](../../spec/rust-core-perf-libs/requirements.md)

## 信頼性レベルサマリー

- 🔵 青信号: 12 件 (92%)
- 🟡 黄信号: 1 件 (8%)
- 🔴 赤信号: 0 件 (0%)

**品質評価**: ✅ 高品質
