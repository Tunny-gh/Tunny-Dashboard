# UIカラー設定一元化 タスク概要

**作成日**: 2026-05-07
**プロジェクト期間**: Phase 1〜3（3フェーズ）
**推定工数**: 34時間
**総タスク数**: 8件

## 関連文書

- **要件定義書**: [📋 requirements.md](../spec/ui-color-theme/requirements.md)
- **設計文書**: [📐 architecture.md](../design/ui-color-theme/architecture.md)
- **データフロー図**: [🔄 dataflow.md](../design/ui-color-theme/dataflow.md)
- **実装ガイド**: [📝 implementation-guide.md](../design/ui-color-theme/implementation-guide.md)
- **コンテキストノート**: [📝 note.md](../spec/ui-color-theme/note.md)

## フェーズ構成

| フェーズ | 内容 | タスク数 | 工数 | ファイル |
|---------|------|----------|------|----------|
| Phase 1 | theme モジュール基盤構築 | 3 | 9h | TASK-2190〜2192 |
| Phase 2 | ウィジェット・レイヤー移行 | 4 | 22h | TASK-2193〜2196 |
| Phase 3 | 品質確認 | 1 | 2h | TASK-2197 |

## タスク番号管理

**使用済みタスク番号**: TASK-2190 ~ TASK-2197
**次回開始番号**: TASK-2198

## 全体進捗

- [ ] Phase 1: theme モジュール基盤構築
- [x] Phase 2: ウィジェット・レイヤー移行
- [x] Phase 3: 品質確認

## マイルストーン

- **M1: theme 基盤完成** (Phase 1完了): `theme/` ディレクトリ、`mod.rs`、`colormap.rs`、`chart_colors.rs` が揃い `cargo check` が通る
- **M2: 移行完了** (Phase 2完了): 全ウィジェット・レイヤーが `crate::theme` を参照し、インライン色リテラルがゼロ
- **M3: リリース準備完了** (Phase 3完了): `cargo build` 警告ゼロ・`cargo test` 全通過

---

## Phase 1: theme モジュール基盤構築

**目標**: `theme/` ディレクトリを作成し、3ファイル（mod.rs・colormap.rs・chart_colors.rs）を揃える
**成果物**: `egui-app/src/theme/mod.rs`, `colormap.rs`, `chart_colors.rs`

### タスク一覧

- [x] [TASK-2190: theme ディレクトリ作成・theme/mod.rs 変換](TASK-2190.md) - 2h (DIRECT) 🔵
- [x] [TASK-2191: render/colormap.rs → theme/colormap.rs 移動・import パス更新](TASK-2191.md) - 4h (DIRECT) 🔵
- [x] [TASK-2192: theme/chart_colors.rs 新規作成](TASK-2192.md) - 3h (DIRECT) 🔵

### 依存関係

```
TASK-2190 → TASK-2191 → TASK-2192
```

---

## Phase 2: ウィジェット・レイヤー移行

**目標**: 全ウィジェットのインライン色リテラルを `crate::theme::chart_colors` の定数参照に置換する
**成果物**: 各ウィジェットファイルの `Color32::from_rgb(...)` が定数参照に置換された状態

### タスク一覧

- [x] [TASK-2193: Pareto・MCDMウィジェット色定数移行](TASK-2193.md) - 6h (TDD) 🔵
- [x] [TASK-2194: 分析系ウィジェット色定数移行](TASK-2194.md) - 6h (TDD) 🔵
- [x] [TASK-2195: その他ウィジェット色定数移行](TASK-2195.md) - 4h (TDD) 🔵
- [x] [TASK-2196: UIレイヤー・Stateレイヤー色定数移行](TASK-2196.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-2192 → TASK-2193
TASK-2192 → TASK-2194
TASK-2192 → TASK-2195
TASK-2192 → TASK-2196
```

TASK-2193〜2196 は互いに独立しており、TASK-2192 完了後に並行実施可能。

---

## Phase 3: 品質確認

**目標**: ビルド警告ゼロ・全テスト通過・インライン色リテラルゼロを確認する
**成果物**: 品質確認完了レポート

### タスク一覧

- [x] [TASK-2197: ビルド・テスト品質確認](TASK-2197.md) - 2h (DIRECT) 🔵

### 依存関係

```
TASK-2193 ─┐
TASK-2194 ─┤
TASK-2195 ─┤→ TASK-2197
TASK-2196 ─┘
```

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 8件
- 🔵 **青信号**: 8件 (100%)
- 🟡 **黄信号**: 0件 (0%)
- 🔴 **赤信号**: 0件 (0%)

### フェーズ別信頼性

| フェーズ | 🔵 青 | 🟡 黄 | 🔴 赤 | 合計 |
|---------|-------|-------|-------|------|
| Phase 1 | 3 | 0 | 0 | 3 |
| Phase 2 | 4 | 0 | 0 | 4 |
| Phase 3 | 1 | 0 | 0 | 1 |

**品質評価**: ✅ 高品質

## クリティカルパス

```
TASK-2190 → TASK-2191 → TASK-2192 → TASK-2193 → TASK-2197
                                   → TASK-2194 ↗
                                   → TASK-2195 ↗
                                   → TASK-2196 ↗
```

**クリティカルパス工数**: 2 + 4 + 3 + 6 + 2 = 17時間（TASK-2193経由）
**並行作業可能工数**: TASK-2194〜2196 の16時間を TASK-2193 と並行実施可能

## 移行対象ファイルサマリー

### Phase 1 で移動・新規作成するファイル

| 操作 | 対象ファイル |
|------|------------|
| 変換 | `egui-app/src/theme.rs` → `egui-app/src/theme/mod.rs` |
| 移動 | `egui-app/src/render/colormap.rs` → `egui-app/src/theme/colormap.rs` |
| 新規 | `egui-app/src/theme/chart_colors.rs` |
| 削除 | `egui-app/src/render/colormap.rs`（移動後） |

### Phase 2 で import パスを更新するファイル（Phase 1 分）

| ファイル | 変更内容 |
|---------|---------|
| `egui-app/src/state/types.rs` | `render::colormap` → `theme::colormap` |
| `egui-app/src/state/app_state.rs` | `render::colormap` → `theme::colormap`, `Color32::RED` → `theme::ERROR_COLOR` |
| `egui-app/src/render/mod.rs` | `pub mod colormap;` を削除 |

### Phase 2 でインライン色を置換するウィジェット（TASK-2193〜2196 分）

| タスク | 対象ウィジェット |
|-------|--------------|
| TASK-2193 | pareto_2d, pareto_3d, slice_chart, mcdm_scatter_chart, mcdm_chart |
| TASK-2194 | importance_chart, sensitivity_heatmap, scatter_matrix, parallel_coords, pdp_chart, pdp_2d |
| TASK-2195 | optimization_history, hv_history, cluster_scatter, ahp_chart, trial_table |
| TASK-2196 | toolbar, layout, grid_canvas, bottom_panel, comparison_panel, state/app_state |

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-2190`
