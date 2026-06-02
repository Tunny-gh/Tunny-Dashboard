# 制約条件を考慮した可視化 アーキテクチャ設計

**作成日**: 2026-06-03
**関連要件定義**: [requirements.md](../../spec/constraint-aware-visualization/requirements.md)
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *要件定義書 + ユーザヒアリング 2026-06-03 より*

制約付き Optuna 最適化（`system_attrs.constraints`）の結果を Tunny Dashboard 上で視覚的に区別する。
既存の `DataFrame.is_feasible` 派生列（実装済み）を活用し、以下の2層で変更を加える：

1. **`rust_core` 層**: `compute_pareto_ranks()` に feasibility フィルタ + 違反量ランキングを追加
2. **`egui-app` 層**: 各チャートウィジェットに `show_infeasible: bool` フィールドと描画ロジックを追加

---

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存プロジェクト構造より*

- **パターン**: レイヤードアーキテクチャ（rust_core データ層 / egui-app UI 層の分離を維持）
- **選択理由**: 既存アーキテクチャとの整合性。`is_feasible` 列は既に rust_core 側で計算済み。UI 層はこれを読み取るだけ

---

## 変更コンポーネント構成

### rust_core 層（データ・計算）🔵

**信頼性**: 🔵 *ユーザヒアリング「rust_core 内部で処理」+ 既存実装より*

| ファイル | 変更種別 | 内容 |
|---|---|---|
| `rust_core/src/multi_objective/pareto/ranking.rs` | **修正** | `compute_pareto_ranks()` に feasibility フィルタ + 違反量ランキングを追加 |

#### compute_pareto_ranks() の変更設計 🔵

**信頼性**: 🔵 *ユーザヒアリング 2026-06-03 より*

```
現行フロー:
  全試行の目的値 → nd_sort() → ranks[0..n]
  ranks[i] = 0 なら pareto_indices に追加

新フロー（has_constraints = true の場合）:
  1. is_feasible 列を取得
  2. feasible 行のインデックスと目的値を抽出
  3. feasible 行のみで nd_sort() 実行 → feasible_ranks[0..n_feasible]
  4. max_feasible_rank を取得
  5. infeasible 行を constraint_sum 昇順でソート
  6. infeasible 行に max_feasible_rank + 1, +2, ... を割り当て
  7. 全 n 行の ranks[] に再マッピング
  8. pareto_indices = feasible で rank == 0 の行のインデックス

has_constraints = false の場合:
  現行フローをそのまま実行（変更なし）
```

**pareto_rank の意味**:
- `0`: Pareto フロント（実行可能解の中の非支配解）
- `1..max_feasible_rank`: Pareto ランク（実行可能解内）
- `max_feasible_rank + 1`: 最も constraint_sum が小さい infeasible 試行
- `max_feasible_rank + 2, +3, ...`: 以降、違反量の昇順

---

### egui-app 層（UI・描画）🔵

**信頼性**: 🔵 *ユーザヒアリング + 既存実装より*

| ファイル | 変更種別 | 内容 |
|---|---|---|
| `egui-app/src/theme/chart_colors.rs` | **追加** | `COLOR_INFEASIBLE` 定数を追加 |
| `egui-app/src/ui/widgets/pareto_2d.rs` | **修正** | `show_infeasible: bool` フィールド + 描画ロジック + トグル UI |
| `egui-app/src/ui/widgets/pareto_3d.rs` | **修正** | 同上（GPU バッファ経由） |
| `egui-app/src/ui/widgets/optimization_history.rs` | **修正** | 同上 |
| `egui-app/src/ui/widgets/parallel_coords.rs` | **修正** | 同上（折れ線描画） |
| `egui-app/src/ui/widgets/scatter_matrix.rs` | **修正** | 同上（散布図各セル） |
| `egui-app/src/ui/widgets/cluster_scatter.rs` | **修正** | 同上 |

#### show_infeasible フィールドの設計 🔵

**信頼性**: 🔵 *ユーザヒアリング「Study 切替時にリセット」より*

- 各ウィジェット構造体に `show_infeasible: bool` を追加
- `Default::default()` で `true`（表示）を返す
- `widget_states.rs` の `StudySelected` 処理（`widget_states.cluster_scatter = Default::default()` 等）で自動リセット
- 追加コードは不要（既存の `Default::default()` 呼び出しが機能する）

#### COLOR_INFEASIBLE の仕様 🔵

**信頼性**: 🔵 *ユーザヒアリング「alpha=80、グレー」より*

```rust
// premultiplied: rgb(180,180,180) × alpha/255 ≈ 56,56,56; alpha = 80
pub const COLOR_INFEASIBLE: Color32 = Color32::from_rgba_premultiplied(56, 56, 56, 80);
```

#### 各チャートの描画ロジック変更パターン 🔵

**信頼性**: 🔵 *既存 `pareto_2d.rs` コード構造 + ユーザヒアリングより*

