# AHP アーキテクチャ設計

**作成日**: 2026-04-29
**関連要件定義**: [requirements.md](../../spec/ahp/requirements.md)
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:

- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 _要件定義書概要・ユーザヒアリングより_

既存の MCDM 機能（TOPSIS / VIKOR / PROMETHEE I/II）とは独立した AHP（Analytic Hierarchy Process）チャートを新規追加する。

- 一対比較行列（Saaty 1-9 スケール）から優先度ベクトルを固有ベクトル近似法で導出する
- 整合性比率（CR）を計算し、CR > 0.10 の場合に警告を表示する（計算は続行）
- 加重和法 + Min-Max 正規化でトライアルをスコアリングし、降順にランキングする
- UI は **既存 McdmChart とは完全に独立した新規ウィジェット** `AhpChart` / `AhpTable` として実装する

---

## アーキテクチャパターン 🔵

**信頼性**: 🔵 _既存 egui-app アーキテクチャ・ユーザヒアリングより_

- **パターン**: 4 層メッセージパッシングアーキテクチャ（既存 TOPSIS/VIKOR/PROMETHEE パターン踏襲）
- **選択理由**: 既存 MCDM 実装が同一フローで完成しており開発コストが低い。AHP 固有の一対比較行列 UI のために独立チャートとする

```
Layer 1: アルゴリズム層 (rust_core/src/mcdm/ahp.rs)
  ↓ tunny_core::mcdm::ahp::compute_ahp()
Layer 2: 型・状態管理層 (egui-app/src/state/)
  ↓ AppMessage::AhpDone(AhpResult)
Layer 3: タスク起動層 (egui-app/src/ui/chart_registry.rs)
  ↓ spawn_task → pending_compute.take()
Layer 4: UI 描画層 (egui-app/src/ui/widgets/ahp_chart.rs)
```

---

## コンポーネント構成

### Layer 1: アルゴリズム層 🔵

**信頼性**: 🔵 _REQ-AHP-001〜008・既存 topsis.rs / vikor.rs パターンより_

**新規ファイル**: `rust_core/src/mcdm/ahp.rs`

| 要素                  | 詳細                                                                                                        |
| --------------------- | ----------------------------------------------------------------------------------------------------------- |
| 公開関数              | `compute_ahp(values, n_trials, n_objectives, pairwise_matrix, is_minimize) -> Result<AhpResult, String>`    |
| 入力: pairwise_matrix | 上三角のみ (row-major, len = n\*(n-1)/2); 下三角は関数内で逆数補完                                          |
| 優先度ベクトル        | 固有ベクトル近似法: 列正規化 → 行平均                                                                       |
| λmax 計算             | `Σ_j (A のj列合計 × w[j])`                                                                                  |
| CI / RI / CR          | CI=(λmax-n)/(n-1), RI=[0.00,0.00,0.58,0.90,1.12], CR=CI/RI                                                  |
| スコア計算            | Min-Max 正規化（方向考慮）+ 加重和                                                                          |
| NaN 処理              | `filter_valid_indices` 既存関数流用; NaN トライアルはスコア=0.0でランキング末尾                             |
| バリデーション        | `validate_inputs` 既存関数流用                                                                              |
| 出力型                | `AhpResult { priority_vector, scores, ranked_indices, lambda_max, ci, ri, cr, is_consistent, duration_ms }` |

**RI テーブル**:

```rust
const RI: [f64; 6] = [0.0, 0.0, 0.58, 0.90, 1.12, 1.24];
//                    n=1  n=2  n=3   n=4   n=5   n≥6(近似)
```

**変更ファイル**:

- `rust_core/src/mcdm/mod.rs`: `pub mod ahp;` 追加
- `rust_core/src/lib.rs`: `pub use mcdm::ahp;` 追加（既存パターン確認要）

### Layer 2: 型・状態管理層 🔵

**信頼性**: 🔵 _REQ-AHP-010〜014・既存 results.rs / messages.rs パターンより_

**変更ファイル**: `egui-app/src/state/results.rs`

```rust
// AhpResult: McdmResult とは完全に独立した新規型
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AhpResult {
    pub priority_vector: Vec<f64>,  // [n_objectives] 各目的関数の重み
    pub scores: Vec<f64>,           // [n_trials] AHP スコア（NaN→0.0）
    pub ranked_indices: Vec<u32>,   // score 降順、NaN 末尾
    pub lambda_max: f64,
    pub ci: f64,
    pub ri: f64,
    pub cr: f64,
    pub is_consistent: bool,        // cr <= 0.10
    pub duration_ms: f64,
}
```

**変更ファイル**: `egui-app/src/state/messages.rs`

```rust
// AppMessage に AhpDone を独立追加 (McdmDone とは別)
pub enum AppMessage {
    // ... 既存 ...
    McdmDone(McdmResult),
    AhpDone(AhpResult),  // ← 新規追加
    // ...
}
```

