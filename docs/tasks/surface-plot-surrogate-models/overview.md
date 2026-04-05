# surface-plot-surrogate-models タスク概要

**作成日**: 2026-04-05
**プロジェクト期間**: 2026-04-06 - 2026-04-11（6日）
**推定工数**: 48時間
**総タスク数**: 12件

## 関連文書

- **設計文書**: [📐 architecture.md](../../design/surface-plot-surrogate-models/architecture.md)
- **データフロー図**: [🔄 dataflow.md](../../design/surface-plot-surrogate-models/dataflow.md)
- **インターフェース定義**: [📝 interfaces.ts](../../design/surface-plot-surrogate-models/interfaces.ts)
- **ヒアリング記録**: [📋 design-interview.md](../../design/surface-plot-surrogate-models/design-interview.md)

## フェーズ構成

| フェーズ | 期間 | 成果物 | タスク数 | 工数 | ファイル |
|---------|------|--------|----------|------|----------|
| Phase 1 | Day 1-2 | Random Forest Rust 実装 | 3 | 16h | [TASK-1630~1632](#phase-1-random-forest-rust-実装) |
| Phase 2 | Day 3-5 | Kriging Rust 実装 | 4 | 20h | [TASK-1633~1636](#phase-2-kriging-rust-実装) |
| Phase 3 | Day 6 | WASM連携 + TypeScript 統合 | 5 | 12h | [TASK-1637~1641](#phase-3-wasm連携--typescript-統合) |

## タスク番号管理

**使用済みタスク番号**: TASK-1630 ~ TASK-1641
**次回開始番号**: TASK-1642

## 全体進捗

- [ ] Phase 1: Random Forest Rust 実装
- [ ] Phase 2: Kriging Rust 実装
- [ ] Phase 3: WASM連携 + TypeScript 統合

## マイルストーン

- **M1: RF 完成** (Day 2): `rust_core/src/rf.rs` CART+Bagging 実装、`compute_pdp_2d_rf()` 完了
- **M2: Kriging 完成** (Day 5): `rust_core/src/kriging.rs` ARD Matérn 5/2 + L-BFGS 実装、`compute_pdp_2d_kriging()` 完了
- **M3: 統合完成** (Day 6): WASM リビルド、TypeScript 修正、SurfacePlot3D 有効化、E2E 確認完了

---

## Phase 1: Random Forest Rust 実装

**期間**: Day 1-2（16時間）
**目標**: `rust_core/src/rf.rs` に CART 決定木 + Bagging を純 Rust 実装
**成果物**: `rf.rs` 新規作成、`compute_pdp_2d_rf()` 関数

### タスク一覧

- [ ] [TASK-1630: rf.rs CART 決定木実装](TASK-1630.md) - 6h (TDD) 🔵
- [ ] [TASK-1631: rf.rs Random Forest Bootstrap + アンサンブル実装](TASK-1631.md) - 6h (TDD) 🔵
- [ ] [TASK-1632: pdp.rs compute_pdp_2d_rf() 実装](TASK-1632.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-1630 → TASK-1631 → TASK-1632
```

---

## Phase 2: Kriging Rust 実装

**期間**: Day 3-5（20時間）
**目標**: `rust_core/src/kriging.rs` に ARD Matérn 5/2 + Cholesky + L-BFGS を実装
**成果物**: `kriging.rs` 新規作成、`compute_pdp_2d_kriging()` 関数

### タスク一覧

- [ ] [TASK-1633: kriging.rs ARD Matérn 5/2 カーネル + Cholesky 分解 + GP 基本構造実装](TASK-1633.md) - 6h (TDD) 🔵
- [ ] [TASK-1634: kriging.rs 対数周辺尤度計算 + 解析的勾配実装](TASK-1634.md) - 4h (TDD) 🔵
- [ ] [TASK-1635: kriging.rs L-BFGS ハイパーパラメータ最適化 + GP 学習統合](TASK-1635.md) - 6h (TDD) 🔵
- [ ] [TASK-1636: pdp.rs compute_pdp_2d_kriging() 実装](TASK-1636.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-1633 → TASK-1634 → TASK-1635 → TASK-1636
TASK-1632 --------→ TASK-1636（pdp.rs への組み込みパターン参照）
```

---

## Phase 3: WASM連携 + TypeScript 統合

**期間**: Day 6（12時間）
**目標**: WASM API に `model_type` 引数を追加し、フロントエンドで RF/Kriging を有効化
**成果物**: WASM リビルド済みパッケージ、wasmLoader.ts 更新、analysisStore.ts バグ修正、SurfacePlot3D 有効化

### タスク一覧

- [ ] [TASK-1637: lib.rs / pdp.rs model_type ディスパッチ追加 + WASM リビルド](TASK-1637.md) - 4h (DIRECT) 🔵
- [ ] [TASK-1638: wasmLoader.ts computePdp2d に modelType 引数追加](TASK-1638.md) - 2h (TDD) 🔵
- [ ] [TASK-1639: analysisStore.ts surrogateModelType を WASM に渡すバグ修正](TASK-1639.md) - 2h (TDD) 🔵
- [ ] [TASK-1640: SurfacePlot3D.tsx RF / Kriging の disabled 解除](TASK-1640.md) - 2h (TDD) 🔵
- [ ] [TASK-1641: E2E 動作確認](TASK-1641.md) - 2h (DIRECT) 🔵

### 依存関係

```
TASK-1636 → TASK-1637 → TASK-1638 → TASK-1639 → TASK-1640 → TASK-1641
```

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 12件
- 🔵 **青信号**: 12件 (100%)
- 🟡 **黄信号**: 0件 (0%)
- 🔴 **赤信号**: 0件 (0%)

### フェーズ別信頼性

| フェーズ | 🔵 青 | 🟡 黄 | 🔴 赤 | 合計 |
|---------|-------|-------|-------|------|
| Phase 1 | 3 | 0 | 0 | 3 |
| Phase 2 | 4 | 0 | 0 | 4 |
| Phase 3 | 5 | 0 | 0 | 5 |

**品質評価**: ✅ 高品質

## クリティカルパス

```
TASK-1630 → TASK-1631 → TASK-1632
                              ↓
                         TASK-1633 → TASK-1634 → TASK-1635 → TASK-1636
                                                                    ↓
                                                               TASK-1637 → TASK-1638 → TASK-1639 → TASK-1640 → TASK-1641
```

**クリティカルパス工数**: 48時間（全タスクがシリアル依存）
**並行作業可能工数**: 0時間（モデル種別間は独立だが実装順序依存）

## 実装上の注意事項

### Random Forest

- LCG 疑似乱数（線形合同法）を使用（外部クレート不要）
- 2D 部分空間射影: `(param1_idx, param2_idx)` 列のみ抽出してモデル学習
- ハイパーパラメータ: `n_trees=100, max_depth=10, min_samples_leaf=2`
- OOB R² が計算できない（N_oob < 10 等）場合は訓練 R² にフォールバック

### Kriging

- ARD **Matérn 5/2** カーネル（RBF ではない）: `k(x1,x2) = σ_f²·(1+√5·r+5r²/3)·exp(−√5·r)`
- n > 1000 の場合はランダムサブサンプリングして 1000 点に削減
- L-BFGS: m=5 履歴ベクトル、Armijo バックトラッキング線探索（c₁=1e-4）、最大 100 イテレーション
- Cholesky ジッター: `1e-6`（数値安定性のため）
- 計算量警告: N=1000 で L-BFGS 100 ステップは最大 10¹¹ ops — ローディングスピナー表示を忘れずに

### WASM 連携

- 既存バグ: `analysisStore.ts` の `computeSurface3d` が `surrogateModelType` をキャッシュキーには使うが WASM に渡していない（TASK-1639 で修正）
- WASM リビルドコマンド: `wasm-pack build --target web --out-dir ../frontend/src/wasm/pkg`（`rust_core/` 内で実行）

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-1630`
