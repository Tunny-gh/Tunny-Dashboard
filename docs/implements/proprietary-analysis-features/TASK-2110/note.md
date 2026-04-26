# タスクノート: TASK-2110 — AppState 8フィールド追加

## 技術スタック

- **言語**: Rust 2021
- **GUI フレームワーク**: egui 0.30 / eframe 0.30
- **シリアライズ**: serde 1 + serde_json 1
- **ファイルダイアログ**: rfd 0.15
- **テストフレームワーク**: Rust built-in `#[cfg(test)]`

## 実装対象ファイル

- `egui-app/src/state/app_state.rs` — AppState 構造体にフィールド追加

## 追加フィールド

| フィールド名 | 型 | 関連 REQ |
|---|---|---|
| `tradeoff_weights` | `Vec<f64>` | REQ-001 |
| `tradeoff_sorted_indices` | `Option<Vec<u32>>` | REQ-001 |
| `comparison_mode` | `bool` | REQ-006 |
| `comparison_studies` | `Vec<StudyContext>` | REQ-006 |
| `comparison_colors` | `Vec<egui::Color32>` | REQ-006 |
| `artifacts_dir` | `Option<std::path::PathBuf>` | REQ-007 |
| `artifact_map` | `HashMap<u32, Vec<std::path::PathBuf>>` | REQ-007 |
| `best_trial_history` | `Option<Vec<(u32, f64)>>` | REQ-008 |

## 既存コードのコンテキスト

- `StudyContext` は `egui-app/src/state/types.rs` で定義済み
- `AppState::clear()` に新フィールドのリセット処理も追加が必要
- `egui::Color32` は `egui` クレートに依存

## テスト方針

- `#[cfg(test)]` ブロック内に単体テストを追加
- 各フィールドのデフォルト値確認テスト
- `AppState::clear()` でリセットされることの確認
- `cargo check` で型チェックを確認

## 関連ファイル

- `docs/tasks/proprietary-analysis-features/TASK-2110.md`
- `docs/spec/proprietary-analysis-features/`
- `docs/design/proprietary-analysis-features/`
