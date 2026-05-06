# UIカラー設定一元化 設計ヒアリング記録

**作成日**: 2026-05-07
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

コードベース分析で明らかになった設計上の分岐点について、実装方針を確定するためのヒアリングを実施。

---

## 質問と回答

### Q1: ロジック関数の配置場所

**カテゴリ**: アーキテクチャ
**背景**: `normalize_trial` / `compute_chart_colors` は `state::app_state` の型（ColorMode, TrialRow）に依存しており、移動先によってモジュール責務の意味が変わる。Rustでは同一クレート内の循環参照が許可されるため、どちらも技術的には実現可能。

**回答**: 「theme/colormap.rs にそのまま」
→ 現在の `render/colormap.rs` の内容をそのまま `theme/colormap.rs` に移動する。`state::app_state` への依存はそのまま維持する。

**信頼性への影響**:
- アーキテクチャ設計の CMAP モジュール定義の信頼性が 🟡 → 🔵 に向上
- EDGE-001（循環依存懸念）が「Rust同一クレート内では問題なし」として解決（実装考慮事項に格下げ）

---

### Q2: render/colormap の呼び出し元更新戦略

**カテゴリ**: 技術選択
**背景**: `crate::render::colormap` を参照するファイルが8箇所あり、`theme::colormap` への更新は2つの方法がある。1) `render/colormap.rs` を `pub use crate::theme::colormap::*;` の re-export ラッパーとして残す（呼び出し元変更ゼロ）、2) 全呼び出し元を直接 `theme::colormap` に更新する（クリーンな構成）。

**回答**: 「全呼び出し元を直接更新」
→ `render/colormap.rs` を削除し、8箇所の import パスをすべて `theme::colormap` に更新する。

**信頼性への影響**:
- REQ-052の解決方針が「全呼び出し元直接更新」として確定し、信頼性が 🟡 → 🔵 に向上
- dataflow.md の更新マップが確定した

---

## ヒアリング結果サマリー

### 確認できた事項
- ロジック関数は `theme/colormap.rs` に配置（移動するだけ）
- `render/colormap.rs` は完全削除し、呼び出し元8箇所を直接更新する

### 設計方針の決定事項
1. **ファイル移動**: `render/colormap.rs` → `theme/colormap.rs`（内容変更なし）
2. **ファイル削除**: `render/colormap.rs` を削除
3. **パス更新**: 8箇所の `crate::render::colormap` → `crate::theme::colormap`
4. **re-export なし**: `render/mod.rs` から `colormap` モジュール宣言を削除

### 残課題
- `Color32::from_rgba_unmultiplied` を使うインライン色の `const` 化方針（`const` 化できないため `from_rgba_premultiplied` に変換するか関数にするか）— 実装者の判断に委ねる
- `importance_chart.rs` の `0x0c0c6a`（ダークネイビー）が `mcdm_chart.rs` の `0x0c6ac0`（ブルー）と異なる色であることの確認

### 信頼性レベル分布

**ヒアリング前**:
- 🔵 青信号: 8件
- 🟡 黄信号: 5件
- 🔴 赤信号: 0件

**ヒアリング後**:
- 🔵 青信号: 13件 (+5)
- 🟡 黄信号: 2件 (-3)
- 🔴 赤信号: 0件 (変化なし)

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
- **実装ガイド**: [implementation-guide.md](implementation-guide.md)
- **要件定義**: [requirements.md](../../spec/ui-color-theme/requirements.md)
