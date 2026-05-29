# memory-efficiency タスク概要

**作成日**: 2026-05-29
**推定工数**: 136時間（17タスク × 8h）
**総タスク数**: 17件
**タスク番号範囲**: TASK-2328 〜 TASK-2344

## 関連文書

- **要件定義書**: [📋 requirements.md](../../spec/memory-efficiency/requirements.md)
- **アーキテクチャ設計**: [📐 architecture.md](../../design/memory-efficiency/architecture.md)
- **データフロー図**: [🔄 dataflow.md](../../design/memory-efficiency/dataflow.md)
- **型定義**: [📝 types.rs](../../design/memory-efficiency/types.rs)
- **設計ヒアリング**: [💬 design-interview.md](../../design/memory-efficiency/design-interview.md)
- **コンテキストノート**: [📝 note.md](../../spec/memory-efficiency/note.md)
- **準備タスク**: [🔧 prep.md](../../spec/memory-efficiency/prep.md)

## フェーズ構成

| フェーズ | 成果物 | タスク数 | 工数 | ファイル |
|---------|--------|----------|------|----------|
| Phase 1 | 共有Arcストア基盤（thread_local 廃止・全 study 常駐） | 3 | 24h | TASK-2328〜2330 |
| Phase 2 | StudyView/StudyContext 再設計（trial_rows 撤廃・互換シム） | 3 | 24h | TASK-2331〜2333 |
| Phase 3 | UI層ウィジェット段階移行（MEM-002/003/004） | 5 | 40h | TASK-2334〜2338 |
| Phase 4 | 比較・ライブ更新・ロード・GPU撤廃・定量検証 | 6 | 48h | TASK-2339〜2344 |

## タスク番号管理

**使用済みタスク番号**: TASK-2328 〜 TASK-2344
**次回開始番号**: TASK-2345

## 全体進捗

- [ ] Phase 1: データ層基盤（共有Arcストア）
- [ ] Phase 2: 状態層（StudyView / StudyContext 再設計）
- [ ] Phase 3: UI層（ウィジェット段階移行）
- [ ] Phase 4: 比較・ライブ更新・ロード・検証

## マイルストーン

- **M1: 共有ストア基盤完成**: thread_local 廃止・全 study 常駐（TASK-2330 完了）
- **M2: 行複製の根絶**: StudyContext から trial_rows 撤廃・StudyView 稼働（TASK-2332 完了）
- **M3: ウィジェット移行完了**: 主要ウィジェットの列アクセス化（TASK-2338 完了）
- **M4: 検証完了**: 定量ベンチで -50% 達成・全テストグリーン（TASK-2344 完了）

---

## Phase 1: データ層基盤（共有Arcストア）

**目標**: thread_local `GLOBAL_STATE` を廃止し、UI/ワーカー両スレッドから読める共有 Arc ストアを確立。全 study を初回パースで常駐化。

### タスク一覧

- [x] [TASK-2328: arc-swap 依存追加と共有ストア スケルトン](TASK-2328.md) - 8h (DIRECT) 🔵 ✅完了
- [x] [TASK-2329: SharedStudyStore 実装と thread_local 置換](TASK-2329.md) - 8h (TDD) 🔵 ✅完了
- [x] [TASK-2330: 全 study を初回パースで常駐化](TASK-2330.md) - 8h (TDD) 🔵 ✅完了

### 依存関係

```
TASK-2328 → TASK-2329 → TASK-2330
```

---

## Phase 2: 状態層（StudyView / StudyContext 再設計）

**目標**: `Vec<TrialRow>` 永続複製を排除。StudyView（Arc<DataFrame> + 並行配列）を導入し、互換シム `row_at` で段階移行を可能に。

### タスク一覧

- [x] [TASK-2331: StudyView 実装（互換シム付き）](TASK-2331.md) - 8h (TDD) 🔵 ✅完了
- [x] [TASK-2332: StudyContext 再設計と study 選択経路の刷新](TASK-2332.md) - 8h (TDD) 🔵 ✅完了
- [ ] [TASK-2333: 派生属性算出経路の StudyView 化](TASK-2333.md) - 8h (TDD) 🔵

### 依存関係

```
TASK-2329 → TASK-2331 → TASK-2332 → TASK-2333
TASK-2330 → TASK-2332
```

---

## Phase 3: UI層（ウィジェット段階移行）

**目標**: trial_rows を参照する全ウィジェットを StudyView 列アクセスへ移行し、行クローン/重複列キャッシュ/一時行列を排除（MEM-002/003/004）。

### タスク一覧

