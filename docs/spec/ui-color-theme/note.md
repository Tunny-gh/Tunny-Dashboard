# UIカラー設定一元化 コンテキストノート

## プロジェクト概要

Tunny Dashboard は Optuna 最適化結果を分析する Rust/egui デスクトップアプリ。

## 技術スタック

| 項目 | 内容 |
|------|------|
| 言語 | Rust (ネイティブのみ、WASM不要) |
| UIフレームワーク | eframe + egui |
| GPUレンダリング | wgpu + egui-wgpu |
| チャート | egui_plot |

## 現状の色管理

### 既存の色定義ファイル

| ファイル | 内容 |
|---------|------|
| `egui-app/src/theme.rs` | UIテーマ色（TOOLBAR_BG, PANEL_BG, ACCENT_BLUE等）+ `tunny_light_visuals()` |
| `egui-app/src/render/colormap.rs` | ColorMap struct、連続グラデーション（viridis/plasma/jet等）、tab10パレット、ロジック関数 |

### ウィジェットに散在する色定義

| ファイル | ハードコード色の例 |
|---------|----------------|
| `ui/widgets/mcdm_scatter_chart.rs` | `COLOR_RED`, `COLOR_ORANGE`, `COLOR_YELLOW`, `COLOR_GRAY` (定数として定義) |
| `ui/widgets/pareto_2d.rs` | `COLOR_PARETO`(赤), `COLOR_NON_PARETO`(青), DIM バリアント |
| `ui/widgets/slice_chart.rs` | `COLOR_PARETO`(赤), `COLOR_NON_PARETO`(青) |
| `ui/widgets/pareto_3d.rs` | 軸色(赤/緑/青), Pareto点色(赤/青), 選択ハイライト(黄) |
| `ui/widgets/optimization_history.rs` | 試行線色(青/赤/緑/金) |
| `ui/widgets/hv_history.rs` | グリーン系ライン色 |
| `ui/widgets/importance_chart.rs` | フィット品質色(赤/黄/緑), バー色 |
| `ui/widgets/mcdm_chart.rs` | バー色(青/赤/橙) |
| `ui/widgets/parallel_coords.rs` | 背景白, テキスト黒, ティック色, ライン色 |
| `ui/widgets/scatter_matrix.rs` | ドット色, ヒートマップ色, バー色, テキスト黒 |
| `ui/widgets/sensitivity_heatmap.rs` | ヒートマップ色, グリッド灰, テキスト黒 |
| `ui/widgets/pdp_chart.rs` | 信頼区間色(青系), ICEライン色(灰), PDP線色(青), Pareto線色(赤) |
| `ui/widgets/pdp_2d.rs` | 等高線色(黄) |
| `ui/widgets/cluster_scatter.rs` | エラーラベル色(赤) |
| `ui/widgets/ahp_chart.rs` | バー色(青), 正/負ラベル色(緑/赤), グレーラベル |
| `ui/widgets/trial_table.rs` | セル色(青系) |
| `ui/bottom_panel.rs` | リンク色(青系) |
| `ui/comparison_panel.rs` | フォールバック灰色 |
| `ui/grid_canvas.rs` | セル背景白, 選択ハイライト青(アルファ付き) |
| `ui/toolbar.rs` | エラー赤, TRANSPARENT, 白 |
| `ui/layout.rs` | TRANSPARENT, 白(ストローク) |
| `state/app_state.rs` | `Color32::RED`（初期値用） |

## 目標アーキテクチャ

```
egui-app/src/theme/
├── mod.rs          # UIテーマ色（TOOLBAR_BG等）+ tunny_light_visuals()
├── colormap.rs     # ColorMap struct + 連続グラデーション + ロジック関数
└── chart_colors.rs # チャート固有色定数（COLOR_PARETO等）
```

## 注意事項

- `colormap.rs` のロジック関数（`normalize_trial`, `compute_chart_colors`）は `state::app_state` 型に依存しているため、循環依存に注意
- `Color32::TRANSPARENT`・`Color32::WHITE`・`Color32::BLACK` 等の egui 組み込み定数は場合によってはtheme化しなくても問題ない（判断は実装者に委ねる）
- エラー表示用の `Color32::RED` は意味的に明確なので、セマンティックな名前（`ERROR_COLOR`）でthemeに定義する
