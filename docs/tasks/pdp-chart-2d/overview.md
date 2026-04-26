# pdp-chart-2d タスク概要

**作成日**: 2026-04-15
**プロジェクト期間**: 2026-04-15 - 2026-04-24（7〜9日）
**推定工数**: 50時間
**総タスク数**: 9件

## 関連文書

- **設計文書**: [📐 architecture.md](../../design/pdp-chart-2d/architecture.md)
- **データフロー図**: [🔄 dataflow.md](../../design/pdp-chart-2d/dataflow.md)
- **ヒアリング記録**: [📋 design-interview.md](../../design/pdp-chart-2d/design-interview.md)
- **参照: chart-implementation 設計**: [📐 ../chart-implementation/architecture.md](../../design/chart-implementation/architecture.md)

## フェーズ構成

| フェーズ | 目標 | タスク数 | 工数 | ファイル |
|---------|------|----------|------|----------|
| Phase 1 | rust_core GP分散計算基盤（並列実行可能） | 3 | 18h | TASK-2053〜2055 |
| Phase 2 | rust_core 不確実性計算統合 | 1 | 6h | TASK-2056 |
| Phase 3 | egui-app 接続（ChartId・UI・tx伝播） | 3 | 16h | TASK-2057〜2059 |
| Phase 4 | 不確実性可視化（デュアルヒートマップ） | 1 | 6h | TASK-2060 |
| Phase 5 | 統合確認・動作テスト | 1 | 4h | TASK-2061 |

## タスク番号管理

**使用済みタスク番号**: TASK-2053 〜 TASK-2061
**次回開始番号**: TASK-2062

## 全体進捗

- [x] Phase 1: rust_core GP分散計算基盤
- [x] Phase 2: rust_core 不確実性計算統合
- [x] Phase 3: egui-app 接続
- [x] Phase 4: 不確実性可視化
- [x] Phase 5: 統合確認

## マイルストーン

- **M1: GP分散基盤完成** (Day 2): GpModel拡張・predict_variance・SparseFitcModel・PdpResult2d型統一 完了
- **M2: rust_core不確実性計算完成** (Day 3): kriging_core.rs で uncertainties グリッド生成 完了
- **M3: チャート表示完成** (Day 5): PDP Chart 2D がグリッドに表示・基本計算動作 完了
- **M4: 不確実性可視化完成** (Day 7): Kriging/Sparse Kriging でデュアルヒートマップ表示 完了
- **M5: リリース準備完了** (Day 9): 全テスト通過・目視確認 完了

---

## Phase 1: rust_core GP分散計算基盤

**期間**: Day 1-2
**目標**: GP事後分散・FITC分散の計算基盤を整備し、PdpResult2d 型を統一する
**成果物**: GpModel拡張、predict_variance、SparseFitcModel、PdpResult2d 型統一
**並列実行**: TASK-2053, TASK-2054, TASK-2055 は相互に独立して並行実行可能

### タスク一覧

- [x] [TASK-2053: GpModel拡張 + predict_variance 実装](TASK-2053.md) - 6h (TDD) 🔵
- [x] [TASK-2054: SparseFitcModel + FITC分散予測 実装](TASK-2054.md) - 8h (TDD) 🔵
- [x] [TASK-2055: PdpResult2d 型統一（uncertainties追加）](TASK-2055.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-2053 ─┐
TASK-2054 ─┼→ TASK-2056
TASK-2055 ─┘
```

---

## Phase 2: rust_core 不確実性計算統合

**期間**: Day 3
**目標**: kriging_core.rs で不確実性グリッドを計算し PdpResult2d::uncertainties に格納する
**成果物**: 不確実性計算済みの PdpResult2d を返す compute_pdp_2d

### タスク一覧

- [x] [TASK-2056: kriging_core.rs 不確実性グリッド計算](TASK-2056.md) - 6h (TDD) 🔵

### 依存関係

```
TASK-2053 + TASK-2054 + TASK-2055 → TASK-2056
```

---

## Phase 3: egui-app 接続

**期間**: Day 4-5
**目標**: チャートピッカーに PDP Chart 2D を追加し、Run ボタンで計算を起動できる状態にする
**成果物**: PDP Chart 2D がグリッドに配置・計算・結果表示できる状態

### タスク一覧

- [x] [TASK-2057: ChartId::PdpChart2D + AppMessage::Pdp2dDone 追加](TASK-2057.md) - 4h (TDD) 🔵
- [x] [TASK-2058: pdp_2d.rs UI拡張（モデル選択・Runボタン・pending_compute）](TASK-2058.md) - 6h (TDD) 🔵
- [x] [TASK-2059: tx伝播 + grid_canvas PdpChart2Dケース + app.rs Pdp2dDoneハンドラ](TASK-2059.md) - 6h (TDD) 🔵

### 依存関係

```
TASK-2055 → TASK-2057
TASK-2057 → TASK-2058
TASK-2057 + TASK-2058 → TASK-2059
```

---

## Phase 4: 不確実性可視化

**期間**: Day 6
**目標**: Kriging / Sparse Kriging 選択時にデュアルヒートマップで平均と標準偏差を表示する
**成果物**: 平均（viridis）+ σ（plasma）の 2 ペインヒートマップ

### タスク一覧

- [x] [TASK-2060: pdp_2d.rs デュアルヒートマップ描画](TASK-2060.md) - 6h (TDD) 🔵

### 依存関係

```
TASK-2059 → TASK-2060
```

---

## Phase 5: 統合確認

**期間**: Day 7
**目標**: 全テスト通過・全モデルで動作確認
**成果物**: 本番リリース可能な PDP Chart 2D 実装

### タスク一覧

- [x] [TASK-2061: 統合確認・動作テスト](TASK-2061.md) - 4h (DIRECT) 🔵

### 依存関係

```
TASK-2056 + TASK-2060 → TASK-2061
```

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 9件
- 🔵 **青信号**: 9件 (100%)
- 🟡 **黄信号**: 0件 (0%)
- 🔴 **赤信号**: 0件 (0%)

### フェーズ別信頼性

| フェーズ | 🔵 青 | 🟡 黄 | 🔴 赤 | 合計 |
|---------|-------|-------|-------|------|
| Phase 1 | 3 | 0 | 0 | 3 |
| Phase 2 | 1 | 0 | 0 | 1 |
| Phase 3 | 3 | 0 | 0 | 3 |
| Phase 4 | 1 | 0 | 0 | 1 |
| Phase 5 | 1 | 0 | 0 | 1 |

**品質評価**: ✅ 高品質

## クリティカルパス

```
TASK-2053 → TASK-2056 → TASK-2059 → TASK-2060 → TASK-2061
```

**クリティカルパス工数**: 26時間
**並行作業可能工数**: 24時間（TASK-2054, TASK-2055, TASK-2057, TASK-2058 が並行実行可能）

## スコープ外

- **ParetoScatter3D**: wgpu GPU レンダリングが必要なため今回スコープ外

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-2053`
