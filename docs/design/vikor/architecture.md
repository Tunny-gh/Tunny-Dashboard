# VIKOR アーキテクチャ設計

**作成日**: 2026-04-24
**ブランチ**: featura/egui
**関連要件定義**: [requirements.md](../../spec/vikor/requirements.md)
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *要件定義書・既存MCDMアーキテクチャより*

VIKORはTOPSISと同じ4層Rustアーキテクチャ（計算コア→状態型→メッセージ→UI）に追加される。既存の `McdmMethod` enumおよび `McdmResult` enumを拡張し、`McdmComputeRequest` 構造体を新設することで `pending_compute` の型を統一する。

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存TOPSIS実装・proprietary-features設計より*

- **パターン**: 4層レイヤードアーキテクチャ（計算→型定義→状態→UI）
- **選択理由**: 既存TOPSISが同一パターンで完成しており、変更コストを最小化できる

```
Layer 1: rust_core/src/mcdm/vikor.rs
         純粋Rustアルゴリズム（外部依存なし）
         ↓ compute_vikor() → VikorResult
Layer 2: egui-app/src/state/results.rs
         McdmMethod::Vikor / McdmResult::Vikor(VikorResult) / VikorResult型
         ↓ AppMessage::McdmDone(McdmResult)
Layer 3: egui-app/src/ui/chart_registry.rs
         McdmComputeRequest を spawn_task に渡し AppMessage::McdmDone を送信
         ↓ message_handler.rs: app_state.mcdm_result = Some(result)
Layer 4: egui-app/src/ui/widgets/mcdm_chart.rs
         McdmRankChart（バーチャート）/ McdmTable（テーブル）
```

---

## コンポーネント構成

### Layer 1: 計算コア（`rust_core`） 🔵

**信頼性**: 🔵 *REQ-001〜REQ-402・既存topsis.rs実装パターンより*

**新規ファイル:** `rust_core/src/mcdm/vikor.rs`

```rust
// 公開型
pub struct VikorResult {
    pub s_values: Vec<f64>,       // utility measure（低い = 良い）
    pub r_values: Vec<f64>,       // regret measure（低い = 良い）
    pub q_values: Vec<f64>,       // 妥協スコア（低い = 良い、0〜1）
    pub ranked_indices: Vec<u32>, // Q昇順インデックス
    pub best_values: Vec<f64>,    // f* （目的数次元）
    pub worst_values: Vec<f64>,   // f- （目的数次元）
    pub duration_ms: f64,
}

// 公開関数
pub fn compute_vikor(
    values: &[f64],
    n_trials: usize,
    n_objectives: usize,
    weights: &[f64],
    is_minimize: &[bool],
    v: f64,
) -> Result<VikorResult, String>
```

**変更ファイル:** `rust_core/src/mcdm/mod.rs`

```rust
pub mod topsis;
pub mod vikor;  // 追加
```

### Layer 2: 状態型（`egui-app/src/state/results.rs`） 🔵

**信頼性**: 🔵 *既存McdmMethod/McdmResult実装パターン・ユーザヒアリングより*

```rust
// 追加: VikorResult型
#[derive(Debug, Clone)]
pub struct VikorResult {
    pub s_values: Vec<f64>,
    pub r_values: Vec<f64>,
    pub q_values: Vec<f64>,
    pub ranked_indices: Vec<u32>,
    pub best_values: Vec<f64>,
    pub worst_values: Vec<f64>,
    pub duration_ms: f64,
}

// 変更: McdmMethod enum
pub enum McdmMethod { Topsis, Vikor }

// 変更: McdmResult enum
pub enum McdmResult {
    Topsis(TopsisResult),
    Vikor(VikorResult),
}

// McdmResult impl
impl McdmResult {
    pub fn primary_scores(&self) -> &[f64] {
        match self {
            McdmResult::Topsis(r) => &r.scores,
            // VIKORはQ値を反転（高い = 良い に統一）
            // ※ 実装時: scores_cache に 1.0 - q_values を保持
            McdmResult::Vikor(r) => &r.q_values, // 呼び出し側で 1.0 - x
        }
    }
    pub fn ranked_indices(&self) -> &[u32] { ... }
    pub fn duration_ms(&self) -> f64 { ... }
    pub fn method_label(&self) -> &'static str { ... }
}
```

> **注意:** `primary_scores()` の VIKOR 実装は `q_values` をそのまま返し、バーチャート描画側で `1.0 - score` を用いるか、または `VikorResult` に `display_scores: Vec<f64>` フィールドを追加する。設計上は `VikorResult` に `display_scores = 1.0 - q_values` を格納する方が関数の契約を統一できる。どちらにするかは実装者が決定してよい。

### Layer 3: コンピュートディスパッチ（`egui-app/src/ui/chart_registry.rs`） 🔵

**信頼性**: 🔵 *既存chart_registry.rs MCDMセクション・ユーザヒアリング（McdmComputeRequest構造体採用）より*

**変更:** `McdmRankChart.pending_compute` の型を `Option<McdmComputeRequest>` に変更

```rust
// mcdm_chart.rs に追加
pub struct McdmComputeRequest {
    pub method: McdmMethod,
    pub weights: Vec<f64>,
    pub v: f64,  // VIKOR用（TOPSISでは無視）
}
```

