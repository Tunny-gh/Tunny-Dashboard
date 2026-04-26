# sensitivity-analysis-statistical-importance 要件定義書

**作成日**: 2026-04-25

## 概要

感度分析（Sensitivity Analysis）の各重要度メトリクスに、帰無仮説を考慮した統計的有意性指標を追加する。
現在の実装はすべて点推定値のみであり、「偶然重要に見える」パラメータと「統計的に有意に重要なパラメータ」を区別できない。
本機能により、最適化パラメータの重要度評価を統計的根拠のある指標に昇格させる。

## 関連文書

- **ヒアリング記録**: [💬 interview-record.md](interview-record.md)
- **ユーザストーリー**: [📖 user-stories.md](user-stories.md)
- **受け入れ基準**: [✅ acceptance-criteria.md](acceptance-criteria.md)
- **コンテキストノート**: [📝 note.md](note.md)

## 機能要件（EARS記法）

**【信頼性レベル凡例】**:
- 🔵 **青信号**: ユーザヒアリング・設計文書・既存実装を参考にした確実な要件
- 🟡 **黄信号**: ユーザヒアリング・設計文書から妥当な推測による要件
- 🔴 **赤信号**: ユーザヒアリング・設計文書にない推測による要件

---

### 通常要件

- **REQ-STAT-001**: システムは、感度分析の全メトリクス（Spearman, Ridge, RF-ANOVA, MDI, SHAP, Sobol）に対して、パラメータごとのp値を計算しなければならない 🔵 *ユーザヒアリング2026-04-25より*

- **REQ-STAT-002**: システムは、パラメータごとに95%信頼区間（下限・上限）を計算しなければならない 🔵 *ユーザヒアリング2026-04-25より*

- **REQ-STAT-003**: システムは、Bonferroni補正を適用した調整済みp値（adjusted p-value）を計算しなければならない 🔵 *ユーザヒアリング2026-04-25より*

- **REQ-STAT-004**: システムは、調整済みp値に基づく有意性マーク（* / ** / ***）を付与しなければならない 🔵 *ユーザヒアリング2026-04-25より*

- **REQ-STAT-005**: システムは、統計情報をパラメータ重要度バーのインライン表示（右側）に「0.032 *」形式で表示しなければならない 🔵 *ユーザヒアリング2026-04-25より*

---

### Spearman の統計的指標（解析的計算）

- **REQ-STAT-010**: Spearman相関係数 ρ に対して、t統計量 `t = ρ√(n-2) / √(1-ρ²)` を計算しなければならない 🔵 *ユーザヒアリング（解析的計算選択）・Spearman理論より*

- **REQ-STAT-011**: Spearman の自由度 `df = n-2` の t 分布から両側p値を計算しなければならない 🔵 *ユーザヒアリング（解析的計算選択）・Spearman理論より*

- **REQ-STAT-012**: Spearman の95%信頼区間は Fisher の z 変換を用いて計算しなければならない: `z = atanh(ρ)`, `SE_z = 1/√(n-3)`, `CI: [tanh(z ± 1.96 × SE_z)]` 🟡 *解析的計算選択から妥当な推測*

- **REQ-STAT-013**: `n < 4` の場合、Spearman のp値と信頼区間は `None` を返さなければならない 🟡 *数学的制約から妥当な推測*

---

### Ridge の統計的指標（解析的計算）

- **REQ-STAT-020**: Ridge 回帰の各係数 β_j に対して、残差分散 `σ² = RSS/(n-p-1)` を計算しなければならない 🔵 *ユーザヒアリング（解析的計算選択）・Ridge理論より*

- **REQ-STAT-021**: 標準誤差 `SE_j = √(σ² × [(X^T X + αI)^{-1}]_{jj})` を計算しなければならない 🔵 *ユーザヒアリング（解析的計算選択）・Ridge理論より*

- **REQ-STAT-022**: t 統計量 `t_j = β_j / SE_j`、自由度 `df = n-p-1` で両側p値を計算しなければならない 🔵 *ユーザヒアリング（解析的計算選択）・Ridge理論より*

- **REQ-STAT-023**: Ridge の95%信頼区間は `β_j ± t_{0.025, df} × SE_j` で計算しなければならない 🟡 *Ridge理論から妥当な推測*

