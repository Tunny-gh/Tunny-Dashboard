# カラーテーマ洗練 アーキテクチャ設計

**作成日**: 2026-05-12
**関連要件定義**: [requirements.md](../../spec/color-theme-refinement/requirements.md)
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 要件定義書・参考画像分析・ヒアリングを参考にした確実な設計
- 🟡 **黄信号**: 要件定義書・参考画像から妥当な推測による設計
- 🔴 **赤信号**: 要件定義書・参考画像にない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *要件定義書より*

既存の `theme/ui_colors.rs` と `theme/chart_colors.rs` に定義された色定数のRGB値を、
参考画像（Optuna Dashboard風）に基づくGoogle Material Design系パレットに刷新する。
ファイル構成・コード構造は一切変更せず、数値のみを変更する。

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存設計（ui-color-theme要件）より*

- **パターン**: 集中定数パターン（Centralized Constants）
- **選択理由**: 前回要件で確立済み。`theme/` モジュールが色のSingle Source of Truth。
  今回はその値のみを更新する。

## 色パレット設計

### ベースパレット: Google Material Colors 🔵

**信頼性**: 🔵 *参考画像分析・ヒアリング「忠実に再現」より*

参考画像の配色を分析し、Google Material Designの主要色に基づく統一パレットを定義:

| 役割 | 色名 | HEX | RGB | 出典 |
|------|------|-----|-----|------|
| アクセント | Google Blue | #4285f4 | (66, 133, 244) | 参考画像ボタン・アクティブタブ |
| エラー/Pareto | Google Red | #ea4335 | (234, 67, 53) | 参考画像Pareto点・エラー表示 |
| 成功/Running | Google Green | #34a853 | (52, 168, 83) | セマンティック色の統一 |
| 警告/Best | Google Yellow | #fbbc04 | (251, 188, 4) | 参考画像の警告・best値表示 |
| プライマリテキスト | Dark Gray | #202124 | (32, 33, 36) | 参考画像メインテキスト |
| セカンダリテキスト | Medium Gray | #5f6368 | (95, 99, 104) | 参考画像ラベル・メタデータ |
| ボーダー | Light Gray | #dadce0 | (218, 220, 224) | 参考画像パネル境界 |
| パネル背景 | Off White | #f0f2f5 | (240, 242, 245) | 参考画像メイン背景 |
| ハイライト | Deep Purple | #7c4dff | (124, 77, 255) | ライト背景で高コントラスト |

---

### `ui_colors.rs` 新旧マッピング 🔵

**信頼性**: 🔵 *参考画像分析・要件定義REQ-001〜REQ-026より*

#### ツールバー系

| 定数 | 旧RGB | 新RGB | HEX | 変更理由 |
|------|-------|-------|-----|---------|
| `TOOLBAR_BG` | (26, 35, 50) | (32, 33, 36) | #202124 | Google Dark Gray 🔵 |
| `TOOLBAR_TEXT` | (220, 230, 245) | (232, 234, 237) | #e8eaed | Google Light Gray 🔵 |
| `TOOLBAR_BTN_HOVER` | (55, 78, 120) | (55, 65, 81) | #374151 | ニュートラルダークグレー 🟡 |
| `TOOLBAR_BTN_ACTIVE` | (37, 99, 235) | (66, 133, 244) | #4285f4 | Google Blue 🔵 |
| `TOOLBAR_BTN_FG` | WHITE | WHITE | — | 変更なし |
| `TOOLBAR_INPUT_BG` | (45, 62, 90) | (48, 49, 52) | #303134 | Google Dark 🔵 |
| `TOOLBAR_INPUT_STROKE` | (100, 130, 180) | (95, 99, 104) | #5f6368 | TEXT_SECONDARYと統一 🟡 |

#### パネル・キャンバス系

| 定数 | 旧RGB | 新RGB | HEX | 変更理由 |
|------|-------|-------|-----|---------|
| `PANEL_BG` | (225, 233, 248) | (240, 242, 245) | #f0f2f5 | 青み排除→ニュートラルグレー 🔵 |
| `CENTRAL_BG` | WHITE | WHITE | — | 変更なし |
| `CELL_TOOLBAR_BG` | (232, 239, 251) | (245, 247, 250) | #f5f7fa | 青み排除・PANEL_BGより明るく 🔵 |

#### ウィジェット系

| 定数 | 旧RGB | 新RGB | HEX | 変更理由 |
|------|-------|-------|-----|---------|
| `WIDGET_BG` | (235, 241, 252) | (240, 244, 248) | #f0f4f8 | 青み排除・ニュートラルグレー 🔵 |
| `WIDGET_BG_HOVER` | (220, 230, 247) | (232, 236, 242) | #e8ecf2 | WIDGET_BGより暗いグレー 🔵 |
| `CLOSE_BTN_TEXT` | gray(180) | gray(180) | — | 変更なし |

