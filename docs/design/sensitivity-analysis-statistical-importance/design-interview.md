# sensitivity-analysis-statistical-importance 設計ヒアリング記録

**作成日**: 2026-04-25
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

既存の感度分析実装（Spearman, Ridge, RF-ANOVA, MDI, SHAP, Sobol の点推定のみ）に
統計的有意性指標を追加するにあたり、設計の未確定事項を明確化する。

---

## 質問と回答

### Q1: 設計規模

**質問日時**: 2026-04-25
**カテゴリ**: プロセス
**背景**: 設計文書の詳細度を決定する必要があった

**回答**: フル設計（推奨）

**信頼性への影響**:
- architecture.md, dataflow.md, design-interview.md, interfaces.rs, implementation-guide.md の全文書を作成する方針が確定
- 信頼性レベル: 🔵

---

### Q2: egui UIでの信頼区間表示方式

**質問日時**: 2026-04-25
**カテゴリ**: UI設計
**背景**: 現在の `importance_chart.rs` は `ui.label(format!("{score:.3}"))` でスコアのみ表示。
信頼区間の視覚化に複数の方法（エラーバー、テキスト、マークのみ）があり、設計判断が必要だった。

**選択肢**:
- エラーバーオーバーレイ（I字型を描画）
- CI数値テキスト（`[lo, hi]` 形式）
- p値とマークのみ（CI省略）

**回答**: エラーバーオーバーレイ（推奨）

**信頼性への影響**:
- `importance_chart.rs` でのエラーバー描画ロジックが確定
  - `lo_x`, `hi_x` を スコア/max_score × bar_max_width で計算
  - I字型（横線 + 縦線×2）を `ui.painter()` で描画
  - p値テキストはバーの右側に引き続き表示
- 信頼性レベル: 🔵（REQ-STAT-005の実現方法が確定）

---

### Q3: Ridge の (X^TX + αI)^{-1} 対角成分の取得方式

**質問日時**: 2026-04-25
**カテゴリ**: アルゴリズム設計
**背景**: Ridge の標準誤差 `SE_j = sqrt(σ² × [A^{-1}]_{jj})` を計算するには逆行列対角成分が必要。
既存の `gaussian_elimination()` を使って対角成分のみ取得する方法か、完全逆行列を計算するかの判断が必要だった。

**選択肢**:
- 対角成分のみ追加ガウス消去（p 回 O(p²)）
- 完全逆行列計算（O(p³)、将来拡張可能）
- 近似（正則化項無視、最軽量）

**回答**: 対角成分のみ追加ガウス消去（推奨）

**信頼性への影響**:
- `ridge.rs` に `compute_ridge_diagonal_inv(a: &Vec<Vec<f64>>) -> Vec<f64>` を追加する設計が確定
- 実装: `e_j` を右辺として既存の `gaussian_elimination()` を p 回呼び出し、j 番目の要素を取得
- 計算コスト: p=30 で 30回の O(900) = O(27000) → 十分に高速
- 信頼性レベル: 🔵（REQ-STAT-021の実現方法が確定）

---

### Q4: t分布CDFのpure Rust実装精度

**質問日時**: 2026-04-25
**カテゴリ**: 技術選択
**背景**: NFR-STAT-020（外部クレート不可）より、t分布CDFをpure Rustで実装する必要がある。
精度とコストのトレードオフがあった。

**選択肢**:
- 高精度（不完全ベータ関数・誤差 < 10^{-6}）
- 中精度（正規分布近似・誤差 < 5×10^{-4}）

**回答**: 高精度（推奨）

**信頼性への影響**:
- `statistics.rs` で不完全ベータ関数の継続分数展開（Lentz法）を実装する設計が確定
  - `x = df/(df + t²)` → `I_x(df/2, 1/2)` を計算
  - `df > 30` の場合は正規分布近似にフォールバック（精度は同等）
- 信頼性レベル: 🔵（NFR-STAT-020の実現方法が確定）

---

### Q5: 新規モジュールの配置場所

**質問日時**: 2026-04-25
**カテゴリ**: アーキテクチャ
**背景**: t分布/正規分布CDF等の汎用統計関数をどこに配置するか。
感度分析専用 vs 数学コアライブラリ としての配置の判断が必要だった。

**選択肢**:
- `rust_core/src/core/math/` 配下（汎用数学コア）
- `rust_core/src/sensitivity/` 配下（感度分析専用）

**回答**: `rust_core/src/core/math/` 配下（推奨）

**信頼性への影響**:
- `statistics.rs` を `rust_core/src/core/math/` に追加し、`mod.rs` に `pub(crate) mod statistics;` を追加する設計が確定
- 将来他の機能（PDPの信頼区間等）での再利用も可能
- 信頼性レベル: 🔵（新規ファイルの配置が確定）

---

## ヒアリング結果サマリー

### 確認できた事項

- egui UIのエラーバー描画は `ui.painter()` の `line_segment()` で実装
- Ridge の標準誤差は対角成分のみのガウス消去（p回実行）
- t分布CDF は不完全ベータ関数の高精度実装
- 統計コアは `rust_core/src/core/math/statistics.rs` に配置

### 設計方針の決定事項

1. **新規ファイル**: `rust_core/src/core/math/statistics.rs` の1ファイルのみ
2. **既存ファイル変更**: `types.rs`, `spearman.rs`, `ridge.rs`, `rf_anova.rs`, `mdi.rs`, `shap.rs`, `sobol.rs`, `analysis/full.rs`, `analysis/selected.rs`, `importance_chart.rs`
3. **エラーバー**: I字型をパレット描画、p値テキストは右側にインライン
4. **Ridge**: 近似バイアスを `is_approximate: bool` フィールドでフラグ管理、UIで「～」表示
5. **MDI**: 正規化前の raw 重要度を木ごとに記録してSEを計算

### 残課題

- 不完全ベータ関数の実装: Lentz 継続分数展開の数値安定性の詳細検証は実装時に実施
- MDIのスケール変換: 正規化前→正規化後のCIスケール変換が必要（実装詳細は implementation-guide.md で記述）

---

### 信頼性レベル分布

**ヒアリング前**:
- 🔵 青信号: 5件（要件定義書・既存実装の把握）
- 🟡 黄信号: 8件（UI描画方式・逆行列計算・CDF精度・配置等の未確認）
- 🔴 赤信号: 2件（エラーバー描画の具体的実装・MDIスケール変換）

**ヒアリング後**:
- 🔵 青信号: 13件 (+8)
- 🟡 黄信号: 2件 (-6)（残課題として残存）
- 🔴 赤信号: 0件 (-2)

---

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
- **型定義（Rust）**: [interfaces.rs](interfaces.rs)
- **実装ガイド**: [implementation-guide.md](implementation-guide.md)
- **要件定義**: [requirements.md](../../spec/sensitivity-analysis-statistical-importance/requirements.md)
