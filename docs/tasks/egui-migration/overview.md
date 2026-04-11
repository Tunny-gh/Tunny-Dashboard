# egui-migration タスク概要

**作成日**: 2026-04-11
**プロジェクト期間**: 2026-04-11 〜 （推定 16日間）
**推定工数**: 128時間（32タスク × 4h）
**総タスク数**: 32件

## 関連文書

- **設計文書 (アーキテクチャ)**: [📐 architecture.md](../../design/egui-migration/architecture.md)
- **設計文書 (データフロー)**: [🔄 dataflow.md](../../design/egui-migration/dataflow.md)
- **設計文書 (型定義)**: [📝 interfaces.rs](../../design/egui-migration/interfaces.rs)
- **ヒアリング記録**: [📝 design-interview.md](../../design/egui-migration/design-interview.md)
- **要件定義書**: [📋 tunny-dashboard-requirements.md](../../spec/tunny-dashboard-requirements.md)

## 移行方針

TypeScript/React UIを**Rust eguiデスクトップアプリ**に完全移行する。

| 旧（削除） | 新（作成） |
|---|---|
| `frontend/` ディレクトリ全体 | `egui-app/` クレート |
| Zustand stores | Rust `AppState` struct |
| WASM JS Bridge | Rust 直接関数呼び出し |
| deck.gl (WebGL) | wgpu 直接 (Phase 10) |
| ECharts | egui_plot |
| OffscreenCanvas | egui::Painter API |

## フェーズ構成

| フェーズ | 内容 | タスク数 | 工数 | タスクID |
|---------|------|----------|------|----------|
| Phase 1 | 基盤構築 (Workspace + Crate) | 2 | 8h | TASK-2001〜2002 |
| Phase 2 | 状態管理・型定義 | 3 | 12h | TASK-2003〜2005 |
| Phase 3 | Journal I/O 連携 | 2 | 8h | TASK-2006〜2007 |
| Phase 4 | 基本UIレイアウト | 4 | 16h | TASK-2008〜2011 |
| Phase 5 | Brushing & Linking 基盤 | 2 | 8h | TASK-2012〜2013 |
| Phase 6 | egui_plot チャート | 4 | 16h | TASK-2014〜2017 |
| Phase 7 | カスタムウィジェット | 5 | 20h | TASK-2018〜2022 |
| Phase 8 | 分析機能 async 連携 | 4 | 16h | TASK-2023〜2026 |
| Phase 9 | 追加機能 | 2 | 8h | TASK-2027〜2028 |
| Phase 10 | wgpu 3D描画 | 2 | 8h | TASK-2029〜2030 |
| Phase 11 | クリーンアップ | 2 | 8h | TASK-2031〜2032 |

## タスク番号管理

**使用済みタスク番号**: TASK-2001 〜 TASK-2032
**次回開始番号**: TASK-2033

## 全体進捗

- [ ] Phase 1: 基盤構築
- [ ] Phase 2: 状態管理・型定義
- [ ] Phase 3: Journal I/O 連携
- [ ] Phase 4: 基本UIレイアウト
- [ ] Phase 5: Brushing & Linking 基盤
- [ ] Phase 6: egui_plot チャート
- [ ] Phase 7: カスタムウィジェット
- [ ] Phase 8: 分析機能 async 連携
- [ ] Phase 9: 追加機能
- [ ] Phase 10: wgpu 3D描画
- [ ] Phase 11: クリーンアップ

## マイルストーン

- **M1: 動作する基盤** (Phase 1-3完了): Journalファイルを読み込んでStudy選択ができる
- **M2: 基本UI完成** (Phase 4-5完了): 4エリアレイアウト + フィルタースライダー動作
- **M3: チャート表示** (Phase 6-7完了): 主要チャートが全て表示される
- **M4: フル機能** (Phase 8-9完了): 感度分析・クラスタリング・ライブ更新・エクスポート
- **M5: リリース準備** (Phase 10-11完了): wgpu 3D + frontend 削除完了

---

## Phase 1: 基盤構築

**目標**: Cargo ワークスペース化 + egui-app クレートの雛形作成
**成果物**: `Cargo.toml` (workspace) + `egui-app/src/main.rs` (ウィンドウ表示)

### タスク一覧

- [x] [TASK-2001: Cargo ワークスペース設定 + rust_core WASM廃止](TASK-2001.md) - 4h (DIRECT) 🔵
- [x] [TASK-2002: egui-app クレート基本構造作成](TASK-2002.md) - 4h (DIRECT) 🔵

### 依存関係

```
TASK-2001 → TASK-2002
```

---

## Phase 2: 状態管理・型定義