#### アクセントカラー

| 定数 | 旧RGB | 新RGB | HEX | 変更理由 |
|------|-------|-------|-----|---------|
| `ACCENT_BLUE` | (37, 99, 235) | (66, 133, 244) | #4285f4 | Google Blue 🔵 |
| `ACCENT_BLUE_HOVER` | (29, 78, 216) | (51, 103, 214) | #3367d6 | Google Blue暗版 🔵 |
| `ACCENT_BLUE_MUTED` | (219, 234, 254) | (232, 240, 254) | #e8f0fe | Google Blue薄版 🔵 |

#### テキスト・ボーダー系

| 定数 | 旧RGB | 新RGB | HEX | 変更理由 |
|------|-------|-------|-----|---------|
| `TEXT_PRIMARY` | (30, 41, 59) | (32, 33, 36) | #202124 | Google Dark Gray 🔵 |
| `TEXT_SECONDARY` | (100, 116, 139) | (95, 99, 104) | #5f6368 | Google Medium Gray 🔵 |
| `BORDER_COLOR` | (203, 213, 225) | (218, 220, 224) | #dadce0 | Google Border Gray 🔵 |

#### セマンティックカラー

| 定数 | 旧RGB | 新RGB | HEX | 変更理由 |
|------|-------|-------|-----|---------|
| `ERROR_COLOR` | (220, 50, 50) | (234, 67, 53) | #ea4335 | Google Red 🔵 |

---

### `chart_colors.rs` 新旧マッピング 🔵

**信頼性**: 🔵 *参考画像分析・要件定義REQ-031〜REQ-073より*

#### Pareto系

| 定数 | 旧値 | 新値 | 変更理由 |
|------|------|------|---------|
| `COLOR_PARETO` | rgb(220, 50, 50) | rgb(234, 67, 53) | Google Red 🔵 |
| `COLOR_NON_PARETO` | rgb(50, 150, 250) | rgb(66, 133, 244) | Google Blue 🔵 |
| `COLOR_PARETO_DIM` | rgba_premul(52, 12, 12, 60) | rgba_premul(55, 16, 12, 60) | 再計算 🔵 |
| `COLOR_NON_PARETO_DIM` | rgba_premul(12, 35, 59, 60) | rgba_premul(16, 31, 57, 60) | 再計算 🔵 |

#### 3D軸色（パステル系）

| 定数 | 旧RGB | 新RGB | HEX | 変更理由 |
|------|-------|-------|-----|---------|
| `COLOR_AXIS_X` | (220, 80, 80) | (210, 100, 100) | #d26464 | 落ち着いた赤・ライト背景向け 🔵 |
| `COLOR_AXIS_Y` | (80, 220, 80) | (80, 170, 80) | #50aa50 | 落ち着いた緑・ライト背景向け 🔵 |
| `COLOR_AXIS_Z` | (80, 80, 220) | (100, 100, 200) | #6464c8 | 落ち着いた青・ライト背景向け 🔵 |

#### MCDMスコア段階色

| 定数 | 旧RGB | 新RGB | HEX | 変更理由 |
|------|-------|-------|-----|---------|
| `COLOR_MCDM_HIGH` | (255, 0, 0) | (234, 67, 53) | #ea4335 | Google Red 🔵 |
| `COLOR_MCDM_MID` | (255, 165, 0) | (251, 188, 4) | #fbbc04 | Google Yellow 🔵 |
| `COLOR_MCDM_LOW` | (255, 255, 0) | (52, 168, 83) | #34a853 | Google Green 🔵 |
| `COLOR_MCDM_NONE` | (200, 200, 200) | (200, 200, 200) | — | 変更なし |

#### バー・チャート系

| 定数 | 旧RGB | 新RGB | HEX | 変更理由 |
|------|-------|-------|-----|---------|
| `COLOR_BAR_PRIMARY` | (12, 106, 192) | (66, 133, 244) | #4285f4 | Google Blue 🔵 |
| `COLOR_BAR_NEGATIVE` | (192, 32, 32) | (234, 67, 53) | #ea4335 | Google Red 🔵 |
| `COLOR_BAR_ACCENT` | (224, 112, 0) | (251, 188, 4) | #fbbc04 | Google Yellow 🔵 |
| `COLOR_IMPORTANCE_BAR` | (12, 12, 106) | (30, 60, 114) | #1e3c72 | より明るいネイビー 🟡 |

