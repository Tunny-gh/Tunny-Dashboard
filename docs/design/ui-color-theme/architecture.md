# UIカラー設定一元化 アーキテクチャ設計

**作成日**: 2026-05-07
**関連要件定義**: [requirements.md](../../spec/ui-color-theme/requirements.md)
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *要件定義書REQ-001〜005・ユーザヒアリングより*

Tunny Dashboard（Rust/egui デスクトップアプリ）において、現在 `theme.rs`・`render/colormap.rs` および15以上のウィジェットファイルに散在している色定数を `egui-app/src/theme/` ディレクトリ配下の 3 ファイルに集約するリファクタリング。機能追加は行わず、色の定義場所のみを変更する。

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *Rustモジュールシステム・ユーザヒアリングより*

- **パターン**: モジュール分割による関心の分離（Separation of Concerns）
- **選択理由**: Rust の `mod` システムを活用し、色定義の責務を目的別に 3 つのサブモジュールへ分割する。既存のアーキテクチャに影響せず、ファイル移動と import パス変更のみで完結する

## コンポーネント構成

### Before: 現状の色定義箇所 🔵

**信頼性**: 🔵 *コードベース分析より*

```
egui-app/src/
├── theme.rs                         ← UIテーマ色（17定数）+ tunny_light_visuals()
├── render/
│   └── colormap.rs                  ← ColorMap struct + グラデーション9種 + ロジック関数3つ
└── ui/widgets/
    ├── mcdm_scatter_chart.rs        ← 4定数（COLOR_RED/ORANGE/YELLOW/GRAY）
    ├── pareto_2d.rs                 ← 4定数（COLOR_PARETO/NON_PARETO + DIM）
    ├── slice_chart.rs               ← 2定数（COLOR_PARETO/NON_PARETO）
    ├── pareto_3d.rs                 ← インライン色多数
    ├── optimization_history.rs      ← インライン色4色
    ├── hv_history.rs                ← インライン色1色
    ├── importance_chart.rs          ← インライン色4色
    ├── mcdm_chart.rs                ← インライン色3色
    ├── parallel_coords.rs           ← インライン色多数
    ├── scatter_matrix.rs            ← インライン色多数
    ├── sensitivity_heatmap.rs       ← インライン色3色
    ├── pdp_chart.rs                 ← インライン色多数
    ├── pdp_2d.rs                    ← インライン色1色
    ├── cluster_scatter.rs           ← インライン色1色
    ├── ahp_chart.rs                 ← インライン色5色
    └── trial_table.rs               ← インライン色1色
```

### After: 目標モジュール構成 🔵

**信頼性**: 🔵 *ユーザヒアリング・REQ-001〜005より*

```
egui-app/src/
├── theme/                           ← 色定義の唯一の場所（NEW DIRECTORY）
│   ├── mod.rs                       ← UIテーマ色（TOOLBAR_BG等17定数）+ tunny_light_visuals()
│   ├── colormap.rs                  ← ColorMap struct + グラデーション + ロジック関数（render/colormap.rsから移動）
│   └── chart_colors.rs              ← チャート固有色定数（NEW FILE）
├── render/
│   └── colormap.rs                  ← 削除（または空にしてmod.rsのみ残す）
└── ui/widgets/
    └── （全ウィジェット）             ← crate::theme インポートに統一、インライン色なし
```

## モジュール依存関係 🔵

**信頼性**: 🔵 *コードベース分析・ユーザヒアリングより*

```
                    ┌─────────────────────────────┐
                    │   egui-app クレート           │
                    │                             │
 ┌──────────┐      │  ┌────────────────────────┐  │
 │  app.rs  │─────────│  crate::theme           │  │
 └──────────┘      │  │  ├── mod.rs             │  │
                    │  │  ├── colormap.rs        │  │
 ┌──────────────┐  │  │  └── chart_colors.rs   │  │
 │ state/       │  │  └────────────────────────┘  │
 │  app_state   │──────────────────────┘           │
 │  types       │  │     ↑ (使用)                  │
 └──────────────┘  │                             │
                    │  ┌─────────────────────┐    │
 ┌──────────────┐  │  │  ui/widgets/        │    │
 │ ui/widgets/  │──────│  ← crate::theme のみ参照 │
 └──────────────┘  │  └─────────────────────┘    │
                    └─────────────────────────────┘
```

注意: Rust は同一クレート内での循環モジュール参照を許可するため、
`theme::colormap` が `state::app_state` の型を参照しても問題なし。

