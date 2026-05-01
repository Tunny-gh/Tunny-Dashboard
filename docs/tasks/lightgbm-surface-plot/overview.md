# LightGBM Surface Plot タスク概要

**作成日**: 2026-05-01
**推定工数**: 12時間
**総タスク数**: 5件

## 関連文書

- **要件定義書**: [📋 requirements.md](../../spec/lightgbm-surface-plot/requirements.md)
- **設計文書**: [📐 architecture.md](../../design/lightgbm-surface-plot/architecture.md)
- **データフロー図**: [🔄 dataflow.md](../../design/lightgbm-surface-plot/dataflow.md)
- **実装ガイド**: [🔧 implementation-guide.md](../../design/lightgbm-surface-plot/implementation-guide.md)
- **コンテキストノート**: [📝 note.md](../../spec/lightgbm-surface-plot/note.md)
- **ヒアリング記録（要件）**: [💬 interview-record.md](../../spec/lightgbm-surface-plot/interview-record.md)
- **ヒアリング記録（設計）**: [💬 design-interview.md](../../design/lightgbm-surface-plot/design-interview.md)

## フェーズ構成

| フェーズ | 成果物 | タスク数 | 工数 | ファイル |
|---------|--------|----------|------|----------|
| Phase 1 | LightGBM PDP 実装（バックエンド+UI） | 5 | 12h | [TASK-2149~2153](#phase-1---lightgbm-pdp-実装) |

※本機能は変更ファイル4つ・推定工数12hの小規模追加のため、単一フェーズで完結する。

## タスク番号管理

**使用済みタスク番号**: TASK-2149 ~ TASK-2153
**次回開始番号**: TASK-2154

## 全体進捗

- [x] Phase 1: LightGBM PDP 実装

## マイルストーン

- **M1: バックエンド実装完了**: 1D PDP 計算 + API ディスパッチ (TASK-2149, TASK-2150)
- **M2: UI 実装完了**: ModelType 拡張 + ComboBox 更新 (TASK-2151, TASK-2152)
- **M3: 統合確認完了**: ビルド・テスト・手動確認 (TASK-2153)

---

## Phase 1 - LightGBM PDP 実装

**目標**: PDP Chart（1D・2D）に LightGBM RandomForest モデルを追加する
**成果物**: `compute_pdp_1d_lgbm()` 関数、`"random_forest"` ディスパッチ、UI ComboBox 拡張

### タスク一覧

- [x] [TASK-2149: compute_pdp_1d_lgbm() 実装（lgbm.rs）](TASK-2149.md) - 4h (TDD) 🔵
- [x] [TASK-2150: "random_forest" ディスパッチ追加（api.rs）](TASK-2150.md) - 2h (TDD) 🔵
- [x] [TASK-2151: ModelType::RandomForest 追加・1D UI 更新（pdp_chart.rs）](TASK-2151.md) - 3h (TDD) 🔵
- [x] [TASK-2152: 2D ComboBox・n_grid 更新（pdp_2d.rs）](TASK-2152.md) - 2h (TDD) 🔵
- [x] [TASK-2153: 統合確認・ビルドテスト](TASK-2153.md) - 1h (DIRECT) 🔵

### 依存関係

```
TASK-2149 → TASK-2150 → TASK-2151 → TASK-2152 → TASK-2153
```

全タスクが直列依存。各タスクは前タスクの完了を前提とする。

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 5件
- 🔵 **青信号**: 5件 (100%)
- 🟡 **黄信号**: 0件 (0%)
- 🔴 **赤信号**: 0件 (0%)

### タスク別信頼性

| タスク | 🔵 青 | 🟡 黄 | 🔴 赤 | 品質 |
|--------|-------|-------|-------|------|
| TASK-2149 | 5 | 0 | 0 | ✅ 高品質 |
| TASK-2150 | 2 | 0 | 0 | ✅ 高品質 |
| TASK-2151 | 5 | 0 | 0 | ✅ 高品質 |
| TASK-2152 | 3 | 0 | 0 | ✅ 高品質 |
| TASK-2153 | 3 | 0 | 0 | ✅ 高品質 |
| **合計** | **18** | **0** | **0** | ✅ 高品質 |

**品質評価**: ✅ 高品質 — 全項目が要件定義書・設計文書・ユーザヒアリングに基づく確実な実装

## クリティカルパス

```
TASK-2149 → TASK-2150 → TASK-2151 → TASK-2152 → TASK-2153
```

**クリティカルパス工数**: 12時間（全タスクがクリティカルパス上）
**並行作業可能工数**: 0時間（直列依存のため並行不可）

## 変更ファイルマッピング

| ファイル | タスク | 変更内容 |
|---|---|---|
| `rust_core/src/core/lgbm.rs` | TASK-2149 | `compute_pdp_1d_lgbm()` 追加 |
| `rust_core/src/pdp/api.rs` | TASK-2150 | `"random_forest"` ディスパッチ追加 |
| `egui-app/src/ui/widgets/pdp_chart.rs` | TASK-2151 | `ModelType::RandomForest` + 1D UI |
| `egui-app/src/ui/widgets/pdp_2d.rs` | TASK-2152 | 2D ComboBox + n_grid |

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-2149`
