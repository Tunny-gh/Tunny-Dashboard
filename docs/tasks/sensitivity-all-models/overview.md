# sensitivity-all-models タスク概要

**作成日**: 2026-04-15
**プロジェクト期間**: 2026-04-15 - 2026-04-17（3日）
**推定工数**: 10時間
**総タスク数**: 6件

## 関連文書

- **設計文書**: [📐 architecture.md](../../design/sensitivity-all-models/architecture.md)
- **データフロー図**: [🔄 dataflow.md](../../design/sensitivity-all-models/dataflow.md)
- **ヒアリング記録**: [📋 design-interview.md](../../design/sensitivity-all-models/design-interview.md)

## フェーズ構成

| フェーズ | 目標 | タスク数 | 工数 | ファイル |
|---------|------|----------|------|----------|
| Phase 1 | rust_core リファクタリング | 3 | 4h | TASK-2064〜2066 |
| Phase 2 | egui-app 拡張 | 3 | 6h | TASK-2067〜2069 |

## タスク番号管理

**使用済みタスク番号**: TASK-2064 〜 TASK-2069
**次回開始番号**: TASK-2070

## 全体進捗

- [ ] Phase 1: rust_core リファクタリング
- [ ] Phase 2: egui-app 拡張

## マイルストーン

- **M1: rust_core 完成** (Day 1-2): `compute_sensitivity_for` + 個別計算関数が動作
- **M2: 機能完成** (Day 3): ImportanceChart で全メトリクス Run ボタンが動作

---

## Phase 1: rust_core リファクタリング

**期間**: Day 1-2
**目標**: 選択したメトリクスのみ計算できる `compute_sensitivity_for(metric)` を追加
**成果物**: `SensitivityMetric` 列挙型、`compute_sensitivity_for`、個別計算関数

### タスク一覧

- [ ] [TASK-2064: SensitivityMetric 列挙型追加](TASK-2064.md) - 1h (TDD) 🔵
- [ ] [TASK-2065: full.rs / selected.rs 個別計算関数追加](TASK-2065.md) - 2h (TDD) 🔵
- [ ] [TASK-2066: compute_sensitivity_for エントリーポイント追加](TASK-2066.md) - 1h (TDD) 🔵

### 依存関係

```
TASK-2064 → TASK-2065 → TASK-2066
```

---

## Phase 2: egui-app 拡張

**期間**: Day 2-3
**目標**: ImportanceChart に Run ボタンと全メトリクス表示（Spearman/Ridge/RfAnova/Sobol）を追加
**成果物**: 更新済み ImportanceChart、grid_canvas メトリクス別タスクスポーン

### タスク一覧

- [ ] [TASK-2067: SensitivityResult 拡張 + AppMessage::SensitivityError 追加](TASK-2067.md) - 1h (TDD) 🔵
- [ ] [TASK-2068: ImportanceChart 全メトリクス対応 + Run ボタン追加](TASK-2068.md) - 3h (TDD) 🔵
- [ ] [TASK-2069: grid_canvas.rs メトリクス別 spawn_task + 統合確認](TASK-2069.md) - 2h (TDD) 🔵

### 依存関係

```
TASK-2066 → TASK-2067 → TASK-2068 → TASK-2069
```

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 6件
- 🔵 **青信号**: 5件 (83%)
- 🟡 **黄信号**: 1件 (17%)
- 🔴 **赤信号**: 0件 (0%)

### フェーズ別信頼性

| フェーズ | 🔵 青 | 🟡 黄 | 🔴 赤 | 合計 |
|---------|-------|-------|-------|------|
| Phase 1 | 3 | 0 | 0 | 3 |
| Phase 2 | 2 | 1 | 0 | 3 |

**品質評価**: ✅ 高品質

## クリティカルパス

```
TASK-2064 → TASK-2065 → TASK-2066 → TASK-2067 → TASK-2068 → TASK-2069
```

**クリティカルパス工数**: 10時間

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-2064`