## ディレクトリ構造（theme モジュール詳細） 🔵

**信頼性**: 🔵 *ユーザヒアリング・REQ-001〜005より*

```
egui-app/src/theme/
├── mod.rs
│   // UIテーマ色 17定数
│   pub const TOOLBAR_BG: Color32
│   pub const TOOLBAR_TEXT: Color32
│   pub const PANEL_BG: Color32
│   pub const CENTRAL_BG: Color32
│   pub const ACCENT_BLUE: Color32
│   pub const ACCENT_BLUE_HOVER: Color32
│   pub const ACCENT_BLUE_MUTED: Color32
│   pub const BORDER_COLOR: Color32
│   pub const TEXT_PRIMARY: Color32
│   pub const TEXT_SECONDARY: Color32
│   pub const CELL_TOOLBAR_BG: Color32
│   pub const WIDGET_BG: Color32
│   pub const WIDGET_BG_HOVER: Color32
│   pub const TOOLBAR_BTN_HOVER: Color32
│   pub const TOOLBAR_BTN_ACTIVE: Color32
│   pub const TOOLBAR_INPUT_BG: Color32
│   pub const TOOLBAR_INPUT_STROKE: Color32
│   // セマンティック色（新規）
│   pub const ERROR_COLOR: Color32
│   // 関数
│   pub fn tunny_light_visuals() -> Visuals
│
├── colormap.rs
│   // 構造体
│   pub struct ColorMap { pub stops: Vec<(f32, Color32)> }
│   impl ColorMap {
│     pub fn viridis/plasma/blue_yellow/jet/turbo/inferno/coolwarm/spectral/cividis() -> Self
│     pub fn interpolate(&self, t: f32) -> Color32
│   }
│   // 離散パレット
│   pub fn tab10_palette() -> Vec<Color32>
│   // ロジック関数（state::app_state 型を使用）
│   pub fn compute_point_alpha(trial_id: u32, selected_indices: &[u32]) -> u8
│   pub fn normalize_trial(...) -> f32
│   pub fn compute_chart_colors(...) -> Vec<Color32>
│
└── chart_colors.rs
    // Pareto 系
    pub const COLOR_PARETO: Color32         // (220, 50, 50) 赤
    pub const COLOR_NON_PARETO: Color32     // (50, 150, 250) 青
    pub const COLOR_PARETO_DIM: Color32     // (220, 50, 50, alpha=60)
    pub const COLOR_NON_PARETO_DIM: Color32 // (50, 150, 250, alpha=60)
    // MCDM スコア系
    pub const COLOR_MCDM_HIGH: Color32      // 赤   (旧 COLOR_RED)
    pub const COLOR_MCDM_MID: Color32       // 橙   (旧 COLOR_ORANGE)
    pub const COLOR_MCDM_LOW: Color32       // 黄   (旧 COLOR_YELLOW)
    pub const COLOR_MCDM_NONE: Color32      // 灰   (旧 COLOR_GRAY)
    // バー・チャート系
    pub const COLOR_BAR_PRIMARY: Color32    // (12, 106, 192) 青
    pub const COLOR_BAR_NEGATIVE: Color32   // (192, 32, 32) 赤
    pub const COLOR_BAR_ACCENT: Color32     // (224, 112, 0) 橙
    // 最適化履歴系
    pub const COLOR_OPT_TRIAL: Color32      // (50, 150, 250) 青
    pub const COLOR_OPT_PRUNED: Color32     // (220, 50, 50) 赤
    pub const COLOR_OPT_RUNNING: Color32    // (50, 200, 120) 緑
    pub const COLOR_OPT_BEST: Color32       // GOLD
    // フィット品質系
    pub const COLOR_FIT_LOW: Color32        // (220, 80, 80) 赤
    pub const COLOR_FIT_MID: Color32        // (200, 160, 0) 黄
    pub const COLOR_FIT_HIGH: Color32       // (60, 180, 60) 緑
    // HV 履歴系
    pub const COLOR_HV_LINE: Color32        // (50, 200, 100) 緑
    // 選択ハイライト系
    pub const COLOR_SELECTION_HIGHLIGHT: Color32   // (37, 99, 235, alpha=40)
    pub const COLOR_CELL_HIGHLIGHT: Color32        // (37, 99, 235, alpha=80)
    // リンク色
    pub const COLOR_LINK: Color32           // (80, 120, 180) 青
    // 3D軸色
    pub const COLOR_AXIS_X: Color32         // (220, 80, 80) 赤
    pub const COLOR_AXIS_Y: Color32         // (80, 220, 80) 緑
    pub const COLOR_AXIS_Z: Color32         // (80, 80, 220) 青
    // 汎用チャート色
    pub const COLOR_PDP_LINE: Color32       // (50, 100, 255) 青
    pub const COLOR_PDP_CI: Color32         // (50, 100, 255, alpha=50)
    pub const COLOR_ICE_LINE: Color32       // (150, 150, 150, alpha=60)
    pub const COLOR_CONTOUR: Color32        // YELLOW
    pub const COLOR_SCATTER_DOT: Color32    // (70, 130, 220) 青
```