**目標**: `AppState`・`LayoutState`・`AppMessage` 型定義と非同期基盤
**成果物**: `egui-app/src/state/` モジュール完成

### タスク一覧

- [x] [TASK-2003: AppState + 型定義実装](TASK-2003.md) - 4h (TDD) 🔵
- [x] [TASK-2004: LayoutState + AppMessage 型定義実装](TASK-2004.md) - 4h (TDD) 🔵
- [x] [TASK-2005: 非同期タスク基盤 (std::thread + mpsc channel)](TASK-2005.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-2002 → TASK-2003 → TASK-2004 → TASK-2005
```

---

## Phase 3: Journal I/O 連携

**目標**: `.log` ファイル読み込み → Study選択 → TrialData展開
**成果物**: `egui-app/src/io/` + parse_journal・select_study 連携

### タスク一覧

- [x] [TASK-2006: ファイル選択・ドラッグ&ドロップ + parse_journal連携](TASK-2006.md) - 4h (TDD) 🔵
- [x] [TASK-2007: Study選択 + select_study連携 + GpuBufferData生成](TASK-2007.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-2005 → TASK-2006 → TASK-2007
```

---

## Phase 4: 基本UIレイアウト

**目標**: 4エリアレイアウト + ToolBar + LeftPanel + BottomPanel
**成果物**: `egui-app/src/ui/` モジュール完成

### タスク一覧

- [ ] [TASK-2008: メインレイアウト (4エリア + リサイズハンドル)](TASK-2008.md) - 4h (TDD) 🔵
- [ ] [TASK-2009: ToolBar実装 (ファイル操作・モード切り替え)](TASK-2009.md) - 4h (TDD) 🔵
- [ ] [TASK-2010: LeftPanel - Study情報・フィルタースライダー](TASK-2010.md) - 4h (TDD) 🔵
- [ ] [TASK-2011: BottomPanel - Trialテーブル](TASK-2011.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-2007 → TASK-2008 → TASK-2009 → TASK-2010 → TASK-2011
```

---

## Phase 5: Brushing & Linking 基盤

**目標**: フィルター操作 → selected_indices 更新 → 全チャート連動
**成果物**: filter_by_ranges 統合 + 暫定 2D 散布図

### タスク一覧

- [ ] [TASK-2012: filter_by_ranges 統合 + selected_indices管理](TASK-2012.md) - 4h (TDD) 🔵
- [ ] [TASK-2013: Simple 2D Scatter (egui_plot) + カラー更新連携](TASK-2013.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-2011 → TASK-2012 → TASK-2013
```

---

## Phase 6: egui_plot チャート

**目標**: 最適化履歴・HV・感度分析・PDPチャートの実装
**成果物**: `egui-app/src/widgets/` の4チャートファイル

### タスク一覧

- [ ] [TASK-2014: 最適化履歴 + Hypervolume推移チャート](TASK-2014.md) - 4h (TDD) 🔵
- [ ] [TASK-2015: 感度分析バーチャート (ImportanceChart)](TASK-2015.md) - 4h (TDD) 🔵
- [ ] [TASK-2016: PDP 1D折れ線チャート](TASK-2016.md) - 4h (TDD) 🔵
- [ ] [TASK-2017: PDP 2Dヒートマップ](TASK-2017.md) - 4h (TDD) 🟡

### 依存関係

```
TASK-2013 → TASK-2014 → TASK-2015 → TASK-2016 → TASK-2017
```

---

## Phase 7: カスタムウィジェット

**目標**: Scatter Matrix + 平行座標図 + Pareto2D（ダウンサンプリング統合）
**成果物**: egui Painter ベースの複雑なカスタムウィジェット群

### タスク一覧

- [ ] [TASK-2018: Scatter Matrix セル描画 (egui Painter)](TASK-2018.md) - 4h (TDD) 🔵
- [ ] [TASK-2019: Scatter Matrix グリッド + モード切り替え](TASK-2019.md) - 4h (TDD) 🔵
- [ ] [TASK-2020: Pareto2D Scatter - ダウンサンプリング統合](TASK-2020.md) - 4h (TDD) 🔵
- [ ] [TASK-2021: 平行座標図 軸・線描画 (egui Painter)](TASK-2021.md) - 4h (TDD) 🟡
- [ ] [TASK-2022: 平行座標図 ブラッシングUI (軸フィルター)](TASK-2022.md) - 4h (TDD) 🟡

### 依存関係

```
TASK-2017 → TASK-2018 → TASK-2019 → TASK-2020 → TASK-2021 → TASK-2022
```

---

## Phase 8: 分析機能 async 連携

**目標**: 感度分析・クラスタリング・PDP・ダウンサンプリングの非同期実行
**成果物**: 全分析機能が AppMessage 経由で非同期動作

### タスク一覧

- [ ] [TASK-2023: 感度分析 async + SensitivityHeatmap表示](TASK-2023.md) - 4h (TDD) 🔵
- [ ] [TASK-2024: クラスタリング (PCA+k-means) async 連携](TASK-2024.md) - 4h (TDD) 🔵
- [ ] [TASK-2025: PDP計算 async + モデル選択UI](TASK-2025.md) - 4h (TDD) 🔵
- [ ] [TASK-2026: ダウンサンプリングキャッシュ管理](TASK-2026.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-2022 → TASK-2023 → TASK-2024 → TASK-2025 → TASK-2026
```

---

## Phase 9: 追加機能

**目標**: ライブ更新・CSV エクスポート・セッション保存
**成果物**: 全機能要件カバー

### タスク一覧

- [ ] [TASK-2027: ライブ更新 (std::thread polling + diff更新)](TASK-2027.md) - 4h (TDD) 🔵
- [ ] [TASK-2028: CSVエクスポート + セッション保存](TASK-2028.md) - 4h (TDD) 🔵

### 依存関係

```
TASK-2026 → TASK-2027 → TASK-2028
```

---

## Phase 10: wgpu 3D描画（後回し実装）

**目標**: GPU 加速による高速 3D 散布図（5万点 60fps）
**成果物**: ScatterRenderer + Pareto3D wgpu実装

### タスク一覧

- [ ] [TASK-2029: wgpu 基盤セットアップ + ScatterRenderer](TASK-2029.md) - 4h (DIRECT) 🟡
- [ ] [TASK-2030: Pareto3D wgpu Point Cloud描画](TASK-2030.md) - 4h (TDD) 🟡

### 依存関係

```
TASK-2028 → TASK-2029 → TASK-2030
```

---

## Phase 11: クリーンアップ

**目標**: frontend/ 削除・WASM クリーンアップ・最終確認
**成果物**: 純粋な Rust プロジェクト完成

### タスク一覧

- [ ] [TASK-2031: frontend/ ディレクトリ削除 + ビルド確認](TASK-2031.md) - 4h (DIRECT) 🔵
- [ ] [TASK-2032: rust_core WASM feature クリーンアップ + 最終確認](TASK-2032.md) - 4h (DIRECT) 🔵

### 依存関係

```
TASK-2030 → TASK-2031 → TASK-2032
```

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 32件
- 🔵 **青信号**: 27件 (84%)
- 🟡 **黄信号**: 5件 (16%) — wgpu/平行座標図カスタム実装部分
- 🔴 **赤信号**: 0件 (0%)

### フェーズ別信頼性

| フェーズ | 🔵 青 | 🟡 黄 | 🔴 赤 | 合計 |
|---------|-------|-------|-------|------|
| Phase 1 | 2 | 0 | 0 | 2 |
| Phase 2 | 3 | 0 | 0 | 3 |
| Phase 3 | 2 | 0 | 0 | 2 |
| Phase 4 | 4 | 0 | 0 | 4 |
| Phase 5 | 2 | 0 | 0 | 2 |
| Phase 6 | 3 | 1 | 0 | 4 |
| Phase 7 | 3 | 2 | 0 | 5 |
| Phase 8 | 4 | 0 | 0 | 4 |
| Phase 9 | 2 | 0 | 0 | 2 |
| Phase 10 | 0 | 2 | 0 | 2 |
| Phase 11 | 2 | 0 | 0 | 2 |

**品質評価**: ✅ 高品質

## クリティカルパス

```
TASK-2001 → TASK-2002 → TASK-2003 → TASK-2004 → TASK-2005
→ TASK-2006 → TASK-2007 → TASK-2008 → TASK-2009 → TASK-2010 → TASK-2011
→ TASK-2012 → TASK-2013 → TASK-2014 → TASK-2015 → TASK-2016 → TASK-2017
→ TASK-2018 → TASK-2019 → TASK-2020 → TASK-2021 → TASK-2022
→ TASK-2023 → TASK-2024 → TASK-2025 → TASK-2026
→ TASK-2027 → TASK-2028 → TASK-2029 → TASK-2030 → TASK-2031 → TASK-2032
```

**クリティカルパス工数**: 128時間（全タスクが直列）
**マイルストーン M1（基盤）まで**: 32時間（TASK-2001〜2007）

## 次のステップ

タスクを実装するには:
- 全タスク順番に実装: `/tsumiki:kairo-implement`
- 特定タスクを実装: `/tsumiki:kairo-implement TASK-2001`
- 最初のタスクから開始: `/tsumiki:kairo-implement TASK-2001`
