# TASK-1615 要件定義: TOPSISアルゴリズム実装

**機能名**: topsis（TOPSISアルゴリズム）
**タスクID**: TASK-1615
**要件名**: mode-frontier-features
**作成日**: 2026-04-04

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 設計文書・タスクファイルを参考にしてほぼ推測していない
- 🟡 **黄信号**: 設計文書・タスクファイルから妥当な推測
- 🔴 **赤信号**: 設計文書にない推測

---

## 1. 機能の概要

**🔵 信頼性**: architecture.md・TASK-1615.md・design-interview.md Q5より

- **何をする機能か**: 多基準意思決定（MCDM）のTOPSIS手法により、N試行×M目的の最適化結果を重み付きスコアでランキングする
- **解決する問題**: Optuna最適化で複数の目的関数が競合する場合に、ユーザが指定した重みに基づいて「どの試行が総合的に最良か」を定量的に判断できる
- **想定ユーザー**: Tunny Dashboard上で多目的最適化結果を分析する設計者
- **システム内位置づけ**: 5層アーキテクチャのLayer 1（WASM Core）— 純粋Rust関数として実装。WASM公開（TASK-1617）・Store（mcdmStore）・UIコンポーネント（TopsisRankingChart）と連携

- **参照設計文書**: `docs/design/mode-frontier-features/architecture.md` TOPSISアルゴリズム概要
- **参照タスク**: `docs/tasks/mode-frontier-features/TASK-1615.md`

---

## 2. 入力・出力の仕様

### 2.1 入力パラメータ 🔵

**参照**: `docs/tasks/mode-frontier-features/TASK-1615.md` 実装詳細・`docs/design/mode-frontier-features/interfaces.ts` TopsisWasmResult

| パラメータ | 型 | 説明 | 制約 |
|-----------|-----|------|------|
| `values` | `&[f64]` | 目的関数値の平坦配列 `[N×M]`（行major: trial0_obj0, trial0_obj1, ...） | `len == n_trials * n_objectives` |
| `n_trials` | `usize` | 試行数 | `>= 1` |
| `n_objectives` | `usize` | 目的関数数 | `>= 1` |
| `weights` | `&[f64]` | 各目的の重み（合計1.0に正規化済みを前提） | `len == n_objectives` |
| `is_minimize` | `&[bool]` | 各目的の最小化フラグ（trueなら最小化） | `len == n_objectives` |

### 2.2 出力値 🔵

**参照**: `docs/design/mode-frontier-features/interfaces.ts` TopsisWasmResult

```rust
pub struct TopsisResult {
    pub scores: Vec<f64>,           // 全試行のスコア（trial順、0〜1、高いほどベター）
    pub ranked_indices: Vec<u32>,   // スコア降順の試行インデックス
    pub positive_ideal: Vec<f64>,   // 正理想解（目的数次元）
    pub negative_ideal: Vec<f64>,   // 負理想解（目的数次元）
    pub duration_ms: f64,           // 計算時間（ms）
}
```

戻り値: `Result<TopsisResult, String>`
- 成功: `Ok(TopsisResult)`
- 失敗: `Err(エラーメッセージ)` — 入力検証エラー時

### 2.3 スコアの意味 🔵

- スコア範囲: 0.0〜1.0（1.0に近いほど理想的）
- 計算式: `score_i = D⁻ᵢ / (D⁺ᵢ + D⁻ᵢ)`
  - `D⁺` = 正理想解へのユークリッド距離
  - `D⁻` = 負理想解へのユークリッド距離
- `D⁺ + D⁻ == 0` の場合（全試行が同一値）: score = 0.5

---

## 3. 制約条件

### 3.1 パフォーマンス要件 🔵

**参照**: `docs/tasks/mode-frontier-features/TASK-1615.md` 完了条件

- 50,000試行 × 4目的で **100ms 以内**
- 外部線形代数ライブラリ（nalgebra等）不使用（既存コードベース制約）
- 手動実装の行列演算で対応

### 3.2 セキュリティ・堅牢性要件 🔵

**参照**: `docs/tasks/mode-frontier-features/TASK-1615.md` NaN処理

- NaN値を含む試行: 計算から除外、スコア=0.0、ranked_indices末尾に配置
- ゼロ除算: D⁺ + D⁻ == 0 の場合 score = 0.5（クラッシュしない）
- 入力検証: 以下はエラーとして返す
  - `n_trials == 0`
  - `n_objectives == 0`
  - `values.len() != n_trials * n_objectives`
  - `weights.len() != n_objectives`
  - `is_minimize.len() != n_objectives`

