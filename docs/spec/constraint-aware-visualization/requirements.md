# 制約条件を考慮した可視化 要件定義書

## 概要

Optuna の制約付き最適化（`system_attrs.constraints`）に対応した可視化機能を追加する。
各 trial の `System_attr.constraints` 配列の全値が `<= 0.0` の trial は「実行可能解（feasible）」として現状通りに表示する。
1つでも `> 0.0` の値を持つ trial は「実行不可能解（infeasible）」として、プロット上でグレーアウト（半透明）表示することで視覚的に区別する。

実行不可能解はデフォルトで表示するが、各チャートのツールバーに "Show Infeasible" トグルを追加し、個別に非表示にできる。

## 関連文書

- **ヒアリング記録**: [💬 interview-record.md](interview-record.md)
- **ユーザストーリー**: [📖 user-stories.md](user-stories.md)
- **受け入れ基準**: [✅ acceptance-criteria.md](acceptance-criteria.md)
- **コンテキストノート**: [📝 note.md](note.md)

## 機能要件（EARS記法）

**【信頼性レベル凡例】**:
- 🔵 **青信号**: PRD・EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な要件
- 🟡 **黄信号**: PRD・EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による要件
- 🔴 **赤信号**: PRD・EARS要件定義書・設計文書・ユーザヒアリングにない推測による要件

---

### REQ-CAV-001: 実行可能性の判定

- REQ-CAV-001-A: `system_attrs.constraints` 配列の全値が `<= 0.0` の trial を「実行可能解」として扱わなければならない 🔵 *ユーザヒアリング 2026-06-03 より*
- REQ-CAV-001-B: `system_attrs.constraints` 配列に `> 0.0` の値が1つでも存在する trial を「実行不可能解」として扱わなければならない 🔵 *ユーザヒアリング 2026-06-03 より*
- REQ-CAV-001-C: `system_attrs.constraints` が存在しない Study（`has_constraints = false`）では、全 trial を実行可能解として扱わなければならない 🔵 *既存実装 `model.rs` L128 の `max_constraints > 0` 条件より*
- REQ-CAV-001-D: 実行可能性の判定は既存の `DataFrame` 派生列 `is_feasible`（1.0 = feasible, 0.0 = infeasible）を使用しなければならない 🔵 *既存実装 `model.rs` L139–150 より*

---

### REQ-CAV-010〜013: 実行不可能解のグレーアウト表示

- REQ-CAV-010: 各チャートは、実行不可能解の trial 点をグレーアウト（半透明）で表示しなければならない 🔵 *ユーザヒアリング 2026-06-03 より*
- REQ-CAV-011: グレーアウト色は `Color32::from_rgba_premultiplied(56, 56, 56, 80)` を使用しなければならない（RGB 約 180,180,180 × alpha/255 ≈ 56、alpha = 80） 🔵 *ユーザヒアリング「軽め alpha=80」より*
- REQ-CAV-012: グレーアウトは制約あり Study のみ適用し、制約なし Study では既存の色分けをそのまま使用しなければならない 🔵 *ユーザヒアリング 2026-06-03 より*
- REQ-CAV-013: 実行不可能解のマーカーサイズ・形状は実行可能解と同一にしなければならない（グレーアウトのみで区別） 🔵 *ユーザヒアリング「マーカー形状は変更しない」より*
- REQ-CAV-014: `chart_colors.rs` に定数 `COLOR_INFEASIBLE: Color32` を追加しなければならない 🟡 *既存 `chart_colors.rs` パターンから妥当な推測*

---

### REQ-CAV-020〜022: Show Infeasible トグル

- REQ-CAV-020: 制約あり Study が選択されている場合、対象チャートのツールバー（または UI コントロール領域）に "Show Infeasible" チェックボックスを表示しなければならない 🔵 *ユーザヒアリング「各チャートのツールバーで個別対応」より*
- REQ-CAV-021: "Show Infeasible" トグルのデフォルト値は `true`（表示）でなければならない 🔵 *ユーザヒアリング「デフォルトで表示」より*
- REQ-CAV-022: "Show Infeasible" が `false` のとき、実行不可能解の trial 点はプロット上に一切描画してはならない 🔵 *ユーザヒアリング 2026-06-03 より*
- REQ-CAV-023: "Show Infeasible" トグルは、制約なし Study では表示してはならない 🟡 *UI の整合性から妥当な推測*

---

### REQ-CAV-030〜034: Pareto ランク計算からの除外

