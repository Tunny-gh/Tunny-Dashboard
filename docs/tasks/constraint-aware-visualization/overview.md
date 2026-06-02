# constraint-aware-visualization タスク概要

**作成日**: 2026-06-03
**推定工数**: 40時間（5日）
**総タスク数**: 6件

## 関連文書

- **要件定義書**: [📋 requirements.md](../spec/constraint-aware-visualization/requirements.md)
- **設計文書**: [📐 architecture.md](../design/constraint-aware-visualization/architecture.md)
- **データフロー**: [🔄 dataflow.md](../design/constraint-aware-visualization/dataflow.md)
- **実装ガイド**: [🔧 implementation-guide.md](../design/constraint-aware-visualization/implementation-guide.md)
- **コンテキストノート**: [📝 note.md](../spec/constraint-aware-visualization/note.md)

## フェーズ構成

| フェーズ | 成果物 | タスク数 | 工数 |
|---------|--------|----------|------|
| Phase 1 - rust_core 変更 | `compute_pareto_ranks()` feasibility 対応 | 1 | 8h |
| Phase 2 - egui-app 変更 | 基盤変更 + 全チャート feasibility 描画対応 | 5 | 32h |

## タスク番号管理

**使用済みタスク番号**: TASK-2345 〜 TASK-2350
**次回開始番号**: TASK-2351

## 全体進捗

- [ ] Phase 1: rust_core 変更
- [ ] Phase 2: egui-app 変更

## マイルストーン

- **M1: rust_core 完成**: `compute_pareto_ranks()` が feasibility フィルタに対応（TASK-2345 完了後）
- **M2: 基盤完成**: `COLOR_INFEASIBLE` + 全ウィジェット `show_infeasible` フィールド追加（TASK-2346 完了後）
- **M3: 全チャート対応完了**: 全6チャートでグレーアウト表示が動作（TASK-2347〜2350 完了後）

---

## Phase 1: rust_core 変更

**目標**: Pareto ランク計算を feasibility 考慮に変更
**成果物**: `compute_pareto_ranks()` の feasibility フィルタ + 違反量ランキング

### タスク一覧

- [ ] [TASK-2345: compute_pareto_ranks() の feasibility フィルタ + 違反量ランキング実装](TASK-2345.md) - 8h (TDD) 🔵

### 依存関係

```
（前提タスクなし）TASK-2345 → Phase 2 全タスク
```

---

## Phase 2: egui-app 変更

**目標**: 全チャートで実行不可能解のグレーアウト表示 + Show Infeasible トグルを実装
**成果物**: 6 チャートウィジェットの feasibility 描画対応

### タスク一覧

- [ ] [TASK-2346: egui-app 基盤変更（COLOR_INFEASIBLE・show_infeasible フィールド・Study 切替リセット）](TASK-2346.md) - 4h (DIRECT) 🔵
- [ ] [TASK-2347: ParetoScatter2D の feasibility 描画対応](TASK-2347.md) - 8h (TDD) 🔵
- [ ] [TASK-2348: ParetoScatter3D の feasibility 描画対応](TASK-2348.md) - 8h (TDD) 🔵
- [ ] [TASK-2349: OptimizationHistory・ParallelCoords の feasibility 描画対応](TASK-2349.md) - 6h (TDD) 🔵
- [ ] [TASK-2350: ScatterMatrix・ClusterScatter の feasibility 描画対応](TASK-2350.md) - 6h (TDD) 🔵

### 依存関係

```
TASK-2345 → TASK-2346
TASK-2346 → TASK-2347 ┐
TASK-2346 → TASK-2348 ├─ 並列実行可能
TASK-2346 → TASK-2349 │
TASK-2346 → TASK-2350 ┘
```

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 6件
- 🔵 **青信号**: 6件（100%）
- 🟡 **黄信号**: 0件（0%）
- 🔴 **赤信号**: 0件（0%）

（TASK-2348 の GPU バッファ部分に 🟡 が含まれるが、タスク全体では 🔵 評価）

**品質評価**: ✅ 高品質

## クリティカルパス

```
TASK-2345 → TASK-2346 → TASK-2347 → 完了
                       → TASK-2348 ┐（並列）
                       → TASK-2349 │
                       → TASK-2350 ┘
```

**クリティカルパス工数**: 8 + 4 + 8 = 20時間（最長パス）
**最短完了**: 並列実行で約 20時間

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-2345`
