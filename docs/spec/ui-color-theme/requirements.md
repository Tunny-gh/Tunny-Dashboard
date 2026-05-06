# UIカラー設定一元化 要件定義書

## 概要

`egui-app/src/theme.rs` および各ウィジェットに散在している色定数を
`egui-app/src/theme/` ディレクトリ配下の 3 ファイルに集約し、
デザイン変更を 1 箇所の修正で完結させる。

## 関連文書

- **ヒアリング記録**: [💬 interview-record.md](interview-record.md)
- **ユーザストーリー**: [📖 user-stories.md](user-stories.md)
- **受け入れ基準**: [✅ acceptance-criteria.md](acceptance-criteria.md)
- **コンテキストノート**: [📝 note.md](note.md)

## 機能要件（EARS記法）

**【信頼性レベル凡例】**:
- 🔵 **青信号**: ユーザヒアリング・既存実装を参考にした確実な要件
- 🟡 **黄信号**: 既存実装・設計から妥当な推測による要件
- 🔴 **赤信号**: ヒアリング・実装にない推測による要件

---

### ディレクトリ構造

- REQ-001: システムは `egui-app/src/theme/` ディレクトリを色定義の唯一の場所としなければならない 🔵 *ユーザヒアリングより*
- REQ-002: `theme/` は `mod.rs`・`colormap.rs`・`chart_colors.rs` の 3 ファイルで構成されなければならない 🔵 *ユーザヒアリングより*
- REQ-003: システムは既存の `egui-app/src/theme.rs` を `egui-app/src/theme/mod.rs` に変換しなければならない 🔵 *ユーザヒアリングより*
- REQ-004: システムは既存の `egui-app/src/render/colormap.rs` を `egui-app/src/theme/colormap.rs` に移動しなければならない 🔵 *ユーザヒアリングより*
- REQ-005: システムは `egui-app/src/theme/chart_colors.rs` を新規作成しなければならない 🔵 *ユーザヒアリングより*

---

### `theme/mod.rs`（UIテーマ色）

- REQ-011: `theme/mod.rs` は現 `theme.rs` の全定数（TOOLBAR_BG, PANEL_BG, ACCENT_BLUE 等）を保持しなければならない 🔵 *既存実装より*
- REQ-012: `theme/mod.rs` は `tunny_light_visuals()` 関数を保持しなければならない 🔵 *既存実装より*
- REQ-013: `theme/mod.rs` は `ERROR_COLOR`（エラー表示用の赤）定数を提供しなければならない 🟡 *複数ウィジェットでの使用から妥当な推測*

---

### `theme/colormap.rs`（連続グラデーション）

- REQ-021: `theme/colormap.rs` は `ColorMap` struct と `interpolate` メソッドを保持しなければならない 🔵 *既存実装より*
- REQ-022: `theme/colormap.rs` は viridis / plasma / blue_yellow / jet / turbo / inferno / coolwarm / spectral / cividis の全グラデーション定義を保持しなければならない 🔵 *既存実装より*
- REQ-023: `theme/colormap.rs` は `tab10_palette()` 関数を保持しなければならない 🔵 *既存実装より*
- REQ-024: `theme/colormap.rs` は `compute_point_alpha` / `normalize_trial` / `compute_chart_colors` のロジック関数を保持しなければならない 🔵 *既存実装より*
- REQ-025: `render/colormap.rs` のすべての既存テストは移動後も通過しなければならない 🔵 *既存実装より*

---

### `theme/chart_colors.rs`（チャート固有色）

- REQ-031: `theme/chart_colors.rs` は `pareto_2d.rs` の COLOR_PARETO / COLOR_NON_PARETO / DIM バリアントを統合しなければならない 🔵 *既存実装より*
- REQ-032: `theme/chart_colors.rs` は `slice_chart.rs` の COLOR_PARETO / COLOR_NON_PARETO を統合しなければならない 🔵 *既存実装より*
- REQ-033: `theme/chart_colors.rs` は `mcdm_scatter_chart.rs` の COLOR_RED / COLOR_ORANGE / COLOR_YELLOW / COLOR_GRAY を統合しなければならない 🔵 *ユーザヒアリングより*
- REQ-034: `theme/chart_colors.rs` はウィジェット固有のチャート色で意味的に重複するもの（Pareto赤/青、バー青等）を統一名称で定義しなければならない 🟡 *複数ウィジェットの重複から妥当な推測*
- REQ-035: `theme/chart_colors.rs` は `optimization_history.rs` の試行線色（青/赤/緑/金）を定数として定義しなければならない 🟡 *既存実装から妥当な推測*

---

### インポートルール

- REQ-041: ウィジェット・UIファイルは色定数を `crate::theme` 経由でインポートしなければならない 🔵 *ユーザヒアリングより*
- REQ-042: 当 `Color32::from_rgb(...)` のインラインリテラルによる色定数定義をウィジェット・UIファイルに残してはならない 🔵 *ユーザヒアリングより*
  - 例外: 動的に計算される色（アルファ値に変数を使う場合等）は対象外とする
- REQ-043: `egui` 組み込み定数（`Color32::TRANSPARENT`・`Color32::WHITE`・`Color32::BLACK`・`Color32::GRAY` 等）は theme 化を任意とする 🟡 *実装難易度から妥当な判断*

---

### 後方互換性

- REQ-051: 移行後のアプリケーション外観は移行前と同一でなければならない 🔵 *ユーザヒアリングより（「色の数値は変えない」）*
- REQ-052: 移行後の `crate::render::colormap` は `crate::theme::colormap` の re-export として互換性を維持するか、すべての呼び出し元を更新しなければならない 🟡 *Rustモジュール設計から妥当な推測*

---

## 非機能要件

### コンパイル

- NFR-001: `cargo build` が警告ゼロで成功しなければならない 🔵 *Rustプロジェクト標準から*
- NFR-002: `cargo clippy` がエラーを出力してはならない 🟡 *プロジェクト慣習から妥当な推測*

### テスト

- NFR-011: `cargo test` が既存テストをすべて通過しなければならない 🔵 *既存テスト保護のため*

### 保守性

- NFR-021: 色のデザイン変更は `egui-app/src/theme/` 配下のみを編集することで完結しなければならない 🔵 *ユーザヒアリングより（要件の核心）*
- NFR-022: 新規チャートウィジェット追加時のカラー追加先は `theme/chart_colors.rs` 一択であることがコード上から明確でなければならない 🟡 *設計原則から妥当な推測*

---

## Edgeケース

### 依存関係

- EDGE-001: `normalize_trial` / `compute_chart_colors` は `state::app_state` 型を参照するため `theme/colormap.rs` に移動した際に循環依存が発生しないよう確認が必要 🔵 *Rustモジュール構造の分析より*
- EDGE-002: `state/app_state.rs` の `Color32::RED` 初期値はセマンティックな定数（`theme::ERROR_COLOR` 等）に置き換えるか、またはそのままにするか判断が必要 🟡 *コード分析から妥当な推測*

### 色の重複

- EDGE-011: `pareto_2d.rs` と `slice_chart.rs` は両方 `COLOR_PARETO`（赤: 220,50,50）と `COLOR_NON_PARETO`（青系）を定義しており、`chart_colors.rs` への統合時に同一定数を共有させる 🔵 *コード分析より*
- EDGE-012: `importance_chart.rs` のフィット品質色（赤/黄/緑）は `mcdm_scatter_chart.rs` のスコア色と意味が異なるため、別名の定数として定義する 🟡 *コード分析から妥当な推測*
