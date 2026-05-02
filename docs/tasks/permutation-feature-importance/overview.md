# Permutation Feature Importance タスク概要

**作成日**: 2026-05-02
**プロジェクト期間**: Phase 1 + Phase 2（合計 2〜3 日）
**推定工数**: 20時間
**総タスク数**: 5件

## 関連文書

- **要件定義書**: [📋 requirements.md](../spec/permutation-feature-importance/requirements.md)
- **設計文書**: [📐 architecture.md](../design/permutation-feature-importance/architecture.md)
- **データフロー図**: [🔄 dataflow.md](../design/permutation-feature-importance/dataflow.md)
- **実装ガイド**: [📝 implementation-guide.md](../design/permutation-feature-importance/implementation-guide.md)
- **コンテキストノート**: [📝 note.md](../spec/permutation-feature-importance/note.md)

## フェーズ構成

| フェーズ | 成果物 | タスク数 | 工数 | ファイル |
|---------|--------|----------|------|----------|
| Phase 1 - rust_core 実装 | permutation.rs 新規作成、型システム統合 | 2 | 10h | [TASK-2156〜2157](#phase-1-rust_core-実装) |
| Phase 2 - egui-app 実装 | UI・ディスパッチ・データ型追加 | 3 | 10h | [TASK-2158〜2160](#phase-2-egui-app-実装) |

## タスク番号管理

**使用済みタスク番号**: TASK-2156 ~ TASK-2160
**次回開始番号**: TASK-2161

## 全体進捗

- [x] Phase 1: rust_core 実装
- [x] Phase 2: egui-app 実装

## マイルストーン

- **M1: rust_core 完成**: permutation.rs + types.rs + full.rs が完成し `cargo test -p tunny-core` PASS
- **M2: egui-app 完成**: すべての UI / ディスパッチ変更が完成し `cargo test` PASS
- **M3: 統合確認**: `cargo clippy` クリーン、Permutation が UI で動作確認

---

## Phase 1: rust_core 実装

**目標**: `compute_permutation_importances` 関数の実装と rust_core への統合
**成果物**: `permutation.rs`（新規）、`types.rs` / `mod.rs` / `full.rs` の変更

### タスク一覧

- [x] [TASK-2156: permutation.rs 新規作成（コアアルゴリズム実装）](TASK-2156.md) - 6h (TDD) 🔵
- [x] [TASK-2157: rust_core 型定義・統合（types.rs / mod.rs / full.rs）](TASK-2157.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-2156 → TASK-2157
```

---

## Phase 2: egui-app 実装

**目標**: egui-app に Permutation UI・ディスパッチ・データ型を追加してエンドツーエンドを完成させる
**成果物**: `results.rs` / `importance_chart.rs` / `chart_registry.rs` の変更

### タスク一覧

- [x] [TASK-2158: egui-app データ型追加（results.rs）](TASK-2158.md) - 2h (TDD) 🔵
- [x] [TASK-2159: egui-app UI実装（importance_chart.rs）](TASK-2159.md) - 4h (TDD) 🔵
- [x] [TASK-2160: egui-app ディスパッチ実装（chart_registry.rs）](TASK-2160.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-2157 → TASK-2158
TASK-2158 → TASK-2159
TASK-2158 → TASK-2160
TASK-2159 → TASK-2160
```

---

## 変更対象ファイル一覧

| ファイル | 変更種別 | 担当タスク |
|---------|---------|-----------|
| `rust_core/src/sensitivity/permutation.rs` | **新規作成** | TASK-2156 |
| `rust_core/src/sensitivity/types.rs` | 変更 | TASK-2157 |
| `rust_core/src/sensitivity/mod.rs` | 変更 | TASK-2157 |
| `rust_core/src/sensitivity/analysis/full.rs` | 変更 | TASK-2157 |
| `egui-app/src/state/results.rs` | 変更 | TASK-2158 |
| `egui-app/src/ui/widgets/importance_chart.rs` | 変更 | TASK-2159 |
| `egui-app/src/ui/chart_registry.rs` | 変更 | TASK-2160 |

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 5件
- 🔵 **青信号**: 5件 (100%)
- 🟡 **黄信号**: 0件 (0%)
- 🔴 **赤信号**: 0件 (0%)

### フェーズ別信頼性

| フェーズ | 🔵 青 | 🟡 黄 | 🔴 赤 | 合計 |
|---------|-------|-------|-------|------|
| Phase 1 | 2 | 0 | 0 | 2 |
| Phase 2 | 3 | 0 | 0 | 3 |

**品質評価**: ✅ 高品質

## クリティカルパス

```
TASK-2156 → TASK-2157 → TASK-2158 → TASK-2159 → TASK-2160
```

**クリティカルパス工数**: 20時間
**並行作業可能工数**: 0時間（直列依存）

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-2156`