- [x] [TASK-2334: Pareto 2D/3D の行クローンキャッシュ撤廃（MEM-002）](TASK-2334.md) - 8h (TDD) 🔵 ✅完了
- [x] [TASK-2335: Parallel Coordinates / Scatter Matrix の列キャッシュ共有化（MEM-003）](TASK-2335.md) - 8h (TDD) 🔵 ✅完了
- [x] [TASK-2336: Trial Table の列アクセス移行](TASK-2336.md) - 8h (TDD) 🔵 ✅完了
- [ ] [TASK-2337: Cluster Scatter / MCDM 系ウィジェットの移行](TASK-2337.md) - 8h (TDD) 🔵
- [ ] [TASK-2338: 分析パイプライン入力の列参照化（MEM-004）](TASK-2338.md) - 8h (TDD) 🔵

### 依存関係

```
TASK-2332 → TASK-2334, TASK-2336
TASK-2332 → TASK-2335 → TASK-2338
TASK-2333 → TASK-2337
```

---

## Phase 4: 比較・ライブ更新・ロード・検証

**目標**: 比較 study 軽量化（MEM-005）、ライブ更新の ArcSwap 化、ロードピーク削減（MEM-006）、gpu_data 撤廃（MEM-007）、定量検証。

### タスク一覧

- [ ] [TASK-2339: 比較 study の軽量化と再パース廃止（MEM-005）](TASK-2339.md) - 8h (TDD) 🔵
- [x] [TASK-2340: ライブ更新の ArcSwap スナップショット差替え](TASK-2340.md) - 8h (TDD) 🔵 ✅完了（TASK-2332 と同時実装）
- [ ] [TASK-2341: ジャーナルパースのピークメモリ削減（MEM-006）](TASK-2341.md) - 8h (TDD) 🟡
- [~] [TASK-2342: gpu_data 撤廃と互換シム除去（MEM-007）](TASK-2342.md) - 8h (TDD) 🔵 gpu_data撤廃✅完了 / 互換シムrow_at除去は全ウィジェット移行後に保留
- [ ] [TASK-2343: メモリ計測ベンチマーク基盤とベースライン測定](TASK-2343.md) - 8h (DIRECT) 🟡
- [ ] [TASK-2344: 改修後メモリ定量検証と等価性確認](TASK-2344.md) - 8h (TDD) 🔵

### 依存関係

```
TASK-2330 → TASK-2339 → TASK-2344
TASK-2332/2329 → TASK-2340 → TASK-2342
TASK-2330 → TASK-2341 → TASK-2344
TASK-2334/2336/2337/2340 → TASK-2342 → TASK-2344
TASK-2343 → TASK-2344
TASK-2338 → TASK-2344
```

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 17件
- 🔵 **青信号**: 15件 (88%)
- 🟡 **黄信号**: 2件 (12%) — TASK-2341（パース最適化の実装方式選択）、TASK-2343（Windows 測定手段の確定）
- 🔴 **赤信号**: 0件 (0%)

### フェーズ別信頼性

| フェーズ | 🔵 青 | 🟡 黄 | 🔴 赤 | 合計 |
|---------|-------|-------|-------|------|
| Phase 1 | 3 | 0 | 0 | 3 |
| Phase 2 | 3 | 0 | 0 | 3 |
| Phase 3 | 5 | 0 | 0 | 5 |
| Phase 4 | 4 | 2 | 0 | 6 |

**品質評価**: 高品質

## クリティカルパス

```
TASK-2328 → TASK-2329 → TASK-2330 → TASK-2332 → TASK-2334/2336/2337 → TASK-2342 → TASK-2344
```

並びに `TASK-2332 → TASK-2335 → TASK-2338 → TASK-2344` が並走。

**クリティカルパス工数**: 約 56〜64h（依存直列分）
**並行作業可能**: Phase 3 のウィジェット移行（TASK-2334〜2338）は TASK-2332/2333 完了後に大部分を並行実施可能。

## 注意事項（実装前の確認）

- **破壊的変更**: `StudyContext` から `trial_rows`/`gpu_data` を撤廃。互換シム `row_at`（TASK-2331）で段階移行し、全移行後に TASK-2342 で除去。
- **前提準備**: TASK-2343/2344 は 100k×22 の代表ベンチマークデータセット（[prep.md](../../spec/memory-efficiency/prep.md) 必須タスク）が前提。
- **要再確認**: TASK-2341（パース最適化方式）と TASK-2340（ライブ更新の全列再ビルドコスト）はベンチ結果で方式を確定。

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-2328`
