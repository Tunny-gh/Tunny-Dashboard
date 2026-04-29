---
name: PROMETHEE Ranking コンテキストノート
description: PROMETHEE I/II 実装のための技術スタック・既存実装・注意事項
type: project
---

# PROMETHEE Ranking — コンテキストノート

## プロジェクト技術スタック

| 層 | 技術 |
|---|---|
| アルゴリズム実装 | Rust (`rust_core/src/mcdm/`) |
| デスクトップ UI | egui/eframe (`egui-app/src/`) |
| 状態管理 | `egui-app/src/state/` |
| 非同期通信 | `std::sync::mpsc::SyncSender<AppMessage>` |

> **注意**: `frontend/` (TypeScript/WASM) は削除済み。全実装は Rust (`egui-app`) で行う。

---

## 既存 MCDM 実装の構成

### アルゴリズム層 (`rust_core/src/mcdm/`)

| ファイル | 内容 |
|---|---|
| `mod.rs` | `validate_inputs` / `filter_valid_indices` 共通ユーティリティ |
| `topsis.rs` | TOPSIS アルゴリズム + テスト |
| `vikor.rs` | VIKOR アルゴリズム + テスト |
| `entropy.rs` | エントロピー重み計算 |

### 状態管理層 (`egui-app/src/state/`)

| ファイル | 関連型 |
|---|---|
| `results.rs` | `McdmMethod`, `McdmResult`, `TopsisResult`, `VikorResult`, `EntropyResult` |
| `messages.rs` | `AppMessage::McdmDone(McdmResult)`, `AppMessage::EntropyDone` |
| `message_handler.rs` | `MessageHandler::handle` — McdmDone/EntropyDone ハンドリング |

### UI 層 (`egui-app/src/ui/widgets/`)

| ファイル | 内容 |
|---|---|
| `mcdm_chart.rs` | `McdmRankChart` / `McdmTable` — バーチャート + テーブル表示 |

---

## 既存 McdmMethod enum (results.rs:77)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McdmMethod {
    Topsis,
    Vikor,
}

impl McdmMethod {
    pub fn all() -> &'static [McdmMethod] {
        &[McdmMethod::Topsis, McdmMethod::Vikor]
    }
}
```

→ `PrometheeI` / `PrometheeII` を追加予定。

---

## 既存 McdmResult enum (results.rs:122)

```rust
pub enum McdmResult {
    Topsis(TopsisResult),
    Vikor(VikorResult),
}
```

→ `Promethee(PrometheeResult)` を追加予定。  
PROMETHEE I / II は同一の計算結果を持ち、`method` フィールドで区別する。

---

## PROMETHEE アルゴリズム概要

### Linear 選好関数

```
d = f(a) - f(b)  (minimize方向に正規化)
if d ≤ q:   P = 0
if q<d≤p:   P = (d - q) / (p - q)
if d > p:   P = 1
```

- q（無差別閾値）= 0（自動）
- p（完全選好閾値）= 0.2 × range_j（自動）

### フロー計算

```
π(a,b) = Σ_j w_j × P_j(a,b)          (集約選好指数)
Φ+(a)  = 1/(n-1) × Σ_{b≠a} π(a,b)    (正フロー)
Φ-(a)  = 1/(n-1) × Σ_{b≠a} π(b,a)    (負フロー)
Φ(a)   = Φ+(a) - Φ-(a)               (純フロー, PROMETHEE II)
```

### PROMETHEE I（部分順位付け）

- 主スコア = Φ+（降順）、タイブレーク = Φ-（昇順）
- 不比較ペアは UI 非表示

### PROMETHEE II（完全順位付け）

- 主スコア = Φnet（降順）

---

## NaN 試行処理（既存パターン踏襲）

`filter_valid_indices` で NaN 除外 → 有効試行のみ計算 → NaN 試行のフロー = 0.0 → ranked_indices の末尾に配置。

---

## UI 統合パターン（mcdm_chart.rs）

- メソッドセレクタ: `egui::ComboBox` に `PrometheeI` / `PrometheeII` を追加
- バーチャート:
  - PROMETHEE I: Φ+ バー（青）と Φ- バー（赤）を横並びで表示
  - PROMETHEE II: Φnet バー（青、負値は別色）で表示
- キャッシュ: `McdmRankChart` に `cached_promethee: Option<PrometheeResult>` を追加

---

## 関連ファイルパス

| 役割 | パス |
|---|---|
| 新規アルゴリズム | `rust_core/src/mcdm/promethee.rs` |
| mod.rs 追加 | `rust_core/src/mcdm/mod.rs` |
| 新規結果型 | `egui-app/src/state/results.rs` |
| メッセージハンドラ | `egui-app/src/state/message_handler.rs` |
| UI ウィジェット | `egui-app/src/ui/widgets/mcdm_chart.rs` |
| 理論ドキュメント | `theory/mcdm/promethee.md` |
