# テストケース: TASK-2110 — AppState 8フィールド追加

## テストケース一覧

### TC-001: デフォルト値確認（tradeoff_weights）

- **Given**: `AppState::new()` を呼ぶ
- **When**: `app_state.tradeoff_weights` にアクセスする
- **Then**: 空 Vec が返る

### TC-002: デフォルト値確認（tradeoff_sorted_indices）

- **Given**: `AppState::new()` を呼ぶ
- **When**: `app_state.tradeoff_sorted_indices` にアクセスする
- **Then**: `None` が返る

### TC-003: デフォルト値確認（comparison_mode）

- **Given**: `AppState::new()` を呼ぶ
- **When**: `app_state.comparison_mode` にアクセスする
- **Then**: `false` が返る

### TC-004: デフォルト値確認（comparison_studies）

- **Given**: `AppState::new()` を呼ぶ
- **When**: `app_state.comparison_studies` にアクセスする
- **Then**: 空 Vec が返る

### TC-005: デフォルト値確認（comparison_colors）

- **Given**: `AppState::new()` を呼ぶ
- **When**: `app_state.comparison_colors_proprietary` にアクセスする
- **Then**: 空 Vec が返る

### TC-006: デフォルト値確認（artifacts_dir）

- **Given**: `AppState::new()` を呼ぶ
- **When**: `app_state.artifacts_dir` にアクセスする
- **Then**: `None` が返る

### TC-007: デフォルト値確認（artifact_map）

- **Given**: `AppState::new()` を呼ぶ
- **When**: `app_state.artifact_map` にアクセスする
- **Then**: 空 HashMap が返る（`is_empty() == true`）

### TC-008: デフォルト値確認（best_trial_history）

- **Given**: `AppState::new()` を呼ぶ
- **When**: `app_state.best_trial_history` にアクセスする
- **Then**: `None` が返る

### TC-009: clear() でリセットされる（artifacts 関連）

- **Given**: `artifact_map` にエントリを追加後 `clear()` を呼ぶ
- **When**: `artifact_map` にアクセスする
- **Then**: `is_empty() == true`

### TC-010: clear() で best_trial_history がリセットされる

- **Given**: `best_trial_history = Some(vec![(0, 1.0)])` 後 `clear()` を呼ぶ
- **When**: `best_trial_history` にアクセスする
- **Then**: `None`

### TC-011: フィールド書き込みが可能

- **Given**: `AppState::new()` を呼ぶ
- **When**: `tradeoff_weights = vec![0.5, 0.5]` を設定する
- **Then**: `tradeoff_weights` が `[0.5, 0.5]` を返す
