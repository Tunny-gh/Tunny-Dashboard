# pdp-linspace-dedup タスク概要

**作成日**: 2026-05-01
**推定工数**: 1時間
**総タスク数**: 2件

## 関連文書

- **要件定義書**: [📋 requirements.md](../../spec/pdp-linspace-dedup/requirements.md)
- **設計文書**: [📐 architecture.md](../../design/pdp-linspace-dedup/architecture.md)
- **データフロー図**: [🔄 dataflow.md](../../design/pdp-linspace-dedup/dataflow.md)

## フェーズ構成

| フェーズ | 成果物 | タスク数 | 工数 | ファイル |
|---------|--------|----------|------|----------|
| Phase 1 | linspace 共通化 | 2 | 1h | [TASK-2154~2155](#phase-1---linspace-共通化) |

※本機能は変更ファイル4つ・推定工数1hの小規模リファクタリングのため、単一フェーズで完結する。

## タスク番号管理

**使用済みタスク番号**: TASK-2154 ~ TASK-2155
**次回開始番号**: TASK-2156

## 全体進捗

- [x] Phase 1: linspace 共通化

## マイルストーン

- **M1: 共通モジュール作成完了**: grid.rs 新規作成 (TASK-2154)
- **M2: 移行完了**: 全テスト通過 (TASK-2155)

---

## Phase 1 - linspace 共通化

**目標**: `linspace` 関数を `core::math::grid` に集約し重複を解消する
**成果物**: `core::math::grid::linspace`（pub(crate)）、重複関数の削除

### タスク一覧

- [x] [TASK-2154: core::math::grid モジュール新規作成](TASK-2154.md) - 0.5h (DIRECT) 🔵
- [x] [TASK-2155: 既存インポートの切り替えと重複削除](TASK-2155.md) - 0.5h (DIRECT) 🔵

### 依存関係

```
TASK-2154 → TASK-2155
```

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 2件
- 🔵 **青信号**: 2件 (100%)
- 🟡 **黄信号**: 0件 (0%)
- 🔴 **赤信号**: 0件 (0%)

### タスク別信頼性

| タスク | 🔵 青 | 🟡 黄 | 🔴 赤 | 品質 |
|--------|-------|-------|-------|------|
| TASK-2154 | 4 | 0 | 0 | ✅ 高品質 |
| TASK-2155 | 6 | 0 | 0 | ✅ 高品質 |
| **合計** | **10** | **0** | **0** | ✅ 高品質 |

**品質評価**: ✅ 高品質 — 全項目が要件定義書・設計文書に基づく確実な実装

## クリティカルパス

```
TASK-2154 → TASK-2155
```

**クリティカルパス工数**: 1時間

## 変更ファイルマッピング

| ファイル | タスク | 変更内容 |
|---|---|---|
| `rust_core/src/core/math/grid.rs` | TASK-2154 | 新規: `linspace` 関数 |
| `rust_core/src/core/math/mod.rs` | TASK-2154 | `mod grid;` 追加 |
| `rust_core/src/core/lgbm.rs` | TASK-2155 | `pdp_linspace` 削除 + use 追加 |
| `rust_core/src/pdp/ridge_core.rs` | TASK-2155 | import 分離 |
| `rust_core/src/pdp/kriging_core.rs` | TASK-2155 | import 変更 |
| `rust_core/src/pdp/utils.rs` | TASK-2155 | `linspace` 定義削除 |

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-2154`