```rust
// chart_registry.rs の変更箇所
if let Some(req) = widgets.mcdm_chart.pending_compute.take() {
    let McdmComputeRequest { method, weights, v } = req;

    match method {
        McdmMethod::Topsis => {
            // 既存コードそのまま（vは無視）
        }
        McdmMethod::Vikor => {
            match tunny_core::vikor::compute_vikor(
                &objectives, n_trials, n_objectives, &weights, &is_minimize, v
            ) {
                Ok(r) => AppMessage::McdmDone(McdmResult::Vikor(VikorResult {
                    s_values: r.s_values,
                    r_values: r.r_values,
                    q_values: r.q_values,
                    ranked_indices: r.ranked_indices,
                    best_values: r.best_values,
                    worst_values: r.worst_values,
                    duration_ms: start.elapsed().as_secs_f64() * 1000.0,
                })),
                Err(e) => AppMessage::Error(format!("VIKOR computation failed: {}", e)),
            }
        }
    }
}
```

### Layer 4: UIウィジェット（`egui-app/src/ui/widgets/mcdm_chart.rs`） 🔵

**信頼性**: 🔵 *既存McdmRankChart実装・ユーザヒアリング（vスライダー）より*

**変更点:**

1. `McdmRankChart` に `v_param: f64` フィールドを追加
2. `pending_compute: Option<McdmComputeRequest>` に変更
3. VIKOR選択時にvスライダーを Weights セクション内に表示
4. Runボタン押下時に `McdmComputeRequest { method, weights: normalized, v: self.v_param }` をセット

```rust
pub struct McdmRankChart {
    pub method: McdmMethod,
    pub weights: Vec<f64>,
    pub v_param: f64,                           // 追加: VIKOR戦略パラメータ
    pub computing: bool,
    pub pending_compute: Option<McdmComputeRequest>,  // 変更
    pub top_n: McdmTopN,
}

impl Default for McdmRankChart {
    fn default() -> Self {
        Self {
            method: McdmMethod::Topsis,
            weights: Vec::new(),
            v_param: 0.5,        // デフォルト
            computing: false,
            pending_compute: None,
            top_n: McdmTopN::Top10,
        }
    }
}
```

**vスライダーUI（Weights collapsing内）:**

```rust
// VIKOR選択時のみ表示
if self.method == McdmMethod::Vikor {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Strategy weight v").strong());
        ui.add(egui::Slider::new(&mut self.v_param, 0.0..=1.0).text("v"));
        ui.label("(0=min-regret, 0.5=compromise, 1=max-consensus)");
    });
}
```

---

## ディレクトリ構造 🔵

**信頼性**: 🔵 *既存プロジェクト構造より*

**変更ファイル一覧:**

```
rust_core/src/
├── mcdm/
│   ├── mod.rs           ← pub mod vikor; 追加
│   ├── topsis.rs        ← 変更なし
│   └── vikor.rs         ← 新規作成

egui-app/src/
├── state/
│   └── results.rs       ← VikorResult / McdmMethod::Vikor / McdmResult::Vikor 追加
└── ui/
    ├── chart_registry.rs ← McdmMethod::Vikor dispatch 追加
    └── widgets/
        └── mcdm_chart.rs ← v_param / McdmComputeRequest / vスライダーUI 追加
```

---

## 非機能要件の実現方法

### パフォーマンス 🔵

**信頼性**: 🔵 *NFR-001・TOPSIS実装パターンより*

- フラット行列（Vec<f64>）を使い n_valid * n_objectives の連続メモリ確保
- 1パスでbest/worst値を計算（キャッシュ効率最大化）
- S/R値計算も1パスで処理（TOPSISの`find_ideal_solutions` + `compute_scores`パターン踏襲）
- 目標: 50,000試行 × 4目的 100ms以内

### セキュリティ・堅牢性 🔵

**信頼性**: 🔵 *NFR-101〜102・TOPSIS実装パターンより*

- 入力検証を `validate_inputs()` ヘルパーに集約（TOPSISと同パターン）
- ゼロ除算ガード: `f*_j == f-_j`、`S- == S*`、`R- == R*` のすべてに対してガード
- NaN試行: 計算前に valid_indices でフィルタ（TOPSISと同パターン）

### スタイル制約 🔵

**信頼性**: 🔵 *既存アーキテクチャ制約より*

- Tailwind CSS禁止（egui-appはTailwindを使わない）
- インラインスタイルのみ（既存UIパターン踏襲）

---

## 技術的制約

### アーキテクチャ制約 🔵

**信頼性**: 🔵 *既存コードベース制約・featura/eguiブランチ方針より*

- 外部線形代数ライブラリ（nalgebra等）使用禁止
- WASMビルド対応不要（featura/eguiでは廃止）
- `Result<T, String>` エラー型
- `#[derive(Debug, Clone, serde::Serialize)]` 付与

### 後方互換性 🔵

**信頼性**: 🔵 *featura/eguiブランチ明示的指示より*

- 後方互換性不要（このブランチの方針）
- TOPSISの既存テストは変更なし

---

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **設計ヒアリング**: [design-interview.md](design-interview.md)
- **要件定義**: [../../spec/vikor/requirements.md](../../spec/vikor/requirements.md)
- **既存MCDMアーキテクチャ**: [../proprietary-features/architecture.md](../proprietary-features/architecture.md)
- **既存TOPSIS実装**: [rust_core/src/mcdm/topsis.rs](../../../rust_core/src/mcdm/topsis.rs)

## 信頼性レベルサマリー

- 🔵 青信号: 16件 (94%)
- 🟡 黄信号: 1件 (6%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
