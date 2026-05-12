# dashboard-missing-features コンテキストノート

**生成日**: 2026-05-12
**要件名**: ダッシュボード不足機能 (dashboard-missing-features)

---

## 技術スタック

| 層 | 技術 |
|---|---|
| Core 処理 | Rust + ndarray |
| UI フレームワーク | eframe + egui (v0.31+) |
| GPU 描画 | wgpu + egui-wgpu |
| チャート | egui_plot |
| 機械学習 | linfa |
| ビルドツール | cargo |

---

## プロジェクト構造

```
egui-app/
├── src/
│   ├── app.rs              # メインアプリループ
│   ├── io/
│   │   ├── export.rs       # CSVエクスポートロジック（ExportTarget, select_rows_for_export）
│   │   └── session.rs      # セッション保存（pinned_trialsフィールドあり）
│   ├── state/
│   │   ├── app_state.rs    # AppState（comparison_studies: Vec<StudyContext>）
│   │   ├── layout_state.rs # ChartId, PanelItem, LayoutState
│   │   ├── messages.rs     # AppMessage（PdpResult1d, PdpResult2d含む）
│   │   └── results.rs      # McdmResult, PrometheeResult, AhpResult, EntropyResult
│   └── ui/
│       ├── toolbar.rs      # ToolbarAction enum（Export未実装, AddComparisonStudy未実装）
│       ├── left_panel.rs   # フィルタースライダー、Trade-off Navigator、Convergence Card
│       ├── bottom_panel.rs # Trial Table, Best History Table
│       ├── right_panel.rs  # ウィジェット一覧（グループ別, D&D）
│       ├── comparison_panel.rs # Stats/HV History/Pareto/KDE（AddComparison UIなし）
│       └── widgets/        # 17チャート + ArtifactModal
rust_core/
└── src/
    ├── pdp/                # PDP計算ロジック
    ├── sensitivity/        # 感度分析
    └── clustering/         # k-means, PCA
```

---

## 実装済みウィジェット（ChartId 17種）

| ChartId | 表示名 | 実装ファイル |
|---|---|---|
| ParetoScatter2D | Pareto Scatter 2D | pareto_2d.rs |
| ParetoScatter3D | Pareto Scatter 3D | pareto_3d.rs |
| ParallelCoordinates | Parallel Coordinates | parallel_coords.rs |
| ScatterMatrix | Scatter Matrix | scatter_matrix.rs |
| ImportanceChart | Importance Chart | importance_chart.rs |
| PdpChart | PDP Chart | pdp_chart.rs |
| PdpChart2D | PDP Chart 2D | pdp_2d.rs |
| OptimizationHistory | Optimization History | optimization_history.rs |
| HvHistory | Hypervolume History | hv_history.rs |
| SensitivityHeatmap | Sensitivity Heatmap | sensitivity_heatmap.rs |
| ClusterScatter | Cluster Scatter | cluster_scatter.rs |
| McdmRankChart | MCDM Ranking | mcdm_chart.rs |
| McdmScatterChart | MCDM Scatter Chart | mcdm_scatter_chart.rs |
| McdmTable | MCDM Table | mcdm_chart.rs |
| AhpRankChart | AHP Ranking | ahp_chart.rs |
| AhpTable | AHP Table | ahp_chart.rs |
| SliceChart | Slice Chart | slice_chart.rs |

TrialTable は PanelItem として別管理。

---

## 実装済み主要機能

- **分析メトリクス**: Spearman, Ridge, RF-Anova, MDI, SHAP, Sobol (First/Total), Permutation
- **MCDM**: TOPSIS, VIKOR, PROMETHEE I/II, AHP（Entropy Weight込み）
- **ライブ更新**: ファイルポーリング（1〜30秒）
- **セッション**: Save/Load (JSON)
- **アーティファクト**: artifact_modal.rs（画像・CSV プレビュー）
- **HTML Report**: GenerateHtmlReport アクション
- **Trade-off Navigator**: Left Panel に組み込み（多目的Study時）
- **Convergence Card**: Left Panel に組み込み（単目的Study時）
- **比較パネル**: Stats/HV History/Pareto/KDE タブ（AddStudy UI なし）
- **Help モーダル**: help_modal.rs

---

## 確認済みの未実装機能（本要件定義の対象）

| 機能 | 根拠コード | 状態 |
|---|---|---|
| CSV Export UI | `io/export.rs` に `ExportTarget` ロジックあり、toolbar.rs に UI なし | **未実装** |
| Comparison Study 追加 UI | `comparison_panel.rs` に "Add comparison studies via toolbar" メッセージあり、ToolbarAction に該当アクションなし | **未実装** |
| ピン留め UI | `session.rs` の `SessionSnapshot.pinned_trials: Vec<u32>` あり、TrialTable 行に Pin ボタンなし | **未実装** |
| PDP Observed Data Overlay | `docs/design/pdp-observed-overlay/` 設計文書あり、pdp_chart.rs に overlay なし | **未実装** |
| Surface Plot ウィジェット | `docs/design/lightgbm-surface-plot/` 設計文書あり、ChartId に SurfacePlot なし | **未実装** |
| Brushing (Brush Selection + PCP) | ドラッグ矩形選択・PCP 軸ブラッシング コード未確認 | **未実装** |
| Comparison Diff モード | `comparison_panel.rs` に Diff タブなし | **未実装** |
| チャート PNG 保存 | 各 ChartId ウィジェットにコンテキストメニューなし | **未実装** |

---

## 関連設計文書

- `docs/design/pdp-observed-overlay/` — PDP 観測データオーバーレイ設計
- `docs/design/lightgbm-surface-plot/` — Surface Plot 設計
- `docs/design/fast-rendering-downsampling/` — レンダリング最適化
- `docs/design/free-layout-dashboard/` — フリーレイアウト設計
- `docs/spec/tunny-dashboard-requirements.md` — 元要件定義書
- `docs/spec/first_spec.md` — 初期設計仕様書（v0.8.0）

---

## 注意事項

- **`comparison_studies` の追加フロー**: `message_handler.rs` L156 に `app_state.comparison_studies.push(*context)` があり、`AppMessage` で受信する設計になっている。ツールバーに "Open Journal for Comparison" ダイアログ → 別スレッドでパース → `AppMessage::ComparisonStudyParsed` → `message_handler.rs` の追加処理、の流れが必要。
- **CSV の書き出し先**: `egui-app` は Desktop アプリのため、`rfd::FileDialog::save_file()` でファイル保存ダイアログを出す。
- **PDP Observed Overlay**: `PdpResult1d` に観測点 `Vec<(f64, f64)>` フィールドを追加する必要がある可能性がある。`messages.rs` の `PdpResult1d` を確認すること。
- **Surface Plot**: `PdpChart2D` (pdp_2d.rs) はすでに 2D ヒートマップを表示するが、3D サーフェス表示は別 ChartId として実装を検討する。`egui_plot` は 3D 非対応のため、wgpu ベースのカスタム描画が必要になる可能性がある。
- **Brushing**: `app_state.selected_indices` と `highlighted_trial` は存在する。Pareto Scatter 2D での矩形選択 UI が欠けている。
