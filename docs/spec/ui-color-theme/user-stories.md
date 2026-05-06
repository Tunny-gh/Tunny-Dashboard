# UIカラー設定一元化 ユーザストーリー

**作成日**: 2026-05-07
**関連要件定義**: [requirements.md](requirements.md)
**ヒアリング記録**: [interview-record.md](interview-record.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: ユーザヒアリング・既存実装を参考にした確実なストーリー
- 🟡 **黄信号**: 既存実装・設計から妥当な推測によるストーリー
- 🔴 **赤信号**: ヒアリング・実装にない推測によるストーリー

---

## エピック1: テーマカラーの集約

### ストーリー 1.1: UIテーマ色の移行 🔵

**信頼性**: 🔵 *ユーザヒアリング・既存 theme.rs より*

**私は** 開発者 **として**
**既存の `theme.rs` を `theme/mod.rs` に変換したい**
**そうすることで** 以降の色定義を theme ディレクトリに統一できる

**関連要件**: REQ-003, REQ-011, REQ-012

**詳細シナリオ**:
1. `egui-app/src/theme/` ディレクトリを作成する
2. `theme.rs` の内容を `theme/mod.rs` にコピーする
3. `theme.rs` を削除する
4. `crate::theme` モジュールパスが変わらないことを確認する（`mod.rs` により自動解決）

**前提条件**:
- `egui-app/src/theme.rs` が存在する

**制約事項**:
- 既存の定数名・型・値を変更しない

**優先度**: Must Have

---

### ストーリー 1.2: カラーマップの theme 移動 🔵

**信頼性**: 🔵 *ユーザヒアリング・既存 colormap.rs より*

**私は** 開発者 **として**
**`render/colormap.rs` を `theme/colormap.rs` に移動したい**
**そうすることで** グラデーション色もテーマ管理の傘下に入る

**関連要件**: REQ-004, REQ-021〜025

**詳細シナリオ**:
1. `egui-app/src/theme/colormap.rs` を新規作成し `render/colormap.rs` の内容を移す
2. `render/mod.rs` から `colormap` モジュール宣言を削除または re-export に変更する
3. `colormap` を参照している全ファイルのインポートパスを更新する
4. `cargo test` で既存テストが通ることを確認する

**前提条件**:
- `theme/mod.rs` が存在する（ストーリー 1.1 完了後）

**制約事項**:
- `normalize_trial` / `compute_chart_colors` の `state::app_state` 依存で循環依存が発生しないことを確認する

**優先度**: Must Have

---

### ストーリー 1.3: チャート固有色の集約 🔵

**信頼性**: 🔵 *ユーザヒアリング・複数ウィジェットの色重複分析より*

**私は** 開発者 **として**
**各ウィジェットに散在しているチャート色定数を `theme/chart_colors.rs` に集約したい**
**そうすることで** Pareto 色等の重複定義が解消され、変更が 1 箇所で完結する

**関連要件**: REQ-005, REQ-031〜035

**詳細シナリオ**:
1. `theme/chart_colors.rs` を新規作成する
2. 下記の定数をウィジェットから移植する:
   - `COLOR_PARETO`（赤: 220,50,50）—  `pareto_2d.rs` / `slice_chart.rs` で重複定義
   - `COLOR_NON_PARETO`（青: 50,150,250）— 同上
   - `COLOR_PARETO_DIM` / `COLOR_NON_PARETO_DIM`（アルファ付き）
   - `COLOR_MCDM_SCORE_HIGH`・`COLOR_MCDM_SCORE_MID`・`COLOR_MCDM_SCORE_LOW`・`COLOR_MCDM_SCORE_NONE`（`mcdm_scatter_chart.rs`）
   - `COLOR_BAR_PRIMARY`（バー青: 0x0c6ac0 — `mcdm_chart.rs` / `importance_chart.rs` 等で使用）
   - `COLOR_OPT_HISTORY_*`（`optimization_history.rs` の青/赤/緑/金）
   - その他意味が重複する色定数
3. 元のウィジェットファイルから定数定義を削除し `use crate::theme::chart_colors::*` または具体名でインポートする

**前提条件**:
- `theme/mod.rs` が存在する

**優先度**: Must Have

---

## エピック2: ウィジェットからのインラインカラー除去

### ストーリー 2.1: インライン色リテラルの置換 🔵

**信頼性**: 🔵 *ユーザヒアリング（REQ-042）より*

**私は** 開発者 **として**
**ウィジェット・UIファイル内の `Color32::from_rgb(...)` インラインリテラルを theme 定数に置き換えたい**
**そうすることで** 全色が theme ファイルで一元管理される**

**関連要件**: REQ-041, REQ-042

**詳細シナリオ**:
1. 対象ファイルをリストアップ（`grep -r "Color32::from_rgb" egui-app/src --include="*.rs"` で特定）
2. 各インラインカラーの意味を確認し、対応する theme 定数名を決定する
3. `theme/chart_colors.rs` または `theme/mod.rs` に定数を追加する
4. ウィジェットファイルのインラインリテラルを定数参照に置き換える
5. 動的に計算される色（アルファ値に変数を使う等）は置換対象外とする

**前提条件**:
- `theme/chart_colors.rs` が存在する（ストーリー 1.3 完了後）

**制約事項**:
- `egui` 組み込み定数（`Color32::TRANSPARENT` 等）は任意対応

**優先度**: Must Have

---

### ストーリー 2.2: ERROR_COLOR セマンティック定数の導入 🟡

**信頼性**: 🟡 *複数ウィジェットでの `Color32::RED` 使用から妥当な推測*

**私は** 開発者 **として**
**エラー表示に使われる `Color32::RED` を `theme::ERROR_COLOR` として定義したい**
**そうすることで** エラー色の変更が theme 一箇所で完結する**

**関連要件**: REQ-013

**詳細シナリオ**:
1. `theme/mod.rs` に `pub const ERROR_COLOR: Color32 = Color32::from_rgb(220, 50, 50);` 等を追加する
2. `toolbar.rs`・`cluster_scatter.rs`・`mcdm_scatter_chart.rs` 等の `Color32::RED` 参照を `crate::theme::ERROR_COLOR` に置き換える

**優先度**: Should Have

---

## エピック3: 品質保証

### ストーリー 3.1: ビルド・テスト通過の確認 🔵

**信頼性**: 🔵 *プロジェクト品質基準より*

**私は** 開発者 **として**
**移行後も `cargo build` と `cargo test` が通ることを確認したい**
**そうすることで** リグレッションなく色の一元化が完了する**

**関連要件**: NFR-001, NFR-011, REQ-025

**詳細シナリオ**:
1. 各ステップ（ストーリー 1.1〜2.2）の後に `cargo check` で確認する
2. 全移行完了後に `cargo build` で警告ゼロを確認する
3. `cargo test` で全テスト通過を確認する
4. アプリを実行して外観が変わっていないことを視覚確認する

**優先度**: Must Have

---

## ストーリーマップ

```
エピック1: テーマカラーの集約
├── ストーリー 1.1 UIテーマ色の移行 (🔵 Must Have)
├── ストーリー 1.2 カラーマップの theme 移動 (🔵 Must Have)
└── ストーリー 1.3 チャート固有色の集約 (🔵 Must Have)

エピック2: ウィジェットからのインラインカラー除去
├── ストーリー 2.1 インライン色リテラルの置換 (🔵 Must Have)
└── ストーリー 2.2 ERROR_COLOR セマンティック定数の導入 (🟡 Should Have)

エピック3: 品質保証
└── ストーリー 3.1 ビルド・テスト通過の確認 (🔵 Must Have)
```

## 信頼性レベルサマリー

- 🔵 青信号: 5件 (83%)
- 🟡 黄信号: 1件 (17%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質
