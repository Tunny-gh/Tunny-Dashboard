# AHP 設計ヒアリング記録

**作成日**: 2026-04-29
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

既存の MCDM 設計（TOPSIS/VIKOR/PROMETHEE）パターンと AHP の要件定義書を参照し、AHP 固有の設計決定事項（ウィジェット統合方針・実装ガイド包含・独立型の状態管理）を明確化するためのヒアリングを実施しました。

## 質問と回答

### Q1: AhpChart ウィジェットの UI 統合方針

**質問日時**: 2026-04-29
**カテゴリ**: アーキテクチャ（UI統合）
**背景**: 一対比較行列入力グリッド・CR 表示・優先度ベクトルバーチャートを別々のウィジェットに分割するか、同一ウィジェット（`AhpChart`）に集約するか判断が必要だった。PROMETHEE のように McdmChart を拡張する案もあったが、AHP は行列入力 UI が根本的に異なるため独立チャートが確定していた。

**選択肢**:

1. 同一ウィジェットに集約（推奨）— `AhpChart` が `ChartId::AhpRankChart` と `ChartId::AhpTable` の両描画を担当
2. 入力・結果を別ウィジェットに分割 — 入力専用 Widget と結果専用 Widget を別々に作成

**回答**: 同一ウィジェットに集約（推奨）

**信頼性への影響**:

- `AhpChart` が `show_rank_chart()` と `show_table()` の 2 メソッドを持つ設計の信頼性が 🟡 → 🔵 に向上
- `WidgetStates.ahp_chart: AhpChart` の単一フィールド設計が確定（🔵）

---

### Q2: 実装ガイドの包含

**質問日時**: 2026-04-29
**カテゴリ**: 設計文書構成
**背景**: PROMETHEE 設計に `implementation-guide.md` が含まれており、AHP でも同様のガイドを作成するか確認が必要だった。AHP は固有の一対比較行列数学があるため、実装ガイドの価値が高い。

**選択肢**:

1. 含める（推奨）— architecture.md + dataflow.md + interfaces.rs + implementation-guide.md の 4 点セット
2. 含めない — architecture.md + dataflow.md + interfaces.rs のみ

**回答**: 含める（推奨）

**信頼性への影響**:

- `docs/design/ahp/implementation-guide.md` の作成が確定（🔵）
- 実装手順・テスト戦略・段階的リリース計画を含む包括的ガイドの方針が確定

---

## ヒアリング結果サマリー

### 確認できた事項

- `AhpChart` 単一構造体が `ChartId::AhpRankChart` と `ChartId::AhpTable` の描画を担当する
- `docs/design/ahp/implementation-guide.md` を作成する
- `AppMessage::AhpDone(AhpResult)` は `AppMessage::McdmDone(McdmResult)` とは完全独立バリアント
- `AppState.ahp_result: Option<AhpResult>` は `AppState.mcdm_result` とは別フィールド

### 設計方針の決定事項

- **4 層メッセージパッシング**: 既存 TOPSIS/VIKOR/PROMETHEE と同一パターンを踏襲
- **完全独立**: 既存 McdmMethod / McdmResult enum は変更しない
- **Study 切替リセット**: `StudySelected` ハンドラで `ahp_chart = AhpChart::default()`
- **上三角ストレージ**: `pairwise: Vec<f64>` に上三角のみ格納 (len = n\*(n-1)/2)

### 残課題

- n ≥ 6 の RI 値の詳細（note.md の 1.24 近似で対応予定）
- 一対比較行列グリッドの具体的なセル幅・行の高さ（実装時に egui の制約で調整）

### 信頼性レベル分布

**ヒアリング前**:

- 🔵 青信号: 8
- 🟡 黄信号: 4
- 🔴 赤信号: 1

**ヒアリング後**:

- 🔵 青信号: 13 (+5)
- 🟡 黄信号: 2 (-2)
- 🔴 赤信号: 0 (-1)

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **実装ガイド**: [implementation-guide.md](implementation-guide.md)
- **要件定義**: [../../spec/ahp/requirements.md](../../spec/ahp/requirements.md)
