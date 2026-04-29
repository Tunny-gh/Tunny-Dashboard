# PROMETHEE Ranking タスク概要

**作成日**: 2026-04-29
**プロジェクト期間**: Phase 1〜5（推定 10日）
**推定工数**: 31時間
**総タスク数**: 7件

## 関連文書

- **要件定義書**: [📋 requirements.md](../spec/promethee-ranking/requirements.md)
- **設計文書**: [📐 architecture.md](../design/promethee-ranking/architecture.md)
- **データフロー図**: [🔄 dataflow.md](../design/promethee-ranking/dataflow.md)
- **型定義**: [📝 interfaces.rs](../design/promethee-ranking/interfaces.rs)
- **実装ガイド**: [📝 implementation-guide.md](../design/promethee-ranking/implementation-guide.md)
- **受け入れ基準**: [✅ acceptance-criteria.md](../spec/promethee-ranking/acceptance-criteria.md)
- **コンテキストノート**: [📝 note.md](../spec/promethee-ranking/note.md)

## フェーズ構成

| フェーズ | レイヤー | 成果物 | タスク数 | 工数 | ファイル |
|---------|---------|--------|----------|------|----------|
| Phase 1 | アルゴリズム層 (rust_core) | promethee.rs + モジュール登録 | 2 | 9h | TASK-2137〜2138 |
| Phase 2 | 型・状態管理層 (egui-app state) | results.rs + message_handler.rs 拡張 | 2 | 6h | TASK-2139〜2140 |
| Phase 3 | タスク起動層 (chart_registry) | Promethee spawn_task 分岐 | 1 | 4h | TASK-2141 |
| Phase 4 | UI描画層 (mcdm_chart) | PROMETHEE I/II バー描画 | 1 | 8h | TASK-2142 |
| Phase 5 | 統合検証 | 受け入れ基準全件確認 | 1 | 4h | TASK-2143 |

## タスク番号管理

**使用済みタスク番号**: TASK-2137 ~ TASK-2143
**次回開始番号**: TASK-2144

## 全体進捗

- [ ] Phase 1: アルゴリズム層 (rust_core)
- [ ] Phase 2: 型・状態管理層 (egui-app state)
- [ ] Phase 3: タスク起動層 (chart_registry)
- [ ] Phase 4: UI描画層 (mcdm_chart)
- [ ] Phase 5: 統合検証

## マイルストーン

- **M1: アルゴリズム完成**: promethee.rs の compute_promethee + テスト通過
- **M2: 状態管理完成**: results.rs / message_handler.rs の型拡張・分岐追加完了
- **M3: 起動層完成**: chart_registry.rs spawn_task 分岐追加・egui-app ビルド通過
- **M4: UI完成**: PROMETHEE I/II バーチャート表示・キャッシュ切替動作確認
- **M5: 受け入れ完了**: 受け入れ基準 29件全通過・Release ビルドパフォーマンス確認

---

## Phase 1: アルゴリズム層 (rust_core)

**目標**: PROMETHEE I / II 計算ロジックを rust_core に実装し、egui-app から呼び出せる状態にする
**成果物**: `rust_core/src/mcdm/promethee.rs`、mod.rs / lib.rs 更新

### タスク一覧

- [ ] [TASK-2137: promethee.rs アルゴリズム実装](TASK-2137.md) — 8h (TDD) 🔵
- [ ] [TASK-2138: rust_core mod.rs / lib.rs モジュール登録](TASK-2138.md) — 1h (DIRECT) 🔵

### 依存関係

```
(なし) → TASK-2137 → TASK-2138
```

---

## Phase 2: 型・状態管理層 (egui-app state)

**目標**: egui-app 側の型定義・enum を拡張し、Promethee 計算結果を状態として保持できるようにする
**成果物**: `results.rs` 拡張、`message_handler.rs` Promethee 分岐

### タスク一覧

- [ ] [TASK-2139: results.rs PrometheeResult・McdmMethod・McdmResult 型拡張](TASK-2139.md) — 4h (TDD) 🔵
- [ ] [TASK-2140: message_handler.rs Promethee 分岐追加](TASK-2140.md) — 2h (TDD) 🔵

### 依存関係

```
TASK-2138 → TASK-2139 → TASK-2140
```

---

## Phase 3: タスク起動層 (chart_registry)

**目標**: Run ボタン押下から非同期計算までのフローを完成させる
**成果物**: `chart_registry.rs` Promethee spawn_task 分岐

### タスク一覧

- [ ] [TASK-2141: chart_registry.rs Promethee spawn_task 分岐追加](TASK-2141.md) — 4h (TDD) 🔵

### 依存関係

```
TASK-2139, TASK-2140 → TASK-2141
```

---

## Phase 4: UI描画層 (mcdm_chart)

**目標**: PROMETHEE I（Φ+ 青バー + Φ- 赤バー）と PROMETHEE II（Φnet バー、負値オレンジ）を描画する
**成果物**: `mcdm_chart.rs` cached_promethee・2本バー・Φnet バー実装

### タスク一覧

- [ ] [TASK-2142: mcdm_chart.rs PROMETHEE I / II UI 描画実装](TASK-2142.md) — 8h (TDD) 🔵

### 依存関係

```
TASK-2139, TASK-2141 → TASK-2142
```

---

## Phase 5: 統合検証

**目標**: 受け入れ基準 29件全件の通過確認とパフォーマンス測定
**成果物**: 全テスト通過・Release ビルド確認

### タスク一覧

- [ ] [TASK-2143: 統合テスト・最終検証](TASK-2143.md) — 4h (TDD) 🔵

### 依存関係

```
TASK-2137〜2142 → TASK-2143
```

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 7件
- 🔵 **青信号**: 7件 (100%)
- 🟡 **黄信号**: 0件 (0%)
- 🔴 **赤信号**: 0件 (0%)

### フェーズ別信頼性

| フェーズ | 🔵 青 | 🟡 黄 | 🔴 赤 | 合計 |
|---------|-------|-------|-------|------|
| Phase 1 | 2 | 0 | 0 | 2 |
| Phase 2 | 2 | 0 | 0 | 2 |
| Phase 3 | 1 | 0 | 0 | 1 |
| Phase 4 | 1 | 0 | 0 | 1 |
| Phase 5 | 1 | 0 | 0 | 1 |

**品質評価**: ✅ 高品質

## クリティカルパス

```
TASK-2137 → TASK-2138 → TASK-2139 → TASK-2140 → TASK-2141 → TASK-2142 → TASK-2143
```

**クリティカルパス工数**: 31時間（全タスクが直列）
**並行実行可能**: TASK-2139 と TASK-2140 は TASK-2138 完了後に並行開始可能（2h の短縮）

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-2137`
