# Entropy Weight Method アーキテクチャ設計

**作成日**: 2026-04-24
**関連要件定義**: [requirements.md](../../spec/entropy-weight/requirements.md)
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *要件定義書・既存MCDMアーキテクチャより*

Entropy Weight Methodは新しいMCDM手法ではなく、既存手法（TOPSIS/VIKOR）の重みを自動算出する前処理機能。Shannonエントロピーに基づいて各目的のデータ分散を評価し、分散が大きい目的ほど大きな重みを付与する。既存の4層アーキテクチャに「重み計算」レイヤーを追加する形で統合する。

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存TOPSIS/VIKOR実装パターン・ユーザヒアリングより*

- **パターン**: 4層レイヤードアーキテクチャ（計算→状態型→ディスパッチ→UI）に重み計算レイヤーを追加
- **選択理由**: Entropy法はMCDM手法ではなく重み決定手法であるため、既存McdmMethod enumには追加せず独立したWeightModeとして管理する

```
Layer 0: rust_core/src/mcdm/entropy.rs
         純粋Rustアルゴリズム（外部依存なし）
         ↓ compute_entropy_weights() → EntropyResult
Layer 1: egui-app/src/state/results.rs
         WeightMode enum / EntropyResult型 / McdmRankChartへのweight_modeフィールド追加
         ↓ WeightMode切替 → エントロピー計算リクエスト
Layer 2: egui-app/src/ui/chart_registry.rs
         WeightMode::Entropy時にエントロピー計算を spawn_task で実行
         ↓ 結果を McdmRankChart.weights に反映
Layer 3: egui-app/src/ui/widgets/mcdm_chart.rs
         WeightModeセレクタ / エントロピーテーブル / スライダー制御
```

---

## コンポーネント構成

### Layer 0: 計算コア（`rust_core`） 🔵

**信頼性**: 🔵 *REQ-001〜REQ-404・既存topsis.rs/vikor.rs実装パターンより*

**新規ファイル:** `rust_core/src/mcdm/entropy.rs`

```rust
// 公開型
pub struct EntropyResult {
    pub weights: Vec<f64>,           // 正規化済みエントロピー重み（sum = 1.0）
    pub entropies: Vec<f64>,         // 各目的の情報エントロピー値 e_j
    pub diversities: Vec<f64>,       // 各目的の分散度 d_j = 1 - e_j
    pub normalized_matrix: Vec<f64>, // 正規化行列 p_ij（表示用）
    pub duration_ms: f64,            // 計算時間
}

// 公開関数
pub fn compute_entropy_weights(
    values: &[f64],
    n_trials: usize,
    n_objectives: usize,
) -> Result<EntropyResult, String>
```

**変更ファイル:** `rust_core/src/mcdm/mod.rs`

```rust
pub mod topsis;
pub mod vikor;
pub mod entropy;  // 追加
```

**変更ファイル:** `rust_core/src/lib.rs`

```rust
pub use mcdm::entropy;  // 追加
```

### Layer 1: 状態型（`egui-app/src/state/results.rs`） 🔵

**信頼性**: 🔵 *REQ-005・REQ-006・ユーザヒアリングより*

```rust
// 新規追加: WeightMode enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeightMode {
    Manual,
    Entropy,
}

// 新規追加: エントロピー計算結果（UI表示用）
#[derive(Debug, Clone)]
pub struct EntropyResult {
    pub weights: Vec<f64>,
    pub entropies: Vec<f64>,
    pub diversities: Vec<f64>,
    pub normalized_matrix: Vec<f64>,
    pub duration_ms: f64,
}
```

### Layer 2: コンピュートディスパッチ（`egui-app/src/ui/chart_registry.rs`） 🔵

**信頼性**: 🔵 *既存chart_registry.rsパターン・ユーザヒアリングより*

**変更点:**
1. WeightMode切替時にエントロピー計算を spawn_task で実行
2. 計算結果を `McdmRankChart` に反映