**変更ファイル**: `egui-app/src/state/app_state.rs`

```rust
pub struct AppState {
    // ... 既存フィールド ...
    pub mcdm_result: Option<McdmResult>,
    pub ahp_result: Option<AhpResult>,  // ← 新規追加
}
```

**変更ファイル**: `egui-app/src/state/message_handler.rs`

```rust
AppMessage::AhpDone(result) => {
    widget_states.ahp_chart.computing = false;
    app_state.ahp_result = Some(result);
}
```

### Layer 3: タスク起動層 🔵

**信頼性**: 🔵 _既存 chart_registry.rs の pending_compute パターン・ユーザヒアリングより_

**変更ファイル**: `egui-app/src/ui/chart_registry.rs`

```rust
// ChartId::AhpRankChart ブランチで AhpChart の pending_compute を処理
if let Some(req) = widget_states.ahp_chart.pending_compute.take() {
    let tx = tx.clone();
    let objectives = req.objectives;
    let n_trials = req.n_trials;
    let n_objectives = req.n_objectives;
    let pairwise_matrix = req.pairwise_matrix;
    let is_minimize = req.is_minimize;
    widget_states.ahp_chart.computing = true;
    crate::app::spawn_task(tx, move || {
        match tunny_core::mcdm::ahp::compute_ahp(
            &objectives, n_trials, n_objectives, &pairwise_matrix, &is_minimize,
        ) {
            Ok(r) => AppMessage::AhpDone(AhpResult {
                priority_vector: r.priority_vector,
                scores: r.scores,
                ranked_indices: r.ranked_indices,
                lambda_max: r.lambda_max,
                ci: r.ci,
                ri: r.ri,
                cr: r.cr,
                is_consistent: r.is_consistent,
                duration_ms: r.duration_ms,
            }),
            Err(e) => AppMessage::Error(format!("AHP computation failed: {e}")),
        }
    });
}
```

**変更ファイル**: `egui-app/src/state/layout_state.rs`

```rust
pub enum ChartId {
    // ... 既存 ...
    McdmRankChart,
    McdmTable,
    AhpRankChart,  // ← 新規: 一対比較行列入力 + CR + 優先度ベクトルバーチャート
    AhpTable,      // ← 新規: ランキングテーブル (Top5/10/20)
}
```

### Layer 4: UI 描画層 🔵

**信頼性**: 🔵 _REQ-AHP-020〜027・ユーザヒアリング・既存 mcdm_chart.rs パターンより_

**新規ファイル**: `egui-app/src/ui/widgets/ahp_chart.rs`

| 要素                | 詳細                                                                                                             |
| ------------------- | ---------------------------------------------------------------------------------------------------------------- |
| `AhpChart` 構造体   | `pairwise: Vec<f64>` (上三角), `computing: bool`, `pending_compute: Option<AhpComputeRequest>`, `top_n: AhpTopN` |
| `AhpComputeRequest` | `objectives, n_trials, n_objectives, pairwise_matrix, is_minimize`                                               |
| `AhpTopN`           | `enum { Top5, Top10, Top20 }`                                                                                    |
| AhpRankChart        | 一対比較行列グリッド（上三角: DragValue 1-9、下三角: 逆数表示、対角: 1.0）+ CR 表示 + 優先度ベクトルバーチャート |
| AhpTable            | ランキングテーブル（順位 / Trial ID / AHP スコア / 目的関数値列）+ Top5/10/20 切替                               |
| CR 色分け           | CR ≤ 0.10: 緑 (`Color32::GREEN`), CR > 0.10: 赤 (`Color32::RED`)                                                 |
| Study 切替リセット  | `StudySelected` ハンドラで `ahp_chart = AhpChart::default()`                                                     |

**変更ファイル**: `egui-app/src/ui/widget_states.rs`

```rust
pub struct WidgetStates {
    // ... 既存 ...
    pub mcdm_chart: McdmRankChart,
    pub ahp_chart: AhpChart,  // ← 新規追加
}
```

---

## システム構成図 🔵

**信頼性**: 🔵 _既存アーキテクチャ・ユーザヒアリングより_

```
ユーザー操作
  └─ AhpChart.show_rank_chart()           (ChartId::AhpRankChart)
       ├─ 一対比較行列グリッド (DragValue 上三角, 逆数下三角表示)
       └─ "Run" ボタン押下
            ↓ pending_compute = Some(AhpComputeRequest { pairwise_matrix, ... })

chart_registry.rs
  └─ if let Some(req) = widget_states.ahp_chart.pending_compute.take()
       └─ spawn_task(tx, move || {
              compute_ahp(objectives, n_trials, n_obj, pairwise, is_minimize)
              → AppMessage::AhpDone(AhpResult)
          })

message_handler.rs
  └─ AppMessage::AhpDone(result)
       ├─ app_state.ahp_result = Some(result)
       └─ ahp_chart.computing = false

chart_registry.rs (次フレーム)
  ├─ AhpChart.show_rank_chart() → CR表示 + 優先度ベクトルバーチャート
  └─ AhpChart.show_table()      → ランキングテーブル (ChartId::AhpTable)
```

