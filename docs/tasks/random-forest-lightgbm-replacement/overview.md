# RandomForest → LightGBM 置き換え タスク概要

**作成日**: 2026-04-28
**プロジェクト期間**: Phase 1 〜 Phase 4（8日）
**推定工数**: 64時間
**総タスク数**: 8件

## 関連文書

- **要件定義書**: [📋 requirements.md](../spec/random-forest-lightgbm-replacement/requirements.md)
- **設計文書**: [📐 architecture.md](../design/random-forest-lightgbm-replacement/architecture.md)
- **データフロー図**: [🔄 dataflow.md](../design/random-forest-lightgbm-replacement/dataflow.md)
- **型定義**: [📝 interfaces.rs](../design/random-forest-lightgbm-replacement/interfaces.rs)
- **設計ヒアリング**: [📝 design-interview.md](../design/random-forest-lightgbm-replacement/design-interview.md)
- **コンテキストノート**: [📝 note.md](../spec/random-forest-lightgbm-replacement/note.md)

## フェーズ構成

| フェーズ | 成果物 | タスク数 | 工数 | タスク |
|---------|--------|----------|------|--------|
| Phase 1 - ビルド基盤構築 | libs/ 配置・build.rs・Cargo.toml | 1 | 8h | TASK-2129 |
| Phase 2 - コアラッパー実装 | core::lgbm 完成 | 2 | 16h | TASK-2130〜2131 |
| Phase 3 - 各モジュール置き換え | PDP/SHAP/MDI/RF-ANOVA 完成 | 4 | 32h | TASK-2132〜2135 |
| Phase 4 - 統合検証 | 全テスト通過・egui-app ビルド | 1 | 8h | TASK-2136 |

## タスク番号管理

**使用済みタスク番号**: TASK-2129 〜 TASK-2136
**次回開始番号**: TASK-2137

## 全体進捗

- [ ] Phase 1: ビルド基盤構築
- [ ] Phase 2: コアラッパー実装
- [ ] Phase 3: 各モジュール置き換え
- [ ] Phase 4: 統合検証

## マイルストーン

- **M1: ビルド基盤完成** (Day 1): DLL配置・build.rs・Cargo.toml 完了、cargo check 通過
- **M2: コアラッパー完成** (Day 3): core::lgbm の全関数実装完了、random_forest 整理済み
- **M3: 全モジュール置き換え完了** (Day 7): PDP/SHAP/MDI/RF-ANOVA が LightGBM ベースに更新
- **M4: リリース準備完了** (Day 8): 全テスト通過、egui-app ビルド成功、クレートバージョン固定

---

## Phase 1: ビルド基盤構築

**期間**: Day 1（8h）
**目標**: LightGBM DLL のリンク設定を確立し、lightgbm クレートを依存に追加する
**成果物**: `libs/` ディレクトリ、`rust_core/build.rs`、更新済み `Cargo.toml`

### タスク一覧

- [ ] [TASK-2129: ビルド基盤構築（DLL配置・build.rs・Cargo.toml）](TASK-2129.md) - 8h (DIRECT) 🔵

### 依存関係

```
（依存なし）TASK-2129
```

---

## Phase 2: コアラッパー実装

**期間**: Day 2〜3（16h）
**目標**: `core::lgbm` モジュールを完成させ、全感度分析が使える共有 LightGBM ラッパーを提供する
**成果物**: `rust_core/src/core/lgbm.rs`（全関数）、整理済み `random_forest/mod.rs`

### タスク一覧

- [ ] [TASK-2130: core::lgbm 基本ラッパー実装](TASK-2130.md) - 8h (TDD) 🔵
- [ ] [TASK-2131: core::lgbm SHAP/MDI関数実装 + random_forest整理](TASK-2131.md) - 8h (TDD) 🔵

### 依存関係

```
TASK-2129 → TASK-2130 → TASK-2131
```

---

## Phase 3: 各モジュール置き換え

**期間**: Day 4〜7（32h）
**目標**: 4つの RF 使用箇所を全て LightGBM ベースに置き換える
**成果物**: 更新済み `pdp/api.rs`、`sensitivity/rf_anova.rs`、`sensitivity/mdi.rs`、`sensitivity/shap.rs`

### タスク一覧

- [ ] [TASK-2132: 2D PDP の LightGBM 置き換え](TASK-2132.md) - 8h (TDD) 🔵
- [ ] [TASK-2133: RF-ANOVA の LightGBM 置き換え](TASK-2133.md) - 8h (TDD) 🔵
- [ ] [TASK-2134: MDI の LightGBM 完全置き換え](TASK-2134.md) - 8h (TDD) 🔵
- [ ] [TASK-2135: SHAP の LightGBM 完全置き換え](TASK-2135.md) - 8h (TDD) 🔵

### 依存関係

```
TASK-2131 → TASK-2132
TASK-2131 → TASK-2133
TASK-2131 → TASK-2134
TASK-2131 → TASK-2135
```

（TASK-2132〜2135 は並行実行可能）

---

## Phase 4: 統合検証

**期間**: Day 8（8h）
**目標**: 全モジュールの統合テストを行い、egui-app ビルドと Kriging への影響なしを確認する
**成果物**: 統合テスト、lightgbm バージョン固定済み Cargo.toml

### タスク一覧

- [ ] [TASK-2136: 統合テスト・最終検証](TASK-2136.md) - 8h (TDD) 🔵

### 依存関係

```
TASK-2132 → TASK-2136
TASK-2133 → TASK-2136
TASK-2134 → TASK-2136
TASK-2135 → TASK-2136
```

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 8件
- 🔵 **青信号**: 8件 (100%)
- 🟡 **黄信号**: 0件 (0%)
- 🔴 **赤信号**: 0件 (0%)

### フェーズ別信頼性

| フェーズ | 🔵 青 | 🟡 黄 | 🔴 赤 | 合計 |
|---------|-------|-------|-------|------|
| Phase 1 | 1 | 0 | 0 | 1 |
| Phase 2 | 2 | 0 | 0 | 2 |
| Phase 3 | 4 | 0 | 0 | 4 |
| Phase 4 | 1 | 0 | 0 | 1 |

**品質評価**: ✅ 高品質

## クリティカルパス

```
TASK-2129 → TASK-2130 → TASK-2131 → TASK-2132 → TASK-2136
                                    → TASK-2133 ↗
                                    → TASK-2134 ↗
                                    → TASK-2135 ↗
```

**クリティカルパス工数**: 40時間（2129→2130→2131→任意1つ→2136）
**並行作業可能工数**: 24時間（Phase 3 の TASK-2132〜2135 は 3並行可能）

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-2129`
