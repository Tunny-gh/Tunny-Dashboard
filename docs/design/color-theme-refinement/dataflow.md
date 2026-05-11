# カラーテーマ洗練 データフロー図

**作成日**: 2026-05-12
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連要件定義**: [requirements.md](../../spec/color-theme-refinement/requirements.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 要件定義書・既存実装を参考にした確実なフロー
- 🟡 **黄信号**: 要件定義書・既存実装から妥当な推測によるフロー

---

## テーマ適用フロー（起動時） 🔵

**信頼性**: 🔵 *既存実装 app.rs・theme/mod.rs より*

```mermaid
flowchart TD
    A[eframe::App::new] --> B["app.rs: new()"]
    B --> C["theme::tunny_light_visuals()"]
    C --> D["Visuals 構築"]
    D --> E["ui_colors::PANEL_BG → panel_fill"]
    D --> F["ui_colors::CENTRAL_BG → window_fill"]
    D --> G["ui_colors::ACCENT_BLUE → widgets.active"]
    D --> H["ui_colors::TEXT_PRIMARY → override_text_color"]
    D --> I["ui_colors::BORDER_COLOR → window_stroke"]
    E --> J["cc.builder.init_visuals = tunny_light_visuals()"]
    F --> J
    G --> J
    H --> J
    I --> J
    J --> K["アプリケーション起動"]
```

**詳細**:
1. `app.rs` の `new()` で `tunny_light_visuals()` を呼び出す
2. `tunny_light_visuals()` は `ui_colors.rs` の定数を参照して `Visuals` を構築
3. **今回の変更**: 定数のRGB値が変わるだけで、フロー自体は一切変更なし

---

## UI描画時の色参照フロー 🔵

**信頼性**: 🔵 *既存実装の各UIコンポーネントより*

```mermaid
flowchart TD
    subgraph "theme モジュール（Single Source of Truth）"
        UC["ui_colors.rs<br/>UI色定数"]
        CC["chart_colors.rs<br/>チャート色定数"]
        CM["colormap.rs<br/>カラーマップ"]
    end

    subgraph "UIコンポーネント"
        TB["toolbar.rs"]
        GC["grid_canvas.rs"]
        P2["pareto_2d.rs"]
        P3["pareto_3d.rs"]
        PC["parallel_coords.rs"]
        SM["scatter_matrix.rs"]
        OH["optimization_history.rs"]
        PD["pdp_chart.rs"]
        BC["bar系チャート"]
        TT["trial_table.rs"]
    end

    UC -->|"PANEL_BG, ACCENT_BLUE等"| TB
    UC -->|"CENTRAL_BG, CELL_TOOLBAR_BG"| GC
    UC -->|"WIDGET_BG, ACCENT_BLUE"| GC
    CC -->|"COLOR_PARETO, COLOR_NON_PARETO"| P2
    CC -->|"COLOR_PARETO, COLOR_AXIS_*"| P3
    CC -->|"COLOR_PARALLEL_*"| PC
    CC -->|"COLOR_SCATTER_DOT"| SM
    CC -->|"COLOR_OPT_*"| OH
    CC -->|"COLOR_PDP_*, COLOR_CONTOUR"| PD
    CC -->|"COLOR_BAR_*"| BC
    CC -->|"COLOR_CELL_HIGHLIGHT"| TT
    CM -->|"ColorMap::viridis()等"| SM
    CM -->|"ColorMap::*"| P2
```

**変更の影響**: 全UIコンポーネントは `crate::theme::*` 経由で定数を参照しているため、
`ui_colors.rs` と `chart_colors.rs` の値を変更するだけで全画面に反映される。

---

## チャート描画の色計算フロー 🔵

**信頼性**: 🔵 *既存実装 color_compute.rs より*

```mermaid
flowchart TD
    A["チャート描画リクエスト"] --> B{"ColorMode"}
    B -->|"ParetoRank"| C["color_compute::compute_chart_colors()"]
    B -->|"ObjectiveValue"| C
    B -->|"TrialNumber"| C
    B -->|"ClusterId"| C

    C --> D["normalize_trial()"]
    D --> E["ColorMap::interpolate(t)"]
    E --> F["グラデーション色を返す"]

    B -->|"デフォルト表示"| G["chart_colors::COLOR_PARETO / COLOR_NON_PARETO"]
    B -->|"選択ハイライト"| H["color_compute::compute_point_alpha()"]

    H --> I{"選択状態"}
    I -->|"選択済み"| J["元の色 + 完全アルファ"]
    I -->|"未選択"| K["chart_colors::COLOR_*_DIM"]
```

**今回の変更点**:
- `COLOR_PARETO`, `COLOR_NON_PARETO`, `COLOR_*_DIM` のRGB値が変更される
- `color_compute.rs` の計算ロジック自体は変更なし
- `colormap.rs` のグラデーション定義は変更なし

---

## 3Dビューの色フロー（ライト化） 🔵

**信頼性**: 🔵 *既存実装 pareto_3d.rs より*

```mermaid
flowchart TD
    A["3Dビュー描画"] --> B["COLOR_3D_BG で背景クリア"]
    B --> C["COLOR_3D_GRID でグリッド描画"]
    C --> D["COLOR_AXIS_X/Y/Z で軸描画"]
    D --> E["COLOR_PARETO でPareto点描画"]
    E --> F["COLOR_NON_PARETO でNon-Pareto点描画"]
    F --> G["COLOR_HIGHLIGHT_PT でハイライト点描画"]
```

**新旧比較**:

| ステップ | 旧動作 | 新動作 |
|---------|--------|--------|
| 背景クリア | ダーク（20,20,30） | ライトグレー（240,242,245） |
| グリッド描画 | 半透明ダークグレー | 半透明グレー（33,33,36,α=70） |
| 軸描画 | 高彩度RGB（220/220/220） | パステルRGB（210/170/200） |
| 点描画 | 赤青（220,50,50/50,150,250） | Google色（234,67,53/66,133,244） |
| ハイライト | YELLOW | Deep Purple（124,77,255） |

---

## エラーハンドリングフロー 🟡

**信頼性**: 🟡 *既存実装パターンから推測*

今回の変更は色定数の値変更のみであり、エラーハンドリングフローに影響はない。
ただし、ビルド時の潜在的な問題として:

```mermaid
flowchart TD
    A["色定数変更"] --> B{"cargo build"}
    B -->|"成功"| C["cargo test"]
    B -->|"失敗"| D["コンパイルエラー確認"]
    D --> E["定数の型・値を修正"]
    E --> B
    C -->|"全テスト通過"| F["完了"]
    C -->|"テスト失敗"| G["テストケース確認"]
    G --> H["該当するテストの期待値を更新"]
    H --> C
```

**注意**: `colormap.rs` のテスト（`interpolate_at_half_returns_midpoint`等）は
RGB値を直接アサートしているため、colormapを変更しない限り影響なし。

---

## 状態遷移フロー 🔵

**信頼性**: 🔵 *既存実装より*

色テーマの状態はコンパイル時定数のみ。ランタイムでの状態変化なし。

```mermaid
stateDiagram-v2
    [*] --> コンパイル時: 色定数確定
    コンパイル時 --> 起動: Visuals構築
    起動 --> 描画: チャート/UI描画
    描画 --> 描画: フレーム更新（同じ色定数）
```

---

## 関連文書

- **アーキテクチャ**: [architecture.md](architecture.md)
- **設計ヒアリング**: [design-interview.md](design-interview.md)
- **要件定義**: [requirements.md](../../spec/color-theme-refinement/requirements.md)

## 信頼性レベルサマリー

- 🔵 青信号: 5件 (83%)
- 🟡 黄信号: 1件 (17%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
