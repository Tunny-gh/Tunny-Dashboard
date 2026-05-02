# Permutation Feature Importance データフロー図

**作成日**: 2026-05-02
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連要件定義**: [requirements.md](../../spec/permutation-feature-importance/requirements.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実なフロー
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測によるフロー
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測によるフロー

---

## システム全体のデータフロー 🔵

**信頼性**: 🔵 *既存 chart_registry.rs の ImportanceChart ディスパッチパターン・ユーザヒアリングより*

```mermaid
flowchart TD
    U[ユーザー: Permutation を選択して Run]
    IC[ImportanceChart widget]
    CR[chart_registry.rs]
    ST[spawn_task バックグラウンドスレッド]
    RC[rust_core: compute_sensitivity_single_obj]
    PFI[rust_core: compute_permutation_importances]
    MH[message_handler.rs]
    AC[app_state.importance_cache]

    U --> IC
    IC --> CR
    CR --> ST
    ST --> RC
    RC --> PFI
    PFI --> ST
    ST --> MH
    MH --> AC
    AC --> IC
    IC --> U
```

---

## 主要機能のデータフロー

### 機能 1: Permutation 計算トリガー 🔵

**信頼性**: 🔵 *既存 ImportanceChart の pending_compute パターン・ユーザヒアリングより*

**関連要件**: REQ-PFI-005, REQ-PFI-006

```mermaid
sequenceDiagram
    participant U as ユーザー
    participant IC as ImportanceChart
    participant CR as chart_registry
    participant SP as spawn_task
    participant RC as rust_core
    participant MH as MessageHandler

    U->>IC: "Permutation" を選択 → "Run" をクリック
    IC->>IC: pending_compute = Some((Permutation, obj_idx))
    IC->>IC: computing = true

    Note over IC: 次フレームで chart_registry が pending_compute を処理

    CR->>IC: widgets.importance.pending_compute.take()
    CR->>CR: already_cached? (7, obj_idx) → false

    CR->>CR: trial_rows から DataFrame を構築
    CR->>SP: spawn_task(tx, move || { ... })

    SP->>RC: compute_sensitivity_single_obj(&df, Permutation, 0)
    RC->>RC: x_matrix, y を構築
    RC->>RC: compute_permutation_importances(x_matrix, y)

    Note over RC: アルゴリズム実行（5〜5,000ms）

    RC-->>SP: SensitivityResult { permutation: Some(...) }
    SP->>SP: AppMessage::SensitivityDone { key: (7, obj_idx), result }
    SP->>MH: tx.send(msg)

    MH->>MH: app_state.importance_cache.insert((7, obj_idx), result)
    MH->>IC: widgets.importance.computing = false
    IC-->>U: バーチャートを描画
```

**詳細ステップ**:
1. ユーザーがドロップダウンで "Permutation" を選択し "Run" ボタンをクリック
2. `ImportanceChart` が `pending_compute = Some((Permutation, obj_idx))` をセット
3. 次フレームで `chart_registry.show_chart()` が `pending_compute.take()` を実行
4. キャッシュ `(7, obj_idx)` が未存在であれば `spawn_task` でバックグラウンド計算を開始
5. `compute_sensitivity_single_obj` が `compute_permutation_importances` を呼び出す
6. 計算完了後 `AppMessage::SensitivityDone` を送信
7. `message_handler` がキャッシュに格納し、`computing = false` にセット
8. 次フレームでバーチャートが描画される

---

### 機能 2: rust_core 内部のアルゴリズムフロー 🔵

**信頼性**: 🔵 *ユーザヒアリング（n_repeats=5, 平均MSE増加量）+ 既存 rf_anova.rs パターンより*

**関連要件**: REQ-PFI-002

```mermaid
flowchart TD
    subgraph "compute_permutation_importances(x_matrix, y)"
        A["NaN/Inf フィルタリング\n(valid_indices 収集)"]
        B{"有効行 < 2?"}
        C["ダウンサンプリング\n(MAX_ROWS=2,000, seed=42)"]
        D["80/20 holdout 分割\n(Fisher-Yates, seed=43)"]
        E["LightGBM RF 学習\n(train_lgbm_rf, 100 trees, seed=42)"]
        F["baseline_mse = lgbm_mse(booster, x_eval, y_eval)"]
        G{"feature_idx loop\n0..p"}
        H{"repeat_idx loop\n0..N_REPEATS(=5)"}
        I["seed = 42 + feat*5 + rep\npermuted = permute_single_column(x_eval, feat, seed)"]
        J["delta = max(lgbm_mse(booster,permuted,y_eval) - baseline_mse, 0.0)"]
        K["importances[feat] += delta"]
        L["importances[feat] /= N_REPEATS"]
        M["normalize (sum=1.0)"]
        N["r2 = mse_to_r_squared(baseline_mse, y_eval)"]
        O["return (importances, r2)"]
        EARLY["return (vec![0.0;p], 0.0)"]
    end

    A --> B
    B -->|Yes| EARLY
    B -->|No| C
    C --> D --> E --> F --> G
    G --> H --> I --> J --> K
    K -->|repeat_idx+1| H
    H -->|done| L
    L -->|feat+1| G
    G -->|done| M --> N --> O
```

**シード計算の詳細**:

```
feature_idx=0, repeat_idx=0: seed = 42
feature_idx=0, repeat_idx=1: seed = 43
feature_idx=0, repeat_idx=2: seed = 44
feature_idx=0, repeat_idx=3: seed = 45
feature_idx=0, repeat_idx=4: seed = 46
feature_idx=1, repeat_idx=0: seed = 47
feature_idx=1, repeat_idx=1: seed = 48
...
feature_idx=p-1, repeat_idx=4: seed = 42 + (p-1)*5 + 4
```

---

### 機能 3: ImportanceChart 表示フロー 🔵

**信頼性**: 🔵 *既存 ImportanceChart.show() の compute_sorted_importance パターンより*

**関連要件**: REQ-PFI-005-F, REQ-PFI-005-G

```mermaid
flowchart TD
    subgraph "importance_chart.rs show()"
        M{metric の判定}
        S1[Spearman: spearman[obj_idx]の絶対値]
        S2[Ridge: ridge[obj_idx].beta の絶対値]
        S3[RfAnova: rf_anova.importances の [param][obj_idx]]
        S4[Mdi: mdi.importances の [param][obj_idx]]
        S5[Shap: shap.importances の [param][obj_idx]]
        S6[Permutation: permutation.importances の [param][obj_idx]]
        SORT[param_names と scores を zip して降順ソート]
        CHART[水平バーチャート描画\n+ R² 表示（右端）]
    end

    M -->|Spearman| S1
    M -->|Ridge| S2
    M -->|RfAnova| S3
    M -->|Mdi| S4
    M -->|Shap| S5
    M -->|Permutation| S6
    S1 --> SORT --> CHART
    S2 --> SORT
    S3 --> SORT
    S4 --> SORT
    S5 --> SORT
    S6 --> SORT
```

**`compute_sorted_importance` の Permutation ケース追加**:

```rust
ImportanceMetric::Permutation => {
    let Some(ref perm) = result.permutation else {
        return vec![];
    };
    perm.importances
        .iter()
        .map(|param_imp| param_imp.get(obj_idx).copied().unwrap_or(0.0).abs())
        .collect()
}
```

---

## データ処理パターン

### 非同期処理（バックグラウンドスレッド） 🔵

**信頼性**: 🔵 *既存 spawn_task パターンより*

`compute_permutation_importances()` は最大 5,000ms かかる可能性があるため、必ずバックグラウンドスレッドで実行する。
`spawn_task(tx, move || { ... })` パターンを使用し、UI スレッドをブロックしない。

### キャッシング戦略 🔵

**信頼性**: 🔵 *既存 importance_cache パターンより*

```mermaid
stateDiagram-v2
    [*] --> Uncached: Study ロード時
    Uncached --> Computing: Run ボタンクリック (cache miss)
    Computing --> Cached: SensitivityDone 受信
    Cached --> Cached: Run ボタン再クリック (cache hit → no-op)
    Cached --> Uncached: Study 変更 (app_state.clear())
```

- `app_state.importance_cache: HashMap<(u8, usize), SensitivityResult>`
- キー: `(cache_id=7, obj_idx)`
- Study 変更時: `app_state.clear()` で全キャッシュを一括破棄

### エラーハンドリングフロー 🟡

**信頼性**: 🟡 *既存 SensitivityError パターンから妥当な推測*

```mermaid
flowchart TD
    PFI[compute_permutation_importances]
    E1{"有効行 < 2\n(NaN/Inf 除去後)"}
    E2{"p = 0\n(パラメータなし)"}
    E3{"LightGBM 学習失敗\n(train_lgbm_rf → None)"}
    E4{"全 importance = 0\n(baseline ≈ permuted)"}
    OK["(importances_normalized, r2)"]
    R1["(vec![0.0; p], 0.0)"]
    R2["(vec![], 0.0)"]
    R3["(vec![0.0; p], 0.0)"]
    R4["全 0.0 正規化ロジックで処理"]

    PFI --> E1
    E1 -->|Yes| R1
    E1 -->|No| E2
    E2 -->|Yes| R2
    E2 -->|No| E3
    E3 -->|Yes| R3
    E3 -->|No| E4
    E4 -->|Yes| R4
    E4 -->|No| OK
```

---

## 状態遷移（egui-app 側） 🔵

**信頼性**: 🔵 *既存 ImportanceChart.computing / pending_compute パターンより*

```mermaid
stateDiagram-v2
    [*] --> Idle
    Idle --> PendingCompute: Run ボタンクリック\n(pending_compute = Some, computing = true)
    PendingCompute --> BackgroundComputing: chart_registry が spawn_task
    BackgroundComputing --> Idle: SensitivityDone 受信\n(computing = false, cache に格納)
    BackgroundComputing --> Idle: SensitivityError 受信\n(computing = false)
    Idle --> Idle: Run ボタン再クリック（キャッシュヒット）\ncomputing = false 即時
```

---

## 関連文書

- **アーキテクチャ**: [architecture.md](architecture.md)
- **実装ガイド**: [implementation-guide.md](implementation-guide.md)
- **設計ヒアリング記録**: [design-interview.md](design-interview.md)
- **要件定義**: [requirements.md](../../spec/permutation-feature-importance/requirements.md)

---

## 信頼性レベルサマリー

- 🔵 青信号: 8件 (89%)
- 🟡 黄信号: 1件 (11%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
