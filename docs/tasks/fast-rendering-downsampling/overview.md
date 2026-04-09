# 高速描画ダウンサンプリング タスク概要

**作成日**: 2026-04-09
**プロジェクト期間**: 2026-04-09 - 2026-04-23（9日）
**推定工数**: 72時間（4時間 × 18タスク）
**総タスク数**: 18件

## 関連文書

- **要件定義書**: [📋 requirements.md](../spec/tunny-dashboard-requirements.md)
- **設計文書**: [📐 architecture.md](../design/fast-rendering-downsampling/architecture.md)
- **インターフェース定義**: [📝 interfaces.ts](../design/fast-rendering-downsampling/interfaces.ts)
- **データフロー図**: [🔄 dataflow.md](../design/fast-rendering-downsampling/dataflow.md)
- **ヒアリング記録**: [📝 design-interview.md](../design/fast-rendering-downsampling/design-interview.md)

## フェーズ構成

| フェーズ | 期間 | 成果物 | タスク数 | 工数 |
|---------|------|--------|----------|------|
| Phase 1 | 2.5日 | sampling.rs + WASM バインディング | 5件 | 20h |
| Phase 2 | 1.5日 | downsampleStore.ts | 3件 | 12h |
| Phase 3 | 4日 | 全9チャートへの getIndices 統合 | 8件 | 32h |
| Phase 4 | 1日 | 統合テスト + パフォーマンス計測 | 2件 | 8h |

## タスク番号管理

**使用済みタスク番号**: TASK-1656 〜 TASK-1673
**次回開始番号**: TASK-1674

## 全体進捗

- [x] Phase 1: WASM Core（sampling.rs + バインディング）
- [x] Phase 2: State Management（downsampleStore）
- [x] Phase 3: Chart Integration（全9チャート）
- [x] Phase 4: Integration Test & Performance

## マイルストーン

- **M1: WASM関数完成**（+2.5日）: 4関数の Rust 実装 + WASM バインディング完了
- **M2: Store完成**（+4日）: downsampleStore の全アクション実装完了
- **M3: 全チャート統合**（+8日）: 9チャートすべてへの getIndices 統合完了
- **M4: リリース準備完了**（+9日）: 統合テスト・パフォーマンス計測完了

---

## Phase 1: WASM Core

**期間**: 2.5日（20時間）
**目標**: `rust_core/src/sampling.rs` に 4 関数を実装し、WASM バインディングを整備する
**成果物**: sampling.rs・lib.rs 更新・tunny_core.d.ts 更新・wasmLoader.ts 更新

### タスク一覧

- [x] [TASK-1656: downsample_smart 関数の Rust 実装](TASK-1656.md) - 4h (TDD) 🔵
- [x] [TASK-1657: downsample_for_thumbnail 関数の Rust 実装](TASK-1657.md) - 4h (TDD) 🔵
- [x] [TASK-1658: downsample_stratified_by_rank 関数の Rust 実装](TASK-1658.md) - 4h (TDD) 🔵
- [x] [TASK-1659: downsample_by_cluster 関数の Rust 実装](TASK-1659.md) - 4h (TDD) 🔵
- [x] [TASK-1660: WASM バインディング追加](TASK-1660.md) - 4h (DIRECT) 🔵

### 依存関係

```
TASK-1656 ┐
TASK-1657 ├→ TASK-1660
TASK-1658 ┤
TASK-1659 ┘

並列実行可能: TASK-1656, TASK-1657, TASK-1658
TASK-1659 は TASK-1656 完了後に開始
```

---

## Phase 2: State Management

**期間**: 1.5日（12時間）
**目標**: `downsampleStore.ts` を新設し、全チャート共有のインデックスキャッシュを管理する
**成果物**: downsampleStore.ts・downsampling.ts（型定義・定数）

### タスク一覧

