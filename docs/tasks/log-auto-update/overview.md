# Log Auto Update タスク概要

**作成日**: 2026-05-11
**推定工数**: 52時間
**総タスク数**: 8件

## 関連文書

- **要件定義書**: [requirements.md](../../spec/log-auto-update/requirements.md)
- **アーキテクチャ設計**: [architecture.md](../../design/log-auto-update/architecture.md)
- **データフロー図**: [dataflow.md](../../design/log-auto-update/dataflow.md)
- **型定義**: [interfaces.rs](../../design/log-auto-update/interfaces.rs)
- **設計ヒアリング**: [design-interview.md](../../design/log-auto-update/design-interview.md)

## フェーズ構成

| フェーズ | 成果物 | タスク数 | 工数 | ファイル |
|---------|--------|----------|------|----------|
| Phase 1 | Core型定義・差分パーサー拡張 | 2 | 12h | TASK-2217~2218 |
| Phase 2 | ポーリングスレッド基盤 | 2 | 12h | TASK-2219~2220 |
| Phase 3 | メッセージハンドラー・状態更新 | 1 | 8h | TASK-2221 |
| Phase 4 | UI拡張 | 1 | 8h | TASK-2222 |
| Phase 5 | 統合・ライフサイクル管理 | 2 | 12h | TASK-2223~2224 |

## タスク番号管理

**使用済みタスク番号**: TASK-2217 ~ TASK-2224
**次回開始番号**: TASK-2225

## 全体進捗

- [x] Phase 1: Core差分パーサー拡張
- [x] Phase 2: ポーリングスレッド基盤
- [x] Phase 3: メッセージハンドラー・状態更新
- [x] Phase 4: UI拡張
- [x] Phase 5: 統合・ライフサイクル管理

## マイルストーン

- **M1: Core完成** ✅: rust_coreのappend_journal_diff_v2と型定義完了
- **M2: ポーラー完成** ✅: ポーリングループとエラー処理完了
- **M3: データ反映完成** ✅: MessageHandlerでStudyContext更新・Pareto再計算完了
- **M4: UI完成** ✅: 試行数カウンタ・間隔調整・トグル制御完了
- **M5: リリース準備完了** ✅: 全統合テスト・エッジケース検証完了

---

## Phase 1: Core差分パーサー拡張

**目標**: rust_coreにインクリメンタルTrialRow構築機能を追加
**成果物**: append_journal_diff_v2、LiveUpdateContext、TrialRowV2

### タスク一覧

- [x] [TASK-2217: LiveUpdateContext構造体定義とLiveUpdateState拡張](TASK-2217.md) - 4h (DIRECT) 🔵
- [x] [TASK-2218: append_journal_diff_v2 — TrialRow構築付き差分パーサー実装](TASK-2218.md) - 8h (TDD) 🔵

### 依存関係

```
TASK-2217 → TASK-2218
```

---

## Phase 2: ポーリングスレッド基盤

**目標**: 専用ポーリングスレッドでファイル変化を検出し差分データを送信
**成果物**: live_update_poller.rs、エラー処理・完了通知ロジック

### タスク一覧

- [x] [TASK-2219: LiveUpdatePollerモジュール作成とポーリングループ実装](TASK-2219.md) - 8h (TDD) 🔵
- [x] [TASK-2220: エラーカウント・自動停止・最適化完了通知ロジック](TASK-2220.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-2218 → TASK-2219 → TASK-2220
```

---

## Phase 3: メッセージハンドラー・状態更新

**目標**: LiveUpdateDoneメッセージでAppStateを更新しグラフを再描画
**成果物**: MessageHandler LiveUpdateDone実装、Pareto再計算、GPU再構築

### タスク一覧

- [x] [TASK-2221: LiveUpdateDoneメッセージ拡張とMessageHandler実装](TASK-2221.md) - 8h (TDD) 🔵

### 依存関係

```
TASK-2218 → TASK-2221
```

---

## Phase 4: UI拡張

**目標**: ツールバーに試行数・間隔調整UIを追加
**成果物**: 試行数カウンタ、間隔スライダー、トグル無効化

### タスク一覧

- [x] [TASK-2222: ツールバー拡張（試行数カウンタ・間隔調整UI・トグル無効化）](TASK-2222.md) - 8h (TDD) 🔵

### 依存関係

```
(独立 — Phase 1-3と並行実装可能)
```

---

## Phase 5: 統合・ライフサイクル管理

**目標**: 全コンポーネントを統合しライフサイクル管理とE2Eテストを完了
**成果物**: app.rs Poller制御、統合テストスイート

### タスク一覧

- [x] [TASK-2223: app.rs Pollerライフサイクル管理とファイル切替](TASK-2223.md) - 8h (TDD) 🔵
- [x] [TASK-2224: 統合テスト・エッジケース検証](TASK-2224.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-2219 + TASK-2220 + TASK-2221 + TASK-2222 → TASK-2223 → TASK-2224
```

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 8件
- 🔵 **青信号**: 7件 (88%)
- 🟡 **黄信号**: 1件 (12%)
- 🔴 **赤信号**: 0件 (0%)

### フェーズ別信頼性

| フェーズ | 🔵 青 | 🟡 黄 | 🔴 赤 | 合計 |
|---------|-------|-------|-------|------|
| Phase 1 | 2 | 0 | 0 | 2 |
| Phase 2 | 1 | 1 | 0 | 2 |
| Phase 3 | 1 | 0 | 0 | 1 |
| Phase 4 | 1 | 0 | 0 | 1 |
| Phase 5 | 2 | 0 | 0 | 2 |

**品質評価**: 高品質 — 全タスクが設計文書に基づき、既存アーキテクチャパターンの派生

## クリティカルパス

```
TASK-2217 → TASK-2218 → TASK-2219 → TASK-2220 → TASK-2223 → TASK-2224
                                                          ↑
TASK-2218 → TASK-2221 ──────────────────────────────────┘
TASK-2222 ──────────────────────────────────────────────┘
```

**クリティカルパス工数**: 36時間
**並行作業可能工数**: 16時間（TASK-2221 + TASK-2222）

## 並行実装の機会

Phase 2（TASK-2219）とPhase 3（TASK-2221）はTASK-2218完了後に並行開始可能。
Phase 4（TASK-2222）はPhase 1-3と完全に独立して並行実装可能。

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-2217`
