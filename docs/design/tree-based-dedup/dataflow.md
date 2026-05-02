# tree-based-dedup データフロー図

**作成日**: 2026-05-02
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連要件定義**: [requirements.md](../../spec/tree-based-dedup/requirements.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 要件定義書・既存実装を参考にした確実なフロー
- 🟡 **黄信号**: 要件定義書・既存実装から妥当な推測によるフロー
- 🔴 **赤信号**: 要件定義書・既存実装にない推測によるフロー

---

## リファクタリング前後のモジュール依存関係 🔵

**信頼性**: 🔵 *既存 mod.rs・各ファイル use 文より*

### Before（現状）

```
                    ┌─────────────┐
                    │  mod.rs     │
                    │  (re-export)│
                    └──────┬──────┘
           ┌───────────────┼───────────────┐
           ▼               ▼               ▼
    ┌──────────────┐ ┌───────────┐ ┌──────────────┐
    │  rf_anova.rs │ │permutation│ │ types.rs     │
    │  ├ permute() │ │ ├ permute()│ │ RfAnovaResult│
    │  └ normalize │ │ └ normalize│ │ MdiResult    │
    └──────┬───────┘ └─────┬─────┘ │ ShapResult   │
           │               │        │ PermResult   │
           └───────┬───────┘        └──────────────┘
                   ▼
          ┌─────────────────┐
          │ analysis/       │
          │ common.rs       │
          │ ├ transpose_mdi │
          │ ├ transpose_rf  │
          │ ├ transpose_shap│
          │ └ transpose_perm│
          └────────┬────────┘
                   ▼
          ┌─────────────────┐
          │ analysis/full.rs│
          │ (4つの呼び出し) │
          └─────────────────┘
```

### After（リファクタリング後）

```
                    ┌─────────────────┐
                    │  mod.rs         │
                    │  + mod tree_common
                    │  + TreeImportanceResult
                    └────────┬────────┘
           ┌─────────────────┼─────────────────┐
           ▼                 ▼                 ▼
    ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐
    │  rf_anova.rs │  │ permutation  │  │ types.rs        │
    │  import ←    │  │ import ←     │  │ TreeImportance  │
    │  tree_common │  │ tree_common  │  │ + 4 aliases     │
    └──────┬───────┘  └──────┬───────┘  └─────────────────┘
           │                 │
           └────────┬────────┘
                    ▼
           ┌─────────────────┐
           │ tree_common.rs  │
           │ ├ permute()     │
           │ └ normalize     │
           └─────────────────┘

          ┌─────────────────────┐
          │ analysis/common.rs  │
          │ transpose_to_tree   │
          │   _result()         │
          └────────┬────────────┘
                   ▼
          ┌─────────────────────┐
          │ analysis/full.rs    │
          │ (統一された呼び出し) │
          └─────────────────────┘
```

---

## Step 別データフロー

### Step 1: tree_common.rs 新規作成 🔵

**信頼性**: 🔵 *アーキテクチャ設計 Step 1 より*

**関連要件**: REQ-401〜406

```mermaid
flowchart LR
    subgraph Before
        A1[rf_anova.rs<br/>permute_single_column<br/>normalize]
        A2[permutation.rs<br/>permute_single_column<br/>normalize]
    end

    subgraph After
        B1[tree_common.rs<br/>permute_single_column<br/>normalize]
        B2[rf_anova.rs<br/>import from tree_common]
        B3[permutation.rs<br/>import from tree_common]
    end

    A1 -->|移動| B1
    A2 -->|移動| B1
    B1 -.->|pub(crate)| B2
    B1 -.->|pub(crate)| B3
```

### Step 2: types.rs 型エイリアス化 🔵

**信頼性**: 🔵 *アーキテクチャ設計 Step 2 より*

**関連要件**: REQ-001〜004

```mermaid
flowchart TD
    subgraph Before
        T1[RfAnovaResult<br/>importances + r_squared]
        T2[MdiResult<br/>importances + r_squared]
        T3[ShapResult<br/>importances + r_squared]
        T4[PermutationResult<br/>importances + r_squared]
    end

    subgraph After
        T0[TreeImportanceResult<br/>importances + r_squared]
        A1[type RfAnovaResult = TreeImportanceResult]
        A2[type MdiResult = TreeImportanceResult]
        A3[type ShapResult = TreeImportanceResult]
        A4[type PermutationResult = TreeImportanceResult]
    end

    T1 -->|エイリアス化| T0
    T2 -->|エイリアス化| T0
    T3 -->|エイリアス化| T0
    T4 -->|エイリアス化| T0
    T0 --> A1
    T0 --> A2
    T0 --> A3
    T0 --> A4
```

### Step 3: transpose 関数統合 🔵

**信頼性**: 🔵 *アーキテクチャ設計 Step 3 より*

**関連要件**: REQ-201〜203, REQ-301

```mermaid
flowchart LR
    subgraph Before
        C1[transpose_mdi_importances]
        C2[transpose_rf_anova_importances]
        C3[transpose_shap_importances]
        C4[transpose_permutation_importances]
    end

    subgraph After
        D1[transpose_to_tree_result<br/>→ TreeImportanceResult]
    end

    C1 -->|統合| D1
    C2 -->|統合| D1
    C3 -->|統合| D1
    C4 -->|統合| D1
```

full.rs の呼び出し更新フロー:

```mermaid
sequenceDiagram
    participant F as full.rs
    participant C as common.rs

    Note over F,C: Before
    F->>C: transpose_rf_anova_importances(&[imp], r2, p, obj)
    C-->>F: RfAnovaResult

    Note over F,C: After
    F->>C: transpose_to_tree_result(&[imp], r2, p, obj)
    C-->>F: TreeImportanceResult (= RfAnovaResult)
```

### Step 4: egui-app 型統合 🔵

**信頼性**: 🔵 *アーキテクチャ設計 Step 4 より*

**関連要件**: REQ-701, REQ-702

```mermaid
flowchart LR
    subgraph rust_core
        RC[types.rs<br/>TreeImportanceResult<br/>+ 4 aliases]
    end

    subgraph egui_app
        EA[results.rs<br/>TreeImportanceResult<br/>+ 4 aliases]
    end

    RC -.->|パターン参照| EA
```

### Step 5: UI match arm 統合 🔵

**信頼性**: 🔵 *アーキテクチャ設計 Step 5 より*

**関連要件**: REQ-601〜603

```mermaid
flowchart TD
    subgraph Before
        M1[RfAnova arm<br/>unwrap + iter + map]
        M2[Mdi arm<br/>unwrap + iter + map]
        M3[Shap arm<br/>unwrap + iter + map]
        M4[Permutation arm<br/>unwrap + iter + map]
    end

    subgraph After
        H[extract_tree_importance<br/>match 4 variants → Vec f64]
        U[RfAnova|Mdi|Shap|Permutation<br/>→ extract_tree_importance]
    end

    M1 -->|統合| H
    M2 -->|統合| H
    M3 -->|統合| H
    M4 -->|統合| H
    H --> U
```

---

## RF-ANOVA R² 統一のデータフロー 🔵

**信頼性**: 🔵 *要件 REQ-501 より*

```mermaid
sequenceDiagram
    participant RF as rf_anova.rs
    participant P as permutation.rs
    participant L as core/lgbm

    Note over RF,P: Before
    RF->>RF: 独自 R² 計算<br/>(ss_res / ss_tot)
    P->>L: mse_to_r_squared(baseline_mse, y)

    Note over RF,P: After
    RF->>L: mse_to_r_squared(baseline_mse, y)
    P->>L: mse_to_r_squared(baseline_mse, y)
```

---

## エラー処理フロー 🔵

**信頼性**: 🔵 *要件 EDGE-001, EDGE-002 より*

```mermaid
flowchart TD
    A[permute_single_column 呼び出し] --> B{n == 0?}
    B -->|Yes| C[None を返す]
    B -->|No| D[Fisher-Yates シャッフル実行]
    D --> E[Some(permuted_matrix)]

    F[normalize 呼び出し] --> G{sum < EPSILON?}
    G -->|Yes| H[全要素を 0.0 に設定]
    G -->|No| I[各要素を sum で除算]
```

---

## 新メトリクス追加時のフロー（リファクタリング後）🟡

**信頼性**: 🟡 *NFR-101 から妥当な推測*

リファクタリング後、新しい Tree-based メトリクス追加時に必要な変更箇所:

```mermaid
flowchart LR
    A[1. 新メトリクス計算関数<br/>新規ファイル作成] --> B[2. SensitivityMetric<br/>バリアント追加]
    B --> C[3. types.rs<br/>型エイリアス追加のみ]
    C --> D[4. full.rs<br/>dispatch 追加]
    D --> E[5. importance_chart.rs<br/>match に1行追加]
```

**変更削減効果**:
- リファクタリング前: `transpose_*` 関数の新規定義が必要
- リファクタリング後: `TreeImportanceResult` + 型エイリアス1行で完了

---

## 関連文書

- **アーキテクチャ**: [architecture.md](architecture.md)
- **ヒアリング記録**: [design-interview.md](design-interview.md)
- **要件定義**: [requirements.md](../../spec/tree-based-dedup/requirements.md)

---

## 信頼性レベルサマリー

- 🔵 青信号: 9件 (90%)
- 🟡 黄信号: 1件 (10%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質 — 全フローが要件定義書・既存実装に基づく
