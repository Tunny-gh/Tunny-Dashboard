# VIKOR 設計ヒアリング記録

**作成日**: 2026-04-24
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

既存のTOPSIS実装・McdmアーキテクチャおよびVIKORアルゴリズム仕様を確認し、設計上の選択が必要な部分（`pending_compute`型設計）を明確化するためのヒアリングを実施しました。

---

## 質問と回答

### Q1: `pending_compute` の型設計

**カテゴリ**: アーキテクチャ  
**背景**: 現状 `Option<(McdmMethod, Vec<f64>)>` にVIKOR固有のvパラメータを追加する方法として、タプル拡張 vs 構造体新設の2択があった。

**選択肢A（タプル拡張）**: `Option<(McdmMethod, Vec<f64>, f64)>`  
- メリット: 変更最小限
- デメリット: 第3引数の意味が不明瞭、将来のパラメータ追加で型が肥大化

**選択肢B（構造体新設）**: `Option<McdmComputeRequest>` where `McdmComputeRequest { method, weights, v }`  
- メリット: フィールド名で意味が明確、将来拡張しやすい
- デメリット: 変更範囲がやや広い（型定義+pending_compute+pending_compute.take()分岐）

**回答**: MCDMCompute構造体に統一（選択肢B）

**信頼性への影響**:
- `McdmComputeRequest` 構造体設計が 🔴 → 🔵 に向上
- `pending_compute: Option<McdmComputeRequest>` の型が確定
- chart_registry.rs の dispatch パターンが確定

---

## ヒアリング結果サマリー

### 確認できた事項
- `McdmComputeRequest { method: McdmMethod, weights: Vec<f64>, v: f64 }` 構造体を新設
- `McdmRankChart.pending_compute: Option<McdmComputeRequest>` に変更
- TOPSIS dispatch では `v` フィールドを無視するだけ

### 設計方針の決定事項
- `pending_compute` はタプルではなく名前付き構造体
- `v` のデフォルト値は 0.5（McdmRankChart::default()）
- `mcdm_chart.rs` に `McdmComputeRequest` 型を定義（chart_registry.rs に対して公開）

### 残課題
- `primary_scores()` の実装: `VikorResult` に `display_scores` フィールドを持たせるか、呼び出し側で `1.0 - q` するかは実装者が決定
  - 推奨: `VikorResult` に `display_scores: Vec<f64>` フィールドを追加し、コンストラクト時に `1.0 - q` を計算して格納

### 信頼性レベル分布

**ヒアリング前**:
- 🔵 青信号: 12
- 🟡 黄信号: 3
- 🔴 赤信号: 2

**ヒアリング後**:
- 🔵 青信号: 16 (+4)
- 🟡 黄信号: 1 (-2)
- 🔴 赤信号: 0 (-2)

---

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **要件定義**: [../../spec/vikor/requirements.md](../../spec/vikor/requirements.md)