#### 最適化履歴系

| 定数 | 旧RGB | 新RGB | HEX | 変更理由 |
|------|-------|-------|-----|---------|
| `COLOR_OPT_TRIAL` | (50, 150, 250) | (66, 133, 244) | #4285f4 | Google Blue 🔵 |
| `COLOR_OPT_PRUNED` | (220, 50, 50) | (234, 67, 53) | #ea4335 | Google Red 🔵 |
| `COLOR_OPT_RUNNING` | (50, 200, 120) | (52, 168, 83) | #34a853 | Google Green 🔵 |
| `COLOR_OPT_BEST` | GOLD | rgb(251, 188, 4) | #fbbc04 | Google Yellow 🟡 |

#### HV履歴系

| 定数 | 旧RGB | 新RGB | HEX | 変更理由 |
|------|-------|-------|-----|---------|
| `COLOR_HV_LINE` | (50, 200, 100) | (52, 168, 83) | #34a853 | Google Green 🔵 |

#### フィット品質系

| 定数 | 旧RGB | 新RGB | HEX | 変更理由 |
|------|-------|-------|-----|---------|
| `COLOR_FIT_LOW` | (220, 80, 80) | (234, 67, 53) | #ea4335 | Google Red 🔵 |
| `COLOR_FIT_MID` | (200, 160, 0) | (251, 188, 4) | #fbbc04 | Google Yellow 🔵 |
| `COLOR_FIT_HIGH` | (60, 180, 60) | (52, 168, 83) | #34a853 | Google Green 🔵 |

#### PDP系

| 定数 | 旧値 | 新値 | 変更理由 |
|------|------|------|---------|
| `COLOR_PDP_LINE` | rgb(50, 100, 255) | rgb(66, 133, 244) | Google Blue 🔵 |
| `COLOR_PDP_CI` | rgba_premul(10, 20, 50, 50) | rgba_premul(13, 26, 48, 50) | 再計算 🔵 |
| `COLOR_ICE_LINE` | rgba_premul(35, 35, 35, 60) | rgba_premul(35, 35, 35, 60) | 変更なし |
| `COLOR_CONTOUR` | YELLOW | rgb(124, 77, 255) | Deep Purple・ライト背景向け 🔵 |

#### スキャッタ系

| 定数 | 旧RGB | 新RGB | HEX | 変更理由 |
|------|-------|-------|-----|---------|
| `COLOR_SCATTER_DOT` | (70, 130, 220) | (66, 133, 244) | #4285f4 | Google Blue 🔵 |

#### 選択ハイライト系

| 定数 | 旧値 | 新値 | 変更理由 |
|------|------|------|---------|
| `COLOR_SELECTION_HIGHLIGHT` | rgba_premul(6, 16, 37, 40) | rgba_premul(10, 21, 38, 40) | 再計算 🔵 |
| `COLOR_CELL_HIGHLIGHT` | rgba_premul(12, 31, 74, 80) | rgba_premul(21, 42, 77, 80) | 再計算 🔵 |

#### リンク色

| 定数 | 旧RGB | 新RGB | HEX | 変更理由 |
|------|-------|-------|-----|---------|
| `COLOR_LINK` | (80, 120, 180) | (66, 133, 244) | #4285f4 | Google Blue 🔵 |

#### 3Dビュー系（ライト化）

| 定数 | 旧値 | 新値 | 変更理由 |
|------|------|------|---------|
| `COLOR_3D_BG` | rgb(20, 20, 30) | rgb(240, 242, 245) | PANEL_BGと統一 🔵 |
| `COLOR_3D_GRID` | rgba_premul(33, 33, 38, 70) | rgba_premul(33, 33, 36, 70) | ライト背景向けグリッド 🔵 |

#### ハイライト試行点

| 定数 | 旧値 | 新値 | 変更理由 |
|------|------|------|---------|
| `COLOR_HIGHLIGHT_PT` | YELLOW | rgb(124, 77, 255) | Deep Purple・ライト背景向け 🔵 |

#### パラレルコーディネート系

| 定数 | 旧RGB | 新RGB | HEX | 変更理由 |
|------|-------|-------|-----|---------|
| `COLOR_PARALLEL_TICK` | gray(60) | rgb(95, 99, 104) | #5f6368 | TEXT_SECONDARY統一 🟡 |
| `COLOR_PARALLEL_LINE_DEFAULT` | rgb(100, 150, 220) | rgb(66, 133, 244) | #4285f4 | Google Blue 🔵 |
| `COLOR_PARALLEL_AXIS` | gray(80) | rgb(218, 220, 224) | #dadce0 | BORDER_COLOR統一 🟡 |

