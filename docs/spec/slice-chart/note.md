# slice-chart コンテキストノート

## プロジェクト概要

Tunny Dashboard — Optuna 最適化結果をインタラクティブに可視化するデスクトップアプリ（egui-app）。

## 技術スタック

| 項目 | 内容 |
|------|------|
| 言語 | Rust 2021 |
| UI ライブラリ | egui 0.30 / egui_plot 0.30 |
| パッケージ名 | `tunny-desktop` (`egui-app/`) |
| エントリポイント | `egui-app/src/main.rs` |
| アーキテクチャ | IO → AppMessage (mpsc) → MessageHandler → AppState → UI |

## データ構造

```rust
// egui-app/src/state/types.rs
pub struct TrialRow {
    pub trial_id: u32,
    pub trial_number: u32,
    pub params: HashMap<String, f64>,
    pub objectives: Vec<f64>,
    pub pareto_rank: u32,   // 0 = Pareto-optimal
    pub cluster_id: Option<i32>,
    pub state: TrialState,
    pub user_attrs: HashMap<String, String>,
}

pub struct StudyContext {
    pub meta: StudyMeta,
    pub trial_rows: Vec<TrialRow>,
    pub pareto_indices: Vec<u32>,
}
```

## 既存ウィジェットのパターン

- `egui-app/src/ui/widgets/scatter_matrix.rs` — 構造体 + `show()` メソッド
- `egui-app/src/ui/widgets/optimization_history.rs` — HistoryMode 列挙型 + `show()` / `show_with_history()`
- `egui-app/src/ui/widgets/parallel_coords.rs` — 軸可視性 HashMap + `show()`
- 共通テーマ定数: `egui-app/src/theme.rs`（`TOOLBAR_TEXT`, `ACCENT_BLUE` 等）

## 関連設計文書

- `docs/spec/chart-wiring-continuation-requirements.md` — REQ-C05: slice チャート配線定義
- `docs/spec/chart-catalog-requirements.md` — チャートカタログ要件
- `docs/design/fast-rendering-downsampling/` — ダウンサンプリング設計

## 注意事項

- ComboBox は `egui::ComboBox::from_id_salt("unique_id")` を使うこと（重複 ID 禁止）
- egui 0.30 では ComboBox 背景は `weak_bg_fill` を設定すること
- テストは `#[cfg(test)]` ブロック内でロジック関数のみをテスト（`egui::Ui` を使わない）
