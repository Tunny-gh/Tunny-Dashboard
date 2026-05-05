# cluster-widget-chart-not-displayed verification note

Date: 2026-05-05

## Implemented scope

- Added manual run controls in ClusterScatter header (k, k-mode, target space, init strategy).
- Added pending compute request model and runtime status slots.
- Wired async clustering compute in chart_registry via spawn_task.
- Added cluster-specific error type with debug/release detail policy.
- Added result-length guard and StudySelected reset path in message handler.

## Checks run

- cargo check (package: tunny-desktop): PASS
- Focused tests:
  - validate_cluster_request_rejects_manual_k_too_small: PASS
  - clustering_done_updates_state_when_lengths_match: PASS
  - clustering_done_rejects_mismatched_label_length: PASS
  - study_selected_resets_cluster_widget_runtime_state: PASS

## Performance note

- No runtime benchmark harness was added in this task.
- Current implementation keeps heavy work off the UI thread and avoids duplicate runs while computing.
- End-to-end "Run click to chart redraw" timing still needs interactive measurement in real dataset sessions.

## Regression note

- Changes are isolated to cluster widget rendering, cluster message handling, and chart registry dispatch.
- Existing chart types keep the same flow and compile successfully after the change.