- **REQ-STAT-024**: `n ≤ p + 2` の場合、Ridge の統計情報は `None` を返さなければならない（自由度不足） 🟡 *数学的制約から妥当な推測*

- **REQ-STAT-025**: 注意: Ridge の正則化項 α はバイアスを導入するため、p値は近似であることをUIに表示しなければならない（「～」マーク等） 🟡 *Ridge特性から妥当な推測*

---

### RF-ANOVA / MDI / SHAP の統計的指標（ツリー間分散）

- **REQ-STAT-030**: RF-ANOVA, MDI, SHAP において、各パラメータの重要度を100本の木それぞれで個別に記録しなければならない 🔵 *ユーザヒアリング（ツリー間分散選択）より*

- **REQ-STAT-031**: 木ごとの重要度から標準誤差 `SE_j = std(importances_across_trees) / √(n_trees)` を計算しなければならない 🔵 *ユーザヒアリング（ツリー間分散選択）より*

- **REQ-STAT-032**: t 統計量 `t_j = mean(importances_across_trees) / SE_j`、自由度 `df = n_trees - 1 = 99` で片側p値（重要度は0以上なので）を計算しなければならない 🔵 *ユーザヒアリング（ツリー間分散選択）より*

- **REQ-STAT-033**: 95%信頼区間は `mean ± t_{0.025, 99} × SE_j` で計算しなければならない 🟡 *ツリー間分散方式から妥当な推測*

- **REQ-STAT-034**: MDI の場合、不純度低下量を木ごとに合計して正規化前の値で標準誤差を計算しなければならない（正規化後は合計=1制約があり独立でない） 🟡 *MDI特性から妥当な推測*

---

### Sobol の統計的指標（Jansen分散ブートストラップ）

- **REQ-STAT-040**: Sobol 一次指標 S_i および全効果指標 ST_i に対して、Jansen 推定量の分散公式から信頼区間を計算しなければならない 🔵 *ユーザヒアリング（Jansen分散選択）より*

- **REQ-STAT-041**: Jansen の一次指標の分散推定: `Var(S_i) ≈ Var(f_B(f_{AB_i} - f_A)) / (N × Var_Y²)` を計算しなければならない 🔵 *Jansen (1999) の公式より*

- **REQ-STAT-042**: Sobol の95%信頼区間は `S_i ± 1.96 × √Var(S_i)` で計算しなければならない 🔵 *ユーザヒアリング（Jansen分散選択）より*

- **REQ-STAT-043**: Sobol の p値は、帰無仮説 S_i = 0（パラメータが分散に寄与しない）のもとで正規近似 `z = S_i / √Var(S_i)` から計算しなければならない 🟡 *Jansen方式から妥当な推測*

- **REQ-STAT-044**: サロゲートの R² < 0.5 の場合、Sobol の統計情報の信頼性が低いことをUIに警告しなければならない 🟡 *既存R²色分け設計から妥当な推測*

---

### 多重比較補正

- **REQ-STAT-050**: Bonferroni 補正を適用する: `p_adjusted_j = min(p_raw_j × P, 1.0)`（P = パラメータ数） 🔵 *ユーザヒアリング（Bonferroni選択）より*

- **REQ-STAT-051**: 有意性マークは調整済みp値に基づいて付与しなければならない:
  - `p_adjusted < 0.001` → `***`
  - `p_adjusted < 0.01` → `**`
  - `p_adjusted < 0.05` → `*`
  - `p_adjusted ≥ 0.05` → 表示なし（マーク省略） 🔵 *ユーザヒアリング（有意性マーク選択）より*

- **REQ-STAT-052**: 信頼区間は補正なしの raw CI を表示し、p値のみ補正を適用しなければならない 🟡 *統計的慣習から妥当な推測*

---

### データ型拡張

- **REQ-STAT-060**: `types.rs` の `RfAnovaResult`, `MdiResult`, `ShapResult` に以下フィールドを追加しなければならない:
  ```rust
  pub p_values: Option<Vec<Vec<f64>>>,    // [param][objective]
  pub ci_lower: Option<Vec<Vec<f64>>>,    // [param][objective]
  pub ci_upper: Option<Vec<Vec<f64>>>,    // [param][objective]
  ```
  🔵 *ユーザヒアリング・既存実装から確実な変更点*