```rust
// chart_registry.rs に追加
// WeightMode::Entropy 切替時の処理
if let Some(pending) = widgets.mcdm_chart.pending_entropy.take() {
    // spawn_task で compute_entropy_weights() を実行
    // 結果を AppMessage::EntropyDone(EntropyResult) で送信
}
```

### Layer 3: UIウィジェット（`egui-app/src/ui/widgets/mcdm_chart.rs`） 🔵

**信頼性**: 🔵 *REQ-003・REQ-004・REQ-006・ユーザヒアリングより*

**変更点:**

1. `McdmRankChart` に以下のフィールドを追加:
   - `weight_mode: WeightMode`
   - `entropy_result: Option<EntropyResult>` — エントロピー計算結果キャッシュ
   - `pending_entropy: bool` — エントロピー計算要求フラグ

2. WeightModeセレクタ: 手法セレクタ（TOPSIS/VIKOR）の横に配置

3. エントロピーテーブル: Weights collapsingセクション内に表示（WeightMode::Entropy時のみ）

---

## ディレクトリ構造 🔵

**信頼性**: 🔵 *既存プロジェクト構造より*

**変更ファイル一覧:**

```
rust_core/src/
├── mcdm/
│   ├── mod.rs           ← pub mod entropy; 追加
│   ├── topsis.rs        ← 変更なし
│   ├── vikor.rs         ← 変更なし
│   └── entropy.rs       ← 新規作成
├── lib.rs               ← pub use mcdm::entropy; 追加

egui-app/src/
├── state/
│   └── results.rs       ← WeightMode / EntropyResult 追加
└── ui/
    ├── chart_registry.rs ← Entropy dispatch / AppMessage拡張 追加
    ├── message_handler.rs ← AppMessage::EntropyDone ハンドラ追加
    └── widgets/
        └── mcdm_chart.rs ← weight_mode / entropy_result / WeightModeセレクタ / エントロピーテーブル 追加
```

---

## 非機能要件の実現方法

### パフォーマンス 🔵

**信頼性**: 🔵 *NFR-001・既存MCDMパフォーマンス要件より*

- フラット行列（Vec<f64>）を使い n_valid * n_objectives の連続メモリ確保
- 比例正規化→エントロピー計算→分散度→重みを1パスで処理可能
- 目標: 50,000試行 × 4目的 100ms以内

### 堅牢性 🔵

**信頼性**: 🔵 *REQ-101〜REQ-104・既存MCDMパターンより*

- `filter_valid_indices()` でNaN試行を除外（TOPSIS/VIKORと共通）
- ゼロ除算ガード: p_ij = 0 の項は 0 として扱う（Shannonエントロピー定義）
- 全目的 d_j = 0 の場合、均等重み 1/n を返す
- 負の値含む場合は min-max 正規化で前処理

### UI応答性 🔵

**信頼性**: 🔵 *NFR-002・既存spawn_taskパターンより*

- WeightMode切替時に spawn_task でバックグラウンド計算
- 計算中は spinner を表示
- Run ボタン（MCDM計算）とは独立して動作

---

## 技術的制約

### アーキテクチャ制約 🔵

**信頼性**: 🔵 *既存コードベース制約・note.md開発ルールより*

- 外部線形代数ライブラリ（nalgebra等）使用禁止
- `Result<T, String>` エラー型
- `#[derive(Debug, Clone, serde::Serialize)]` 付与（EntropyResult）
- テスト命名規則: `tc_entropy_<seq>_<description>`

### 後方互換性 🔵

**信頼性**: 🔵 *featura/eguiブランチ方針より*

- 後方互換性不要
- 既存TOPSIS/VIKORテストは変更なし

---

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **設計ヒアリング**: [design-interview.md](design-interview.md)
- **要件定義**: [requirements.md](../../spec/entropy-weight/requirements.md)
- **既存VIKOR設計**: [../vikor/architecture.md](../vikor/architecture.md)

## 信頼性レベルサマリー

- 🔵 青信号: 15件 (94%)
- 🟡 黄信号: 1件 (6%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
