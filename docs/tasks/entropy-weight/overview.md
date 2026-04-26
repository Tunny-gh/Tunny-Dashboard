# entropy-weight タスク概要

**作成日**: 2026-04-24
**推定工数**: 24時間
**総タスク数**: 7件

## 関連文書

- **要件定義書**: [📋 requirements.md](../../spec/entropy-weight/requirements.md)
- **設計文書**: [📐 architecture.md](../../design/entropy-weight/architecture.md)
- **インターフェース定義**: [📝 interfaces.rs](../../design/entropy-weight/interfaces.rs)
- **データフロー図**: [🔄 dataflow.md](../../design/entropy-weight/dataflow.md)
- **コンテキストノート**: [📝 note.md](../../spec/entropy-weight/note.md)

## フェーズ構成

| フェーズ | 成果物 | タスク数 | 工数 | ファイル |
|---------|--------|----------|------|----------|
| Phase 1 | 計算コア実装 | 2 | 8h | TASK-0001〜0002 |
| Phase 2 | 状態・ディスパッチ実装 | 3 | 9h | TASK-0003〜0005 |
| Phase 3 | UI実装 | 2 | 7h | TASK-0006〜0007 |

## タスク番号管理

**使用済みタスク番号**: TASK-0001 ~ TASK-0007
**次回開始番号**: TASK-0008

## 全体進捗

- [ ] Phase 1: 計算コア実装
- [ ] Phase 2: 状態・ディスパッチ実装
- [ ] Phase 3: UI実装

## マイルストーン

- **M1: 計算コア完成**: entropy.rs + パフォーマンステスト完了
- **M2: ディスパッチ完成**: WeightMode切替 → エントロピー計算 → 結果反映のE2E完了
- **M3: UI完成**: WeightModeセレクタ + エントロピーテーブル表示完了

---

## Phase 1: 計算コア実装

**目標**: Shannonエントロピーに基づく重み計算アルゴリズムをrust_coreに実装
**成果物**: `rust_core/src/mcdm/entropy.rs` + パフォーマンステスト

### タスク一覧

- [ ] [TASK-0001: エントロピー計算アルゴリズム実装](TASK-0001.md) - 6h (TDD) 🔵
- [ ] [TASK-0002: モジュール登録・パフォーマンステスト](TASK-0002.md) - 2h (DIRECT) 🔵

### 依存関係

```
TASK-0001 → TASK-0002
```

---

## Phase 2: 状態・ディスパッチ実装

**目標**: WeightMode enum・EntropyResult型・dispatch・message_handlerをegui-appに追加
**成果物**: WeightMode/EntropyResult型 + AppMessage::EntropyDone + chart_registry dispatch

### タスク一覧

- [ ] [TASK-0003: WeightMode・EntropyResult 型追加](TASK-0003.md) - 3h (TDD) 🔵
- [ ] [TASK-0004: AppMessage拡張・message_handler追加](TASK-0004.md) - 2h (DIRECT) 🔵
- [ ] [TASK-0005: chart_registry エントロピー dispatch追加](TASK-0005.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-0002 → TASK-0003
TASK-0003 → TASK-0004
TASK-0004 → TASK-0005
```

---

## Phase 3: UI実装

**目標**: WeightModeセレクタ・スライダー制御・エントロピーテーブルをUIに追加
**成果物**: WeightModeセレクタ + 読み取り専用スライダー + エントロピー値テーブル

### タスク一覧

- [ ] [TASK-0006: WeightModeセレクタ・スライダー制御](TASK-0006.md) - 4h (TDD) 🔵
- [ ] [TASK-0007: エントロピーテーブル表示](TASK-0007.md) - 3h (TDD) 🔵

### 依存関係

```
TASK-0005 → TASK-0006
TASK-0006 → TASK-0007
```

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 7件
- 🔵 **青信号**: 7件 (100%)
- 🟡 **黄信号**: 0件 (0%)
- 🔴 **赤信号**: 0件 (0%)

### フェーズ別内訳

| フェーズ | 🔵 青 | 🟡 黄 | 🔴 赤 | 合計 | 工数 |
|---------|-------|-------|-------|------|------|
| Phase 1 | 2 | 0 | 0 | 2 | 8h |
| Phase 2 | 3 | 0 | 0 | 3 | 9h |
| Phase 3 | 2 | 0 | 0 | 2 | 7h |

**品質評価**: ✅ 高品質

## クリティカルパス

```
TASK-0001 → TASK-0002 → TASK-0003 → TASK-0004 → TASK-0005 → TASK-0006 → TASK-0007
```

**クリティカルパス工数**: 24時間
**並行作業可能工数**: 0時間（全タスク直列）

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-0001`
