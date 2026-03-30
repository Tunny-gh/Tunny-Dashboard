# visualization-enablement タスク概要

**作成日**: 2026-03-30
**プロジェクト期間**: Phase 1〜3（推定6日）
**推定工数**: 41時間
**総タスク数**: 9件

## 関連文書

- **要件定義書**: [📋 requirements.md](../spec/visualization-enablement/requirements.md)
- **設計文書（アーキテクチャ）**: [📐 architecture.md](../design/visualization-enablement/architecture.md)
- **データフロー図**: [🔄 dataflow.md](../design/visualization-enablement/dataflow.md)
- **インターフェース定義**: [📝 interfaces.ts](../design/visualization-enablement/interfaces.ts)
- **設計ヒアリング記録**: [💬 design-interview.md](../design/visualization-enablement/design-interview.md)

## フェーズ構成

| フェーズ | 成果物 | タスク数 | 工数 | ファイル |
|---------|--------|----------|------|----------|
| Phase 1 - WASM インフラ | WASM バインディング・型定義・WasmLoader | 2件 | 7h | [TASK-1601〜1602](#phase-1-wasm-インフラ) |
| Phase 2 - Zustand ストア | analysisStore・clusterStore | 2件 | 11h | [TASK-1603〜1604](#phase-2-zustand-ストア) |
| Phase 3 - コンポーネント＋配線 | チャートコンポーネント・FreeLayoutCanvas配線 | 5件 | 23h | [TASK-1605〜1609](#phase-3-チャートコンポーネント配線) |

## タスク番号管理

**使用済みタスク番号**: TASK-1601 〜 TASK-1609
**次回開始番号**: TASK-1701

## 全体進捗

- [ ] Phase 1: WASM インフラ
- [ ] Phase 2: Zustand ストア
- [ ] Phase 3: チャートコンポーネント＋配線

## マイルストーン

- **M1: WASM インフラ完成**: TASK-1602 完了時 — TypeScript コンパイルエラーゼロ
- **M2: ストア完成**: TASK-1604 完了時 — analysisStore・clusterStore テスト全件合格
- **M3: 全チャート動作**: TASK-1609 完了時 — 4チャートが FreeLayoutCanvas で動作

---

## Phase 1: WASM インフラ

**目標**: 6つの WASM 関数をブラウザから呼び出せる状態にする
**成果物**: `rust_core/src/lib.rs`（6バインディング追加）・`tunny_core.d.ts`（5型+6関数）・`wasmLoader.ts`（6メソッド）

### タスク一覧

- [ ] [TASK-1601: WASM バインディング追加 (lib.rs 6関数)](TASK-1601.md) - 4h (DIRECT) 🔵
- [ ] [TASK-1602: TypeScript型宣言 + WasmLoaderバインディング追加](TASK-1602.md) - 3h (DIRECT) 🔵

### 依存関係

```
TASK-1601 → TASK-1602
```

---

## Phase 2: Zustand ストア

**目標**: analysisStore・clusterStore を TDD で実装し、WASM アクションをテスト済みにする
**成果物**: `analysisStore.ts` + `.test.ts`・`clusterStore.ts` + `.test.ts`

### タスク一覧

- [ ] [TASK-1603: analysisStore 実装 + テスト](TASK-1603.md) - 5h (TDD) 🔵
- [ ] [TASK-1604: clusterStore 実装 + テスト](TASK-1604.md) - 6h (TDD) 🔵

### 依存関係

```
TASK-1602 → TASK-1603
TASK-1602 → TASK-1604
（TASK-1603 と TASK-1604 は並行実行可能）
```

---

## Phase 3: チャートコンポーネント＋配線

**目標**: 4チャートを FreeLayoutCanvas で実際に動作させる
**成果物**: `ImportanceChart.tsx`・`ClusterScatter.tsx`・`DimReductionScatter.tsx`（各+test）・`FreeLayoutCanvas.tsx`（4ケース追加）・`LeftPanel.tsx`（clusterStore接続）

### タスク一覧

- [ ] [TASK-1605: ImportanceChart コンポーネント実装 + テスト](TASK-1605.md) - 6h (TDD) 🔵
- [ ] [TASK-1606: ClusterScatter コンポーネント実装 + テスト](TASK-1606.md) - 5h (TDD) 🔵
- [ ] [TASK-1607: DimReductionScatter コンポーネント実装 + テスト](TASK-1607.md) - 5h (TDD) 🔵
- [ ] [TASK-1608: FreeLayoutCanvas 4ケース配線](TASK-1608.md) - 4h (TDD) 🔵
- [ ] [TASK-1609: LeftPanel → clusterStore 配線](TASK-1609.md) - 3h (TDD) 🔵

### 依存関係

```
TASK-1603 → TASK-1605
TASK-1604 → TASK-1606
TASK-1604 → TASK-1607
TASK-1604 → TASK-1609
TASK-1605, TASK-1606, TASK-1607 → TASK-1608
（TASK-1605, 1606, 1607, 1609 は並行実行可能）
```

---

## 信頼性レベルサマリー

### タスク別信頼性

| タスク | 全体レベル |
|--------|-----------|
| TASK-1601 | 🔵 高品質 |
| TASK-1602 | 🔵 高品質 |
| TASK-1603 | 🔵 高品質 |
| TASK-1604 | 🔵 高品質 |
| TASK-1605 | 🔵 高品質 |
| TASK-1606 | 🔵 高品質（🟡 1件: クラスタラベルなし表示） |
| TASK-1607 | 🔵 高品質 |
| TASK-1608 | 🔵 高品質 |
| TASK-1609 | 🔵 高品質（🟡 1件: onEstimateK Props確認） |

### 全体統計

- **総タスク数**: 9件
- 🔵 **青信号タスク**: 9件 (100%)
- 🟡 **黄信号項目（タスク内）**: 2件（TASK-1606・TASK-1609 各1件）
- 🔴 **赤信号**: 0件 (0%)

**品質評価**: ✅ 高品質

## クリティカルパス

```
TASK-1601 → TASK-1602 → TASK-1603 → TASK-1605 → TASK-1608
                       → TASK-1604 → TASK-1606 ↗
                                   → TASK-1607 ↗
                                   → TASK-1609
```

**クリティカルパス工数**: 4 + 3 + 5 + 6 + 4 = 22時間
**並行作業可能**: TASK-1603/1604 並行（11h→6h）、TASK-1605/1606/1607/1609 並行（19h→6h）

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-1601`