- REQ-CAV-030: Pareto ランク計算（`pareto_indices` / `pareto_rank` の算出）において、実行不可能解は除外しなければならない 🔵 *ユーザヒアリング「Pareto 計算から除外」より*
- REQ-CAV-031: `StudyMeta.has_constraints == true` の場合に限り、Pareto ランク計算前に `is_feasible == 0.0` の trial を除外しなければならない 🔵 *ユーザヒアリング 2026-06-03 より*
- REQ-CAV-032: 実行不可能解の `pareto_rank` は計算対象外を示す特別な値（`u32::MAX` または `999`）をセットしなければならない 🟡 *既存 `pareto_rank: Vec<u32>` の型から妥当な推測*
- REQ-CAV-033: Pareto フロント（`pareto_indices`）に実行不可能解のインデックスは含めてはならない 🔵 *ユーザヒアリング 2026-06-03 より*

---

### REQ-CAV-040〜044: Pareto 散布図 2D（ParetoScatter2D）への対応

- REQ-CAV-040: `ParetoScatter2D` に `show_infeasible: bool` フィールドを追加しなければならない 🔵 *ユーザヒアリング + 既存 `ParetoScatter2D` 実装より*
- REQ-CAV-041: `ParetoScatter2D` の描画ループで、`is_feasible == 0.0` の trial 点を `COLOR_INFEASIBLE` で描画しなければならない（`show_infeasible = true` の場合） 🔵 *ユーザヒアリング 2026-06-03 より*
- REQ-CAV-042: `ParetoScatter2D` のコントロール行に "Show Infeasible" チェックボックスを追加しなければならない（制約あり Study のみ表示） 🔵 *ユーザヒアリング 2026-06-03 より*
- REQ-CAV-043: 実行不可能解は実行可能解の下（Z オーダー的に背面）に描画しなければならない 🟡 *視認性から妥当な推測*

---

### REQ-CAV-050〜053: Pareto 散布図 3D（ParetoScatter3D）への対応

- REQ-CAV-050: `ParetoScatter3D` に `show_infeasible: bool` フィールドを追加しなければならない 🔵 *ユーザヒアリング 2026-06-03 より*
- REQ-CAV-051: `ParetoScatter3D` の GPU バッファ構築時に、実行不可能解の頂点に `COLOR_INFEASIBLE` の色をセットしなければならない 🔵 *ユーザヒアリング 2026-06-03 より*
- REQ-CAV-052: `show_infeasible = false` の場合、実行不可能解の頂点を GPU バッファから除外（または不可視頂点として処理）しなければならない 🟡 *3D GPU バッファ構造から妥当な推測*
- REQ-CAV-053: `ParetoScatter3D` のコントロール領域に "Show Infeasible" チェックボックスを追加しなければならない（制約あり Study のみ表示） 🔵 *ユーザヒアリング 2026-06-03 より*

---

### REQ-CAV-060〜063: 並行座標（ParallelCoordinates）への対応

- REQ-CAV-060: `ParallelCoordinates` ウィジェットに `show_infeasible: bool` フィールドを追加しなければならない 🔵 *ユーザヒアリング 2026-06-03 より*
- REQ-CAV-061: 並行座標の折れ線描画時に、実行不可能解の折れ線を `COLOR_INFEASIBLE` で描画しなければならない 🔵 *ユーザヒアリング 2026-06-03 より*
- REQ-CAV-062: `show_infeasible = false` の場合、実行不可能解の折れ線を描画してはならない 🔵 *ユーザヒアリング 2026-06-03 より*
- REQ-CAV-063: 並行座標のコントロール領域に "Show Infeasible" チェックボックスを追加しなければならない（制約あり Study のみ表示） 🔵 *ユーザヒアリング 2026-06-03 より*

---

### REQ-CAV-070〜073: 散布行列（ScatterMatrix）への対応

- REQ-CAV-070: `ScatterMatrix` ウィジェットに `show_infeasible: bool` フィールドを追加しなければならない 🔵 *ユーザヒアリング 2026-06-03 より*
- REQ-CAV-071: 散布行列の各セル（散布図）で実行不可能解の点を `COLOR_INFEASIBLE` で描画しなければならない 🔵 *ユーザヒアリング 2026-06-03 より*
- REQ-CAV-072: `show_infeasible = false` の場合、実行不可能解の点を描画してはならない 🔵 *ユーザヒアリング 2026-06-03 より*
- REQ-CAV-073: 散布行列のコントロール領域に "Show Infeasible" チェックボックスを追加しなければならない（制約あり Study のみ表示） 🔵 *ユーザヒアリング 2026-06-03 より*