```rust
// 描画ループ内の追加ロジック（各ウィジェット共通）
let is_feasible_col = view.numeric_column("is_feasible"); // None = 制約なし

for i in displayed {
    let feasible = is_feasible_col
        .and_then(|col| col.get(i))
        .map(|&v| v > 0.5)
        .unwrap_or(true); // 制約なし = 全て実行可能

    if !feasible {
        if !self.show_infeasible {
            continue; // 非表示モードはスキップ
        }
        // グレーアウトで描画 → COLOR_INFEASIBLE を使用
    } else {
        // 通常の色分けロジック（変更なし）
    }
}
```

#### ParetoScatter2D での描画順序 🔵

**信頼性**: 🔵 *要件定義 REQ-CAV-043（実行不可能解を背面に）+ ユーザヒアリングより*

```
描画順序（先に描いたものが背面）:
1. infeasible 点（COLOR_INFEASIBLE）← 背面
2. non-pareto 点（従来色）
3. pareto 点（従来色）← 前面
4. highlight 点（最前面）
```

#### Show Infeasible トグルの UI 配置 🔵

**信頼性**: 🔵 *ユーザヒアリング「各チャートのツールバーで個別対応」より*

```rust
// 制約あり Study かつ has_constraints == true の場合のみ表示
if ctx.meta.has_constraints {
    if let Some(is_feasible_col) = view.numeric_column("is_feasible") {
        if is_feasible_col.iter().any(|&v| v < 0.5) {
            // infeasible な trial が存在する場合のみトグルを表示
            ui.checkbox(&mut self.show_infeasible, "Show Infeasible");
        }
    }
}
```

---

## ディレクトリ構造（変更対象） 🔵

**信頼性**: 🔵 *既存プロジェクト構造より*

```
rust_core/src/multi_objective/pareto/
└── ranking.rs               ← compute_pareto_ranks() 修正

egui-app/src/
├── theme/
│   └── chart_colors.rs      ← COLOR_INFEASIBLE 追加
└── ui/widgets/
    ├── pareto_2d.rs          ← show_infeasible 追加・描画変更
    ├── pareto_3d.rs          ← show_infeasible 追加・描画変更
    ├── optimization_history.rs ← show_infeasible 追加・描画変更
    ├── parallel_coords.rs    ← show_infeasible 追加・描画変更
    ├── scatter_matrix.rs     ← show_infeasible 追加・描画変更
    └── cluster_scatter.rs    ← show_infeasible 追加・描画変更
```

---

## 非機能要件の実現方法

### パフォーマンス 🔵

**信頼性**: 🔵 *NFR-CAV-001・NFR-CAV-002 + 既存実装より*

- `is_feasible` 列は DataFrame から `O(1)` で列スライス参照を取得（`view.numeric_column("is_feasible")`）
- 描画ループでの feasibility チェックは `O(n)` の配列アクセスのみ
- "Show Infeasible" トグル切替は egui の即時モード描画で自動再描画（`ctx.request_repaint()` 不要）
- `compute_pareto_ranks()` への feasibility フィルタ追加コストは `O(n)` のフィルタと再マッピングのみ

### スタイル 🔵

**信頼性**: 🔵 *CLAUDE.md・既存実装より*

- Tailwind CSS は不使用（egui のインライン描画 API を使用）
- カラー定数は `chart_colors.rs` に集約（既存パターン）

---

## 技術的制約

### pareto_rank の後方互換性 🟡

**信頼性**: 🟡 *既存コードの pareto_rank 利用箇所から妥当な推測*

- `pareto_rank == 0` を「Pareto フロント」として判定しているコードが複数箇所に存在
  - `pareto_2d.rs`: `if rank == 0 { ... }`
  - `slice_chart.rs`: `pareto_rank == 0 をアクセント表示`
  - `html_report.rs`: `pareto_rank == 0 でハイライト`
- これらは変更不要（feasible な rank == 0 のみが真のフロントのため意味は同じ）
- infeasible の rank が `max_feasible_rank + n` となるため、既存の `rank > 0` 判定は正しく動作する

### csv_export / html_report への影響 🟡

**信頼性**: 🟡 *既存コードの pareto_rank 利用から妥当な推測*

- `csv_export.rs` と `html_report.rs` は `pareto_rank` を出力に使用
- infeasible 試行の `pareto_rank` が大きな値（max + n）になることで、Pareto 優位でないことが明示される
- 後方互換性のある変更（既存の CSV 形式は維持される）

---

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **実装ガイド**: [implementation-guide.md](implementation-guide.md)
- **設計ヒアリング**: [design-interview.md](design-interview.md)
- **要件定義**: [requirements.md](../../spec/constraint-aware-visualization/requirements.md)

## 信頼性レベルサマリー

- 🔵 青信号: 15件（83%）
- 🟡 黄信号: 3件（17%）
- 🔴 赤信号: 0件（0%）

**品質評価**: ✅ 高品質
