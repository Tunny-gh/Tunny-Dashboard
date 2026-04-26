# 要件定義: TASK-2110 — AppState 8フィールド追加

## 機能概要

`egui-app/src/state/app_state.rs` の `AppState` 構造体に、
新機能（Trade-off Navigator / Multi-study 比較 / Artifacts / 収束診断）に
必要な 8 つのフィールドを追加する。

## 機能要件

### REQ-001: Trade-off 重みフィールド

- `AppState.tradeoff_weights: Vec<f64>` を追加する
- デフォルト値: 空 Vec (`Vec::new()`)
- `AppState.tradeoff_sorted_indices: Option<Vec<u32>>` を追加する
- デフォルト値: `None`
- `clear()` 時は `tradeoff_weights.clear()` + `tradeoff_sorted_indices = None`

### REQ-006: Multi-study 比較フィールド

- `AppState.comparison_mode: bool` を追加する
- デフォルト値: `false`
- `AppState.comparison_studies: Vec<StudyContext>` を追加する
- デフォルト値: 空 Vec (`Vec::new()`)
- `AppState.comparison_colors: Vec<egui::Color32>` を追加する
- デフォルト値: 空 Vec (`Vec::new()`)
- `clear()` 時は comparison_mode/studies/colors は **リセットしない**（比較セッションは維持）

### REQ-007: Artifacts フィールド

- `AppState.artifacts_dir: Option<std::path::PathBuf>` を追加する
- デフォルト値: `None`
- `AppState.artifact_map: HashMap<u32, Vec<std::path::PathBuf>>` を追加する
- デフォルト値: 空 HashMap
- `clear()` 時は `artifacts_dir = None` + `artifact_map.clear()`

### REQ-008: 収束診断フィールド

- `AppState.best_trial_history: Option<Vec<(u32, f64)>>` を追加する
- デフォルト値: `None`
- `clear()` 時は `best_trial_history = None`

## 非機能要件

- `AppState::new()` の初期化が全フィールドを含む
- `AppState::clear()` が新フィールドを適切にリセット
- `cargo check` エラーなし