---

## ディレクトリ構造 🔵

**信頼性**: 🔵 _既存プロジェクト構造・note.md より_

**新規ファイル**:

```
rust_core/src/mcdm/
└── ahp.rs                      ← アルゴリズム実装 + テスト

egui-app/src/ui/widgets/
└── ahp_chart.rs                ← AhpChart / AhpTable ウィジェット
```

**変更ファイル**:

```
rust_core/src/mcdm/
└── mod.rs                      ← pub mod ahp; 追加

egui-app/src/state/
├── results.rs                  ← AhpResult 新規追加
├── messages.rs                 ← AppMessage::AhpDone 追加
├── app_state.rs                ← ahp_result フィールド追加
└── message_handler.rs          ← AhpDone 分岐追加

egui-app/src/state/
└── layout_state.rs             ← ChartId::AhpRankChart / AhpTable 追加

egui-app/src/ui/
├── chart_registry.rs           ← AhpRankChart spawn_task 分岐追加
├── widget_states.rs            ← ahp_chart フィールド追加
└── widgets/mod.rs              ← pub mod ahp_chart; 追加
```

---

## 非機能要件の実現方法

### パフォーマンス 🟡

**信頼性**: 🟡 _NFR-AHP-001〜002・O(n²) 固有ベクトル近似特性から妥当な推測_

- 固有ベクトル近似は行列サイズ n ≤ 4（プロジェクト制約）のため計算量は無視できる
- トライアルスコアリングは O(n_trials × n_objectives) で TOPSIS と同等
- 50,000 試行 × 4 目的: 100 ms 以内（TOPSIS と同等目標）

### エラー耐性 🔵

**信頼性**: 🔵 _NFR-AHP-020・既存パターンより_

- `compute_ahp` がエラーを返した場合は `AppMessage::Error(...)` を送信
- UI はエラーメッセージを `egui::Label` で表示し、クラッシュしない
- CR > 0.10 は警告表示のみ（計算は続行、ランキング結果を表示）

### コード規約 🔵

**信頼性**: 🔵 _NFR-AHP-010〜011・既存 topsis.rs パターンより_

- `ahp.rs` のスタイルは `topsis.rs` に準拠（英語コメント、テスト命名 `tc_ahp_XXX_NN`）
- egui-app 側はインラインスタイルのみ（Tailwind CSS 禁止）
- Clippy 警告 0 件を維持

---

## 技術的制約

### 一対比較行列の上三角ストレージ 🔵

**信頼性**: 🔵 _REQ-AHP-021・ユーザヒアリングより_

- `AhpChart.pairwise` は上三角 (row-major) で長さ `n*(n-1)/2`
- n=1: 長さ0（比較不要）, n=2: 1, n=3: 3, n=4: 6
- `ahp.rs` 内でフル行列 `n×n` に展開してから計算する

### McdmResult との完全独立 🔵

**信頼性**: 🔵 _ユーザヒアリングより_

- AHP は `McdmMethod` enum を拡張しない
- `AppMessage::AhpDone` は `AppMessage::McdmDone` とは別バリアント
- `AppState.ahp_result: Option<AhpResult>` は `AppState.mcdm_result` とは別フィールド
- これにより既存 TOPSIS/VIKOR/PROMETHEE のロジックに一切変更が不要

### Study 切替時リセット 🔵

**信頼性**: 🔵 _REQ-AHP-027・既存 StudySelected ハンドラパターンより_

- `AppMessage::StudySelected` ハンドラで `widget_states.ahp_chart = AhpChart::default()` を呼び出す
- `pairwise` は全要素 1.0、`computing = false`、`pending_compute = None` にリセット

---

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **実装ガイド**: [implementation-guide.md](implementation-guide.md)
- **設計ヒアリング**: [design-interview.md](design-interview.md)
- **要件定義**: [../../spec/ahp/requirements.md](../../spec/ahp/requirements.md)
- **既存 MCDM 設計**: [../promethee-ranking/architecture.md](../promethee-ranking/architecture.md)
- **コンテキストノート**: [../../spec/ahp/note.md](../../spec/ahp/note.md)

## 信頼性レベルサマリー

- 🔵 青信号: 13 件 (87%)
- 🟡 黄信号: 2 件 (13%)
- 🔴 赤信号: 0 件 (0%)

**品質評価**: ✅ 高品質