### 3.3 アーキテクチャ制約 🔵

**参照**: `docs/implements/mode-frontier-features/TASK-1615/note.md` 開発ルール

- 純粋Rust実装（DataFrame グローバルストア非依存）
- WASM公開は TASK-1617 で実施（本タスクでは `pub fn` のみ）
- `lib.rs` に `pub mod topsis;` を追加
- `std::time::Instant` を使用した計時（WASM環境でも wasm-bindgen が shim 提供）

### 3.4 コーディング規約 🔵

**参照**: `rust_core/src/pareto.rs`, `rust_core/src/sensitivity.rs`

- Rust edition 2021
- エラーは `Result<T, String>` で返す
- `#[derive(Debug, Clone, serde::Serialize)]` を TopsisResult に付与（WASM公開時に必要）
- テスト命名: `tc_1615_<seq>_<description>`

---

## 4. 想定される使用例

### 4.1 基本使用パターン 🔵

**参照**: `docs/design/mode-frontier-features/dataflow.md` TOPSIS計算ステップ

```rust
// 3試行 × 2目的（両方minimize）
let values = [1.0, 2.0,  // trial0: obj0=1.0, obj1=2.0
              3.0, 1.0,  // trial1: obj0=3.0, obj1=1.0
              2.0, 2.0]; // trial2: obj0=2.0, obj1=2.0
let weights = [0.5, 0.5];
let is_minimize = [true, true];

let result = compute_topsis(&values, 3, 2, &weights, &is_minimize).unwrap();
// scores: [~0.5, ~0.5, ~0.0〜1.0 の中間]
// ranked_indices: スコア降順
```

### 4.2 maximize目的混合 🔵

**参照**: `docs/tasks/mode-frontier-features/TASK-1615.md` テストケース2

```rust
// is_minimize=[false, true]: obj0はmaximize、obj1はminimize
let is_minimize = [false, true];
// obj0が大きい試行 & obj1が小さい試行 → 高スコア
```

### 4.3 エッジケース 🔵

| ケース | 入力 | 期待動作 |
|-------|------|---------|
| 全試行が同一値 | 全values同一 | D⁺+D⁻=0 → score=0.5（クラッシュなし） |
| NaN含む試行 | values に NaN | その試行のscore=0.0、末尾ランク |
| 1試行 | n_trials=1 | score=1.0（または0.5） |
| 1目的 | n_objectives=1 | 正常計算 |
| n_trials=0 | — | `Err("n_trials must be >= 1")` |
| values長さ不一致 | len != n*m | `Err("values length mismatch")` |

### 4.4 パフォーマンステスト 🔵

**参照**: `docs/tasks/mode-frontier-features/TASK-1615.md` 完了条件

```rust
// 50,000試行 × 4目的
let n = 50_000;
let m = 4;
let values: Vec<f64> = (0..n*m).map(|i| i as f64).collect();
let weights = [0.25; 4];
let is_minimize = [true; 4];
let start = std::time::Instant::now();
let _ = compute_topsis(&values, n, m, &weights, &is_minimize).unwrap();
assert!(start.elapsed().as_millis() < 100);
```

---

## 5. 設計文書との対応関係

| カテゴリ | 参照先 |
|---------|-------|
| アーキテクチャ | `docs/design/mode-frontier-features/architecture.md` — TOPSIS Rust実装設計 |
| データフロー | `docs/design/mode-frontier-features/dataflow.md` — TOPSIS計算ステップ図 |
| 型定義 | `docs/design/mode-frontier-features/interfaces.ts` — TopsisWasmResult, TopsisRankingResult |
| タスク詳細 | `docs/tasks/mode-frontier-features/TASK-1615.md` |
| 実装パターン | `rust_core/src/pareto.rs`, `rust_core/src/sensitivity.rs` |
| ヒアリング記録 | `docs/design/mode-frontier-features/design-interview.md` Q5, Q7 |

---

## 品質評価

| 観点 | 状態 |
|-----|------|
| 要件の曖昧さ | ✅ なし（アルゴリズム手順・エッジケースを明確化） |
| 入出力定義の完全性 | ✅ 完全（型・制約・範囲すべて定義） |
| 制約条件の明確性 | ✅ 明確（パフォーマンス・NaN・ゼロ除算すべて記載） |
| 実装可能性 | ✅ 確実（既存Rustパターンあり・外部依存なし） |
| 信頼性レベル | 🔵 青信号: 18/18 (100%) |

**品質評価**: ✅ 高品質
