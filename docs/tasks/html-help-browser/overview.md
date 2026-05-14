# html-help-browser タスク概要

**作成日**: 2026-05-14
**プロジェクト期間**: 2026-05-14 - 2026-05-25（8日）
**推定工数**: 46時間
**総タスク数**: 10件（既存3件 + 新規7件）

## 関連文書

- **要件定義書**: [📋 requirements.md](../spec/html-help-browser/requirements.md)
- **設計文書**: [📐 architecture.md](../design/html-help-browser/architecture.md)
- **データフロー**: [🔄 dataflow.md](../design/html-help-browser/dataflow.md)
- **型定義**: [📝 interfaces.rs](../design/html-help-browser/interfaces.rs)
- **受け入れ基準**: [📋 acceptance-criteria.md](../spec/html-help-browser/acceptance-criteria.md)

## フェーズ構成

| フェーズ | 期間 | 成果物 | タスク数 | 工数 |
|---------|------|--------|----------|------|
| Phase 1 | 5/14〜5/16 | build.rs HTML変換・LaTeX移行 | 3件 | 18h |
| Phase 2 | 5/19〜5/20 | Rustランタイム型・コンテンツ・起動 | 3件 | 12h |
| Phase 3 | 5/21〜5/22 | AppState統合・言語切替UI・旧システム削除 | 3件 | 12h |
| Phase 4 | 5/25 | 統合確認・E2E動作確認 | 1件 | 4h |

## タスク番号管理

**使用済みタスク番号**: TASK-2248 ~ TASK-2257
**次回開始番号**: TASK-2258

## 全体進捗

- [ ] Phase 1: ビルドパイプライン
- [ ] Phase 2: Rustランタイム実装
- [ ] Phase 3: アプリ統合
- [ ] Phase 4: 統合確認

## マイルストーン

- **M1: ビルドパイプライン完成** (2026-05-16): build.rs HTML生成・LaTeX移行完了
- **M2: Rustランタイム完成** (2026-05-20): help_types / help_content / help_launcher 完成
- **M3: アプリ統合完成** (2026-05-22): 言語切替UI・旧システム削除完了
- **M4: リリース準備完了** (2026-05-25): 全テスト通過・E2E確認完了

---

## Phase 1: ビルドパイプライン

**期間**: 2026-05-14 〜 2026-05-16
**目標**: build.rs による Markdown→HTML 変換パイプライン構築と LaTeX 記法移行
**成果物**: `OUT_DIR/help/{en,ja}/` に HTML ファイル生成

### タスク一覧

- [ ] [TASK-2248: pulldown-cmark 依存追加と build.rs HTML 変換パイプライン](TASK-2248.md) - 8h (DIRECT) 🔵
- [ ] [TASK-2249: theory/en/ 数式 LaTeX 記法移行](TASK-2249.md) - 4h (DIRECT) 🔵
- [ ] [TASK-2250: theory/ja/ 数式 LaTeX 記法移行と widgets/ 新規作成](TASK-2250.md) - 6h (DIRECT) 🔵

### 依存関係

```
TASK-2248 ──┐
             ├──→ TASK-2251（Phase 2）
TASK-2249 ──┤
TASK-2250 ──┘→ TASK-2257（Phase 4）

TASK-2249: TASK-2248 と並行可能（検証は TASK-2248 完了後）
TASK-2250: TASK-2249 完了後
```

---

## Phase 2: Rustランタイム実装

**期間**: 2026-05-19 〜 2026-05-20
**目標**: help_types.rs 再設計・help_content.rs HTML参照化・help_launcher.rs 新規実装
**成果物**: `help_launcher::open_help()` が動作する状態

### タスク一覧

- [ ] [TASK-2251: help_types.rs 再設計（HelpLanguage / HelpContent）](TASK-2251.md) - 4h (TDD) 🔵
- [ ] [TASK-2252: help_content.rs HTML参照ベース再実装](TASK-2252.md) - 4h (TDD) 🔵
- [ ] [TASK-2253: help_launcher.rs 新規実装（一時ファイル書き出し＋ブラウザ起動）](TASK-2253.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-2248 → TASK-2251 → TASK-2252 → TASK-2253
                   └──→ TASK-2254（Phase 3 と並行可能）
```

---

## Phase 3: アプリ統合

**期間**: 2026-05-21 〜 2026-05-22
**目標**: AppState 統合・言語切替UI・旧ヘルプシステム削除
**成果物**: `cargo build` が通り、ヘルプボタンがブラウザを開く

### タスク一覧

- [ ] [TASK-2254: AppState.help_language フィールド追加 + WidgetStates 変更](TASK-2254.md) - 3h (TDD) 🔵
- [ ] [TASK-2255: layout.rs 言語切替メニュー追加](TASK-2255.md) - 4h (TDD) 🟡
- [ ] [TASK-2256: grid_canvas.rs 変更 + 旧ヘルプファイル削除](TASK-2256.md) - 5h (DIRECT) 🔵

### 依存関係

```
TASK-2251 → TASK-2254 → TASK-2255 ──┐
TASK-2253 ──────────────────────────┼──→ TASK-2256
TASK-2254 ──────────────────────────┘
```

---

## Phase 4: 統合確認

**期間**: 2026-05-25
**目標**: ビルド通過・全テスト通過・E2E動作確認
**成果物**: リリース可能な状態

### タスク一覧

- [ ] [TASK-2257: 統合確認・ビルド通過・E2E動作確認](TASK-2257.md) - 4h (DIRECT) 🔵

### 依存関係

```
TASK-2256 ──┐
TASK-2249 ──┼──→ TASK-2257
TASK-2250 ──┘
```

---

## 信頼性レベルサマリー

### 全タスク統計

- **総項目数**: 73項目
- 🔵 **青信号**: 66項目 (90%)
- 🟡 **黄信号**: 7項目 (10%)
- 🔴 **赤信号**: 0項目 (0%)

### フェーズ別信頼性

| フェーズ | 🔵 青 | 🟡 黄 | 🔴 赤 | 合計 |
|---------|-------|-------|-------|------|
| Phase 1 | 30 | 0 | 0 | 30 |
| Phase 2 | 17 | 3 | 0 | 20 |
| Phase 3 | 14 | 3 | 0 | 17 |
| Phase 4 | 5 | 1 | 0 | 6 |

**品質評価**: ✅ 高品質 — 🔴赤信号ゼロ。🟡黄信号は主に layout.rs の配置確認（TASK-2255）と egui UI テストの限界による不確実性

## クリティカルパス

```
TASK-2248 → TASK-2251 → TASK-2252 → TASK-2253 → TASK-2256 → TASK-2257
    8h    →    4h     →    4h     →    4h     →    5h     →    4h
                                                          = クリティカル 29h
```

**クリティカルパス工数**: 29時間
**並行作業可能工数**: 17時間（TASK-2249, 2250, 2254, 2255）

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-2248`