---

### REQ-CAV-080〜083: 最適化履歴（OptimizationHistory）への対応

- REQ-CAV-080: `OptimizationHistory` ウィジェットに `show_infeasible: bool` フィールドを追加しなければならない 🔵 *ユーザヒアリング 2026-06-03 より*
- REQ-CAV-081: 最適化履歴の trial 点描画時に、実行不可能解を `COLOR_INFEASIBLE` で描画しなければならない 🔵 *ユーザヒアリング 2026-06-03 より*
- REQ-CAV-082: `show_infeasible = false` の場合、実行不可能解の点を描画してはならない 🔵 *ユーザヒアリング 2026-06-03 より*
- REQ-CAV-083: 最適化履歴のコントロール領域に "Show Infeasible" チェックボックスを追加しなければならない（制約あり Study のみ表示） 🔵 *ユーザヒアリング 2026-06-03 より*

---

### REQ-CAV-090〜093: クラスター散布図（ClusterScatter）への対応

- REQ-CAV-090: `ClusterScatter` ウィジェットに `show_infeasible: bool` フィールドを追加しなければならない 🔵 *ユーザヒアリング 2026-06-03 より*
- REQ-CAV-091: クラスター散布図の点描画時に、実行不可能解を `COLOR_INFEASIBLE` で描画しなければならない 🔵 *ユーザヒアリング 2026-06-03 より*
- REQ-CAV-092: `show_infeasible = false` の場合、実行不可能解の点を描画してはならない 🔵 *ユーザヒアリング 2026-06-03 より*
- REQ-CAV-093: クラスター散布図のコントロール領域に "Show Infeasible" チェックボックスを追加しなければならない（制約あり Study のみ表示） 🔵 *ユーザヒアリング 2026-06-03 より*

---

### REQ-CAV-100: 凡例表示

- REQ-CAV-100: 制約あり Study のチャートで実行不可能解が表示されている場合、凡例またはツールチップに「Infeasible（実行不可能解）」の凡例を表示してもよい 🟡 *UX の観点から妥当な推測*

---

## 非機能要件

### パフォーマンス

- NFR-CAV-001: `is_feasible` 列は既存の `DataFrame` 派生列を使用するため、判定処理の追加コストは `O(n)` 列参照のみでなければならない 🔵 *既存 `model.rs` 実装より*
- NFR-CAV-002: "Show Infeasible" トグルの切り替えは即座に反映され、再計算なしに描画更新のみで完結しなければならない 🔵 *egui の即時モード描画モデルより*

### スタイル

- NFR-CAV-010: 新規追加のカラー定数は `egui-app/src/theme/chart_colors.rs` に追加しなければならない 🔵 *既存パターンより*
- NFR-CAV-011: 新規追加のウィジェットフィールドおよび UI 要素はインラインスタイルまたは egui デフォルトスタイルを使用しなければならない（Tailwind CSS 不使用） 🔵 *CLAUDE.md・既存実装より（WASM不要・Tailwind不使用環境）*

---

## Edge ケース

### 制約値の欠損

- EDGE-CAV-001: `constraints` 配列が空（`[]`）の trial は実行可能解として扱わなければならない 🟡 *`model.rs` L142 の `all()` の動作（空イテレータは全要素が条件を満たすと判定）から妥当な推測*
- EDGE-CAV-002: 一部の trial に `constraints` がなく、他の trial に `constraints` がある Study では、`constraints` を持たない trial の `is_feasible` は `1.0`（デフォルト `0.0` でフィルタせず）としなければならない 🟡 *`model.rs` L141–148 の `unwrap_or(0.0)` 動作から妥当な推測*

### 全 trial が実行不可能解の場合

- EDGE-CAV-010: Study の全 trial が実行不可能解の場合、Pareto フロントは空（`pareto_indices = []`）にならなければならない 🔵 *ユーザヒアリング「Pareto 計算から除外」より*
- EDGE-CAV-011: 全 trial が実行不可能解かつ `show_infeasible = false` の場合、各チャートは空状態（データなし）として "No feasible trials" メッセージを表示してもよい 🟡 *UX の観点から妥当な推測*

### 制約なし Study

- EDGE-CAV-020: `has_constraints = false` の Study では "Show Infeasible" トグルを表示してはならない 🔵 *ユーザヒアリング + REQ-CAV-023 より*
- EDGE-CAV-021: `has_constraints = false` の Study では全 trial を通常の色分けで表示し、既存の動作を変更してはならない 🔵 *ユーザヒアリング + REQ-CAV-012 より*
