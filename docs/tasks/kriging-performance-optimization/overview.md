# kriging-performance-optimization タスク概要

**作成日**: 2026-04-06
**プロジェクト期間**: 2026-04-07 - 2026-04-20（14日）
**推定工数**: 52時間
**総タスク数**: 14件

## 関連文書

- **要件定義書**: [📋 requirements.md](../spec/kriging-performance-optimization/requirements.md)
- **設計文書**: [📐 architecture.md](../design/kriging-performance-optimization/architecture.md)
- **インターフェース定義**: [📝 interfaces.ts](../design/kriging-performance-optimization/interfaces.ts)
- **データフロー図**: [🔄 dataflow.md](../design/kriging-performance-optimization/dataflow.md)
- **コンテキストノート**: [📝 note.md](../spec/kriging-performance-optimization/note.md)

## フェーズ構成

| フェーズ | 期間 | 成果物 | タスク数 | 工数 | ファイル |
|---------|------|--------|----------|------|----------|
| Phase 1 | 2026-04-07〜04-08 | Rustアルゴリズム最適化 | 3件 | 10h | [TASK-1642〜1644](#phase-1-rustアルゴリズム最適化) |
| Phase 2 | 2026-04-09〜04-14 | Web Workerオフロード | 5件 | 20h | [TASK-1645〜1649](#phase-2-web-workerオフロード) |
| Phase 3 | 2026-04-15〜04-20 | Sparse Kriging追加 | 6件 | 22h | [TASK-1650〜1655](#phase-3-sparse-kriging追加) |

## タスク番号管理

**使用済みタスク番号**: TASK-1642 ~ TASK-1655
**次回開始番号**: TASK-1656

## 全体進捗

- [x] Phase 1: Rustアルゴリズム最適化
- [x] Phase 2: Web Workerオフロード
- [x] Phase 3: Sparse Kriging追加

## マイルストーン

- **M1: Phase 1 完了** (2026-04-08): Kriging N=1000 が < 10s を達成
- **M2: Phase 2 完了** (2026-04-14): UI フリーズ解消、Worker 経由で計算
- **M3: Phase 3 完了** (2026-04-20): Sparse Kriging モデル追加、NFR-001/002 達成

---

## Phase 1: Rustアルゴリズム最適化

**期間**: 2026-04-07〜04-08（2日間）
**目標**: Kriging 計算の高速化（N=1000 で < 10,000ms）
**成果物**: LML統合計算・L-BFGS削減・サブサンプル変更

### タスク一覧

- [x] [TASK-1642: log_ml_with_gradient() 統合計算実装](TASK-1642.md) - 4h (TDD) 🔵
- [x] [TASK-1643: L-BFGS max_iter=50 + 早期停止実装](TASK-1643.md) - 4h (TDD) 🔵
- [x] [TASK-1644: subsample_n 1000→500 変更](TASK-1644.md) - 2h (TDD) 🔵

### 依存関係

```
TASK-1642 → TASK-1643 → TASK-1644
```

---

## Phase 2: Web Workerオフロード

**期間**: 2026-04-09〜04-14（4日間）
**目標**: UI フリーズ解消、Kriging 計算を Web Worker でオフロード
**成果物**: `wasm_compute_kriging_raw`、KrigingWorker、analysisStore 更新

### タスク一覧

- [x] [TASK-1645: wasm_compute_kriging_raw WASM関数追加](TASK-1645.md) - 4h (TDD) 🔵
- [x] [TASK-1646: KrigingWorker Blob URL生成 + スクリプト文字列](TASK-1646.md) - 4h (TDD) 🟡
- [x] [TASK-1647: KrigingWorker WASM初期化 + メッセージハンドリング](TASK-1647.md) - 4h (TDD) 🟡
- [x] [TASK-1648: analysisStore Worker統合 + extractData実装](TASK-1648.md) - 4h (TDD) 🔵
- [x] [TASK-1649: WASM rebuild + フロントエンド統合確認](TASK-1649.md) - 4h (DIRECT) 🔵

### 依存関係

```
TASK-1644 → TASK-1645 → TASK-1646 → TASK-1647 → TASK-1648 → TASK-1649
```

---

## Phase 3: Sparse Kriging追加

**期間**: 2026-04-15〜04-20（4日間）
**目標**: Sparse Kriging（FITC 近似）モデルをドロップダウンに追加
**成果物**: K-means誘導点選択、FITC実装、UI更新、パフォーマンス達成

### タスク一覧

- [x] [TASK-1650: K-means誘導点選択実装](TASK-1650.md) - 4h (TDD) 🟡
- [x] [TASK-1651: FITC K_ZZ/K_XZ行列構築](TASK-1651.md) - 4h (TDD) 🟡
- [x] [TASK-1652: FITC LML最適化 + グリッド予測](TASK-1652.md) - 4h (TDD) 🟡
- [x] [TASK-1653: compute_pdp_2d_sparse_kriging + lib.rs dispatch](TASK-1653.md) - 4h (TDD) 🔵
- [x] [TASK-1654: TypeScript型拡張 + SurfacePlot3D UI更新](TASK-1654.md) - 4h (TDD) 🔵
- [x] [TASK-1655: 統合テスト + パフォーマンス計測](TASK-1655.md) - 4h (DIRECT) 🔵

### 依存関係

```
TASK-1649 → TASK-1650 → TASK-1651 → TASK-1652 → TASK-1653 → TASK-1654 → TASK-1655
```

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 14件
- 🔵 **青信号**: 8件 (57%)
- 🟡 **黄信号**: 6件 (43%)
- 🔴 **赤信号**: 0件 (0%)

### フェーズ別信頼性

| フェーズ | 🔵 青 | 🟡 黄 | 🔴 赤 | 合計 |
|---------|-------|-------|-------|------|
| Phase 1 | 3 | 0 | 0 | 3 |
| Phase 2 | 3 | 2 | 0 | 5 |
| Phase 3 | 3 | 3 | 0 | 6 |
| **合計** | **8** | **6** | **0** | **14** |

**品質評価**: 高品質（🔴 0件）

### Phase 2 の黄信号について

TASK-1646〜1647 が 🟡 の理由: viteSingleFile + Web Worker + WASM の組み合わせは前例が少なく、具体的な実装詳細（特に Worker 内での wasm-bindgen 初期化方法）は実装時に確認が必要。

---

## クリティカルパス

```
TASK-1642 → TASK-1643 → TASK-1644
                              ↓
TASK-1645 → TASK-1646 → TASK-1647 → TASK-1648 → TASK-1649
                                                       ↓
TASK-1650 → TASK-1651 → TASK-1652 → TASK-1653 → TASK-1654 → TASK-1655
```

**クリティカルパス工数**: 52時間（全タスク直列）
**並行作業可能工数**: 0時間（全タスクが直列依存）

---

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement kriging-performance-optimization`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-1642`