#### PDP CI凡例

| 定数 | 旧値 | 新値 | 変更理由 |
|------|------|------|---------|
| `COLOR_PDP_CI_LEGEND` | rgba_premul(24, 47, 120, 120) | rgba_premul(31, 63, 115, 120) | 再計算 🔵 |

#### AHP一貫性比率

| 定数 | 旧値 | 新値 | 変更理由 |
|------|------|------|---------|
| `COLOR_CR_OK` | GREEN | rgb(52, 168, 83) | Google Green 🟡 |

#### チャート汎用色

| 定数 | 旧値 | 新値 | 変更理由 |
|------|------|------|---------|
| `COLOR_CHART_TEXT` | BLACK | rgb(32, 33, 36) | TEXT_PRIMARY統一 🔵 |
| `COLOR_EMPTY_STATE` | GRAY | rgb(95, 99, 104) | TEXT_SECONDARY統一 🟡 |
| `COLOR_GRID_STROKE` | GRAY | rgb(218, 220, 224) | BORDER_COLOR統一 🟡 |

---

## Premultiplied Alpha 計算参照 🔵

**信頼性**: 🔵 *egui Color32 premultiplied alpha仕様より*

全ての半透明色は `Color32::from_rgba_premultiplied(r, g, b, a)` を使用。
RGB成分はベース色の各チャンネルにアルファを乗算して255で除算した値。

```
r_premul = r_base * alpha / 255  (四捨五入)
g_premul = g_base * alpha / 255
b_premul = b_base * alpha / 255
```

| 定数 | ベース色 | α | R | G | B |
|------|---------|---|---|---|---|
| COLOR_PARETO_DIM | (234, 67, 53) | 60 | 55 | 16 | 12 |
| COLOR_NON_PARETO_DIM | (66, 133, 244) | 60 | 16 | 31 | 57 |
| COLOR_SELECTION_HIGHLIGHT | (66, 133, 244) | 40 | 10 | 21 | 38 |
| COLOR_CELL_HIGHLIGHT | (66, 133, 244) | 80 | 21 | 42 | 77 |
| COLOR_PDP_CI | (66, 133, 244) | 50 | 13 | 26 | 48 |
| COLOR_PDP_CI_LEGEND | (66, 133, 244) | 120 | 31 | 63 | 115 |
| COLOR_3D_GRID | (120, 120, 130) | 70 | 33 | 33 | 36 |

---

## 非変更ファイル 🔵

**信頼性**: 🔵 *要件定義REQ-081・ユーザヒアリングより*

| ファイル | 理由 |
|---------|------|
| `theme/colormap.rs` | ユーザー指示「カラーマップは維持」 |
| `theme/colormap_name.rs` | カラーマップ名列挙型、変更不要 |
| `theme/color_compute.rs` | 動的計算ロジック、色値に非依存 |

---

## 技術的制約

### ファイル変更範囲の厳格な制限 🔵

**信頼性**: 🔵 *要件定義NFR-021〜NFR-023より*

- `theme/mod.rs`: `tunny_light_visuals()` のStroke設定内で参照する色定数は自動的に新値を使うため、`mod.rs`自体の編集は不要
- `theme/colormap.rs`: 一切変更禁止
- ウィジェットファイル群: 一切変更禁止（既に `crate::theme::*` 経由で参照済み）

### コントラスト検証 🔵

**信頼性**: 🔵 *WCAG AA基準・NFR-031より*

主要なテキスト/背景の組み合わせ:

| 組み合わせ | 前景 | 背景 | コントラスト比 | 判定 |
|-----------|------|------|--------------|------|
| プライマリテキスト/パネル | #202124 | #f0f2f5 | ~15.3:1 | AA ✅ |
| セカンダリテキスト/パネル | #5f6368 | #f0f2f5 | ~5.0:1 | AA ✅ |
| ツールバーテキスト/ツールバー | #e8eaed | #202124 | ~13.7:1 | AA ✅ |
| チャートテキスト/キャンバス | #202124 | #ffffff | ~17.4:1 | AA ✅ |
| 3D軸X/3D背景 | #d26464 | #f0f2f5 | ~3.2:1 | AA大文字 ✅ |
| ハイライト/キャンバス | #7c4dff | #ffffff | ~4.7:1 | AA ✅ |

---

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **設計ヒアリング**: [design-interview.md](design-interview.md)
- **要件定義**: [requirements.md](../../spec/color-theme-refinement/requirements.md)

## 信頼性レベルサマリー

- 🔵 青信号: 42件 (84%)
- 🟡 黄信号: 8件 (16%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
