# rust-core-refactoring データフロー図

**作成日**: 2026-05-14
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連要件定義**: [requirements.md](../../spec/rust-core-refactoring/requirements.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実なフロー
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測によるフロー
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測によるフロー

---

## 1. 感度分析ディスパッチ: リファクタリング前後 🔵

**信頼性**: 🔵 *REQ-A01〜A03・B01・コード分析 sensitivity/analysis/full.rs より*

### 変更前（279行、match 6アーム）

```mermaid
flowchart TD
    A[compute_sensitivity_single_obj\ndf, metric: SensitivityMetric enum, obj_idx]
    A --> B{match metric}
    B -->|Spearman| C[spearman 計算]
    B -->|Ridge| D[ridge 計算]
    B -->|RfAnova| E[tree_metric 計算]
    B -->|Mdi| F[tree_metric 計算]
    B -->|Shap| G[tree_metric 計算]
    B -->|Permutation| H[tree_metric 計算]
    C & D & E & F & G & H --> I[SensitivityResult 構築]
```

### 変更後（150行以内、trait dispatch）

```mermaid
flowchart TD
    A[compute_sensitivity_single_obj\ndf, metrics: Vec&lt;Box&lt;dyn SensitivityMetric&gt;&gt;, obj_idx]
    A --> B[metrics.iter をループ]
    B --> C[metric.compute\ndf, obj_idx]
    C -->|Some| D[results に追加]
    C -->|None| E[スキップ ログ記録]
    D --> F[Vec&lt;SensitivityResult&gt; 返却]
```

**関連要件**: REQ-A01, REQ-A02, REQ-A03, REQ-B01

---

## 2. SensitivityMetric トレイト呼び出しフロー 🔵

**信頼性**: 🔵 *REQ-A01・A02・ユーザーストーリー A-1 より*

```mermaid
sequenceDiagram
    participant Full as full.rs
    participant SM as SensitivityMetric impl
    participant TC as tree_common.rs
    participant Core as 各計算モジュール

    Full->>SM: metric.compute(df, obj_idx)
    SM->>TC: prepare_training_data(x, y, max_rows, ...)
    TC-->>SM: Option<PreparedData>
    alt PreparedData あり
        SM->>Core: 計算実行 (TreeMetric / Spearman / Ridge)
        Core-->>SM: importances / coefficients
        SM->>SM: SensitivityResult 構築
        SM-->>Full: Some(SensitivityResult)
    else PreparedData なし
        SM-->>Full: None
    end
```

---

## 3. SamplingContext: グローバル状態廃止フロー 🔵

**信頼性**: 🔵 *REQ-C05〜C08・ユーザーヒアリング・ユーザーストーリー C-1 より*

### 変更前（thread_local グローバル状態）

```mermaid
flowchart LR
    A[egui-app\nデータロード] -->|init_sampling 呼び出し| B[sampling/state.rs\nthread_local STATE]
    B -->|副作用で更新| B
    C[downsample_smart] -->|STATE 参照| B
    D[downsample_stratified_by_rank] -->|STATE 参照| B
    E[set_cluster_labels] -->|STATE 変更| B
    style B fill:#ff9999
```

### 変更後（SamplingContext 値型）

```mermaid
flowchart LR
    A[egui-app\nデータロード] -->|init_sampling 呼び出し| B[SamplingContext 生成\n値を返す]
    B -->|AppState.sampling_ctx\nに保存| C[AppState\nsampling_ctx: Option&lt;SamplingContext&gt;]
    C -->|&SamplingContext を渡す| D[downsample_smart]
    C -->|&SamplingContext を渡す| E[downsample_stratified_by_rank]
    C -->|cluster_labels 直接更新| C
    style C fill:#99cc99
    style B fill:#99cc99
```

**SamplingContext ライフサイクル**:

```mermaid
sequenceDiagram
    participant App as egui-app
    participant AS as AppState
    participant SC as SamplingContext
    participant DS as downsample_*

    App->>SC: init_sampling(is_minimize, pareto_indices, all_ranks)
    SC-->>App: SamplingContext (値)
    App->>AS: app_state.sampling_ctx = Some(ctx)

    Note over App,AS: クラスタリング完了後
    App->>AS: sampling_ctx.as_mut().map(|c| c.cluster_labels = Some(labels))

    Note over App,DS: ダウンサンプリング要求時
    App->>DS: downsample_smart(&ctx, max_points)
    DS-->>App: Vec<u32> (選択インデックス)

    Note over App,AS: 新しいデータロード時
    App->>AS: app_state.sampling_ctx = None
    App->>SC: init_sampling(...)
```

---

## 4. k-means 初期化: select_next_centroid 抽出フロー 🔵

**信頼性**: 🔵 *REQ-A06・A07・コード分析 clustering/kmeans.rs より*

### 変更前（80% コード重複）

```mermaid
flowchart LR
    A[init_kmeans_plusplus] -->|独自の重心選択ロジック| C[重心リスト]
    B[init_deterministic] -->|独自の重心選択ロジック| C
    style A fill:#ffcc99
    style B fill:#ffcc99
```

### 変更後（共通関数を共有）

```mermaid
flowchart LR
    A[init_kmeans_plusplus] -->|ランダム sampling_fn| SC[select_next_centroid\nflat_data, n_cols,\nexisting, n, sampling_fn]
    B[init_deterministic] -->|中央選択 sampling_fn| SC
    SC -->|次の重心を返す| C[重心リスト]
    style SC fill:#99cc99
```

**select_next_centroid の処理**:

```mermaid
sequenceDiagram
    participant Caller as 呼び出し元
    participant SNC as select_next_centroid
    participant SF as sampling_fn

    Caller->>SNC: (flat_data, n_cols, existing_centroids, n, sampling_fn)
    SNC->>SNC: 各点の最小距離² を計算
    SNC->>SF: distances を渡す
    SF-->>SNC: 選択インデックス
    SNC-->>Caller: 選択された点のスライス
```

---

## 5. クラスタ統計: 3 関数分割フロー 🔵

**信頼性**: 🔵 *REQ-B03・コード分析 clustering/stats.rs より*

### 変更前（89行の単一関数）

```mermaid
flowchart TD
    A[compute_cluster_stats_on_data\nflat_data, n, p, labels, k] --> B[グローバル統計計算]
    B --> C[クラスタ統計計算]
    C --> D[有意性検定]
    D --> E[Vec&lt;ClusterStat&gt; 返却]
    style A fill:#ffcc99
```

### 変更後（3 独立関数 + オーケストレーター）

```mermaid
flowchart TD
    A[compute_cluster_stats_on_data\nオーケストレーター 〜20行] --> B[compute_global_stats\nflat_data, n, p\n→ means, stds]
    A --> C[compute_cluster_centroid_std\nflat_data, labels, n, p, k\n→ Vec&lt;ClusterStat&gt;]
    B --> D[compute_significant_features\ncluster_stats, global_mean, global_std, n\n→ significant_features 付与]
    C --> D
    D --> E[Vec&lt;ClusterStat&gt; 返却]
    style A fill:#99cc99
    style B fill:#cce5ff
    style C fill:#cce5ff
    style D fill:#cce5ff
```

---

## 6. Ridge 回帰: 3 関数分割フロー 🔵

**信頼性**: 🔵 *REQ-B04・コード分析 sensitivity/ridge.rs より*

### 変更前（55行の複合関数）

```mermaid
flowchart TD
    A[compute_ridge_from_standardized_columns\nx_cols: &amp;f64, n, y, alpha] --> B[X'X 行列構築]
    B --> C[X'y ベクトル構築]
    C --> D[Ridge 正則化: XTX += αI]
    D --> E[gaussian_elimination 解法]
    E --> F[y_pred 計算]
    F --> G[R² 計算]
    G --> H[RidgeResult 返却]
    style A fill:#ffcc99
```

### 変更後（3 独立関数）

```mermaid
flowchart TD
    A[compute_ridge_from_standardized_columns\nオーケストレーター 〜20行]
    A --> B[compute_xtx_matrix\nx_cols, p, n\n→ Vec&lt;Vec&lt;f64&gt;&gt;]
    A --> C[compute_xty_vector\nx_cols, y, p, n\n→ Vec&lt;f64&gt;]
    B --> D[Ridge 正則化\nXTX += αI]
    C --> D
    D --> E[gaussian_elimination\n解法]
    E --> F[y_pred 計算]
    F --> G[compute_r_squared\ny_actual, y_predicted\n→ f64]
    G --> H[RidgeResult 返却]
    style B fill:#cce5ff
    style C fill:#cce5ff
    style G fill:#cce5ff
```

---

## 7. GpModel 分割フロー 🟡

**信頼性**: 🟡 *REQ-B05・コード分析 gaussian_process/model.rs から妥当な推測*

### 変更前（超パラメータと訓練データの混在）

```mermaid
flowchart LR
    A[GpModel\nlog_ls, log_sf, log_sn\nalpha, x_train, l] -->|optimize| A
    A -->|predict| B[予測値 + 分散]
    style A fill:#ffcc99
```

### 変更後（責務分離）

```mermaid
flowchart LR
    K[GpKernel\nlog_ls, log_sf, log_sn] -->|最適化| K
    K -->|kernel を内包| F[GpFittedModel\nkernel: GpKernel\nalpha, x_train, l]
    F -->|predict| B[予測値 + 分散]
    style K fill:#cce5ff
    style F fill:#99cc99
```

---

## 8. TOPSIS 行列構築: 効率化フロー 🔵

**信頼性**: 🔵 *REQ-C03・コード分析 mcdm/topsis.rs より*

### 変更前（行ごとの push）

```mermaid
flowchart TD
    A[build_weighted_matrix\nvalues, valid_indices, weights] --> B[result: Vec&lt;f64&gt; = vec!]
    B --> C[valid_indices.iter\n行ごとに処理]
    C --> D[row: Vec&lt;f64&gt; = vec!\n目的関数ごとに push]
    D --> E[result.extend_from_slice\nrow を追記]
    E --> C
    style D fill:#ffcc99
```

### 変更後（単一アロケーション）

```mermaid
flowchart TD
    A[build_weighted_matrix\nvalues, valid_indices, weights] --> B[result = vec!\n0.0; n_valid * n_objectives\n単一アロケーション]
    B --> C[valid_indices.iter_enumerate\nidx, original_row]
    C --> D[インデックス代入\nresult[idx*n_objectives + j] = ...]
    D --> C
    style B fill:#99cc99
```

---

## 9. Pearson 相関: 移動フロー 🔵

**信頼性**: 🔵 *REQ-A04・A05・コード分析 sensitivity/spearman.rs より*

```mermaid
flowchart LR
    A[sensitivity/spearman.rs\nfn pearson_correlation\nローカル定義] -->|移動| B[core/math/stats.rs\npub fn pearson_correlation\nx: &amp;f64, y: &amp;f64 → f64]
    B -->|use crate::core::math::stats| C[sensitivity/spearman.rs\n参照に変更]
    style A fill:#ffcc99
    style B fill:#99cc99
```

---

## 10. egui-app 側の変更フロー 🔵

**信頼性**: 🔵 *REQ-C08・ユーザーストーリー C-1 より*

```mermaid
sequenceDiagram
    participant UI as egui-app
    participant AS as AppState
    participant Core as tunny-core

    Note over UI,Core: データロード時
    UI->>Core: journal パース完了
    Core-->>UI: trial_rows, pareto_indices, is_minimize
    UI->>Core: init_sampling(is_minimize, pareto_indices, all_ranks)
    Core-->>UI: SamplingContext
    UI->>AS: sampling_ctx = Some(ctx)

    Note over UI,Core: クラスタリング完了時
    UI->>Core: clustering 実行
    Core-->>UI: ClusterResult
    UI->>AS: sampling_ctx.cluster_labels = Some(labels)

    Note over UI,Core: ダウンサンプリング要求時
    UI->>AS: sampling_ctx を取得
    AS-->>UI: &SamplingContext
    UI->>Core: downsample_smart(&ctx, max_points)
    Core-->>UI: Vec<u32>
```

---

## 関連文書

- **アーキテクチャ**: [architecture.md](architecture.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **ヒアリング記録**: [design-interview.md](design-interview.md)

---

## 信頼性レベルサマリー

- 🔵 青信号: 9件 (90%)
- 🟡 黄信号: 1件 (10%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