- **REQ-STAT-061**: `SensitivityResult` の `spearman: Vec<Vec<f64>>` を以下に置き換えなければならない:
  ```rust
  pub spearman: Vec<Vec<f64>>,            // 既存（相関係数）
  pub spearman_p_values: Option<Vec<Vec<f64>>>,
  pub spearman_ci_lower: Option<Vec<Vec<f64>>>,
  pub spearman_ci_upper: Option<Vec<Vec<f64>>>,
  ```
  🔵 *ユーザヒアリング・既存実装から確実な変更点*

- **REQ-STAT-062**: `RidgeResult` に以下フィールドを追加しなければならない:
  ```rust
  pub std_errors: Option<Vec<f64>>,
  pub p_values: Option<Vec<f64>>,
  pub ci_lower: Option<Vec<f64>>,
  pub ci_upper: Option<Vec<f64>>,
  ```
  🔵 *ユーザヒアリング・既存実装から確実な変更点*

- **REQ-STAT-063**: `SobolResult` に以下フィールドを追加しなければならない:
  ```rust
  pub first_order_ci_lower: Option<Vec<Vec<f64>>>,
  pub first_order_ci_upper: Option<Vec<Vec<f64>>>,
  pub first_order_p_values: Option<Vec<Vec<f64>>>,
  pub total_effect_ci_lower: Option<Vec<Vec<f64>>>,
  pub total_effect_ci_upper: Option<Vec<Vec<f64>>>,
  pub total_effect_p_values: Option<Vec<Vec<f64>>>,
  ```
  🔵 *ユーザヒアリング・既存実装から確実な変更点*

---

## 非機能要件

### パフォーマンス

- **NFR-STAT-001**: Spearman/Ridge の統計指標計算は、既存の重要度計算に対して追加で **+10ms 以内**（p=30 の場合）でなければならない 🟡 *解析的計算の低コストから妥当な推測*

- **NFR-STAT-002**: RF-ANOVA/MDI/SHAP の統計指標計算（ツリー間分散）は、各木ごとに重要度を記録する形に変更するが、合計計算時間は **50ms 以内の増加**でなければならない 🟡 *ツリー間分散方式の低コストから妥当な推測*

- **NFR-STAT-003**: Sobol の Jansen 分散計算は、既存の Sobol 計算に対して **+30ms 以内**でなければならない（追加の行列演算のみ） 🟡 *Jansen公式適用コストから妥当な推測*

### 数値安定性

- **NFR-STAT-010**: `ρ = ±1.0` 等の退化ケースで t 統計量が ±∞ になる場合、p値は `0.0` を返さなければならない（数値オーバーフロー防止） 🔵 *数学的制約より*

- **NFR-STAT-011**: `SE_j = 0` の場合（全木で同じ重要度）、p値は `None` または `1.0`（非有意）を返さなければならない 🟡 *数値安定性の慣習から妥当な推測*

- **NFR-STAT-012**: 計算された信頼区間が [0, 1] の範囲外になる場合（重要度は非負）、クリップして返さなければならない（ただし p値はクリップしない） 🟡 *重要度の非負制約から妥当な推測*

### 外部ライブラリ

- **NFR-STAT-020**: t 分布・正規分布の CDF は外部クレートなしで pure Rust で実装しなければならない（WASM/バイナリサイズ制約） 🔵 *既存プロジェクト方針より*

---

## Edgeケース

### エラー処理

- **EDGE-STAT-001**: パラメータ数 P = 1 の場合、Bonferroni 補正は適用されず（補正係数=1）、そのままのp値を使用する 🟡 *数学的境界値から妥当な推測*

- **EDGE-STAT-002**: データ数 n < 4 の場合（Spearman で df<2）、統計情報は `None` を返し UIに「サンプル数不足」を表示する 🔵 *t分布の数学的制約より*

- **EDGE-STAT-003**: Sobol で Var_Y ≈ 0（目的関数の分散がゼロ）の場合、全パラメータの統計情報は `None` を返す 🔵 *既存Sobol実装の数値安定性処理から*

### 境界値

- **EDGE-STAT-010**: 全パラメータが有意（全て `*` 以上）の場合、UIは通常通りすべてのマークを表示する 🟡 *UI設計から妥当な推測*

- **EDGE-STAT-011**: 全パラメータが非有意の場合、UI は「有意なパラメータなし」のメッセージを表示する 🟡 *UI設計から妥当な推測*
