# VIKOR ヒアリング記録

**作成日**: 2026-04-24
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

既存のTOPSIS実装・McdmアーキテクチャおよびVIKORアルゴリズム仕様を確認し、不明点や設計選択が必要な部分を明確化するためのヒアリングを実施しました。

---

## 質問と回答

### Q1: VIKORに特有の戦略パラメータ v（妥協度、0〜1）の扱い

**カテゴリ**: 未定義部分詳細化  
**背景**: TOPSISにはない固有のパラメータ。UIスライダーで調整可能にするか、固定値にするかを確認。

**回答**: UIスライダーで調整可能（デフォルト v=0.5）

**信頼性への影響**:
- REQ-007（v スライダーUI）が 🔴 → 🔵 に向上
- REQ-201（v パラメータのUI状態保持）が 🔴 → 🔵 に向上
- NFR-201（v スライダー UX仕様）が 🔴 → 🔵 に向上

---

### Q2: VIKORのスコア出力（Q値のみ vs S/R/Q全て）

**カテゴリ**: 未定義部分詳細化  
**背景**: VIKORは S（utility）・R（regret）・Q（compromise）の3種のスコアを計算する。全て格納するかQ値のみにするかはAPI設計に影響。

**回答**: S・R・Q全て出力

**信頼性への影響**:
- REQ-301（VikorResultにs_values/r_values/q_values格納）が 🔴 → 🔵 に向上
- VikorResult型定義の確定

---

### Q3: NaN値を含む試行の処理方針

**カテゴリ**: 既存設計確認  
**背景**: TOPSIS実装（NaN試行はscore=0.0、ranked_indices末尾）を踏襲するかを確認。

**回答**: TOPSISと同じ（NaN試行はQ=1.0、ranked_indices末尾）

**信頼性への影響**:
- REQ-101（NaN処理）が 🟡 → 🔵 に向上

---

## ヒアリング結果サマリー

### 確認できた事項
- v パラメータはUIスライダーで調整可能（デフォルト 0.5）
- VikorResult に S/R/Q 全スコアを格納する
- NaN処理は TOPSIS と完全に同方針

### 追加/変更要件
- `McdmRankChart.pending_compute` の型に v パラメータを追加（`Option<(McdmMethod, Vec<f64>, f64)>`）
- chart_registry の VIKOR dispatch 時に v を渡す

### 残課題
- VikorResult の `primary_scores()` 実装（1.0 - Q で TOPSIS と同一インターフェース）の動作確認は実装時に
- v スライダーを VIKOR 選択時のみ表示するUIロジックの詳細

### 信頼性レベル分布

**ヒアリング前**:
- 🔵 青信号: 18
- 🟡 黄信号: 7
- 🔴 赤信号: 7

**ヒアリング後**:
- 🔵 青信号: 27 (+9)
- 🟡 黄信号: 5 (-2)
- 🔴 赤信号: 0 (-7)

---

## 関連文書

- **要件定義書**: [requirements.md](requirements.md)
- **ユーザストーリー**: [user-stories.md](user-stories.md)
- **受け入れ基準**: [acceptance-criteria.md](acceptance-criteria.md)
