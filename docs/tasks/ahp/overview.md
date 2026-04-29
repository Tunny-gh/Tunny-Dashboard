# AHP タスク概要

**作成日**: 2026-04-30
**プロジェクト期間**: Phase 1（推定 4〜5 日）
**推定工数**: 25時間
**総タスク数**: 5件

## 関連文書

- **要件定義書**: [📋 requirements.md](../spec/ahp/requirements.md)
- **設計文書**: [📐 architecture.md](../design/ahp/architecture.md)
- **データフロー**: [🔄 dataflow.md](../design/ahp/dataflow.md)
- **インターフェース定義**: [📝 interfaces.rs](../design/ahp/interfaces.rs)
- **実装ガイド**: [🔧 implementation-guide.md](../design/ahp/implementation-guide.md)
- **コンテキストノート**: [📝 note.md](../spec/ahp/note.md)

## フェーズ構成

| フェーズ | 期間 | 成果物 | タスク数 | 工数 | ファイル |
|---------|------|--------|----------|------|----------|
| Phase 1 | 4〜5 日 | AHP 完全実装 | 5 | 25h | [TASK-2144〜2148](#phase-1-ahp-実装) |

## タスク番号管理

**使用済みタスク番号**: TASK-2144 〜 TASK-2148
**次回開始番号**: TASK-2149

## 全体進捗

- [ ] Phase 1: AHP 実装

## マイルストーン

- **M1: アルゴリズム完成** (TASK-2144 完了): `compute_ahp` + 8 テスト通過
- **M2: 状態管理完成** (TASK-2145 完了): コンパイル通過・型安全
- **M3: UI 完成** (TASK-2146 完了): ウィジェット実装・グリッド表示
- **M4: 統合完成** (TASK-2147 完了): Run ボタンでエンドツーエンド動作確認
- **M5: ドキュメント完成** (TASK-2148 完了): 理論ドキュメント作成

---

## Phase 1: AHP 実装

**期間**: 推定 4〜5 日
**目標**: AHP（Analytic Hierarchy Process）の完全実装
**成果物**:
- `rust_core/src/mcdm/ahp.rs` — アルゴリズム実装 + 8 テスト
- `egui-app/src/state/` — 型・メッセージ・ハンドラ・ChartId 追加
- `egui-app/src/ui/widgets/ahp_chart.rs` — 新規ウィジェット
- `egui-app/src/ui/chart_registry.rs` — AHP 分岐追加
- `theory/mcdm/ahp.md` — 理論ドキュメント

### タスク一覧

- [ ] [TASK-2144: AHP アルゴリズム実装（rust_core）](TASK-2144.md) - 8h (TDD) 🔵
- [ ] [TASK-2145: 状態管理層追加](TASK-2145.md) - 3h (DIRECT) 🔵
- [ ] [TASK-2146: AhpChart ウィジェット実装](TASK-2146.md) - 8h (TDD) 🔵
- [ ] [TASK-2147: chart_registry AHP 分岐追加](TASK-2147.md) - 4h (TDD) 🔵
- [ ] [TASK-2148: 理論ドキュメント作成](TASK-2148.md) - 2h (DIRECT) 🟡

### 依存関係

```
TASK-2144 (アルゴリズム)
  └→ TASK-2145 (状態管理層)
       └→ TASK-2146 (ウィジェット)
            └→ TASK-2147 (chart_registry)

TASK-2148 (理論Doc, 独立)
```

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 5 件

| タスク | 全体信頼性 |
|--------|-----------|
| TASK-2144 | 🔵 |
| TASK-2145 | 🔵 |
| TASK-2146 | 🔵 |
| TASK-2147 | 🔵 |
| TASK-2148 | 🟡 |

### フェーズ別信頼性

| フェーズ | 🔵 青 | 🟡 黄 | 🔴 赤 | 合計 |
|---------|-------|-------|-------|------|
| Phase 1 | 4 | 1 | 0 | 5 |

**品質評価**: ✅ 高品質（TASK-2148 のみ既存フォーマット確認後に実装）

## クリティカルパス

```
TASK-2144 → TASK-2145 → TASK-2146 → TASK-2147
```

**クリティカルパス工数**: 23時間
**並行作業可能工数**: 2時間（TASK-2148 は独立）

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-2144`