## 技術的制約 🔵

**信頼性**: 🔵 *Rust言語仕様・コードベース分析より*

### Rust モジュールシステムの制約
- `lib.rs` または `main.rs` から `mod theme;` を宣言すれば `theme.rs` → `theme/mod.rs` の変換は自動的に解決される（Rustはどちらの形式も受け付ける）
- 旧 `theme.rs` を削除し `theme/mod.rs` を作成するだけでパスは変わらない

### `Color32` の定数定義制約
- `Color32::from_rgb` は `const` 文脈で使用可能（Rustの `const fn` 対応）
- `Color32::from_rgba_premultiplied` も `const` 使用可能
- `Color32::from_rgba_unmultiplied` は `const fn` 非対応のため、`const` ではなく `pub fn` または `lazy_static!` / `once_cell` が必要

### アルファ付き色の扱い 🟡

**信頼性**: 🟡 *コードベース分析から妥当な推測*

`Color32::from_rgba_unmultiplied` を使うインライン色（例: grid_canvas.rs の選択ハイライト）は `const` 化できない。対応方針：
1. `pub fn color_xxx() -> Color32` として関数にする、または
2. `from_rgba_premultiplied` に変換して `const` 化する（値は異なる計算式になるため注意）

推奨: `Color32::from_rgba_premultiplied` に変換してプリマルチプライド値で `const` 定義する。

## 移行ステップの概要 🔵

**信頼性**: 🔵 *ユーザーストーリー・要件定義より*

| ステップ | 作業内容 | 関連要件 |
|---------|---------|---------|
| Step 1 | `egui-app/src/theme/` ディレクトリ作成 | REQ-001 |
| Step 2 | `theme.rs` → `theme/mod.rs` に変換（内容そのまま + ERROR_COLOR追加） | REQ-003, REQ-011〜013 |
| Step 3 | `render/colormap.rs` → `theme/colormap.rs` に移動 | REQ-004, REQ-021〜025 |
| Step 4 | `state/types.rs`, `state/app_state.rs`, 各ウィジェットの `render::colormap` パスを `theme::colormap` に更新 | REQ-052 |
| Step 5 | `render/colormap.rs` を削除（または空にして re-export に変更 → 今回は削除） | REQ-052 |
| Step 6 | `theme/chart_colors.rs` 新規作成（全チャート固有色を定義） | REQ-005, REQ-031〜035 |
| Step 7 | 各ウィジェットのインライン `Color32::from_rgb` を `crate::theme::chart_colors::COLOR_XXX` に置換 | REQ-041, REQ-042 |
| Step 8 | `cargo build --warnings` で警告ゼロ確認 + `cargo test` で全テスト通過確認 | NFR-001, NFR-011 |

## 非機能要件の実現方法

### 保守性 🔵

**信頼性**: 🔵 *NFR-021・ユーザヒアリングより*

- 色定義の場所が `egui-app/src/theme/` のみになることで、デザイン変更時の編集箇所が一意になる
- `chart_colors.rs` の定数名は意味（Semantic）ベースで命名するため、具体的な色値を知らなくても利用できる

### コンパイル速度への影響 🟡

**信頼性**: 🟡 *Rustビルド特性から妥当な推測*

- ファイル移動と import パス変更のみのため、コンパイル速度への実質的な影響はほぼゼロ

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **実装ガイド（色定数定義）**: [implementation-guide.md](implementation-guide.md)
- **設計ヒアリング**: [design-interview.md](design-interview.md)
- **要件定義**: [requirements.md](../../spec/ui-color-theme/requirements.md)

## 信頼性レベルサマリー

- 🔵 青信号: 12件 (80%)
- 🟡 黄信号: 3件 (20%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
