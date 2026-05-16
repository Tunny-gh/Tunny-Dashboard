# rust-core-perf-libs タスク概要

**作成日**: 2026-05-15
**推定工数**: 76 時間
**総タスク数**: 14 件

## 関連文書

- **要件定義書**: [requirements.md](../../spec/rust-core-perf-libs/requirements.md)
- **設計文書**: [architecture.md](../../design/rust-core-perf-libs/architecture.md)
- **データフロー図**: [dataflow.md](../../design/rust-core-perf-libs/dataflow.md)
- **型定義**: [interfaces.rs](../../design/rust-core-perf-libs/interfaces.rs)
- **ヒアリング記録**: [interview-record.md](../../spec/rust-core-perf-libs/interview-record.md)

## フェーズ構成

| フェーズ | 目標 | タスク数 | 工数 | ファイル |
|---------|------|----------|------|----------|
| Phase 1 - 基盤整備 | デッドコード削除 + rand 移行 | 3 | 12h | TASK-2301~2303 |
| Phase 2 - faer 活用拡大 | データレイアウト移行 + 線形代数高速化 | 6 | 36h | TASK-2304~2309 |
| Phase 3 - argmin 導入 | L-BFGS 最適化の argmin 化 | 2 | 14h | TASK-2310~2311 |
| Phase 4 - linfa-clustering 導入 | K-means の外部 crate 化 | 2 | 12h | TASK-2312~2313 |
| Phase 5 - 全体検証 | テスト・ベンチマーク・ビルド確認 | 1 | 6h | TASK-2314 |

## タスク番号管理

**使用済みタスク番号**: TASK-2301 ~ TASK-2314
**次回開始番号**: TASK-2315

## 全体進捗

- [ ] Phase 1: 基盤整備
- [ ] Phase 2: faer 活用拡大
- [ ] Phase 3: argmin 導入
- [x] Phase 4: linfa-clustering 導入
- [x] Phase 5: 全体検証

## マイルストーン

- **M1: 基盤完成**: デッドコード削除 + rand 統一完了（TASK-2303 完了）
- **M2: 高速化完了**: faer 活用拡大 + データレイアウト移行完了（TASK-2309 完了）
- **M3: 最適化完了**: argmin 導入完了（TASK-2311 完了）
- **M4: 外部 crate 統合完了**: linfa-clustering 導入完了（TASK-2313 完了）
- **M5: リリース準備完了**: 全テスト・ベンチマーク確認完了（TASK-2314 完了）

---

## Phase 1: 基盤整備

**目標**: デッドコード削除 + rand 統一により、後続フェーズの基盤を整える
**成果物**: SeededRng、クリーンな lib.rs

### タスク一覧

- [x] [TASK-2301: 依存パッケージ追加とプロジェクト設定](TASK-2301.md) - 2h (DIRECT) 🔵
- [x] [TASK-2302: SeededRng 実装と乱数生成の rand 統一](TASK-2302.md) - 6h (TDD) 🔵
- [x] [TASK-2303: Random Forest デッドコード削除と lib.rs クリーンアップ](TASK-2303.md) - 4h (DIRECT) 🔵

### 依存関係

```
TASK-2301 → TASK-2302 → TASK-2303
```

---

## Phase 2: faer 活用拡大

**目標**: PCA/FITC/Ridge の自前実装を faer に統一、Vec<Vec<f64>> → faer::Mat 全局移行
**成果物**: faer ベースの線形代数、統一データレイアウト

### タスク一覧

- [x] [TASK-2304: faer::Mat ↔ ndarray::Array2 境界変換ユーティリティ実装](TASK-2304.md) - 4h (TDD) 🔵
- [x] [TASK-2305: Vec<Vec<f64>> → faer::Mat データレイアウト移行](TASK-2305.md) - 8h (TDD) 🔵
- [x] [TASK-2306: PCA 固有値分解の faer 化](TASK-2306.md) - 6h (TDD) 🔵
- [x] [TASK-2307: FITC Cholesky・三角solve の faer 化](TASK-2307.md) - 8h (TDD) 🔵
- [x] [TASK-2308: Ridge 回帰の faer Cholesky 化](TASK-2308.md) - 6h (TDD) 🔵
- [x] [TASK-2309: linear_algebra.rs 整理とモジュール間インターフェース統一](TASK-2309.md) - 4h (DIRECT) 🔵

### 依存関係

```
TASK-2301 → TASK-2304 → TASK-2305 → TASK-2306 ──┐
                                       TASK-2307 ──┤→ TASK-2309
                                       TASK-2308 ──┘
```

---

## Phase 3: argmin 導入

**目標**: 手作り L-BFGS を argmin に置き換え、GP 学習の収束性を改善
**成果物**: argmin ベースの最適化モジュール

### タスク一覧

- [x] [TASK-2310: L-BFGS 最適化の argmin 化](TASK-2310.md) - 8h (TDD) 🔵
- [x] [TASK-2311: GP training.rs の argmin 呼び出し化と動作確認](TASK-2311.md) - 6h (TDD) 🔵

### 依存関係

```
TASK-2309 → TASK-2310 → TASK-2311
```

---

## Phase 4: linfa-clustering 導入

**目標**: 重複 K-means 実装を linfa-clustering に統合
**成果物**: linfa-clustering バックエンドの K-means

### タスク一覧

- [x] [TASK-2312: K-means の linfa-clustering バックエンド化](TASK-2312.md) - 8h (TDD) 🔵
- [x] [TASK-2313: sparse_fitc.rs 重複K-means削除とclustering統合](TASK-2313.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-2304 + TASK-2309 → TASK-2312 → TASK-2313
```

---

## Phase 5: 全体検証

**目標**: 全テスト通過、ベンチマーク目標達成、egui-app 正常動作確認
**成果物**: 性能比較レポート

### タスク一覧

- [x] [TASK-2314: 全テスト実行・ベンチマーク比較・egui-appビルド確認](TASK-2314.md) - 6h (DIRECT) 🔵

### 依存関係

```
TASK-2311 + TASK-2313 + TASK-2309 → TASK-2314
```

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 14 件
- 🔵 **青信号**: 14 件 (100%)
- 🟡 **黄信号**: 0 件 (0%)
- 🔴 **赤信号**: 0 件 (0%)

**品質評価**: ✅ 高品質 — 全タスクが要件定義書・設計文書に基づく確実な定義。

### タスクタイプ別

| タイプ | 件数 | 工数 |
|--------|------|------|
| TDD | 10 | 62h |
| DIRECT | 4 | 16h |

## クリティカルパス

```
TASK-2301 → TASK-2302 → TASK-2303 → TASK-2304 → TASK-2305 → TASK-2306 ─┐
                                                            TASK-2307 ─┤→ TASK-2309 → TASK-2310 → TASK-2311 → TASK-2314
                                                            TASK-2308 ─┘
```

**クリティカルパス工数**: 50 時間（TASK-2301→2302→2304→2305→2307→2309→2310→2311→2314）
**並行作業可能**: TASK-2306/2307/2308 は並行実行可能

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-2301`