- [x] [TASK-1661: downsampleStore 基本構造実装（Study変更対応）](TASK-1661.md) - 4h (TDD) 🔵
- [x] [TASK-1662: downsampleStore フィルタ変更検知と条件付き再計算](TASK-1662.md) - 4h (TDD) 🔵
- [x] [TASK-1663: downsampleStore getIndices と DOWNSAMPLE_CONFIGS](TASK-1663.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-1660 → TASK-1661 → TASK-1662 → TASK-1663
```

---

## Phase 3: Chart Integration

**期間**: 4日（32時間）
**目標**: 9チャートすべてに `useDownsampleStore` を統合する
**成果物**: 各チャートコンポーネントの更新・テスト

### タスク一覧

- [x] [TASK-1664: ParetoScatter2D への scatter インデックス統合](TASK-1664.md) - 4h (TDD) 🔵
- [x] [TASK-1665: ParetoScatter3D への scatter インデックス統合](TASK-1665.md) - 4h (TDD) 🔵
- [x] [TASK-1666: ObjectivePairMatrix への scatter インデックス統合](TASK-1666.md) - 4h (TDD) 🔵
- [x] [TASK-1667: ScatterMatrix への thumbnail/hover インデックス統合](TASK-1667.md) - 4h (TDD) 🔵
- [x] [TASK-1668: ParallelCoordinates への pcp インデックス統合](TASK-1668.md) - 4h (TDD) 🔵
- [x] [TASK-1669: SlicePlot への data_points インデックス統合](TASK-1669.md) - 4h (TDD) 🔵
- [x] [TASK-1670: SurfacePlot3D への data_points インデックス統合](TASK-1670.md) - 4h (TDD) 🔵
- [x] [TASK-1671: ClusterScatter / DimReductionScatter への cluster インデックス統合](TASK-1671.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-1663 → TASK-1664
TASK-1663 → TASK-1665
TASK-1663 → TASK-1666
TASK-1663 → TASK-1667
TASK-1663 → TASK-1668
TASK-1663 → TASK-1669
TASK-1663 → TASK-1670
TASK-1663 → TASK-1671

並列実行可能: TASK-1664〜1671 はすべて並列実行可能
```

---

## Phase 4: Integration Test & Performance

**期間**: 1日（8時間）
**目標**: 統合テストとパフォーマンス計測を完了する
**成果物**: 統合テストファイル・Rust ベンチマーク・パフォーマンスレポート

### タスク一覧

- [x] [TASK-1672: 統合テスト（Study切り替え・フィルタ変更フロー確認）](TASK-1672.md) - 4h (TDD) 🔵
- [x] [TASK-1673: パフォーマンス計測とベンチマーク調整](TASK-1673.md) - 4h (DIRECT) 🔵

### 依存関係

```
TASK-1664〜1671（全チャート統合） → TASK-1672 → TASK-1673
```

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 18件
- 🔵 **青信号**: 17件 (94%)
- 🟡 **黄信号**: 1件 (6%)
- 🔴 **赤信号**: 0件 (0%)

### フェーズ別信頼性

| フェーズ | 🔵 青 | 🟡 黄 | 🔴 赤 | 合計 |
|---------|-------|-------|-------|------|
| Phase 1 | 5 | 0 | 0 | 5 |
| Phase 2 | 3 | 0 | 0 | 3 |
| Phase 3 | 8 | 0 | 0 | 8 |
| Phase 4 | 1 | 1 | 0 | 2 |

**品質評価**: ✅ 高品質

## クリティカルパス

```
TASK-1656/7/8 → TASK-1659 → TASK-1660 → TASK-1661 → TASK-1662 → TASK-1663
  → TASK-1664〜1671 → TASK-1672 → TASK-1673
```

**クリティカルパス工数**: 72時間（全タスク直列の場合）
**並列実行時の短縮**: Phase 1 で 3 タスク並列 → Phase 3 で 8 タスク並列（最短 5.5日）

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-1656`
