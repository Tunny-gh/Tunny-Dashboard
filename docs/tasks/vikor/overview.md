# VIKOR タスク概要

**作成日**: 2026-04-24
**プロジェクト期間**: Phase 1: 1日 + Phase 2: 1日（合計 2日）
**推定工数**: 16時間
**総タスク数**: 4件

## 関連文書

- **要件定義書**: [📋 requirements.md](../spec/vikor/requirements.md)
- **設計文書**: [📐 architecture.md](../design/vikor/architecture.md)
- **データフロー図**: [🔄 dataflow.md](../design/vikor/dataflow.md)
- **型定義**: [📝 interfaces.rs](../design/vikor/interfaces.rs)
- **設計ヒアリング**: [💬 design-interview.md](../design/vikor/design-interview.md)
- **コンテキストノート**: [📝 note.md](../spec/vikor/note.md)

## フェーズ構成

| フェーズ | 期間 | 成果物 | タスク数 | 工数 | ファイル |
|---------|------|--------|----------|------|----------|
| Phase 1 | 1日 | VIKORアルゴリズム（rust_core） | 1 | 6h | [TASK-2033](#phase-1-計算コア) |
| Phase 2 | 1日 | egui-app 型定義・UI・統合 | 3 | 10h | [TASK-2034〜2036](#phase-2-egui-app統合) |

## タスク番号管理

**使用済みタスク番号**: TASK-2033 ~ TASK-2036
**次回開始番号**: TASK-2037

## 全体進捗

- [x] Phase 1: 計算コア（rust_core）
- [x] Phase 2: egui-app 統合

## マイルストーン

- **M1: アルゴリズム完成** (Phase 1終了): `compute_vikor()` が全テストケース通過
- **M2: VIKOR機能完成** (Phase 2終了): VIKOR手法がUIから操作可能、バーチャート・テーブル表示動作

---

## Phase 1: 計算コア

**期間**: 1日（6時間）
**目標**: VIKORアルゴリズムのpure Rust実装完成
**成果物**: `rust_core/src/mcdm/vikor.rs`、`rust_core/src/mcdm/mod.rs` 更新

### タスク一覧

- [x] [TASK-2033: VIKORアルゴリズム実装（rust_core）](TASK-2033.md) - 6h (TDD) 🔵

### 依存関係

```
TASK-2033（独立）
```

---

## Phase 2: egui-app統合

**期間**: 1日（10時間）
**目標**: egui-app に VIKOR を統合し、UIから操作・結果表示を可能にする
**成果物**: `results.rs` 型拡張、`mcdm_chart.rs` UI拡張、`chart_registry.rs` ディスパッチ追加

### タスク一覧

- [x] [TASK-2034: egui-app 状態型拡張（results.rs）](TASK-2034.md) - 3h (TDD) 🔵
- [x] [TASK-2035: egui-app UIウィジェット拡張（mcdm_chart.rs）](TASK-2035.md) - 4h (TDD) 🔵
- [x] [TASK-2036: egui-app チャート統合（chart_registry.rs）](TASK-2036.md) - 3h (TDD) 🔵

### 依存関係

```
TASK-2033 → TASK-2034
TASK-2034 → TASK-2035
TASK-2034 → TASK-2036
TASK-2035 → TASK-2036
```

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 4件
- 🔵 **青信号**: 4件 (100%)
- 🟡 **黄信号**: 0件 (0%)
- 🔴 **赤信号**: 0件 (0%)

### フェーズ別信頼性

| フェーズ | 🔵 青 | 🟡 黄 | 🔴 赤 | 合計 |
|---------|-------|-------|-------|------|
| Phase 1 | 1 | 0 | 0 | 1 |
| Phase 2 | 3 | 0 | 0 | 3 |

**品質評価**: ✅ 高品質

## クリティカルパス

```
TASK-2033 → TASK-2034 → TASK-2035 → TASK-2036
```

**クリティカルパス工数**: 16時間（全タスクが直列依存）
**並行作業可能工数**: 0時間（Phase 2の2035・2036のみ2034完了後に並行可能だが工数差小）

## 変更ファイル一覧

| ファイル | 変更種別 | タスク |
|--------|---------|--------|
| `rust_core/src/mcdm/vikor.rs` | 新規作成 | TASK-2033 |
| `rust_core/src/mcdm/mod.rs` | `pub mod vikor;` 追加 | TASK-2033 |
| `egui-app/src/state/results.rs` | `VikorResult`・`McdmMethod::Vikor`・`McdmResult::Vikor` 追加 | TASK-2034 |
| `egui-app/src/ui/widgets/mcdm_chart.rs` | `McdmComputeRequest`・`v_param`・vスライダー 追加 | TASK-2035 |
| `egui-app/src/ui/chart_registry.rs` | `McdmMethod::Vikor` dispatchアーム追加 | TASK-2036 |

## 次のステップ

タスクを実装するには:
- 最初のタスクから実装: `/tsumiki:kairo-implement TASK-2033`
- 全タスク順番に実装: `/tsumiki:kairo-implement`
