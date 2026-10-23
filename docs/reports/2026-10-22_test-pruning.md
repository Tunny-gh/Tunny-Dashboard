# Test pruning — complete audit

Started: 2026-10-22. Completed: 2026-10-24.

**Complete: every original test body has an individual decision, without sampling.**
The decision tables now account for **2016 original Rust test definitions**:
2004 from the 2004-definition Windows baseline, plus 11 Unix-only runner tests
and one POSIX-only GUI artifact URI test.
All **209 test-bearing Rust files** are accounted for. Source/cfg/macro inventory,
other-language fixture/probe inventory, all 22 original examples and four benchmark
files, and executable documentation-fence discovery are reconciled below. No test
body remains pending. Ignored/platform/opt-in execution limits are reported
separately from body-review completion. Earlier partial checkpoints and continuation
notes are retained as history; this summary and the final census/validation section
supersede their pending-work statements. No commit was made.

## Criterion and evidence

For each reviewed test: what realistic regression would deleting it miss?
Delete means no independent regression guarantee; Merge means assertions move into
an existing test; Keep identifies a concrete retained guarantee. Tests are not
added to restore counts. Production behavior and public APIs are unchanged.

Background read: AGENTS.md, CONTRIBUTING.md, CONTEXT.md, the ADR index,
ADR-0018/0020/0021/0024, the D8 final-boundaries report, Python cross-validation
overview and roadmap. Test bodies/helpers and relevant implementation paths were
read for each recorded module. There are no additional AGENTS.md files found by
`**/AGENTS.md`.

`cargo test --workspace --locked --profile ci-test -- --list` successfully listed
the current Windows/default-feature harnesses before edits. This is an execution
inventory, **not proof that all test source was read**, nor an inventory of
platform-excluded code. `git ls-files` identified Python fixture producers and
Python/JavaScript probes; the three fixture producers and two issue258 probes
recorded below were read. The attempted
`rg` source inventory could not run because ripgrep is not installed.

| Harness | Baseline registered cases | Final registered cases |
| --- | ---: | ---: |
| analysis unit | 704 | 534 |
| I/O unit | 114 | 103 |
| MCP unit / integration | 16 / 6 | 15 / 6 |
| optimization-runner unit | 102 | 97 |
| Study unit / management | 2 / 9 | 2 / 9 |
| GUI lib unit | 862 | 483 |
| GUI bin unit (same source compiled again) | 862 | 483 |
| GUI csv_projection | 11 | 4 |
| GUI d3_csv_attribute_boundary | 1 | 1 |
| GUI d3_io | 8 | 8 |
| GUI d4_analysis | 8 | 8 |
| GUI d4_preparation | 11 | 11 |
| GUI dataframe | 33 | 23 |
| GUI entry_dependencies | 1 | 1 |
| GUI export_projection | 13 | 6 |
| GUI issue258_terminal_attributes | 6 | 6 |
| GUI journal_projection | 64 | 49 |
| GUI rdb_integration | 3 | 2 |
| GUI sqlite_projection | 21 | 17 |
| GUI study_slice | 9 | 7 |
| **Total registered cases, including ignored** | **2866** | **1875** |

Five doc-test harnesses report zero cases. The harness-free native Animation check
is separate from these totals. Removing the duplicated GUI unit suite gives
2004 baseline and 1392 current distinct registered test definitions on this
target. Source audit additionally found 11 Unix-only runner definitions and one
POSIX-only GUI definition, giving a **2016-definition complete source census**.
Of these, **2016 reviewed, 0 baseline definitions still pending**. Reviewed decisions:
**1404 Keep, 276 Delete, 336 Merge**. Removing 612 definitions removes 991 registered
cases because 379 removed GUI tests were compiled in both entry targets.
Counts reconcile with the original source inventory, individual decision rows and
final workspace execution. Manual/native/probe entries are counted separately.

## Individual decisions

Names are qualified by the package, module prefix and function in each section.
Each row denotes exactly one original definition, even for GUI definitions
executed by both lib and bin harnesses. Merge destinations use the same prefix
unless explicitly qualified otherwise. A surviving destination is recorded Keep.

### Final file and package reconciliation

| Package / target | Original definitions identified | Original definitions individually reviewed | Still pending |
| --- | ---: | ---: | ---: |
| analysis | 704 | 704 | 0 |
| I/O | 114 | 114 | 0 |
| optimization-runner (Windows + Unix-only) | 113 | 113 | 0 |
| Study (unit + management) | 11 | 11 | 0 |
| MCP (unit + integration) | 22 | 22 | 0 |
| GUI unit (lib/bin source counted once, Windows + POSIX-only) | 863 | 863 | 0 |
| GUI integration libtest harnesses | 189 | 189 | 0 |
| **Complete source census** | **2016** | **2016** | **0** |

The recorded modules cover all originals in **209 test-bearing Rust source files**:
analysis 59, I/O 13, runner 14, Study 3, MCP 3, GUI 117. File identity follows the
module-qualified sections; split files are stated explicitly (e.g. runner/tests,
snapshot_tests). The final independent source census confirms these are all
test-bearing files in the repository. PCA implementation and fixture-only helper
files were read in addition, not counted as test-bearing. Three Python fixture
producers and two upstream diagnostic probes were read; they are recorded
separately from Rust definition counts.

All package originals are individually recorded. Final reconciliation uses the
uncapped tracked-source inventory and Cargo target metadata, not capped grep
results. Historical partial checkpoints follow the decision tables.

### `tdashboard-analysis::results::tests::` (31 originals)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| convert_ref_point_is_self_inverse_per_component | Keep | Exact mixed-direction sign conversion and both public wrappers; round trip alone would miss cancelling sign errors. |
| convert_ref_point_defaults_missing_dirs_to_minimize | Keep | Missing direction metadata must not flip values or truncate the reference point. |
| mcdm_method_label | Delete | TOPSIS label already checked through the wrapped result's production method_label. |
| mcdm_method_all | Keep | Missing/duplicated/reordered selector methods would hide or misorder UI choices. |
| mcdm_result_topsis_primary_scores | Delete | Exact same payload/accessor assertion exists in mcdm_result_wrap_topsis_integration. |
| mcdm_result_topsis_ranked_indices | Delete | Same permutation and assertion exist in mcdm_result_wrap_topsis_integration. |
| mcdm_result_topsis_method_label | Delete | Same label assertion exists in mcdm_result_wrap_topsis_integration. |
| topsis_result_all_fields | Delete | Only reads fields just assigned by the test; Rust field storage, not production behavior. |
| mcdm_result_wrap_topsis_integration | Keep | Retains TOPSIS score/order/duration/method dispatch in one fixture. |
| mcdm_method_vikor_label | Delete | VIKOR label retained through the result accessor test. |
| mcdm_method_all_includes_vikor | Delete | Exact complete list already asserted by mcdm_method_all. |
| mcdm_result_vikor_primary_scores | Keep | Must dispatch display_scores, not q_values; now also holds VIKOR order/duration/method assertions. |
| mcdm_result_vikor_ranked_indices | Merge | Into mcdm_result_vikor_primary_scores; retains order dispatch without rebuilding the same fixture. |
| mcdm_result_vikor_duration_ms | Merge | Into mcdm_result_vikor_primary_scores; retains duration dispatch. |
| mcdm_result_vikor_method_label | Merge | Into mcdm_result_vikor_primary_scores; retains VIKOR label dispatch. |
| mcdm_result_vikor_method | Merge | Into mcdm_result_vikor_primary_scores; retains enum dispatch. |
| weight_mode_labels | Keep | Distinguishes Manual and Entropy UI labels. |
| weight_mode_all | Keep | Both modes must remain selectable in the intended order. |
| entropy_result_fields | Delete | Reads only the literal fields constructed by the test; no entropy computation or adapter invoked. |
| tc_pr_011_01_promethee_i_label | Delete | Same label is checked through the retained PROMETHEE I result test. |
| tc_pr_011_02_promethee_ii_label | Delete | Same label is checked through the retained PROMETHEE II result test. |
| tc_pr_011_03_all_has_four_elements | Delete | Weaker subset/count assertions duplicate the exact mcdm_method_all list. |
| mcdm_result_promethee_i_primary_scores | Keep | Distinguishes positive flow from net flow; consolidated I dispatch assertions. |
| mcdm_result_promethee_ii_primary_scores | Keep | Distinguishes net flow from positive flow; consolidated II dispatch assertions. Fixtures now use different I/II orders so wrong-branch dispatch is detectable. |
| mcdm_result_promethee_i_ranked_indices | Merge | Into mcdm_result_promethee_i_primary_scores; retains I order dispatch. |
| mcdm_result_promethee_ii_ranked_indices | Merge | Into mcdm_result_promethee_ii_primary_scores; retains II order dispatch with a distinct permutation. |
| mcdm_result_promethee_duration_ms | Merge | Into mcdm_result_promethee_i_primary_scores (also checked for II); retains duration dispatch. |
| mcdm_result_promethee_i_method_label | Merge | Into mcdm_result_promethee_i_primary_scores; retains I label. |
| mcdm_result_promethee_ii_method_label | Merge | Into mcdm_result_promethee_ii_primary_scores; retains II label. |
| mcdm_result_promethee_i_method | Merge | Into mcdm_result_promethee_i_primary_scores; retains I enum. |
| mcdm_result_promethee_ii_method | Merge | Into mcdm_result_promethee_ii_primary_scores; retains II enum. |

### `tdashboard-analysis::mcdm::tests::` (8 originals)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| normalize_weights_empty | Keep | Empty criteria must return an empty result without division/indexing failure. |
| normalize_weights_equal | Keep | Valid already-normalized weights must remain unchanged. |
| normalize_weights_divides_by_sum | Keep | Unequal raw weights must be normalized, not replaced by uniform weights. |
| normalize_weights_zero_sum_falls_back_to_uniform | Keep | Zero-sum criteria require the uniform fallback. |
| normalize_weights_rejects_negatives_before_sum_fallback | Keep | Negative criteria must fail even when NaN/infinite totals would trigger fallback. |
| normalize_weights_nan_falls_back_to_uniform | Keep | Nonfinite/overflow totals and signed zero must preserve the existing normalization policy. |
| rankings_reject_negative_weights_even_without_valid_rows | Keep | Each public ranking entry must reject invalid weights before early-return paths. |
| rankings_preserve_nonnegative_normalization_and_bounds | Keep | Ranking entry wiring must normalize raw weights without changing rankings or producing out-of-bounds scores. |

### `tunny-dashboard::{lib,bin}::ui::widgets::decision::mcdm_chart::tests::` (33 originals)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| adopt_compute_state_syncs_runtime_and_preserves_ui_settings | Keep | Completed work must clear busy state without overwriting each widget's controls. |
| rank_chart_all_paths_keep_journal_gaps_and_study_local_numbers | Keep | Actually painted labels across four methods must use Study numbers while keeping global IDs/scores intact. |
| rank_chart_all_paths_missing_number_falls_back_to_original_row | Keep | Missing numbering must fall back to original row, not rank or truncated position. |
| rank_chart_all_paths_ordinary_single_study_is_unchanged | Keep | Ordinary nonmissing numbering goes through a different path from the fallback fixture. |
| mcdm_top_n_values | Keep | Explicit selector-to-limit mapping is used by chart/table truncation. |
| normalize_weights_equal | Delete | Exact duplicate of analysis::mcdm::tests::normalize_weights_equal through a direct delegating wrapper. |
| normalize_weights_unequal | Delete | Exact duplicate of analysis::mcdm::tests::normalize_weights_divides_by_sum through that wrapper. |
| normalize_weights_three_equal | Keep | Three-element fixture catches hard-coded two-weight fallback/scaling; no equivalent three-weight fixture in the reviewed normalization tests. |
| normalize_weights_zero_fallback | Delete | Exact duplicate of analysis::mcdm::tests::normalize_weights_zero_sum_falls_back_to_uniform. |
| normalize_weights_empty | Delete | Exact duplicate of analysis::mcdm::tests::normalize_weights_empty. |
| controls_run_preserves_negative_weights_for_error_propagation | Keep | Clicks the real Run button; catches UI silently sanitizing invalid weights instead of surfacing errors. |
| negative_weights_cannot_produce_a_cache_key | Keep | Both settings/request key paths must reject invalid criteria for every method. |
| request_and_settings_cache_keys_use_the_same_normalization | Keep | Equivalent normalized requests must select the same result/cache slot. |
| mcdm_rank_chart_default | Keep | Protects actual initial method/weights/busy and Top N policy. |
| mcdm_table_default | Keep | Separately constructed table must start with the intended Top N. |
| enumerate_ranked_top5_with_5_results | Delete | Equal-size truncation and correct scores are exercised by top_n_toggle_updates_display and enumerate_ranked_scores_match_ranked_order. |
| enumerate_ranked_top10_with_20_results | Delete | Same truncation branch as build_ranking_rows_top_n_limit; only limit/data size differ. |
| enumerate_ranked_top5_with_3_results_min_applied | Delete | Oversized-limit behavior is exercised with scores by enumerate_ranked_scores_match_ranked_order. |
| enumerate_ranked_scores_match_ranked_order | Keep | Nonidentity ranking must select corresponding scores; now also checks one-based ranks for all rows. |
| enumerate_ranked_empty_result | Delete | Identical call/fixture/assertion to build_ranking_rows_empty. |
| top_n_toggle_cycle | Delete | Direct field assignment plus enum value lookup, not a control interaction; defaults/value mapping covered separately. |
| build_ranking_rows_basic | Delete | Weaker identity-order case; score/order and actual trial-number mapping covered by retained ranking tests. |
| build_ranking_rows_top_n_limit | Keep | A limit below population size must truncate the returned table. |
| build_ranking_rows_rank_starts_at_1 | Merge | Into enumerate_ranked_scores_match_ranked_order; retains one-based rank and sequence expectations. |
| build_ranking_rows_distinguishes_trial_id_and_number | Keep | Pins global trial identity separately from displayed Study-local number with nonidentity rank order. |
| build_ranking_rows_empty | Keep | No alternatives must produce an empty table, not an indexed dummy row or panic. |
| build_ranking_rows_objectives_included | Keep | Table must include actual objective values, not only ranks/scores. |
| topsis_full_pipeline_equal_weights | Keep | Real numerical result must compose into descending table scores and valid one-based rank. |
| topsis_weight_bias_changes_ranking | Delete | Subsequent complete TOPSIS audit found stronger exact expected-winner checks in analysis::mcdm::topsis::tests::tc_1615_03_weights_affect_ranking; this GUI-located test invokes only the numerical function and direct normalization wrapper, no UI. |
| topsis_single_objective_works | Delete | analysis::mcdm::topsis::tests::tc_1615_05_single_objective checks the same numerical path with exact winner/loser; this GUI-located case only checks success and length. |
| mcdm_chart_run_button_sets_pending_compute | Delete | Does not click a button: assigns pending/busy fields and checks Option::take; real Run interaction remains covered. |
| mcdm_compute_request_vikor_includes_v | Delete | Merely reads the VIKOR literal and v assigned in the same test; no request construction path invoked. |
| top_n_toggle_updates_display | Keep | One computed result can be presented at smaller/equal/oversized limits without refitting. |

### `tunny-dashboard::{lib,bin}::ui::widgets::history::convergence::tests::` (6 originals)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| convergence_chart_default | Keep | Initial auto reference/HV and no queued work are product defaults. |
| pending_ref_point_encodes_auto_and_manual | Delete | Constructs enum/Option literals and matches those literals; only Rust representation is checked. |
| adopt_compute_state_clears_stuck_computing | Keep | Completion must stop the per-widget spinner. |
| convergence_show_uses_index_times_step | Keep | Corrected the existing test to call production to_points rather than duplicate its map; now checks exact x/y points against independent literals. |
| indicator_variants_accessible | Keep | Missing convergence indicator in the custom selector list would hide functionality. |
| indicator_higher_is_better_only_for_hv | Keep | Direction hints must not invert interpretation of improving IGD/epsilon/R2. |

### `tunny-dashboard::{lib,bin}::ui::widgets::importance::importance_chart::tests::` (23 originals)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| categorical_ui_is_explicit_unranked_and_r_squared_is_numerical_only | Keep | Exercises rendered unsupported categories and numerical-only fit labels for mixed/category-only inputs. |
| cache_ids_are_distinct | Keep | Custom IDs can collide and cause reuse of the wrong metric's result; uniqueness is not a Rust enum guarantee. |
| metric_signedness | Keep | Wrong signedness selects an inappropriate diverging/sequential color treatment. |
| metric_expensiveness | Keep | Protects metric-specific execution/admission classification. |
| core_metric_none_only_for_sobol | Keep | Checks supported primitive versus dedicated-path routing, not enum storage. Test name omits the existing ARD exception; no production fix is included. |
| adopt_compute_state_clears_computing_and_preserves_selection | Keep | Completion must clear busy state without changing chosen metric/objective. |
| sorted_importance_descending_order | Keep | Three scores must sort with their parameter names. |
| sorted_importance_uses_abs_value | Keep | Strong negative correlation must rank ahead of weak positive correlation. |
| sorted_importance_invalid_obj_idx_returns_empty | Keep | Unavailable objective must not panic or display another objective's importance. |
| importance_metric_labels_not_empty | Keep | Guards against blank selector labels for the covered metrics. |
| importance_chart_default | Keep | Initial metric/objective/feasibility and no pending computation are observable defaults. |
| sorted_importance_ridge_uses_abs_beta | Keep | Ridge dispatch must read signed beta and sort by magnitude, not Spearman fields. |
| sorted_importance_rf_anova_descending | Keep | RF-ANOVA dispatch must read the per-parameter tree result, not the Spearman fixture. |
| sorted_importance_sobol_returns_empty_from_sensitivity | Keep | Sobol must use its separate result path rather than unrelated sensitivity scores. |
| sorted_importance_mdi_descending | Keep | MDI has a separate optional payload/dispatch arm from RF-ANOVA. |
| importance_metric_mdi_label | Keep | Exact MDI label and non-Sobol routing differ from the nonempty-label checks. |
| sorted_importance_mdi_no_data_returns_empty | Keep | Missing optional MDI result must not reuse another metric's data. |
| sorted_sobol_first_and_total_use_correct_data | Keep | First-order and total-effect selectors must use distinct matrices. |
| test_permutation_cache_id | Delete | Hard-codes private numeric ID 7; arbitrary renumbering is harmless, uniqueness remains tested. |
| test_permutation_is_not_sobol | Keep | Permutation must not be sent down the separate Sobol result path. |
| test_permutation_label | Keep | Exact Permutation label is not covered by the nonempty-label test. |
| sorted_importance_permutation_descending | Keep | Permutation has a separate optional result/dispatch arm. |
| sorted_importance_permutation_no_data_returns_empty | Keep | Missing optional permutation result must not reuse Spearman data. |

## Continuation audit ledger

The tables above describe the first pass. The continuation below preserves its
original baseline identities (including removed definitions). Counts and command
results in the first-pass summary are historical until the final reconciliation.
All table names are qualified by their section prefix, as above.

### `tdashboard-study::manager::tests::` (1 original; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| checked_counter_exhaustion_never_publishes_or_consumes_a_batch | Keep | Exercises manager-owned counter error propagation and atomicity, not merely checked_add; exhaustion must not publish or consume sequence. |

### `tdashboard-study::model::tests::` (1 original; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| source_serial_exhaustion_does_not_wrap_or_reuse_an_identity | Keep | Allocator must not reuse a source identity on counter exhaustion. |

### `tdashboard-study::integration(management)::` (9 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| numeric_attributes_preserve_nonfinite_identity_revisions_and_retained_inputs | Keep | Stable NaN equality prevents endless revisions; numeric infinities, null and text remain distinct after updates. |
| all_states_and_namespaces_are_not_rows_or_trial_numbers | Keep | Manager must retain all states/sparse objective indices and reject cross-source ID collisions. |
| invalid_batches_are_atomic_and_do_not_consume_sequence | Keep | Mixed valid/invalid records, stale tickets/revisions, duplicate orders/numbers and malformed distributions must reject atomically. |
| noop_nan_metadata_and_completion_have_distinct_revision_semantics | Keep | No-op, metadata change and completion must advance different stamps while retained headers stay immutable. |
| all_state_constraint_growth_reclassifies_history_and_matches_replacement | Keep | Running-trial width growth must reclassify completed history without changing row count or old input. |
| reload_failure_handoff_empty_and_superseded_messages | Keep | Retain old display until first acceptance, then retain accepted empty/new generation; reject late messages. |
| retained_roots_and_unchanged_payloads_are_shared_then_released | Keep | Arc checks enforce ADR-0021 payload-sharing/release contract; catches full payload cloning and obsolete-root retention, not arbitrary internal layout. |
| independent_managers_refresh_replacement_and_removal | Keep | Refresh preserves generation and removes omitted records without invalidating another Study; explicit removals work. |
| classifier_preserves_invalid_positions_empty_presence_and_violation_priority | Keep | Direct classifier distinguishes incomplete evidence from violation and genuinely unconstrained data. |

### `tdashboard-mcp::rpc::tests::` (8 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| initialize_handshake | Keep | Server chooses supported protocol rather than blindly echoing client version; initialized notification gets no reply. |
| ping_returns_empty_object | Keep | Ping routing preserves string request IDs and empty result shape, unlike integer-only integration fixtures. |
| tools_list_contains_all_tools | Keep | Advertised tool names/schema objects must be present; stdio integration checks only count. |
| unknown_method_is_error_and_unknown_notification_ignored | Keep | Unsupported request must error while unsupported notification must not produce an unsolicited response. |
| parse_error_returns_null_id | Keep | Invalid JSON requires parse-error response with null ID, not process exit or silence. |
| batch_request_gets_explicit_invalid_request | Keep | Unsupported JSON-RPC batches must fail explicitly instead of leaving clients waiting. |
| unknown_tool_is_invalid_params_error | Keep | Unknown tool differs from a runtime tool error and must use protocol error -32602. |
| tool_execution_error_is_iserror_result | Keep | Missing storage produces a tool-result error, not a JSON-RPC error envelope. |

### `tdashboard-mcp::tools::tests::` (8 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| study_summary_missing_study_id_is_execution_error | Keep | Summary must validate target ID before opening storage and report the missing argument. |
| study_summary_missing_storage_is_execution_error | Keep | Summary must reject absent source with an execution error. |
| trials_missing_study_id_is_execution_error | Keep | Independent trials entry must not silently default to another Study. |
| trials_missing_storage_is_execution_error | Keep | Independent trials entry must reject absent storage. |
| parse_offset_limit_defaults | Keep | Omitted pagination must select offset zero and configured default rather than return no rows. |
| parse_offset_limit_respects_explicit_values | Keep | Explicit pagination must not be replaced by defaults/cap. |
| parse_offset_limit_caps_at_max | Keep | User-supplied huge page must be bounded. |
| parse_offset_limit_exact_cap_boundary_unchanged | Delete | Same min-cap behavior already constrained by above-cap and below-cap tests; exact equal input adds no regression distinction. |

### `tdashboard-mcp::integration(integration)::` (6 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| d7_journal_stdio_preserves_ids_all_state_constraints_and_nonfinite_json | Keep | Real process composition must preserve Study numbering, all-state constraint width, finite candidate filtering, nulls and pagination. |
| d7_sqlite_stdio_preserves_ids_all_state_constraints_and_nonfinite_json | Keep | Same external contract through independent SQLite ingestion; ignored/explicit Python prerequisite, not a duplicate Journal reader. |
| d7_stdio_masks_credentials_in_rejected_storage_requests | Keep | Every tool must mask credentials in fail-closed PostgreSQL/MySQL errors. |
| d3_live_servers_keep_stdio_ingestion_pagination_and_masked_reports | Keep | Success responses from live server backends must also mask report source and honor pagination; ignored external fixtures. |
| categorical_importance_is_explicit_in_mcp_summary_and_report | Keep | Both tools must expose unsupported nominal categories without fabricating numeric importance. |
| full_session_over_stdio | Keep | Covers real process startup/framing, handshake/tool advertisement, unconstrained summary, raw trials and English Markdown escaping. |

`mcp-server/tests/create_sqlite_fixture.py` was read completely: fixture producer,
not a separate assertion test; it creates a reduced schema for the retained
SQLite stdio test. `mcp-server/src/storage.rs` was read as its composition path.

### `tdashboard-optimization-runner::process::definition::tests::` (3 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| json_round_trip_preserves_definition | Keep | Persisted definition must retain nested tagged extractor/input enums and optional command configuration; project serde annotations are not compiler guarantees. |
| validate_accepts_sample_and_rejects_problems | Keep | Reject invalid executable/objective/parameter/template definitions before execution. |
| optional_fields_default_when_absent | Keep | Minimal user-authored JSON must decode with project's defaults and custom tag names. |

### `tdashboard-optimization-runner::process::extract::tests::` (11 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| regex_extracts_first_group | Keep | Adapter must parse captured number rather than whole unit-bearing text. |
| regex_without_group_uses_whole_match | Keep | No-group extractor must fall back to full match. |
| regex_no_match_is_error | Keep | Missing solver result must fail the evaluation rather than fabricate zero. |
| json_path_descends_objects_and_arrays | Keep | Custom dotted traversal must handle both containers and array positions. |
| json_path_missing_key_is_error | Keep | Missing nested solver output must produce extraction error. |
| csv_by_header_and_last_row | Keep | Name resolution and last-row selection must choose final solver step. |
| csv_by_index_without_header | Keep | Headerless indexed data must not lose first row or select wrong cell. |
| csv_row_index_counts_from_first_data_row_with_header | Keep | Header-enabled index origin must match name-based selection. |
| csv_header_column_requires_has_header | Keep | Reject named-column selection without explicit header instead of interpreting data as labels. |
| csv_out_of_range_is_error | Keep | Invalid column must error, not panic/default. |
| ambiguous_or_nonfinite_values_are_errors | Keep | Adapter rejects partial numeric tokens and nonfinite values instead of recording successful corrupt evaluations. |

### `tdashboard-optimization-runner::process::substitute::tests::` (7 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| render_template_substitutes_and_formats | Keep | Name substitution and integral formatting must create solver-readable input. |
| render_template_handles_escaped_braces | Keep | Literal braces must survive custom placeholder parser. |
| render_template_rejects_unknown_and_unclosed | Keep | Invalid template must fail loudly rather than emit incomplete inputs. |
| build_args_expands_per_parameter | Keep | CLI arguments must preserve name/value order and split expansion into intended argv entries. |
| build_env_pairs_names_and_values | Keep | Env input must pair supplied names with formatted values, not transposed/reordered entries. |
| build_json_stdin_uses_integers_for_integral_values | Keep | Custom JSON input builder must retain integer versus fractional representation. |
| count_mismatch_is_an_error | Keep | All three parameter-delivery builders must reject differing arity rather than silently zip/truncate. |

### `tdashboard-optimization-runner::process::evaluator::tests::` (8 originals; complete test module)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| build_problem_aligns_variables_and_counts | Keep | Definition-to-runner bridge must preserve parameter order/ranges and objective/constraint counts and reject missing/empty bounds. |
| evaluates_via_env_and_stdout_regex | Keep | Unix subprocess must receive environment values and return actual captured stdout through extraction. |
| evaluates_via_args_stdout_and_file_constraint | Keep | Unix argv and working-directory/file-output composition differs from pure builders and env mode. |
| template_input_is_written_and_readable_by_command | Keep | Unix template must be written before command execution in the intended directory. |
| nonzero_exit_retries_then_fails | Keep | Unix nonzero child status must propagate; does not prove attempt count, which Windows counterpart checks independently. |
| timeout_kills_a_hanging_command | Keep | Unix child/grandchild pipe inheritance must not defeat timeout by blocking drain-thread joins. |
| post_command_transforms_output_before_extraction | Keep | Unix post-hook must finish before file extraction, not after it. |
| process_optimization_runs_standalone_and_writes_journal | Keep | Unix real command, sampler, problem alignment and Journal readback must agree numerically without Optuna/Python. |

The last seven definitions are `cfg(unix)` and absent from Windows runtime
listing; they were read, not executed on this host.

### `tdashboard-optimization-runner::process::windows_tests::` (4 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| windows_retry_counts_attempts_and_preserves_failure_text | Keep | Real Windows failures must retry exactly three times and retain exit status/stderr. |
| windows_timeout_kills_hanging_command | Keep | Windows child wait path must terminate promptly on timeout. |
| windows_hooks_extraction_and_recorded_readback | Keep | Real pre/main/post execution must precede output extraction and preserve raw parameters/constraints in Journal. |
| windows_hook_failure_and_extraction_failure_are_reported | Keep | Pre-hook and missing-output errors must fail evaluations, not proceed as successes. |

### `tdashboard-optimization-runner::gh::compute_server::tests::` (2 originals; complete module including nested module below)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| classify_url_and_exe_inputs | Keep | Custom trimming/scheme case/trailing-slash rules distinguish remote URL from spaced executable paths. |
| missing_exe_is_reported | Keep | Invalid executable must fail before startup probing with an actionable error. |

### `tdashboard-optimization-runner::gh::compute_server::tests::process_tests::` (4 originals; `cfg(unix)`)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| early_exit_is_detected_before_timeout | Keep | Crashed launch must fail promptly rather than wait the full startup deadline. |
| timeout_kills_child_and_reports | Keep | Unresponsive launched server must return startup-timeout failure. |
| abort_stops_waiting | Keep | Caller cancellation must interrupt readiness polling. |
| ready_when_http_responds | Keep | Production readiness accepts a real HTTP response and returns the correct URL/owned child handle; not merely a fake-server self-test. |

These four definitions and their script-producing helper were read in full;
their process execution is unavailable on the Windows host.

### `tdashboard-optimization-runner::gh::ghx::tests::` (2 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| parses_nested_chunks_and_items | Keep | Project XML event assembly must preserve hierarchy/order/entities while discarding compound-child text. |
| rejects_broken_xml | Keep | Adapter must reject malformed structure and rootless input instead of returning a partial archive. |

### `tdashboard-optimization-runner::gh::compute::tests::` (10 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| request_body_includes_pointer_when_set | Keep | Local-file pointer must be optional while embedded algorithm remains a fallback. |
| gene_pool_input_packs_all_values_into_one_branch | Keep | Flat genes must group into one RH_IN list without shifting later slider values. |
| extracts_outputs_from_response | Keep | Objective extraction must follow requested names, not response order, and accept string/raw numbers. |
| missing_output_is_reported | Keep | Missing required Compute output must fail with its name rather than succeed with missing/zero values. |
| parse_data_attr_uses_declared_type | Keep | Numeric-looking declared strings must retain text identity and double-encoded strings unwrap only once. |
| extract_attr_outputs_tolerates_missing_and_empty | Keep | Auxiliary missing attributes retain positions without failing; malformed envelopes still fail. |
| parses_various_data_encodings | Keep | Custom parser's raw/string/double-encoded/boolean branches are not serde guarantees. |
| evaluate_round_trip_against_fake_server | Keep | Real HTTP request construction and response mapping preserve inputs/objectives/constraints/typed attributes. |
| semaphore_limits_concurrency | Keep | Semaphore is custom production code; permit release/wakeup must cap overlapping work, not a dependency recheck. |
| mocked_compute_run_records_success_and_http_failure | Keep | HTTP status failures must become failed Journal trials while success retains raw values/constraints. |

### `tdashboard-optimization-runner::gh::compute_run::tests::` (3 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| duplicate_rejection_keeps_journal_but_does_not_roll_back_sidecar | Keep | Duplicate validation must not append Journal data; sidecar persistence deliberately precedes validation. |
| preparation_preserves_error_order_and_best_effort_definition_write | Keep | Invalid XML/config must not create Journal; failed sidecar writes must not block prepared remote/cancelled runs. |
| local_launch_failure_preserves_prepared_journal | Keep | Launch failure must leave already prepared Journal intact and surface executable error. |

### `tdashboard-optimization-runner::gh::problem::tests::` (13 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| dedupe_names_produces_unique_names | Keep | Generated suffixes must not collide with literal suffix-bearing names in either order. |
| extracts_variables_and_objectives_from_fixture | Keep | XML wiring must produce correct slider bounds/precision, objective/constraint/attribute source GUIDs and names. |
| extracts_gene_pool_as_one_variable_per_gene | Keep | Gene count, saved values, shared bounds and group slot metadata must align. |
| gene_pool_unparseable_value_keeps_gene_alignment | Keep | Invalid middle gene must fall back in place without shifting subsequent genes. |
| no_attribute_wiring_means_no_constraints_or_attributes | Keep | Ordinary unwired Attributes input is neither constraint evidence nor warning. |
| attribute_component_without_recognized_inputs_warns | Keep | Unrecognized configured attribute wiring must produce a warning, unlike absent wiring. |
| attribute_named_like_constraint_column_is_renamed | Keep | Reserved constraint column collision must rename attribute and notify user. |
| cross_category_duplicate_names_are_uniquified | Keep | Global column namespace must deduplicate objective/attribute names across roles. |
| non_attribute_component_on_attributes_input_is_skipped_with_warning | Keep | Arbitrary components with C/Attr-like inputs must not fabricate constraints. |
| reserved_column_name_detection | Keep | Reserved-name grammar includes feasibility columns but excludes ordinary c-prefixed words. |
| constraint_only_when_attribute_input_is_unwired | Keep | Unwired user-attribute source must not remove a separately wired constraint. |
| error_without_tunny_component | Keep | Missing optimization component must fail extraction rather than use another object. |
| duplicate_names_are_uniquified | Keep | Extraction must actually invoke deduplication on variable names; primitive dedupe test alone would not catch removed wiring. |

### `tdashboard-optimization-runner::runner::tests::` (7 originals; complete test module)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| recording_reuse_is_explicit_compatible_and_starts_a_fresh_invocation | Keep | Generic Random/NSGA-II reuse must reject implicit/incompatible recording, preserve names/IDs/history and start fresh budgets/seed trajectories. |
| random_run_records_all_trials_to_journal | Keep | Generic recorder must retain constrained and nonempty attribute payloads, not only scalar objectives. |
| nsga2_run_completes_and_records | Merge | Into seeds_bounds_rounding_and_method_objective_combinations; preserves expected 8 completions and zero failures on existing NSGA-II fixtures. |
| evaluation_errors_recorded_as_fail | Keep | Generic evaluator errors must consume failed-trial budget without recording completions. |
| seeds_bounds_rounding_and_method_objective_combinations | Keep | Both generic algorithms and objective arities must honor seeds, bounds and decimal/integer precision; now also checks completion budget. |
| penalty_fitness_does_not_replace_recorded_values_or_global_ids | Keep | Penalized optimizer fitness must not overwrite raw recorded objectives or reset global versus Study-local IDs. |
| cancellation_and_journal_failures_stop_new_evaluations | Keep | Before/during cancellation and begin/finish I/O failure must stop both generic sampler loops at the right evaluation boundary. |

### `tdashboard-optimization-runner::gh::runner::tests::` (23 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| recording_reuse_is_explicit_compatible_and_starts_a_fresh_invocation | Keep | Independent GH orchestration must enforce explicit compatible reuse, preserve names/history and fresh seed/budget behavior. |
| random_sampler_records_all_trials | Keep | Actual GH recording must preserve directions, bounds and exact objective/constraint/attribute relationships; receives nonadaptive stop/diagnostic assertions. |
| constrained_penalty_fitness_orders_by_violation | Keep | Feasible/infeasible ranking tiers and total-violation order must remain correct. |
| evaluation_failure_ranks_worse_than_any_infeasible_trial | Keep | Huge violations must cap below failure penalty so optimizer never prefers crashing regions. |
| constraint_arity_mismatch_is_recorded_as_fail | Keep | Wrong evaluator constraint width must fail trials rather than journal misaligned columns. |
| non_finite_constraint_is_recorded_as_fail | Keep | NaN constraint must not be treated feasible by floating max or serialize into a success. |
| empty_attribute_does_not_fail_the_trial | Keep | Optional per-trial absence must not fail valid objective/constraint evaluation. |
| nsga2_sampler_runs_expected_evaluations | Keep | GH NSGA-II budget and progress total/done must include initial population. |
| evaluation_errors_are_recorded_as_fail | Keep | GH solver errors must persist FAIL states, zero COMPLETE rows and correct total count. |
| cancel_before_run_records_nothing | Keep | GH Random cancellation before start must leave no trial records. |
| direction_mismatch_is_rejected | Keep | Problem/config objective arity must be validated even without existing-study reuse. |
| cma_rejects_multi_and_zero_generations_before_creating_journal | Keep | CMA-specific unsupported problem/config must fail before creating Journal. |
| cma_budget_seed_bounds_precision_start_and_constraint_journal | Keep | CMA loop must retain start point, deterministic budget, slider precision and raw recorded objective/constraint/attribute values. |
| cma_minimizes_and_maximizes_real_objective | Keep | CMA direction conversion must steer real evaluation to the correct box endpoint. |
| cma_cancellation_records_only_in_flight_trial | Keep | Cancellation before/at initial CMA evaluation must report correct stop reason and persisted prefix. |
| cma_evaluator_and_invalid_results_record_failures | Keep | CMA adapter must reject solver errors, objective/constraint/attribute arity and nonfinite objectives while retaining FAIL records. |
| cma_journal_open_error_is_returned | Merge | Into cma_journal_write_error_aborts_without_evaluating; preserves directory-as-file open failure without a separate setup. |
| cma_journal_write_error_aborts_without_evaluating | Keep | Journal begin failure must abort CMA before invoking solver; also receives open-failure check. |
| cma_cancel_during_generation_stops_new_trials_and_preserves_journal | Keep | Mid-generation cancellation differs from initial cancellation; later candidates must not be evaluated or recorded. |
| cma_journal_finish_error_aborts_after_in_flight_evaluation | Keep | Finish failure must retain started trial and stop after one solver call, unlike pre-evaluation write failure. |
| bo_bootstrap_journal_begin_error_preserves_existing_records | Keep | Renamed bo_bootstrap_journal_errors_preserve_existing_records; both BO models retain successful/started prefixes and propagate exact I/O error at begin/finish. |
| bo_bootstrap_journal_finish_error_preserves_successful_and_started_trials | Merge | Into bo_bootstrap_journal_errors_preserve_existing_records; same helper now exercises both failure stages in one test. |
| bo_bootstrap_evaluation_failure_still_returns_insufficient_data | Keep | Both BO models must distinguish exhausted failed bootstrap from Journal errors and retain failed trials/progress. |

### `tdashboard-optimization-runner::gh::adaptive::tests::` (10 originals; complete test module)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| both_bo_methods_run_single_and_multi_real_evaluator_loops | Keep | Explicit FITC/VFE and EI/EHVI dispatch plus recording-only reuse must produce new candidates without training/deduplicating against history. |
| adaptive_cancel_at_fit_boundary_preserves_bootstrap_journal | Keep | Cancellation during fitting must retain completed bootstrap and not record suggested trials. |
| bo_multi_deploys_explicit_gp_without_auto_reports | Keep | Linear/nonlinear objectives must still deploy requested GP kind instead of Auto; independent progress/cancellation and EHVI usability are checked. |
| adaptive_records_monotonic_diagnostics | Keep | Fixed-reference iteration diagnostics must include bootstrap baseline and never decrease cumulative metric. |
| adaptive_stops_early_on_convergence | Keep | Patience/threshold must stop before full iteration budget with Converged reason. |
| non_adaptive_samplers_have_no_diagnostics | Merge | Into gh::runner::tests::random_sampler_records_all_trials; preserves empty diagnostics and Completed reason on existing nonadaptive execution. |
| adaptive_multi_objective_records_all_trials | Keep | Constrained multiobjective BO must journal all successful trials and agree with summary across repeated batches. |
| adaptive_single_objective_respects_slider_ranges | Keep | EI/constraint-model suggestions must respect real-unit slider range and resolution. |
| adaptive_cancel_before_run_records_nothing | Keep | BO-specific bootstrap entry must observe pre-cancellation rather than evaluate initial points. |
| adaptive_reports_error_when_bootstrap_fails | Delete | Same empty-bootstrap rejection is checked more strongly for both models with Journal states/progress in gh::runner::tests::bo_bootstrap_evaluation_failure_still_returns_insufficient_data. |

### `tdashboard-optimization-runner::gh::compute_def::tests::` (6 originals; complete test module)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| injects_groups_and_relays | Keep | Serialized GH object counts, RH_IN membership and typed RH_OUT relay source wiring must remain valid for Compute; these are external format constraints, not incidental vector layout. |
| gene_pool_is_replaced_by_a_number_param_input | Keep | Compute cannot drive a Gene Pool directly; replacement must redirect consumers but preserve Tunny's genuine-pool input. |
| pool_sourced_output_reads_the_replacement_param | Keep | Direct pool objective must read injected values rather than stale saved genome. |
| mixed_slider_and_gene_pool_inputs_keep_order_and_counts | Keep | Mixed grouped/scalar inputs must preserve flat variable alignment. |
| colliding_output_nicks_are_uniquified | Keep | Objective names containing reserved prefixes must not collide with constraint ParamNames. |
| escapes_names_in_injected_xml | Keep | Injected XML must escape user names while logical request names remain unescaped. |

Runner reconciliation: all **113 original definitions** were read, including
102 listed on Windows and 11 additional Unix-only definitions. All modules with
test attributes and the complete `gh::fixtures` / `test_support` helpers are
accounted for. `process::windows_tests` is gated at its parent module. The source
contains no Rust doc-test fence. Decisions: 108 Keep, 1 Delete, 4 Merge.

### `tdashboard-io::extras::tests::` (4 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| trial_state_from_journal_maps_known_and_unknown | Keep | Custom numeric state mapping and unknown-state fallback must match Optuna. |
| trial_state_from_rdb_str_maps_known_and_unknown | Keep | Separate RDB textual decoder must preserve states and fallback. |
| trial_state_label_is_uppercase_optuna_name | Keep | Persisted/displayed state labels must not change case or swap identities. |
| study_extras_helpers_detect_presence | Keep | Optional time/intermediate observations must drive availability even on different trials. |

### `tdashboard-io::io::datetime::tests::` (14 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| parses_round_epoch_value | Keep | Custom civil-date conversion must match an independently known Unix epoch value. |
| epoch_zero | Keep | Conversion's epoch origin must be zero rather than have an offset. |
| parses_space_separator | Keep | SQLite's space-separated timestamp format must be accepted. |
| parses_fractional_seconds | Keep | Fractional time must survive conversion; consolidated six-/three-digit and T/space inputs. |
| parses_fractional_seconds_arbitrary_digits | Merge | Into parses_fractional_seconds; retains exact three-digit expected timestamp without separate setup. |
| parses_time_of_day | Keep | Hours/minutes/seconds must use correct multipliers. |
| rejects_garbage | Keep | Invalid shape, separator, date/time ranges and fraction syntax must fail rather than fabricate time. |
| rejects_invalid_day_for_month | Keep | Month-specific calendar validation must reject impossible dates, not merely day >31. |
| leap_day_rules | Keep | Gregorian century/400-year exceptions and non-leap February boundaries must hold. |
| leap_year_and_later_date | Keep | Leap-day validation alone does not prove correct day count in conversion. |
| format_naive_datetime_epoch | Keep | Formatter must emit exact epoch string including microsecond padding. |
| format_naive_datetime_matches_expected_string | Keep | Nonzero date/time output must retain component placement and six-digit microseconds. |
| datetime_roundtrip_representative_values | Keep | Boundary timestamps near midnight/leap days and microseconds must round-trip without truncation/carry drift. |
| format_naive_datetime_handles_non_finite_without_panic | Keep | Custom formatter arithmetic must not panic when supplied nonfinite observation values. |

### `tdashboard-io::io::csv::writer::tests::` (4 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| writer_quotes_and_guards_text_fields | Keep | Header/data paths must both escape quotes/commas and guard spreadsheet formulas. |
| writer_numeric_fields_not_formula_guarded | Keep | Negative numeric cells must stay numbers instead of becoming apostrophe-prefixed text. |
| writer_non_finite_num_is_empty | Keep | Nonfinite and explicit empty fields must keep CSV column positions and row delimiter. |
| writer_guards_all_dangerous_leading_chars | Keep | All four formula-leading characters must be guarded, not only equals/at-sign in mixed-row test. |

### `tdashboard-io::io::artifacts::tests::` (18 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| scan_ticket_rejects_another_source_root_generation_or_scan | Keep | Stale scan result must not attach resources to another source/root/load/scan. |
| csv_resolution_rebases_associations_and_rejects_missing_or_escaping_files | Keep | CSV resource resolution must follow selected root and reject missing, parent-relative and absolute paths. |
| test_artifact_file_type_image | Keep | Renamed file_type_from_extension; retains lowercase/uppercase image extension policy and merged CSV/unknown cases. |
| test_artifact_file_type_csv | Merge | Into file_type_from_extension; retains CSV dispatch with shared extension-policy fixture. |
| test_artifact_file_type_other | Merge | Into file_type_from_extension; retains unknown extension fallback. |
| classify_prefers_mime_then_extension | Keep | Extensionless stored IDs must classify via MIME; missing MIME must fall back to original filename. |
| test_extract_trial_id_numeric_dir | Keep | Legacy pure-digit directory names must map to trial IDs. |
| test_extract_trial_id_with_suffix | Keep | Leading digit run must stop before suffix instead of requiring wholly numeric filename. |
| test_extract_trial_id_no_leading_digit | Keep | Embedded trailing digits must not fabricate trial associations. |
| test_validate_path_within_base | Keep | Valid canonical child must be accepted; containment rejection alone could permit reject-all implementation. |
| test_validate_path_traversal_blocked | Keep | Existing file outside root must fail containment with PathTraversal, not merely missing-file error. |
| parse_artifact_metadata_inline_ids_count_all_studies_and_states | Keep | Inline metadata must count every global CREATE_TRIAL, including artifactless/non-COMPLETE/other-Study trials. |
| parse_artifact_metadata_continues_after_invalid_lines | Keep | Invalid JSON/non-UTF8 lines must not stop scanning subsequent valid metadata. |
| parse_artifact_metadata_reads_system_attr_op9 | Keep | Explicit update ID and decoded filename/MIME/ID must survive nested JSON metadata. |
| parse_artifact_metadata_uses_key_suffix_when_id_missing | Keep | Missing embedded artifact ID must fall back to attribute-key suffix. |
| parse_artifact_metadata_skips_out_of_range_trial_id | Keep | u64 trial ID must not truncate/wrap into another u32 trial. |
| resolve_from_metadata_matches_files_on_disk | Keep | ID-named files must retain original filename/MIME; nonexistent artifacts are excluded. |
| scan_legacy_layout_groups_by_leading_digits | Keep | Actual directory scan must group files by legacy IDs rather than merely parse a name. |

### `tdashboard-io::io::storage::tests::` (3 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| journal_extension_detection | Keep | Storage router must distinguish both Journal suffixes from SQLite/local extensionless sources. |
| missing_storage_is_error_not_panic | Keep | Public scan/load adapters must return missing-database failures rather than panic or empty success. |
| journal_error_does_not_echo_storage_string | Keep | Misspelled credential-bearing URL routed as Journal path must not leak credentials in filesystem diagnostics. |

### `tdashboard-io::io::journal::tests::` (9 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| counts_all_op4_across_studies_and_states | Keep | Global trial ordinal must count creates regardless of state/Study. |
| counts_zero_for_empty_or_trialless_journal | Keep | Empty or metadata-only input must not fabricate created trials. |
| extracts_leading_field | Keep | Renamed extracts_fields_with_varied_positions_and_spacing; retains custom scanner's leading/middle/whitespace recognition with literal expected IDs. |
| extracts_middle_field | Merge | Into extracts_fields_with_varied_positions_and_spacing; retains arbitrary field position. |
| allows_whitespace_around_colon | Merge | Into extracts_fields_with_varied_positions_and_spacing; retains noncanonical spacing. |
| rejects_partial_key_match | Keep | Similar/unquoted key spelling must not be treated as the requested field. |
| skips_lookalike_and_finds_real_key | Keep | Scanner must continue after a lookalike rather than abandon valid later key. |
| rejects_non_numeric_and_missing | Keep | Missing/nonnumeric field must return None, not zero or panic. |
| rejects_out_of_range_value | Keep | Oversized external ID must not truncate. |

### `tdashboard-io::io::optuna_json::tests::` (1 original; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| nonfinite_literals_keep_positions_without_rewriting_strings_or_finite_data | Keep | Custom Python-JSON token normalization must preserve positions/quoted text and reject unrelated invalid syntax. |

### `tdashboard-io::io::journal::writer::tests::` (10 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| duplicate_preparation_requires_explicit_compatible_reuse_without_relabelling | Keep | Direct writer API must enforce explicit reuse/count/direction compatibility without writing or relabelling on reuse/rejection. |
| rejected_optuna_attempts_preserve_study_and_trial_identity_across_reopen | Keep | Legitimate rejected creation records must not consume Study IDs; reopened global trial IDs/local numbers/history remain correct across all states and multiple Studies. |
| unsafe_append_prefixes_are_rejected_without_repair | Keep | Detectable invalid references/deletion/unsupported/truncated records must fail append without altering original bytes. |
| writer_roundtrip_via_existing_parser | Keep | Independent writer/reader composition must preserve directions/bounds/numeric parameters/objectives/states/timestamps and missing parameters. |
| constraints_roundtrip_via_existing_parser | Keep | Constraint op9 serialization must preserve presence, width and ordered signed observations. |
| non_finite_constraints_are_rejected | Keep | Writer must reject values serde would silently encode as null instead of corrupting constraint positions. |
| user_attrs_roundtrip_via_existing_parser | Keep | User-attribute op8 must retain numeric/text JSON through readback. |
| writer_reopen_continues_numbering | Delete | Reopen numbering and Study names already checked more strongly by rejected_optuna_attempts_preserve_study_and_trial_identity_across_reopen plus duplicate_preparation_requires_explicit_compatible_reuse_without_relabelling. |
| set_trial_param_double_serializes_distribution | Keep | Raw format must use Optuna's JSON-string distribution encoding; permissive local reader accepting both shapes would miss this producer incompatibility. |
| writer_includes_worker_id_and_newline_terminated_lines | Keep | Actual lines must retain custom worker ID, required newline framing and project's intentional Optuna-layout op_code ordering. |

### `tdashboard-io::io::rdb::postgres_backend::tests::` (4 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| convert_placeholders_no_params | Keep | Custom SQL rewriting must preserve SQL without bind placeholders. |
| convert_placeholders_single | Delete | Multiple-placeholder test already fixes first bind at $1 and preserves surrounding SQL. |
| convert_placeholders_multiple | Keep | Bind numbers must increment in order for every placeholder, not all map to $1. |
| convert_placeholders_join_query | Delete | Join/literal text has no special treatment in this scanner; weaker single-placeholder suffix check duplicates retained conversion test. |

### `tdashboard-io::io::rdb::mysql_backend::tests::` (4 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| format_mysql_date_pads_components | Delete | Same exact padded date checked through production mysql_value_date_is_formatted_as_text conversion path. |
| format_mysql_date_full_precision | Keep | Six-digit microseconds and full-width components must not truncate/overflow in adapter formatting. |
| mysql_value_conversion_basic_types | Keep | Project value adapter must preserve null/int/uint/float/text and chosen unsupported-time policy. |
| mysql_value_date_is_formatted_as_text | Keep | Date variant must route through timestamp formatting rather than become null or another value type. |

### `tdashboard-io::io::rdb::url::tests::` (37 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| parse_postgresql_scheme | Keep | PostgreSQL source must select correct backend and preserve connection authority/path. |
| parse_postgres_short_scheme_normalizes | Keep | postgres alias must normalize to driver's postgresql scheme. |
| parse_mysql_scheme | Keep | MySQL source must not be dispatched as PostgreSQL. |
| parse_strips_sqlalchemy_driver_suffix_postgres | Keep | SQLAlchemy PostgreSQL driver qualifier must be stripped before connecting. |
| parse_strips_sqlalchemy_driver_suffix_mysql | Keep | Independent MySQL dialect must also strip qualifier while preserving kind. |
| parse_strips_driver_suffix_postgres_short_scheme | Keep | Alias normalization and suffix stripping must compose, not only work independently. |
| parse_rejects_sqlite | Keep | SQLite URI must not be mistaken for a server backend. |
| parse_rejects_unknown_scheme | Keep | Unsupported schemes/local paths/empty input must not fabricate server source. |
| is_rdb_url_true_and_false | Delete | is_rdb_url is exactly parse(...).is_some(); same success/failure schemes are already covered by parser tests. |
| masked_replaces_password | Keep | Display URL must preserve nonsecret connection parts but remove password. |
| masked_no_password_unchanged | Keep | User-only authority must not be mistaken for colon-bearing host/port password. |
| masked_no_userinfo_unchanged | Keep | Host-only authority must remain usable display, not be fully masked. |
| masked_without_port | Keep | Password boundary must not require explicit port. |
| masked_percent_encoded_password_is_kept_opaque | Keep | Encoded @ in password must not alter authority boundary or leak text. |
| masked_password_with_colon_takes_first_colon_as_boundary | Keep | Password containing raw colon must be entirely hidden, not leave suffix visible. |
| masked_with_query_string_and_path | Keep | Password masking must preserve query/path without incorporating them into credentials. |
| masked_password_with_unencoded_slash_fails_closed | Keep | Malformed password delimiter must trigger full mask instead of no-userinfo leak. |
| masked_password_with_unencoded_question_mark_fails_closed | Keep | Query delimiter in malformed password must likewise fail closed. |
| masked_password_with_unencoded_hash_fails_closed | Keep | Fragment delimiter in malformed password must likewise fail closed for MySQL. |
| masked_no_scheme_separator_returns_unchanged | Keep | Publicly constructible RdbUrl without delimiter must not panic/index invalid ranges. |
| check_tls_precondition_no_query_string_is_ok | Delete | Exact localhost/no-query call is contained in check_tls_precondition_loopback_hosts_allow_plaintext_without_sslmode. |
| check_tls_precondition_sslmode_require_is_err | Keep | Explicit encryption requirement must never silently downgrade to plaintext even on localhost. |
| check_tls_precondition_sslmode_disable_is_ok | Keep | Explicit lowercase opt-out must remain accepted. |
| check_tls_precondition_sslmode_disable_case_insensitive_is_ok | Keep | Uppercase disable spelling must be recognized, not rejected as encryption request. |
| check_tls_precondition_sslmode_disabled_word_is_ok | Keep | Separate accepted disabled spelling must remain recognized. |
| check_tls_precondition_mysql_ssl_mode_required_is_err | Keep | MySQL hyphenated SSL option must also reject encryption rather than be ignored. |
| check_tls_precondition_mysql_ssl_mode_disable_is_ok | Keep | MySQL disable option must follow accepted plaintext policy. |
| check_tls_precondition_other_query_params_are_ignored | Keep | Unrelated options must not be interpreted as encryption requests. |
| check_tls_precondition_ignores_fragment | Keep | Fragment containing sslmode=require must not be parsed as active option. |
| check_tls_precondition_loopback_hosts_allow_plaintext_without_sslmode | Keep | Localhost, numeric IPv4 and bracketed IPv6 loopback detection must permit intended local connections. |
| check_tls_precondition_remote_host_without_sslmode_is_err | Keep | Remote PostgreSQL plaintext requires explicit consent and PostgreSQL-specific diagnostic. |
| check_tls_precondition_remote_mysql_without_ssl_mode_is_err | Keep | Remote MySQL plaintext requires explicit consent and correct option diagnostic. |
| check_tls_precondition_remote_host_with_sslmode_disable_is_ok | Keep | Explicit remote consent must not be rejected by blanket nonloopback gate. |
| check_tls_precondition_remote_host_with_sslmode_require_is_err | Keep | Remote encryption request must produce unsupported-TLS error, not suggest silent plaintext. |
| check_tls_precondition_lookalike_hosts_are_not_loopback | Keep | Hostname lookalikes must not bypass remote plaintext gate. |
| has_explicit_plaintext_optin_variants | Keep | UI consent detection independently handles option aliases/case/fragments and distinguishes absent versus explicit opt-out. |
| extract_host_variants | Keep | Custom host extraction must handle userinfo/ports/query and bracketed IPv6, including malformed input. |

### `tdashboard-io::io::rdb::sqlite_backend::snapshot_tests::` (3 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| sqlite_snapshot_interleaved_acquisition | Keep | Deterministic writer interleaving must show public acquisition is whole-old in DELETE/WAL while independent-query control demonstrates possible mixed reads; all metadata/row/extras fields compared. |
| sqlite_snapshot_establishment_and_multiple_writer_transactions | Keep | Read session must establish snapshot on first read, not BEGIN, preserve legitimate intermediate producer state and see new data next session. |
| sqlite_snapshot_errors_release_reader_and_wal_checkpoint | Keep | Query/callback/conversion/write/finish/early-return failures must release transaction/checkpoint resources without corrupting existing transaction or preventing reuse. |

### `tdashboard-io::io::rdb::server_snapshot_tests::` (3 originals; complete file; ignored opt-ins)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| server_snapshot_interleaved_acquisition | Keep | Real PostgreSQL/MySQL public read transactions must prevent mixed concurrent acquisition independently of SQLite. |
| server_snapshot_timing_and_cleanup | Keep | Server first-read snapshot timing, success/failure cleanup, connection-loss stages and reuse must honor backend-specific session ownership. |
| server_snapshot_resource_lifetime_and_public_errors | Keep | Server xmin/metadata-lock lifetimes must end on success/error; public API must fail without leaking credentials after killed session. |

The snapshot helpers and connection adapters were read, including their query
hooks: interleavings invoke production transaction/query paths rather than test
a mock's state machine. Their setup assertions establish controls, not separate
test definitions.

Other-language sources read completely:

| Source / qualified entry | Decision | Specific reason |
| --- | --- | --- |
| crates/io/tests/create_nonfinite_fixture.py::<module> | Keep | Reproducible genuine Optuna 5 producer fixture verifies nonfinite writer/replay semantics used by Rust handoff tests; version/assertions establish producer evidence, not product unit cases. |
| crates/io/tests/create_server_snapshot_fixture.py::main (including storage_url) | Keep | Disposable server fixture producer/mutator required by retained ignored acquisition tests; confines destructive seeding to task-owned loopback database and checks MySQL engine prerequisite. |
| mcp-server/tests/create_sqlite_fixture.py::<module> | Keep | Assertion-free reduced-schema fixture producer required by retained SQLite stdio opt-in, not a separate Python test suite. |

I/O reconciliation: all **114 original Rust test definitions** were read. The
complete test-module inventory is extras, datetime, csv::writer, artifacts,
storage, journal, journal::writer, optuna_json, rdb::{postgres_backend,
mysql_backend,url,sqlite_backend::snapshot_tests,server_snapshot_tests}.
There are no platform-excluded I/O test definitions identified. The only source
doc fence found is `examples/rdb_smoke.rs`'s `text` usage block, not a Rust
doc-test. Decisions: 103 Keep, 6 Delete, 5 Merge.

### `tdashboard-analysis::mcdm::topsis::tests::` (21 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| tc_1615_01_basic_two_obj_minimize | Keep | Competing criteria must produce bounded scores and descending ranking; receives exact compromise winner and complete permutation assertions on its existing fixture. |
| tc_1615_02_maximize_direction | Keep | Mixed direction must favor trial best on maximize/minimize criteria, not uniformly minimize. |
| unnormalized_weights_match_normalized | Delete | mcdm::tests::rankings_preserve_nonnegative_normalization_and_bounds checks raw/normalized weights, all score/ranking/ideal fields and degenerate sums through this entry. |
| tc_1615_03_weights_affect_ranking | Keep | Switching criterion importance must select independently known opposite winners, not merely return different arrays. |
| tc_1615_04_single_trial | Keep | Degenerate distance denominator must return midpoint score and singleton rank without NaN. |
| tc_1615_05_single_objective | Keep | Single criterion must rank nonidentity input from minimum to maximum. |
| tc_1615_06_zero_trials_error | Keep | Public numerical entry must reject zero alternatives with a useful diagnostic. |
| tc_1615_07_values_length_mismatch_error | Keep | Flat input shape mismatch must fail before indexing. |
| tc_1615_08_weights_length_mismatch_error | Keep | Wrong criterion-weight arity must fail through this public entry. |
| tc_1615_09_is_minimize_length_mismatch_error | Keep | Wrong direction arity must fail through this public entry. |
| tc_1615_10_all_same_values_no_crash | Keep | Identical alternatives must get midpoint scores without zero-division contamination. |
| tc_1615_11_nan_trial_ranked_last | Keep | NaN alternative must be excluded while preserving original output position and bottom rank. |
| tc_1615_12_performance_50k_trials | Keep | Release-only 50k workload guards accidental quadratic materialization/allocation failure in a supported large population; debug/ci-test uses 500 and is not 50k evidence. |
| tc_1615_13_ranked_indices_length | Merge | Into tc_1615_01_basic_two_obj_minimize; retains original-index permutation rather than only length checks. |
| tc_1615_14_ideal_solutions_dimension | Keep | Returned positive/negative ideal vectors must follow criterion width, not trial count or fixed two dimensions. |
| tc_1615_15_inf_trial_excluded_and_ranked_last | Keep | Infinity must be rejected as well as NaN; is_nan-only filtering would miss this. |
| tc_1616_01_two_trials_ranking | Keep | Dominated two-trial case must strictly separate scores; complements interior-compromise and mixed-direction fixtures. |
| tc_2268_01_topsis_scores_match_after_single_alloc_refactor | Merge | Into tc_1615_01_basic_two_obj_minimize; identical fixture's only extra guarantee, trial 2 wins, is retained. |
| tc_2268_02_all_nan_returns_empty_valid_scores | Keep | Entirely invalid population takes independent uniform fallback rather than mixed-valid-row exclusion path. |
| tc_2268_03_single_objective_single_alloc | Delete | Exact same input and winner/loser assertions as tc_1615_05_single_objective; refactor provenance adds no guarantee. |
| tc_1616_02_weights_scale_invariant | Delete | Weaker best-index-only scale check duplicates retained mcdm::tests::rankings_preserve_nonnegative_normalization_and_bounds and tc_1615_03_weights_affect_ranking. |

### `tdashboard-analysis::mcdm::vikor::tests::` (25 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| tc_vikor_001_basic_two_obj_minimize | Keep | Tied regret range must not poison utility ranking; output widths, descending display scores and Q bounds remain checked. |
| tc_vikor_002_maximize_direction | Keep | Mixed directions with maximize in first position must produce correct strict winner; distinct from maximize-second exact fixture. |
| tc_vikor_003_v_zero_r_only | Keep | Renamed tradeoff_endpoints_select_regret_or_utility; old single-objective fixture made normalized S and R identical and expected values could be skipped. Consolidated endpoints now use independently computed, different S/R vectors and exact Q literals. |
| tc_vikor_004_v_one_s_only | Merge | Into tradeoff_endpoints_select_regret_or_utility; retains endpoint-one contract without duplicating setup/self-derived formula. |
| tc_vikor_005_weights_affect_ranking | Keep | Reweighting competing criteria must switch known winner. |
| tc_vikor_006_ranked_indices_q_ascending | Delete | Monotonically ordered input would pass omitted sorting; retained basic/maximize/single-objective cases require correct ordering on nonidentity winners and worst alternatives. |
| tc_vikor_007_compromise_c1_c2_hold_single_solution | Keep | Compromise set must collapse to one solution when both advantage/stability conditions hold. |
| tc_vikor_008_compromise_c1_fails_multi_solution | Keep | Failed advantage must return all close alternatives, excluding distant worst. |
| tc_vikor_009_compromise_c2_fails_two_solutions | Keep | Independent hand-picked S/R/Q isolates reachable C2-failure rule and maps sparse original indices in Q order. |
| tc_vikor_010_compromise_single_valid_alternative | Merge | Into tc_vikor_b01_single_trial; retains singleton compromise at public compute boundary; sparse-index mapping remains covered by tc_vikor_009. |
| tc_vikor_011_compromise_no_valid_alternatives | Delete | Direct private-helper empty-input branch is bypassed by public compute_vikor's no-valid-row early return; no reachable product regression is protected. |
| tc_vikor_012_unnormalized_weights_match_normalized | Delete | mcdm::tests::rankings_preserve_nonnegative_normalization_and_bounds already compares raw/normalized Q, S, R, display, ranks and compromise sets through the public entry. |
| tc_vikor_013_single_maximize_objective_discriminates | Keep | Historical maximize initialization bug must not leave infinite extrema and NaN utility. |
| tc_vikor_014_mixed_direction_exact_s_r_q | Keep | Hand-computed mixed-direction utility/regret/Q catches plausible wrong calculations, not just success/range. |
| tc_vikor_015_out_of_range_v_is_clamped | Keep | Out-of-range/NaN tradeoff must remain safe; changed existing fixture to distinct normalized S/R so clamp comparisons are meaningful. |
| tc_vikor_e01_zero_trials_error | Keep | Public zero-alternative validation and diagnostic must remain wired. |
| tc_vikor_e02_values_length_mismatch | Keep | Incorrect flat shape must reject before numerical indexing. |
| tc_vikor_e03_weights_length_mismatch | Keep | Incorrect criterion-weight count must fail this entry. |
| tc_vikor_e04_is_minimize_length_mismatch | Keep | Incorrect direction count must fail this entry. |
| tc_vikor_b01_single_trial | Keep | Singleton range normalization must remain finite and retain singleton ranking/merged compromise guarantee. |
| tc_vikor_b02_all_same_values | Keep | Multiple equal alternatives exercise zero utility/regret ranges without singleton shortcuts. |
| tc_vikor_b03_nan_trial | Keep | Invalid alternative retains worst Q/zero display and bottom rank instead of contaminating valid results. |
| tc_vikor_b04_single_objective | Keep | Nonidentity single-criterion ordering must follow minimize direction. |
| tc_vikor_b05_neg_inf_trial | Keep | Negative infinity must be filtered, not treated as an ideal observation. |
| tc_vikor_perf01_50k_trials | Keep | Large supported population exercises actual allocations/sorting; catches accidental quadratic materialization not covered by small correctness fixtures, without claiming a measured speed bound. |

### `tdashboard-analysis::mcdm::promethee::tests::` (19 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| tc_pr_005_01_linear_d_gt_p | Keep | Independent known preference saturation/weighted flow value must equal 0.5, not merely obey bounds. |
| unnormalized_weights_match_normalized | Delete | Same public-entry raw/normalized flows and rankings already checked with degenerate sums by mcdm::tests::rankings_preserve_nonnegative_normalization_and_bounds. |
| count_incomparable_flags_conflicting_flows | Delete | incomparable_counts_with_invalid_trial exercises same comparable/conflicting pairs through full pipeline with exact counts plus original-index mapping. |
| incomparable_counts_with_invalid_trial | Keep | Must count conflicting flows for both alternatives while excluding invalid rows and retaining original positions. |
| tc_pr_005_02_all_same | Keep | Zero-range criteria must produce zero positive/negative/net flows for every trial. |
| tc_pr_006_02_phi_net_identity | Keep | Output net flow must subtract corresponding positive/negative flows, not use sum or wrong index. |
| tc_pr_007_01_promethee_i_phi_plus_descending | Keep | Both I and merged II rankings must order nonidentity input; fixture now uses original II case and full expected II permutation. |
| tc_pr_008_01_best_trial_first | Merge | Into tc_pr_007_01_promethee_i_phi_plus_descending; retains II winner and strengthens to all three ranks with shared computation. |
| tc_pr_003_e01_zero_trials | Keep | Zero alternatives must return explicit public validation error. |
| tc_edge_pr_001_single_trial | Keep | Singleton flow denominator/self-comparison must not create NaN or spurious preference. |
| tc_edge_pr_002_range_zero | Delete | Same constant-data numerical path as tc_pr_005_02_all_same, with weaker checks (omits net flow). |
| tc_edge_pr_003_nan_trial_ranked_last | Keep | Both I and II must put invalid trial last regardless of zero-flow tie. |
| tc_edge_pr_004_all_nan | Keep | No-valid-row fallback is distinct from constant valid alternatives. |
| tc_edge_pr_005_phi_net_negative_no_crash | Keep | Worse and better alternatives must have opposite signed net flows. |
| tc_pr_003_e02_values_length_mismatch | Keep | Entry must reject invalid row-major shape. |
| tc_pr_003_e03_weights_length_mismatch | Keep | Entry must reject criterion-weight arity mismatch. |
| tc_pr_003_e04_is_minimize_length_mismatch | Keep | Entry must reject direction arity mismatch. |
| tc_pr_nfr_001_02_performance_10k | Keep | Release bounded workload guards quadratic-time regression beyond expected implementation; debug uses 1000 without timing assertion, explicitly not 10k evidence. |
| tc_pr_nfr_001_01_performance_50k | Keep | Ignored opt-in workload detects catastrophic allocation/materialized pairwise-matrix regression at 50k; default suite does not execute it. |

### `tdashboard-analysis::mcdm::entropy::tests::` (21 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| constant_columns_mixed_with_varying_column | Keep | Constant zero/positive/negative/extreme columns must get exact unit entropy, zero diversity/weight and correct normalized matrix. |
| all_constant_columns_use_equal_weight_fallback | Keep | All-constant population must give exact equal weights across multiple row counts/sign mixtures without rounding residuals. |
| single_valid_trial_preserves_existing_policy | Keep | Single retained row, including finite filtering and zero/negative values, must preserve chosen entropy/diversity policy. |
| constancy_uses_only_retained_finite_rows | Keep | Invalid rows must be removed before detecting constant columns and determining normalization row count. |
| nonconstant_calculations_are_unchanged | Keep | Independent literal probabilities fix preprocessing/normalization; textbook entropy on those fixed probabilities checks downstream entropy/diversity/weights, not a copy of preprocessing code. |
| near_constant_column_is_not_constant | Keep | Tiny real variation must not be erased by approximate constancy detection. |
| tc_entropy_01_basic_2objectives | Delete | Weak length/sum/range smoke adds no distinction beyond retained exact mixed-varying/constant probability and weight fixtures. |
| tc_entropy_02_3objectives | Delete | Three-column width and normalized sum already covered more strongly by nonconstant_calculations_are_unchanged. |
| tc_entropy_03_high_variance_higher_weight | Delete | Exact fixture is in constant_columns_mixed_with_varying_column with stronger zero/one expected weights. |
| tc_entropy_b01_single_objective | Keep | Single criterion must receive unit weight, not a hard-coded multi-criterion fallback. |
| tc_entropy_b02_single_trial | Delete | Weaker positive-only uniform-weight case duplicates single_valid_trial_preserves_existing_policy. |
| tc_entropy_b03_all_same_values | Delete | Weaker all-constant fallback case duplicates all_constant_columns_use_equal_weight_fallback. |
| tc_entropy_04_proportional_normalization | Delete | Column-sum-only check is implied by independently fixed matrix in nonconstant_calculations_are_unchanged. |
| tc_entropy_05_zero_variance_objective | Delete | Same 5.0 constant/1,2,3 fixture and expected entropy/diversity/weights already covered exactly by constant_columns_mixed_with_varying_column. |
| tc_entropy_06_nan_exclusion | Delete | Only checks length/sum; constancy_uses_only_retained_finite_rows proves actual finite-row exclusion and correct contents. |
| tc_entropy_07_all_nan_error | Keep | Entirely invalid population must reject rather than normalize empty columns. |
| tc_entropy_08_negative_values | Delete | Only length/sum smoke; retained nonconstant_calculations_are_unchanged fixes negative min-max preprocessing and exact downstream outputs. |
| tc_entropy_09_zero_handling | Delete | Constant-one and varying-zero cases are already covered more strongly by exact retained mixed-constant and nonconstant probability fixtures. |
| tc_entropy_10_equal_weights_sum | Delete | Another ordinary input with sum-only assertion; no independent branch or numerical guarantee beyond exact retained weight fixtures. |
| tc_entropy_11_negative_constant_column | Delete | Exact -3.0 constant fixture is included in constant_columns_mixed_with_varying_column with stronger matrix/entropy/diversity checks. |
| tc_entropy_perf_01_50k_trials | Keep | Large actual allocation workload guards accidental quadratic storage at supported population size; correctness remains covered by small exact fixtures. |

MCDM continuation: **86 additional definitions read**, 60 Keep, 21 Delete,
5 Merge. Two previously provisional GUI numerical-only Keep decisions were
reclassified Delete after reading their stronger numerical equivalents. No
numerical algorithm or public validation policy changed.

### `tdashboard-analysis::math::rng::tests::` (6 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| tc_301_01_seeded_rng_reproducibility | Keep | Wrapper must propagate supplied seed deterministically rather than use ambient randomness. |
| tc_301_02_different_seeds_produce_different_sequences | Keep | Wrapper must not ignore supplied seed or replace it with one fixed seed; reproducibility alone would miss this. |
| tc_301_03_next_f64_range | Delete | next_f64 directly returns rand's f64 distribution; repeats dependency guarantee plus range already asserted by retained reproducibility test. |
| tc_301_04_next_usize_range | Keep | Project wrapper chooses 0..n; changing to inclusive or wrong interval would break shuffle/indexing. |
| tc_b01_seed_zero | Delete | No project special handling of zero seed; same rand float-range assertion already covered. |
| tc_b02_next_usize_bound_one | Delete | Degenerate 0..1 behavior belongs to rand; wrapper's exclusive-bound choice is already exercised by tc_301_04. |

### `tdashboard-analysis::math::grid::tests::` (3 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| linspace_normal | Keep | Custom grid must include both endpoints and evenly spaced midpoint. |
| linspace_zero_returns_empty | Keep | Zero-point requests must not underflow n-1 or fabricate a value. |
| linspace_one_returns_midpoint | Keep | Singleton policy is midpoint, not min endpoint or zero-division result. |

### `tdashboard-analysis::math::stats::tests::` (14 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| normal_data | Keep | Feature standardization uses population variance and correct mean, not sample variance. |
| constant_values | Keep | Zero-variance columns must retain mean and use unit standard deviation; includes singleton fixture after merge. |
| empty_slice | Keep | Empty-column fallback is explicitly (0,1), not NaN/division error. |
| single_element | Merge | Into constant_values; retains singleton population-standard-deviation fallback without separate setup. |
| tc_2261_01_pearson_perfect_positive | Delete | Identity-vector positive correlation is weaker than retained proportional-positive fixture, which additionally catches normalization mistakes. |
| tc_2261_02_pearson_proportional_positive | Keep | Differently scaled positive vectors must still give unit correlation. |
| tc_2261_03_pearson_perfect_negative | Keep | Covariance sign must produce -1, not absolute correlation. |
| tc_2261_04_pearson_zero_variance_returns_nan | Keep | A constant column must return unavailable correlation, not infinity/zero or panic. |
| tc_2261_05_pearson_both_zero_variance_returns_nan | Delete | Same zero-denominator guard already exercised when one variance is zero; both-zero input introduces no separate behavior. |
| tc_2261_06_pearson_empty_returns_nan | Keep | Empty correlation must remain NaN rather than a fabricated coefficient. |
| value_range_normal_data | Keep | Custom iterator reduction must preserve actual unsorted extrema. |
| value_range_empty_returns_inf_neg_inf | Keep | Empty/all-NaN range must preserve sentinel extrema for caller-specific fallbacks; receives all-NaN fixture. |
| value_range_ignores_nan | Keep | Mixed observations must retain finite extrema instead of propagating NaN. |
| value_range_all_nan_returns_inf_neg_inf | Merge | Into value_range_empty_returns_inf_neg_inf; retains distinct all-NaN input while sharing sentinel assertions. |

### `tdashboard-analysis::math::special::tests::` (3 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| ln_gamma_matches_known_values | Keep | In-house Lanczos coefficients must match independent factorial/half-integer identities used for Weibull scaling. |
| norm_ppf_matches_known_quantiles | Keep | In-house normal quantile approximation must match external known z-values across central/tail regions. |
| norm_ppf_out_of_domain_is_nan | Keep | Invalid probability must not become a plausible sigma level. |

### `tdashboard-analysis::convergence::tests::` (10 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| improvement_rate_all_improving | Delete | Merely checks >0; retained exact full-window minimize and windowed rate fixtures already require positive counted improvements. |
| improvement_rate_empty_returns_zero | Keep | Empty/singleton histories have zero rate, not division error; includes singleton fixture after merge. |
| improvement_rate_single_returns_zero | Merge | Into improvement_rate_empty_returns_zero; retains short-history boundary with shared expected value. |
| improvement_rate_maximize_counts_non_decreasing_steps | Keep | Actual best-so-far construction must count strict maximize improvements but not flat steps. |
| improvement_rate_minimize_direction_mismatch_detects_no_improvement | Keep | Same observations must yield different exact minimize/maximize rates; guards direction wiring. |
| improvement_rate_windowed_does_not_overcount_flat_tail | Keep | Window must use pre-window best instead of counting its first flat value as a new improvement. |
| improvement_rate_windowed_counts_only_in_window_improvements | Keep | Genuine improvement at window tail must count once with window denominator. |
| build_best_trial_history_minimize | Keep | Running best must retain original IDs and ignore worsening minimize observations. |
| build_best_trial_history_maximize | Delete | Exact same initial three IDs/values/result already checked by improvement_rate_maximize_counts_non_decreasing_steps, which also covers following flat observation. |
| build_best_trial_history_empty | Keep | Empty construction must not emit a sentinel observation. |

### `tdashboard-analysis::statistics::boxplot::tests::` (6 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| known_quartiles_one_to_nine | Keep | Independent quartiles/whiskers for known data must follow intended type-7/Tukey policy. |
| detects_outliers | Keep | Outlier must be separated from whisker, not simply use max as whisker. |
| single_element | Keep | Singleton summary takes separate path and must preserve all statistics without outliers. |
| empty_input_returns_none | Keep | Empty/all-nonfinite input must not return an invented summary or panic. |
| quantile_interpolates | Keep | Fractional quantile position must interpolate, unlike integer-index quartiles in known fixture. |
| quantile_empty_is_nan | Keep | Public quantile empty guard differs from compute_boxplot early rejection. |

### `tdashboard-analysis::statistics::correlation::tests::` (7 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| pairwise_policy_does_not_require_other_columns | Keep | Each pair independently selects finite rows despite ragged/unrelated invalid columns; checks both methods and minimum paired observations. |
| perfect_correlation_pair | Keep | Matrix must retain labels and symmetric placement of known Pearson value. |
| pairwise_nan_exclusion | Delete | Weaker NaN-only one-cell check duplicates finite-pair selection in pairwise_policy_does_not_require_other_columns. |
| diagonal_is_always_one | Merge | Into constant_column_gives_nan_off_diagonal; identical fixture now verifies unit diagonal alongside unavailable off-diagonal. |
| constant_column_gives_nan_off_diagonal | Keep | Constant column cannot yield finite off-diagonal correlation even though diagonal remains one. |
| empty_columns_returns_none | Keep | No columns means absent matrix, not a dummy cell. |
| spearman_method_used_when_requested | Keep | Nonlinear-rank fixture must route Spearman rather than Pearson; using known underlying primitive is a dispatch check, not duplicating that algorithm. |

### `tdashboard-analysis::statistics::histogram::tests::` (8 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| sturges_bin_count | Keep | Custom rule selection must produce known bin count and retain all observations. |
| constant_data_single_bin | Keep | Degenerate range must collapse to one populated bin with matching endpoints. |
| excludes_non_finite_values | Keep | NaN/Inf must not contribute counts or corrupt inferred range. |
| last_bin_is_right_closed | Keep | Maximum endpoint must not be discarded or index past final bin. |
| freedman_diaconis_falls_back_to_sturges_when_iqr_zero | Keep | Nonconstant zero-IQR input must use intended fallback, not constant-data shortcut. |
| empty_or_all_non_finite_returns_none | Keep | No finite observations must not fabricate histogram. |
| manual_zero_treated_as_one | Keep | Product's minimum manual-bin policy must be applied before bin-width division. |
| bin_count_clamped_to_200 | Keep | Product's resource/display cap must be wired; standard clamp behavior alone does not select bound 200. |

### `tdashboard-analysis::statistics::distribution_fit::tests::` (6 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| normal_fit_recovers_sample_mean | Keep | In-house Normal MLE must recover known symmetric mean and finite fit. |
| lognormal_rejects_non_positive | Keep | Positive-support models must reject invalid sign while Normal remains applicable. |
| weibull_fit_produces_positive_params | Keep | In-house iterative Weibull solver must produce usable positive parameters and finite likelihood. |
| pdf_integrates_roughly_to_one | Keep | Independent numerical integration guards PDF normalization; this is project math, not a dependency library's PDF. |
| fit_all_sorted_by_aic | Keep | Fit comparison must return usable models ordered by quality, not declaration order. |
| too_few_or_degenerate_samples_rejected | Keep | Insufficient/constant observations must not produce invalid zero-sigma models. |

### `tdashboard-analysis::statistics::violin::tests::` (13 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| fewer_than_two_finite_values_returns_none | Keep | No usable sample or singleton must be rejected before KDE. |
| constant_input_returns_none | Keep | Zero-bandwidth constant population must not produce NaN/infinite density. |
| non_finite_values_are_filtered | Keep | Finite count and range must exclude all kinds of nonfinite observation. |
| grid_and_range_are_reported | Keep | KDE output shape/median/range must agree and grid extend beyond observations. |
| grid_points_is_clamped_to_two | Keep | Zero requested grid size must use two points, avoiding invalid step denominator. |
| density_is_finite_and_non_negative | Merge | Into density_integrates_to_approximately_one; same 0..100 sample now asserts finite/nonnegative density alongside total mass. |
| density_integrates_to_approximately_one | Keep | Independent integral guards missing normalization; consolidated density validity assertion also rules out negative lobes. |
| symmetric_sample_peaks_near_mean | Keep | KDE location must reflect input values rather than shifted kernel/grid. |
| bimodal_sample_has_two_peaks | Keep | KDE must preserve two separated modes, not over-smooth into one distribution. |
| scott_bandwidth_matches_formula | Keep | Fixtures independently force sd versus robust-IQR branch; expected separate ingredients catch wrong combined rule/scale. |
| zero_iqr_non_constant_input_falls_back_to_sd | Keep | Nonconstant but zero-IQR sample must render using sd instead of disappearing. |
| large_input_is_subsampled_to_the_cap | Delete | Same cap assertion already made for both forward/reverse large inputs by large_input_subsampling_is_order_independent. |
| large_input_subsampling_is_order_independent | Keep | Cap must be applied after value sorting so row ordering does not alter bandwidth/extrema or drop extreme endpoint. |

Math/convergence/statistics continuation: **76 additional originals** read in
10 complete files, 62 Keep, 9 Delete, 5 Merge. No dependency, algorithm or public
policy change.

### `tdashboard-analysis::data::filter::tests::` (6 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| permissive_unknown_column_ignored | Keep | A filter referring only to absent columns must retain all rows, not exclude everything. |
| permissive_nan_value_rejected | Keep | Existing nonfinite parameter must be excluded even when numeric comparisons alone would let NaN pass. |
| permissive_in_and_out_of_range | Merge | Into permissive_mixed_known_and_missing_columns; identical numeric fixture is checked before adding missing-column condition. |
| permissive_mixed_known_and_missing_columns | Keep | Valid numeric condition must remain applied when another condition names a missing column; includes known-only case after merge. |
| permissive_empty_dataframe_returns_empty | Keep | Empty table must not fabricate row zero while processing conditions. |
| permissive_no_ranges_returns_all_rows | Keep | Unfiltered table takes separate early-return path and must retain every original row index. |

### `tdashboard-analysis::data::finite_rows::tests::` (2 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| required_columns_keep_zero_and_original_alignment | Keep | Required finite-feature selection must retain zero and original row mapping without independently filtering auxiliary constraints. |
| empty_and_missing_cells | Keep | Short columns/missing cells must exclude rows; empty required set must retain all requested rows, unlike an empty column. |

### `tdashboard-analysis::data::dataframe::feasibility::tests::` (2 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| classification_requires_complete_finite_evidence_but_violation_wins | Merge | Into tdashboard-study::integration(management)::classifier_preserves_invalid_positions_empty_presence_and_violation_priority; this analysis test directly calls the re-exported Study classifier, not a projection. Additional signed-infinity/partial-positive cases move to its owner; overlapping finite/empty cases deduplicated. |
| view_preserves_unconstrained_and_partitions_three_states | Keep | Analysis's derived-column view independently interprets missing column as unconstrained, NaN/missing row as unverified and preserves partition order/labels. |

### `tdashboard-analysis::optimization::lbfgs::tests::` (2 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| tc_201_01_lbfgs_optimizer_minimizes_quadratic | Keep | Project wrapper must supply initial values/solver configuration and return best parameters rather than silently fall back to input. |
| tc_201_02_lbfgs_returns_initial_when_zero_iters | Keep | Zero-budget configuration must retain exact input. Replaced original finite-only assertion, which could not detect ignored iteration budget, with equality to initial. |

### `tdashboard-analysis::clustering::tests::` (24 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| public_pca_preserves_mixed_features_and_complete_row_outputs | Keep | Public centered/standardized adapters must preserve category-constant encoding, feature names and retained original rows through missing/nonfinite filtering. |
| public_pca_handles_absent_params_and_insufficient_complete_rows | Keep | Partially observed DataFrame parameters must be filtered and insufficient retained rows rejected by both public entries. |
| pca_filters_before_centering_and_standardizing_and_keeps_source_rows | Keep | Raw-matrix path must filter before statistics, not let invalid rows alter covariance/standardization; public adapter filtering alone would hide this. |
| clustering_public_boundaries_reject_non_finite_input | Keep | K-means/elbow/SOM reject nonfinite inputs while Ward filters them; method-specific policies must not be homogenized. |
| tc_2262_05_kmeans_plusplus_correct_clusters_after_refactor | Delete | Same separated-data membership guarantee as tc_901_04_kmeans_convergence under another seed. Both enum values now use linfa's same k-means algorithm; seed plumbing remains tested for both. |
| tc_2262_06_deterministic_correct_clusters_after_refactor | Merge | Into tc_901_04_kmeans_convergence; identical fixture now checks centroid coordinates alongside labels/cluster counts. |
| tc_2267_01_kmeans_plusplus_determinism_after_clone_reduction | Keep | Renamed kmeans_initialization_is_reproducible; both init strategies must pass deterministic project-selected seeds through linfa and retain labels/centroids. |
| tc_2267_02_deterministic_init_matches_after_extend_from_slice | Merge | Into kmeans_initialization_is_reproducible; adds second init strategy to existing repeated-run fixture. |
| tc_2267_03_empty_cluster_fallback_preserves_centroid | Delete | Does not force empty cluster (four points form two obvious clusters); custom fallback no longer exists after linfa migration. Only weak centroid shape/finite check duplicates retained adapter test. |
| tc_901_01_pca_dominant_axis | Keep | Project covariance/eigenvalue ordering/loading extraction must favor known dominant feature; receives projection shape checks on same fixture. |
| tc_901_02_pca_projection_shape | Merge | Into tc_901_01_pca_dominant_axis; preserves every source row and requested component width without another PCA fit. |
| tc_901_03_pca_empty_data | Keep | Singleton raw input must produce empty result before decomposition. |
| pca_ragged_rows_return_empty_without_panic | Keep | Ragged input must reject safely in both centered and standardized raw-matrix paths. |
| pca_explained_ratio_sums_to_at_most_one | Keep | Returned explained fractions must be bounded/ordered and not confuse eigenvalues with ratios. |
| pca_standardized_is_column_scale_invariant | Keep | Standardized path must actually divide by feature scale; covariance-only PCA would change ratios. |
| pca_standardized_zero_variance_column_is_inert | Keep | Zero-variance feature must retain loading-column alignment rather than be dropped. |
| tc_901_04_kmeans_convergence | Keep | Flat-data adapter must produce complete labels and expected memberships/centroid locations; not just trust dependency success. |
| tc_901_05_kmeans_wcss_decreases_with_k | Keep | Returned quality field must route actual inertia and reflect additional clusters, not fixed zero/default. |
| tc_901_06_elbow_recommended_k_valid | Keep | Project elbow sweep must return k=2..max results and bounded recommendation. |
| tc_901_p01_pca_performance | Keep | Supported large-row allocation path must complete; release uses 50k, debug uses 2k, not a speed measurement. |
| tc_901_p02_kmeans_performance | Keep | Release 50k adapter must return one valid label per row; debug 100 is not release-scale evidence. |
| tc_101_01_pca_p20_works | Keep | More than two features/components must preserve eigenvector width and sorted variance rather than hard-code biplot dimensions. |
| tc_101_b01_pca_n2_p2_minimum | Keep | Minimum valid two-row input must fit instead of being rejected by stricter sample minimum. |
| tc_101_b02_pca_n1_returns_empty | Delete | Same singleton early-return path/input as tc_901_03_pca_empty_data; changing requested components 2 to 1 does not reach another branch. |

### `tdashboard-analysis::clustering::hierarchical::tests::` (10 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| produces_n_minus_one_merges_and_full_leaf_order | Keep | Custom Ward tree must preserve every leaf exactly once and correct merge count; receives monotone-distance assertions on same result. |
| cut_at_two_separates_the_blobs | Keep | Custom cut must produce actual expected cluster membership, not merely a valid merge tree. |
| merge_distances_are_monotone_nondecreasing | Merge | Into produces_n_minus_one_merges_and_full_leaf_order; identical linkage fixture now verifies sorted merges needed by cuts/rendering. |
| standardize_makes_columns_comparable | Delete | Only asserts n-1 merges; would pass if standardization were entirely omitted. Same structural success already covered, no scale-comparability guarantee asserted. |
| subsamples_above_cap | Keep | Quadratic Ward memory must remain capped and sampled leaf/source mapping lengths agree. |
| dendrogram_nodes_have_consistent_geometry | Keep | Separate renderer projection must place parents at child midpoints and not below children. |
| too_small_input_returns_none | Keep | Empty/singleton source cannot form tree and must reject safely. |
| nan_rows_are_excluded_without_panic | Keep | Both standardized/unstandardized Ward paths must drop exact invalid source positions and keep finite distances. |
| all_nan_rows_return_none | Keep | Fewer than two rows after filtering must reject, not proceed based on pre-filter length. |
| ragged_rows_are_excluded_without_panic | Keep | Ward's chosen ragged-row exclusion policy differs from PCA/SOM whole-input rejection. |

### `tdashboard-analysis::clustering::som::tests::` (6 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| shapes_are_consistent | Keep | Custom SOM must retain grid/output/observation dimensions and hit totals; receives repeatability and inverse-unit checks without redundant fits. |
| deterministic_without_seed | Merge | Into shapes_are_consistent; repeats same default fixture once and retains BMU/U-matrix equality. |
| separated_blobs_map_to_different_nodes | Keep | Training must separate distinct populations rather than merely return plausible-sized arrays. |
| component_plane_is_in_original_units | Merge | Into shapes_are_consistent; existing trained result must inverse-transform plane into real input scale. |
| caps_training_rows_but_outputs_full_shapes | Keep | Training cap must not truncate full-population BMU/hit outputs or break repeatability. |
| rejects_degenerate_input | Keep | Too few observations and invalid grid dimensions must reject before allocation/training. |

### `tdashboard-analysis::clustering::standardize::tests::` (4 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| population_variance_standardizes_to_unit | Keep | Shared helper must mutate data to zero mean/unit population variance and return correct inverse-transform statistics. |
| ddof_one_uses_sample_variance | Keep | PCA's requested sample-variance correction must differ from population path. |
| zero_variance_column_maps_to_zero | Keep | Constant feature must be zeroed without suppressing another varying feature. |
| empty_input_is_noop | Keep | Shared helper must not index first row on empty input. |

### `tdashboard-analysis::clustering::kmeans::tests::` (2 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| run_kmeans_rejects_non_divisible_flat_data | Keep | Public flat adapter must not silently truncate shape; debug expects project assertion, release expects empty result. |
| estimate_k_elbow_rejects_non_divisible_flat_data | Keep | Independent elbow flat adapter must enforce same shape contract before sweep. |

Data/optimizer continuation: **12 additional originals**, 10 Keep and 2 Merge.
Clustering continuation: **46 additional originals**, 36 Keep, 4 Delete, 6 Merge.
All five clustering test files, PCA/entry implementation and test-only wrappers
were read. No production algorithm was changed.

### `tdashboard-analysis::lgbm::tests::` (15 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| dataset_boundary_rejects_non_finite_measurements | Keep | FFI dataset adapter must reject nonfinite features/labels instead of delegating missing-data semantics to native code. |
| default_config | Delete | Pins private helper's tuning constants (tree count/depth/fractions) without an external requirement; valid training/prediction/importance behavior is covered through actual native calls. |
| train_returns_booster | Delete | Same default native training must already succeed in pipeline_train_predict_mse and retained contribution/importance/PDP tests. |
| predict_length_matches_input | Delete | Exact train/predict shape guarantee retained by pipeline_train_predict_mse; changing ordinary sample count 30 to 40 adds no branch. |
| mse_is_non_negative | Merge | Into pipeline_train_predict_mse; retains nonnegative error without another training run. |
| mse_empty_returns_none | Keep | Project MSE empty-data policy must return None before reducing predictions. |
| feature_importance_sums_to_one | Keep | Project gain normalization/feature-count adapter must retain dimension and unit sum (or zero fallback), not merely return native gains. |
| mse_to_r2_basic | Keep | Project R² conversion must map perfect fit to one. |
| predict_contrib_shape | Keep | Native contribution buffer must be reshaped into samples × (features+bias), not normal prediction shape. |
| pipeline_train_predict_mse | Keep | Real dataset/booster/prediction/MSE composition must succeed with correct prediction count and finite/nonnegative error. |
| pdp_2d_lgbm_shape | Keep | Project batched prediction averaging must reshape into both axes/grid and retain finite fit metric. |
| pdp_2d_lgbm_returns_none_for_invalid_input | Keep | 2D adapter must reject invalid grid/index/empty data before FFI or out-of-bounds access. |
| pdp_1d_lgbm_shape | Keep | Separate 1D batching must preserve grid/value count and fit metric. |
| pdp_1d_lgbm_returns_none_for_invalid_input | Keep | 1D adapter must reject invalid grid/index/empty input independently. |
| pdp_1d_lgbm_monotone_for_linear_data | Keep | Averaged PDP must follow target feature direction, not mis-batch rows/columns; fixture guards own averaging wiring, not broad LightGBM model quality. |

### `tdashboard-analysis::gaussian_process::tests::` (11 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| fit_is_deterministic | Keep | FITC/VFE/MoE seed and located-inducing configuration must produce repeatable mean/variance, a project integration responsibility. |
| moe_handles_small_n | Keep | Project panic-catching/fallback around small-sample cluster search must keep backend numerical failures from crashing caller. |
| degenerate_inputs_return_none | Keep | Invalid row count/shape/inducing budget must be rejected before backend fitting for every method. |
| duplicate_rows_do_not_break_fit | Keep | Ill-conditioned duplicate samples exercise project noise-floor retry/panic containment, not an assertion that egobox always fits them. |
| ard_theta_is_some_for_sgp_none_for_moe | Keep | Exposes correct method-specific theta payload/dimension. Removed comparative theta[0]>theta[1] assertion: backend's learned sensitivity quality is not project plumbing; wrapper directly forwards theta without reordering. |
| model_is_send_and_sync | Delete | Compile-only generic assertion has no runtime behavior or thread-safety evidence; compiler enforces actual production Send/Sync bounds and this private model has no project unsafe trait implementation to exercise. |
| select_inducing_points_includes_priority_rows | Keep | Custom mixed selection must retain every requested priority point within budget. |
| select_inducing_points_priority_exceeds_budget | Keep | Over-budget priorities must be reduced using priority population only, not arbitrary whole-input centroids. |
| select_inducing_points_empty_priority_behaves_as_before | Keep | Independent no-priority branch must return finite bounded inducing count. |
| fit_front_focused_trains_and_is_deterministic | Keep | Priority-aware selection must be connected to fitting with repeatable finite predictions. |
| fit_front_focused_equals_fit_when_n_le_max_inducing | Keep | When every training row fits budget, priority must not change deployed model. |

### `tdashboard-analysis::native_tests::` (4 originals; complete file removed)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| integration_shap_importances_sum_to_one | Delete | Same direct re-exported production call, shape and sum assertions as sensitivity::tree::shap::tests::importances_sum_to_one; different synthetic seed does not create a distinct integration path. |
| integration_mdi_importances_sum_to_one | Delete | Same direct call/shape/normalization as sensitivity::tree::mdi::tests::importances_sum_to_one; dominant feature position changes fixture only. |
| integration_rf_anova_importances_sum_to_one | Merge | Into sensitivity::tree::rf_anova::tests::dominant_feature_gets_majority_importance; retains normalization while avoiding another forest fit. |
| integration_pdp_2d_returns_result | Delete | Same compute_pdp_2d_lgbm call and 5×5 shape already checked more strongly with finite R² by lgbm::tests::pdp_2d_lgbm_shape. |

Removed the now-unused `make_xy` fixture helper with `native_tests.rs` and its
test-only module declaration. Native LightGBM coverage still executes in retained
module tests; deleting a file called native_tests does not disable native tests.

### `tdashboard-analysis::sensitivity::tree::mdi::tests::` (4 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| importances_sum_to_one | Keep | Full prepared/native/normalization path must return one importance per feature and unit/zero sum. |
| dominant_feature_ranks_first | Keep | MDI must associate gain with the correct feature index rather than merely normalize any vector. |
| empty_input_returns_empty | Keep | Direct public adapter must return empty/zero fallback for empty and merged singleton inputs. |
| single_sample_returns_empty | Merge | Into empty_input_returns_empty; retains insufficient nonempty input fixture without separate function/setup imports. |

### `tdashboard-analysis::sensitivity::tree::shap::tests::` (5 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| importances_sum_to_one | Keep | Full SHAP aggregation must normalize feature contributions with correct shape. |
| dominant_feature_ranks_first | Keep | Absolute per-feature contribution must preserve feature correspondence. |
| bias_column_excluded | Keep | Two-feature output must exclude bias column, complementing three-feature normalization fixture. |
| empty_input_returns_empty | Keep | Public adapter must handle empty and merged singleton inputs with empty/zero fallback. |
| single_sample_returns_empty | Merge | Into empty_input_returns_empty; retains insufficient nonempty input. |

### `tdashboard-analysis::sensitivity::tree::rf_anova::tests::` (4 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| dominant_feature_gets_majority_importance | Keep | Custom forest main effects must identify known dominant feature; receives native_tests normalization guarantee on same actual computation. |
| deterministic_across_repeated_calls | Keep | Tree parallelism must aggregate deterministically with fixed project seed, including holdout R². |
| constant_y_does_not_panic | Keep | Zero total variance/no tree splits must give zero importances and R² rather than divide by zero. |
| constant_feature_column_does_not_panic | Keep | Constant feature ranges must not break marginalization or steal importance from varying feature. |

### `tdashboard-analysis::contour::tests::` (12 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| interpolates_plane_inside_hull_and_masks_outside | Keep | Independent known plane checks barycentric values and no extrapolation beyond hull. |
| sparsity_guard_drops_bridging_triangle | Keep | Custom gap guard must mask unsupported bridge regions while disabled guard fills them. |
| too_few_points_returns_empty | Keep | Fewer than three observations must reject before triangulation. |
| degenerate_range_returns_empty | Keep | Zero coordinate range must not normalize by zero or fabricate grid. |
| bucketed_index_matches_linear_scan_exactly | Keep | Independent brute-force point-location traversal detects missing/reordered bucket candidates; sharing barycentric primitive is appropriate because indexed selection, not interpolation formula, is under test. |
| duplicate_points_do_not_panic | Keep | Duplicate-cleaning adapter must allow valid remaining geometry rather than fail triangulation. |
| edge_cross_detects_straddle | Keep | Marching-squares edge intersection must return correct fraction only when level is crossed. |
| cell_density_grid_bins_and_normalizes | Keep | Density overlay must preserve cells, clamp max endpoints and normalize counts. |
| box_blur_spreads_into_neighbors | Keep | Custom separable smoothing must spread into both axes without gross magnitude drift. |
| box_blur_zero_radius_is_identity | Keep | Disabled smoothing must preserve grid exactly, not blur regardless of radius. |
| contour_segments_cross_simple_gradient | Keep | Actual segment assembly must produce bounded lines for a valid gradient. |
| contour_segments_skip_masked_cells | Keep | Missing-corner cells must not be connected across unobserved regions. |

GP/LightGBM/contour/native/tree continuation: **55 additional originals**,
44 Keep, 7 Delete, 4 Merge. Numerical-backend quality assertions are distinguished
from project-owned preprocessing/selection/FFI shape/aggregation/panic handling.

### `tdashboard-analysis::multi_objective::pareto::tests::` (27 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| tc_201_01_two_obj_all_nondominated | Keep | Tradeoff points must share front zero rather than be ordered lexicographically. |
| tc_201_02_two_obj_clear_domination | Keep | Domination chain must assign successive fronts rather than only front/nonfront flags. |
| tc_201_03_four_objectives | Keep | Sorting must consider every objective and preserve nonidentity original indices. |
| tc_201_04_single_objective_all_rank1 | Keep | Product intentionally assigns all single-objective rows front zero, unlike ordinary scalar ranking. |
| tc_201_05_maximize_direction | Keep | Mixed maximize/minimize must reverse domination correctly. |
| tc_201_06_hypervolume_2d_known_value | Keep | Independent rectangle-union value protects 2D volume formula. |
| tc_201_06b_hypervolume_2d_ignores_dominated_and_duplicate_points | Keep | Historical dominated-point strip regression must not undercount volume; duplicates must not inflate it. |
| tc_201_06c_hypervolume_nd_m2_ignores_dominated_points | Keep | ND dispatcher used by EHVI must retain 2D reduction and additional nondominated candidate contribution. |
| tc_201_07_hypervolume_single_objective_none | Keep | Table Pareto API must return absent HV for scalar objective. |
| tc_201_09_hypervolume_history_single_obj | Keep | Separate legacy history API preserves IDs and zero trajectory for scalar objective, unlike newer indicator empty-trajectory policy. |
| tc_201_e02_empty_dataframe_returns_empty | Keep | Empty input must not fabricate front or HV. |
| tc_201_b01_all_same_coords | Keep | Equality is not strict domination; duplicate observations remain front zero. |
| tc_201_b02_chain_dominance | Delete | Same successive-front guarantee as tc_201_02_two_obj_clear_domination; fourth chain point adds no algorithm branch. |
| tc_201_b03_single_point | Keep | Singleton table must retain rank/front index but no multi-point HV. |
| tc_hv_5d_monotonically_nondecreasing | Keep | Incremental history/front maintenance must work beyond dedicated 2D path without decreasing cumulative volume. |
| tc_hv_nd_3d_known_value | Delete | Unit-cube singleton is weaker than hypervolume::wfg_tests::wfg_single_point_is_box_volume's nonuniform four-dimensional box through same generic singleton path. |
| tc_hv_201_11_compute_pareto_ranks_3obj_two_points_overlap | Keep | Third-dimension volume and automatically derived reference must match hand-computed 0.131; receives tighter 1e-12 precision from merged direct-kernel check. |
| hv_history_with_ref_returns_used_ref_point | Keep | Auto reference must be returned and extend beyond observed nadir. |
| hv_history_with_ref_honors_override | Keep | Manual reference must actually affect reported reference and trajectory. |
| hv_history_with_ref_ignores_wrong_dim_override | Keep | Wrong-width override must fall back instead of indexing invalid dimensions. |
| tc_201_p01_ndsort_1000_points_under_100ms | Keep | Actual 1000-row quadratic domination workload guards truncation/allocation regressions; name notwithstanding, no timing assertion or measured latency guarantee. |
| three_state_pareto_groups_keep_unverified_input_order | Keep | Feasible fronts, violation order and unverified original order must form distinct rank groups. |
| positive_with_missing_is_infeasible_and_all_unverified_has_no_front | Keep | Confirmed violation wins over missing evidence; unverified-only population must not become a verified front/HV. |
| tc_cav_01_infeasible_excluded_from_pareto_front | Keep | Infeasible geometric winner must not displace multiple feasible front members. |
| tc_cav_02_infeasible_ranked_by_constraint_sum_ascending | Keep | No feasible population must retain empty front and rank by violation rather than objectives. |
| tc_cav_03_all_infeasible_gives_empty_pareto_indices | Delete | Weaker duplicate of tc_cav_02, which checks same empty-front path plus all original rank positions. |
| tc_cav_04_no_constraints_unchanged_behavior | Keep | Unconstrained table adapter must preserve objective-only front and exclude dominated rows. |

### `tdashboard-analysis::multi_objective::pareto::hypervolume::wfg_tests::` (5 originals; complete module)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| wfg_matches_slicing_reference_on_random_fronts | Keep | Independent recursive slicing traversal checks WFG exclusive-contribution/limitset recursion across dimensions and front sizes; sharing 2D primitive does not duplicate WFG recurrence. |
| wfg_3d_two_points_hand_computed | Merge | Into pareto::tests::tc_hv_201_11_compute_pareto_ranks_3obj_two_points_overlap; same points/reference/value execute kernel through table API, retaining 1e-12 threshold. |
| wfg_unaffected_by_dominated_points | Keep | Generic >=3D reduction must ignore dominated points and merged duplicates without changing volume. |
| wfg_duplicate_points_counted_once | Merge | Into wfg_unaffected_by_dominated_points; append a duplicate after dominated-point control, retain exact volume invariance without separate setup. |
| wfg_single_point_is_box_volume | Keep | Nonuniform four-dimensional singleton verifies every box width contributes. |

### `tdashboard-analysis::multi_objective::indicators::tests::` (16 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| igd_plus_zero_when_approx_covers_reference | Keep | Multi-reference nearest-match selection must give zero when covered, not only for one point. |
| igd_plus_only_counts_worse_objectives | Keep | IGD+ must count only deficits, unlike symmetric Euclidean distance. |
| additive_epsilon_zero_when_identical | Keep | Multi-reference min/max composition must give zero for identical sets. |
| additive_epsilon_translation_amount | Keep | Fixed reference checks signed translation amount; merged negative shift must not clamp to zero. |
| additive_epsilon_can_be_negative_when_dominating | Merge | Into additive_epsilon_translation_amount; preserve negative expected epsilon alongside positive case. |
| r2_zero_at_ideal | Merge | Into r2_decreases_as_set_approaches_ideal; retain exact ideal-zero check using shared simplex weights. |
| r2_decreases_as_set_approaches_ideal | Keep | R2 must decrease toward ideal rather than return a constant; receives exact ideal boundary. |
| simplex_lattice_sums_to_one | Keep | Custom weight generation must retain dimension, normalization and bounded count across criterion widths. |
| igd_plus_histories_ignore_reference_duplicates_but_keep_trials | Keep | History reference deduplicates observations while trial trajectory and raw kernel reference multiplicity remain unchanged. |
| igd_plus_histories_match_single_study_and_repeated_comparison_union | Keep | Adding comparison copies must not reweight shared reference; each series retains original IDs/trajectory entries. |
| igd_plus_histories_preserve_nearby_distinct_nondominated_points | Keep | Exact identity, not approximate quantization, must distinguish nearby real reference vectors. |
| igd_plus_histories_treat_signed_zeros_as_equal | Keep | Reference deduplication must treat +/-0 numerically equal under either direction. |
| histories_shared_reference_make_series_comparable | Keep | Distinct populations must use union reference and shared scale. Replaced weak final <= assertion with independent exact four-step expectations, so separate per-series normalization cannot pass. |
| single_objective_returns_empty_values | Keep | New indicator API intentionally returns no scalar-objective trajectory. |
| hypervolume_ref_point_bounds_all_observed_points | Keep | Early dominated trials must still fall inside shared reference, not collapse to zero when reference is derived only from final front. |
| hypervolume_history_is_nondecreasing | Keep | Indicator-specific trajectory adapter must accumulate improvements instead of recomputing only latest point. |

Multi-objective continuation: **48 additional originals**, 41 Keep, 3 Delete,
4 Merge, in three fully reviewed test-bearing files/modules. No numerical or
feasibility policy changed. Counts at the report's earlier checkpoint will be
reconciled with this continuation before the next validation checkpoint.

### `tdashboard-analysis::pdp::tests::` (21 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| tc_803_01_pdp_monotone_positive | Keep | Project Ridge PDP must preserve trend, original-output centering and labels; consolidated midpoint/name assertions use same fit and odd 11-point grid. |
| tc_803_02_pdp_monotone_negative | Keep | Negative coefficient must reverse PDP slope rather than be absolutized. |
| tc_803_03_pdp_midpoint_equals_ymean | Merge | Into tc_803_01_pdp_monotone_positive; retains midpoint within 5% of observed mean without second fit. |
| tc_803_04_pdp_r_squared_high_for_linear | Keep | Scaled/offset single-feature fit metric must remain accurate in original units. |
| tc_803_05_empty_data_returns_empty | Keep | Singleton Ridge path must return empty grid/values and zero fit metric. |
| tc_803_06_pdp_2d_grid_shape | Keep | Separate 2D Ridge path must retain full rectangular grid and merged multifeature fit metric. |
| tc_803_07_pdp_2d_empty_data | Keep | 2D singleton rejection must not fabricate axes or surface. |
| tc_803_08_pdp_2d_r_squared | Merge | Into tc_803_06_pdp_2d_grid_shape; same correlated three-feature generator retains >0.95 fit guarantee on existing 100-row fit. |
| tc_803_09_result_names | Merge | Into tc_803_01_pdp_monotone_positive; retains exact requested objective and target parameter names. |
| tc_803_p01_pdp_1d_performance | Keep | Actual large-row/high-feature Ridge allocation path must complete; debug uses 1000, release 50k, without a measured timing claim. |
| tc_803_p02_pdp_2d_performance | Keep | Independent 2D allocation/reshape path must handle same supported large population. |
| tc_1645_01_gp_fitc_raw_grid_shape | Keep | FITC-specific raw path must fit and retain both axes/surface dimensions without Ridge fallback masking failure. |
| tc_1645_e01_insufficient_data_returns_none | Keep | Raw GP minimum of three samples differs from Ridge minimum and must be enforced. |
| tc_1652_tc_005_02_gp_fitc_n100_grid_shape | Delete | Same FITC Z=X path/10×10 shape as tc_1645_01; N=100 still does not enter inducing-point selection. |
| tc_1652_tc_005_03_gp_vfe_small_n | Keep | Separate VFE backend dispatch must produce finite raw surface. |
| tc_nfr_001_01_gp_fitc_n1000_under_10s | Delete | Ignored sparse-GP workload is subsumed by retained 5000-row/stricter release timing workload; both enter same 100-inducing-point branch, no project N=1000 special case. |
| tc_nfr_002_01_gp_fitc_n5000_under_5s | Keep | Ignored opt-in exercises sparse-GP/large-row batching/resource path; release-only timing bound is not claimed verified in ci-test. |
| gp_2d_pdp_marginalises_third_dimension | Keep | Full-dimensional averaging must keep non-target features, finite surface and GP uncertainty, with correct first-axis trend. |
| tc_1653_01_gp_fitc_dispatch_returns_finite_results | Delete | Despite name, calls same raw function, not a dispatcher. Finite shape checks already covered by raw FITC and stronger third-dimension test. |
| public_pdp_boundary_rejects_missing_features_without_ridge_imputation | Keep | Public model selection/fallback paths must all reject ragged/nonfinite measurements instead of silently imputing. |
| gp_2d_wrapper_rejects_invalid_input_at_raw_method_boundary | Keep | All GP wrappers must preserve names and empty/no-uncertainty fallback for invalid raw inputs. |

### `tdashboard-analysis::pdp::utils::tests::` (15 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| mixed_type_pdp_keeps_categories_constant_and_filters_missing_numbers | Keep | Mixed nominal/numeric extraction and public PDP composition retain source observations, constant category encoding and missing-column rejection. |
| extraction_keeps_only_complete_observed_pairs_in_source_order | Keep | Missing/nonfinite numeric cells must exclude rows while preserving exact feature/objective correspondence. |
| tc_101_01_normal_data_normalization | Keep | Project min/range matrix scaling must match independent two-column literals. |
| tc_101_e01_constant_column_clamping | Keep | Constant features must remain finite after own zero-range protection. |
| tc_102_01_y_normalization_accuracy | Keep | Own objective z-score must center values and preserve mean/shape; deliberately not generic column_mean_std convention. |
| tc_102_e01_empty_slice | Keep | Empty objective normalization uses EPSILON scale, not unit-scale generic fallback. |
| tc_201_01_perfect_prediction | Keep | Own R² must return one for exact varying target. |
| tc_201_02_constant_prediction | Keep | Mean-only predictions for varying target must yield zero R². |
| tc_201_e01_constant_y_zero_division_guard | Keep | Constant target reproduced exactly follows special R²=1 policy. |
| tc_201_e02_constant_y_bad_prediction_returns_zero | Keep | Wrong predictions for constant target must yield zero, unlike perfect constant reproduction. |
| tc_301_01_extract_xy_basic | Keep | Two numeric feature columns must retain their exact nonzero values/order and objective alignment. |
| tc_301_e01_missing_column_excludes_rows | Delete | Same absent-column rejection already checked by mixed_type_pdp_keeps_categories_constant_and_filters_missing_numbers with more complete numeric/category input. |
| extract_xy_feasible_only_filters_infeasible_rows | Delete | Same negative/zero feasible and positive-infeasible rows checked more strongly with exact x/y and both flag values by feasible_only_excludes_unverified_nonfinite_constraints. |
| feasible_only_excludes_unverified_nonfinite_constraints | Keep | Feasible-only must exclude unverified NaN/+Inf/-Inf evidence, not treat it as unconstrained; flag off retains all. |
| extract_xy_feasible_only_keeps_all_rows_without_constraint_column | Keep | Genuinely unconstrained input must remain usable with feasible-only option. |

PDP continuation: **36 additional originals**, 28 Keep, 5 Delete, 3 Merge,
in two complete test-bearing files. Ridge/GP/public routing implementations were
read. One redundant ignored workload was removed; current ignored case total
therefore decreases by one, not a newly executed/unsuppressed test.

### `tdashboard-analysis::report::builder::tests::` (21 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| reports_keep_nominal_parameters_unsupported_not_ranked | Keep | Report construction/JSON/HTML/Markdown must expose nominal categories as unsupported, not scored; receives serialized schema-version presence check. |
| trial_summary_keeps_json_attributes_and_omits_missing_keys | Keep | Mixed typed attributes, deterministic key order and present-null versus absent keys must survive trial-summary projection. |
| single_objective_outcome_and_convergence | Keep | Single-objective composition must choose correct best/top rows, convergence metric/status and discovery number; consolidated importance/finding checks share same full report. |
| single_objective_importance_orders_a_first | Merge | Into single_objective_outcome_and_convergence; same fixture retains method/objective labels, known dominant feature and score order. |
| single_objective_key_findings_present | Merge | Into single_objective_outcome_and_convergence; retains actual builder-to-finding wiring and absence of multiobjective summary. |
| finite_front_excludes_nonfinite_rows_before_nondominance | Keep | Finite selection must precede nondominance under mixed directions, preserve trial numbers/original objectives and avoid mutating source data. |
| constrained_finite_front_preserves_feasibility_and_fallback_counts | Keep | Nonfinite feasible observations must not create a front; fallback counts/labels and both renderers must distinguish verified feasibility from objective candidates. |
| nonfinite_only_reports_have_no_front_candidates | Keep | Entirely invalid objectives must yield no scatter/front/MCDM even when constraints are feasible. |
| multi_objective_pareto_and_mcdm_consensus | Keep | Actual builder must compose front/extremes/scatter/MCDM consensus/HV; receives repeat-render Markdown check without another report build. |
| skip_decision_sections_omits_mcdm_and_correlations_but_keeps_ordering | Keep | Lightweight summary mode must skip optional expensive sections without changing candidate order or findings. |
| multi_objective_trade_off_finding | Keep | Real objective anticorrelation must flow into a TradeOff finding and metrics. |
| zero_trials_does_not_panic | Keep | Empty report must have insufficient convergence, absent best/importance and render safely in both languages. |
| single_trial_robustness | Keep | Nonempty singleton keeps best trial but excludes insufficient correlation/importance. |
| nan_objective_triggers_data_quality | Keep | Actual objective validity count must drive DataQuality finding. |
| extras_produce_execution_and_pruning | Keep | All-state extras must produce execution timing/pruning and failure quality findings. |
| markdown_contains_all_sections_for_full_study | Keep | Actual Markdown must contain complete English sections and escaped name; receives Japanese headings with same report. |
| markdown_is_deterministic | Merge | Into multi_objective_pareto_and_mcdm_consensus; repeat actual custom rendering of same multiobjective report retains byte identity. |
| markdown_escapes_pipe_in_names | Delete | Weaker plain-pipe case duplicates actual rendered backslash+pipe+trailing-backslash test below. |
| markdown_escapes_backslash_before_pipe | Keep | Public renderer must invoke escaping in an order that preserves table structure, including preexisting escape characters. |
| markdown_japanese_smoke | Merge | Into markdown_contains_all_sections_for_full_study; same report retains three exact Japanese headings without another numerical build. |
| json_serialization_is_deterministic | Merge | Schema-version presence moves into reports_keep_nominal_parameters_unsupported_not_ranked. Serializing the exact unchanged object twice only rechecks serde/map iteration stability, not independent-build determinism. |

### `tdashboard-analysis::report::html::tests::` (14 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| representative_trial_html_renders_structured_user_attributes | Keep | Structured/null/bool/list/object attributes must be emitted and HTML-escaped rather than dropped/string-misinterpreted. |
| multi_objective_full_section_render | Keep | Self-contained no-JS output must include all sections/TOC/charts/MCDM; receives determinism and Japanese full-section assertions using identical report fixture. |
| single_objective_skips_mcdm_and_scatter | Keep | Inapplicable sections/TOC/chart/execution must be omitted while scalar sections remain. |
| user_strings_are_escaped | Keep | Actual HTML user-name paths must block raw script/tag injection; primitive escaper test alone would not catch a missing call site. |
| output_is_deterministic | Merge | Into multi_objective_full_section_render; same report/render path retains byte-identical repeated HTML generation. |
| japanese_headings_smoke | Merge | Into multi_objective_full_section_render; retain lang attribute and six Japanese headings with same fully populated model. |
| all_infeasible_falls_back_and_marks_violations | Keep | Both renderers/languages must display maximum finite violation rather than signed sum, and clearly label candidate fallback and extreme-trial violations. |
| unverified_candidates_are_not_a_verified_front_or_confirmed_violations | Keep | Unverified candidates must retain distinct counts/tooltips/labels without false front, violation or MCDM/convergence claims. |
| attribute_only_report_fallback_has_zero_confirmed_violations | Keep | Constraint presence with no evaluations is unverified, not unconstrained or confirmed violating; both renderers must say zero violations. |
| infeasible_trial_is_excluded_from_front_when_feasible_exist | Keep | Report-specific feasible-front recomputation must retain infeasible scatter point but omit it from table and prevent inappropriate violation marks on feasible entries. |
| duplicate_objective_trials_are_marked | Keep | Equivalent objective vectors must annotate later trial's canonical identity and render duplicate legends without dropping it. |
| feasible_only_front_has_no_infeasible_mark_or_note | Keep | Entirely feasible constrained report still has constraint column but no failure labels/notes in either renderer. |
| markdown_generated_at_includes_iso_utc | Keep | Public Markdown must render supplied timestamp deterministically as ISO UTC plus Unix seconds. |
| zero_trials_does_not_panic | Keep | Empty actual HTML must remain self-contained with sections/lang support, unlike empty model construction alone. |

### `tdashboard-analysis::report::svg::tests::` (27 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| nice_ticks_basic_range | Keep | Custom tick algorithm must cover bounds, stay finite and ascend; consolidated signed/straddling/tiny ranges retain all inputs. |
| nice_ticks_negative_range | Merge | Into nice_ticks_basic_range; preserves negative range/floor behavior. |
| nice_ticks_straddling_zero | Merge | Into nice_ticks_basic_range; preserves range crossing zero. |
| nice_ticks_tiny_range | Merge | Into nice_ticks_basic_range; preserves subunit exponent/finite behavior. |
| nice_ticks_degenerate_min_equals_max_nonzero | Keep | Renamed nice_ticks_degenerate_ranges; constant ranges must be padded into usable ticks, including separate zero padding rule. |
| nice_ticks_degenerate_min_equals_max_zero | Merge | Into nice_ticks_degenerate_ranges; retain zero-centered fallback input and bounds. |
| nice_ticks_integer_produces_distinct_integers | Keep | Rounded tick adapter must deduplicate, not rely solely on increasing float ticks. |
| nice_ticks_integer_degenerate | Keep | Integer adapter must return ordered containing sequence for constant observation. |
| escape_xml_handles_all_special_chars | Keep | Own XML replacement order must escape all five text/attribute characters exactly once. |
| hbar_chart_escapes_malicious_label | Keep | Actual bar-label rendering must call safe escaper, not merely provide a working primitive. |
| fmt_sig4_basic_cases | Keep | SVG has separate formatter (not shared format_number); its sign/rounding/trim policy needs independent expectations. |
| line_chart_mark_and_title_counts | Keep | Improvement/final markers and native browser tooltip titles must exist, follow CSS theme tokens, and not hard-code colors. |
| line_chart_final_point_already_improvement_no_duplicate_marker | Keep | Final marker must not duplicate an existing improvement mark/title. |
| line_chart_empty_points_no_panic | Keep | Empty chart must explicitly render no-data output. |
| scatter_chart_mark_and_title_counts | Keep | Two-series scatter must preserve data marks, staircase, legend/title semantics and dynamic theme colors. |
| scatter_chart_no_legend_when_single_series | Keep | Single-series legend omission and mark count are separate renderer policy. |
| scatter_chart_single_front_point_no_staircase_path | Keep | Singleton front must not fabricate a connecting path. |
| scatter_chart_marks_infeasible_in_tooltip | Keep | Only confirmed infeasible points get violation labels, with correct trial/value correspondence. |
| scatter_chart_unverified_has_distinct_color_label_and_no_false_front | Keep | Unverified observation must have separate token/label and never be called infeasible or a Pareto front. |
| hbar_chart_mark_and_title_counts | Keep | Every bar/label must retain tooltip and CSS theming; receives truncation/full-label preservation assertions on existing long item. |
| hbar_chart_truncates_long_label_with_ellipsis | Merge | Into hbar_chart_mark_and_title_counts; identical long label already present, so no separate chart build required. |
| histogram_mark_and_title_counts | Keep | Every histogram bin must render a mark/title using theme variables. |
| heatmap_mark_and_title_counts | Keep | Cells plus legend must render corresponding tooltips and CSS variables, not disappear behind valid statistics. |
| heatmap_quantization_boundaries | Delete | Never renders a heatmap: directly repeats theme::diverging_bin_boundaries/diverging_show_label_threshold's same values. Removed now-unused test-only theme import. |
| heatmap_empty_matrix_no_panic | Keep | Empty actual renderer must produce no-data output. |
| heatmap_truncates_long_labels_with_title | Keep | Both heatmap axes must truncate display while keeping original labels in title. |
| truncate_label_respects_max_chars | Keep | Helper must respect exact maximum character count and retain short input unchanged; ellipsis presence alone would miss overlong truncation. |

### `tdashboard-analysis::report::markdown::helpers::esc_tests::` (6 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| escapes_backslash_before_pipe | Delete | Same backslash/pipe/trailing-backslash guarantees already verified through actual renderer by builder::tests::markdown_escapes_backslash_before_pipe. |
| escapes_html_amp_and_lt | Keep | Markdown converted downstream to HTML must not allow tags/entities to bypass own escaping. |
| escapes_markdown_inline_specials | Keep | User content must not become emphasis/link/heading syntax. |
| newlines_become_spaces | Keep | Newline/carriage return must not split table rows. |
| code_span_wraps_plain_names | Merge | Into code_span_handles_embedded_backticks; retains simple code fence expectation alongside special cases. |
| code_span_handles_embedded_backticks | Keep | Custom CommonMark fencing must exceed longest backtick run, pad boundary ticks and retain valid empty span. |

### `tdashboard-analysis::report::findings::tests::` (7 originals; complete module)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| convergence_insufficient_below_threshold | Keep | Sample minimum must take precedence over improvement position. |
| convergence_still_improving_when_last_update_in_tail | Keep | Inclusive 80% tail boundary must classify StillImproving. |
| convergence_converged_when_last_update_early | Keep | Earlier updates must classify Converged rather than blanket StillImproving. |
| pruning_finding_only_when_pruned_positive | Keep | Explicit zero pruning evidence must not emit misleading efficiency finding. |
| data_quality_only_when_nan_or_fail | Keep | Zero issue counts must suppress quality warning; nonzero NaN count must emit it. |
| feasibility_finding_present_only_with_constraints | Keep | Unconstrained study must not be described as partially feasible. |
| multi_objective_emits_pareto_summary_not_best_single | Keep | Finding kind selection must respect objective count and preserve tradeoff. |

### `tdashboard-analysis::report::theme::tests::` (9 originals; complete test module)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| sequential_matches_oklab_derivation | Keep | Production stores constants, not interpolation code: independent derivation catches wrong precomputed intermediate palette values, not a tautological runtime recalculation. |
| diverging_light_matches_oklab_derivation | Keep | Light diverging constants must follow documented neutral/poles/perceptual steps. |
| diverging_dark_matches_oklab_derivation | Keep | Separate dark palette must follow its own neutral/poles, not reuse light constants. |
| diverging_label_ink_light_is_always_black | Keep | Light palette contrast must support chosen direct-label ink; guards unreadable precomputed combinations. |
| diverging_label_ink_dark_extremes_are_black_middle_is_white | Keep | Dark midpoint/extreme contrast requires different ink choices. |
| diverging_bin_boundaries | Keep | Own signed quantization must map exact threshold values to intended bins. |
| diverging_show_label_threshold | Keep | Direct label visibility must use strict magnitude threshold, not >= or signed-only comparison. |
| diverging_var_and_ink_var_cover_all_bins | Keep | Every supported signed bin must resolve to color/ink variable names instead of malformed CSS reference. |
| css_variables_contains_all_tokens | Keep | Actual CSS generator must define light/dark and complete series/scale tokens used by SVG. |

### `tdashboard-analysis::report::format_tests::` (3 originals; complete module)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| integers_render_without_decimals | Keep | Shared public formatter intentionally uses plain integers rather than SVG's exponent policy. |
| four_significant_digits_no_trailing_zeros | Keep | Noninteger rounding must retain requested precision and remove irrelevant decimal padding. |
| non_finite | Keep | Report numeric text must use chosen explicit NaN/positive-/negative-infinity spellings. |

Report continuation: **87 additional originals**, 71 Keep, 3 Delete, 13 Merge,
in seven fully read test-bearing files/modules. Rendered mark counts enforce
actual report/tooltip/theme output, not test-count or coverage padding. Independent
palette reference helpers remain because production holds precomputed constants.

### `tdashboard-analysis::sensitivity::metric_trait_tests::` (22 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| tc_2259_01_spearman_metric_name | Merge | Into tc_2259_03_spearman_positive_correlation; retain exact logging identifier alongside actual payload. |
| tc_2259_02_ridge_metric_name | Merge | Into tc_2259_04_ridge_linear_data; retain exact identifier. |
| tc_2259_03_spearman_positive_correlation | Keep | DataFrame adapter must select correct signs/parameter-objective names and only populate Spearman payload; receives name check. |
| tc_2259_04_ridge_linear_data | Keep | Independent Ridge adapter must preserve labels, coefficient sign, holdout quality and only its own payload; receives name check. |
| tc_2259_05_spearman_matches_legacy | Delete | Supposed legacy API is filter_map directly calling same metric.compute; compares numerical function to itself, not two implementations. Actual orchestrator dispatch/order/None filtering remain tested. |
| tc_2259_06_ridge_matches_legacy | Delete | Same direct metric call through pass-through orchestrator; no independent beta/R² reference. |
| tc_2259_09_spearman_as_trait_object | Delete | Rust dynamic dispatch plus weaker Some/name checks duplicate actual adapter tests and production API dispatch test. |
| tc_2259_10_ridge_as_trait_object | Delete | Same compiler trait-object guarantee and duplicated adapter result. |
| tc_2259_11_multiple_metrics_vector_dispatch | Delete | Indexes a test-owned vector and invokes methods manually; actual compute_sensitivity_single_obj dispatch/order tested by sensitivity::tests::tc_2263_01. |
| tc_2259_12_spearman_obj_idx_1 | Keep | Nonzero objective selection must retain requested objective identity and parameter width. |
| tc_2259_13_ridge_obj_idx_1 | Keep | Separate Ridge objective-index adapter must preserve requested identity/coefficient width. |
| tc_2259_14_spearman_insufficient_data_n1 | Keep | Spearman adapter must report unavailable before numerical kernel's raw short-input zero sentinel. |
| tc_2259_15_ridge_insufficient_data_n1 | Keep | Separate Ridge adapter must reject insufficient observations. |
| tc_2259_16_spearman_empty_data_n0 | Keep | Empty decoded table must return unavailable, not a fabricated empty success. |
| tc_2259_18_spearman_invalid_obj_idx | Keep | Invalid objective selection must fail safely in Spearman adapter. |
| tc_2259_19_ridge_invalid_obj_idx | Keep | Separate Ridge adapter must fail safely for invalid selection. |
| tc_2259_20_spearman_empty_params | Keep | Objective-only table cannot produce numerical sensitivity. |
| tc_2259_22_spearman_min_rows_n2 | Keep | Minimum valid pair must remain available with correct coefficient. |
| tc_2259_23_ridge_min_rows_n2 | Keep | Minimum valid Ridge data must return intended payload rather than imposing tree-model row minimum. |
| tc_2259_27_spearman_large_data | Keep | Multi-column DataFrame adapter must preserve all five result rows and finite bounded scores, including pairwise exclusion of ln(0). |
| tc_2259_29_spearman_send_sync | Delete | SensitivityMetric already requires Send+Sync; compiler cannot accept its implementation without these bounds. |
| tc_2259_30_ridge_send_sync | Delete | Same trait-superbound compile-time redundancy. |

Removed redundant per-line annotation/comment scaffolding while consolidating
this test-only file; surviving fixtures/assertions retain their original data.
Unused compute_sensitivity_single_obj import disappears with self-comparisons.

### `tdashboard-analysis::sensitivity::tree_metric_tests::` (20 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| tc_2260_01_rf_anova_metric_name | Merge | Into tc_2260_05_rf_anova_compute_valid; retain exact identifier. |
| tc_2260_02_mdi_metric_name | Merge | Into tc_2260_06_mdi_compute_valid; retain exact identifier. |
| tc_2260_03_shap_metric_name | Merge | Into tc_2260_07_shap_compute_valid; retain exact identifier. |
| tc_2260_04_permutation_metric_name | Merge | Into tc_2260_08_permutation_compute_valid; retain exact identifier. |
| tc_2260_05_rf_anova_compute_valid | Keep | Actual tree adapter must preserve names/shape/R² width and route only RF-ANOVA payload. |
| tc_2260_06_mdi_compute_valid | Keep | MDI has distinct wrapping arm; no other metric may fill its result field. |
| tc_2260_07_shap_compute_valid | Keep | SHAP has distinct wrapping arm and must not accidentally route MDI/PFI data. |
| tc_2260_08_permutation_compute_valid | Keep | Permutation output has distinct payload routing, not compiler-guaranteed merely because traits compile. |
| tc_2260_09_rf_anova_insufficient_data_n1 | Keep | Renamed tree_metrics_reject_invalid_or_insufficient_inputs; shared blanket implementation tested for every metric against singleton/empty/invalid objective fixtures. |
| tc_2260_10_mdi_insufficient_data_n1 | Merge | Into tree_metrics_reject_invalid_or_insufficient_inputs; identical singleton fixture, MDI invocation retained. |
| tc_2260_11_shap_insufficient_data_n1 | Merge | Into tree_metrics_reject_invalid_or_insufficient_inputs; SHAP invocation retained. |
| tc_2260_12_permutation_insufficient_data_n1 | Merge | Into tree_metrics_reject_invalid_or_insufficient_inputs; PFI invocation retained. |
| tc_2260_13_all_metrics_invalid_obj_idx | Merge | Into tree_metrics_reject_invalid_or_insufficient_inputs; same 20-row/obj_idx=5 check retained for all four metrics. |
| tc_2260_14_all_metrics_empty_data_n0 | Merge | Into tree_metrics_reject_invalid_or_insufficient_inputs; same empty table retained for all metrics. |
| tc_2260_15_rf_anova_matches_new_api | Delete | Pass-through orchestrator invokes exact same metric.compute on same data; repeated training compares no independent implementation. |
| tc_2260_16_mdi_matches_new_api | Delete | Same self-comparison through filter_map; actual payload dispatch remains checked. |
| tc_2260_17_shap_matches_new_api | Delete | Same self-comparison, not compatibility coverage. |
| tc_2260_18_permutation_matches_new_api | Delete | Same self-comparison, not a separate numerical path. |
| tc_2260_19_all_as_trait_objects | Delete | Manual test-owned vector iteration and duplicate name/Some checks exercise compiler dynamic dispatch, not product orchestration. |
| tc_2260_20_send_sync | Delete | Trait's Send+Sync superbound already enforces every implementation. |

### `tdashboard-analysis::sensitivity::ridge::tests::` (14 originals; complete test module)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| tc_2265_01_compute_xtx_matrix_diagonal | Keep | Independent known products guard column-major indexing; consolidated regularization must affect diagonal only. |
| tc_2265_02_compute_xtx_matrix_ridge_regularization | Merge | Into tc_2265_01_compute_xtx_matrix_diagonal; reuse exact unregularized result and retain diagonal delta/off-diagonal invariance. |
| tc_2265_03_compute_xty_vector | Keep | Independent dot product catches wrong data orientation/target association. |
| tc_2265_04_compute_r_squared_perfect_fit | Keep | Own centered coefficient scoring must return one on exact prediction. |
| tc_2265_05_compute_r_squared_zero_variance | Keep | Constant target reproduced exactly has chosen R²=1 convention. |
| tc_2265_05b_compute_r_squared_zero_variance_nonzero_residual | Keep | Constant target not reproduced must return zero, not automatic perfect fit. |
| tc_2265_06_orchestrator_matches_original_behavior | Delete | Does not compare orchestration to original; only nonempty beta/bounded R², weaker than retained linear-model fit tests. |
| tc_103_01_ridge_beta_and_r2_consistent_with_linear_data | Keep | End-to-end standardization/normal-equation adapter must fit known two-feature relationship and positive coefficient. |
| tc_103_b01_ridge_empty_params_returns_empty | Keep | Internal vector entry must return empty/zero fallback for empty input. |
| tc_103_01_solve_ridge_normal_equations_correct | Delete | Merely checks finite coefficient count from SPD solve (zero coefficients also pass); end-to-end linear fit already requires correct nontrivial solution. |
| tc_ridge_holdout_01_nan_row_matches_clean_subset | Keep | Renamed ridge_holdout_filters_nonfinite_rows_before_split; both NaN/Inf exact original fixtures retain clean beta/R² equality with one shared clean fit. |
| tc_ridge_holdout_02_inf_row_matches_clean_subset | Merge | Into ridge_holdout_filters_nonfinite_rows_before_split; retains Inf in another column and insertion position. |
| tc_ridge_holdout_03_deterministic_across_calls | Keep | Seeded sampling/shuffle/holdout must retain bitwise beta/R² repeatability. |
| tc_ridge_holdout_04_r_squared_is_out_of_sample | Keep | Original bounded-score assertion could not detect training leakage. Existing test now supplies contradictory held-out targets and checks positive train-only coefficient plus exact clipped-zero eval R². |

### `tdashboard-analysis::sensitivity::tests::` (42 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| finite_selection_retains_method_specific_requirements | Keep | Spearman pairwise versus Ridge complete-case versus Sobol all-objective requirements and raw short/ragged sentinels must remain distinct. |
| sobol_complete_case_selection_keeps_objectives_and_categories_aligned | Keep | Filtering must preserve all objectives/categories while ignoring non-required invalid constraints. |
| tc_801_01_spearman_perfect_positive | Keep | Raw sensitivity kernel must return known positive correlation. |
| tc_801_02_spearman_perfect_negative | Keep | Raw kernel must preserve negative sign, not absolute importance. |
| tc_801_03_spearman_known_example | Keep | Independent nontrivial expected 13/35 protects ranking/covariance beyond extreme +/-1 cases. |
| tc_801_04_spearman_tied_ranks | Keep | Tied observations must remain strongly correlated with average-rank handling. |
| tc_801_05_spearman_n_less_than_2_returns_zero | Keep | Raw short input returns zero, unlike fewer-than-two finite pairs after filtering. |
| tc_801_16_spearman_nan_pairs_filtered_pairwise | Keep | Invalid observation on either side must be removed before ranking without shifting partners. |
| tc_801_17_spearman_inf_pairs_filtered_pairwise | Keep | Both signs of infinity must be removed as well as NaN. |
| tc_801_18_spearman_fewer_than_two_valid_pairs_returns_nan | Keep | Invalid-pair exhaustion must remain NaN/unavailable, not short-input zero sentinel. |
| tc_801_06_ridge_perfect_linear_r_squared_near_1 | Keep | In-sample generic primitive must fit scaled/offset scalar target accurately. |
| tc_801_07_ridge_beta_sign_correct | Delete | Positive beta already checked more strongly in ridge::tests::tc_103_01 and metric_trait_tests::tc_2259_04 alongside actual fit quality. |
| tc_801_08_ridge_two_params_identifies_stronger | Keep | Standardized coefficient magnitude must identify stronger contribution, not only sign/shape. |
| tc_801_09_ridge_empty_returns_zero_r_squared | Delete | Identical call and assertions to ridge::tests::tc_103_b01_ridge_empty_params_returns_empty. |
| tc_801_11b_sensitivity_categorical_param_unsupported | Delete | Weaker category-only fixture duplicates nominal_categories_never_receive_spearman_or_ridge_scores; actual orchestrator order is independently tested by tc_2263_01. |
| tc_801_14_rf_anova_importances_sum_to_one_per_objective | Keep | DataFrame/trait/orchestrator result must retain normalized values after per-feature/objective reshaping, not only correct raw vector. |
| tc_801_15_rf_anova_small_dataset_non_zero | Keep | Small holdout/forest setup must not collapse to all-zero fallback when valid data remains. |
| tc_801_p01_spearman_50000_x_30_x_4_at_scale | Keep | Large raw ranking allocations and all parameter/objective pairs must produce known coefficients; debug uses smaller documented fixture. |
| tc_801_p02_ridge_50000_x_30_at_scale | Keep | Supported large multifeature solves must retain one coefficient per parameter across objectives. |
| tc_1610_01_build_quad_features_output_length | Keep | Own quadratic/interacting feature basis is required; strengthened length-only assertion to exact independently known feature vector/order. |
| tc_301_06_sobol_regression_after_seeded_rng_migration | Delete | Exact same 50-row/two-feature/1024-sample input and range assertions as tc_1610_04; migration history adds no guarantee. |
| tc_1610_03_compute_sobol_insufficient_data_returns_none | Keep | Sobol's minimum observations must reject singleton input. |
| tc_1610_03b_compute_sobol_zero_n_samples_returns_none | Merge | Into tc_1610_04_sobol_indices_in_range; identical table now checks zero sample rejection before valid quadrature. |
| tc_1610_04_sobol_indices_in_range | Keep | Sampling/estimator clamping must yield valid first/total indices; consolidated zero-budget guard retained. |
| tc_1610_05_compute_sobol_index_pair_zero_variance | Keep | Zero output variance must produce zero indices instead of division failure. |
| tc_1610_06_compute_sobol_index_pair_known_values | Keep | Hand-derived estimator values must enforce total>=first and clamp to unit interval. |
| tc_1610_integration_output_shape | Keep | Multi-parameter Sobol adapter must preserve parameter/objective matrix dimensions. |
| tc_pfi_001_01_normal_case | Keep | Actual train/permute/evaluate/normalize pipeline must produce nonzero normalized feature effects. |
| tc_pfi_001_02_single_feature | Keep | Single useful feature must receive unit importance rather than degenerate empty/zero result. |
| tc_pfi_001_e03_nan_filtering | Delete | Allows all-zero fallback, so does not prove filtering; shape/normalization already checked and shared complete-case pipeline has exact contaminated/clean alignment tests. |
| tc_pfi_001_b01_min_valid_rows | Keep | Minimum valid two-row direct PFI input must preserve one-feature result. |
| tc_pfi_001_e02_empty_input | Keep | PFI public entry must retain empty/zero fallback for empty input. |
| tc_pfi_int_01_single_obj_returns_some | Delete | Identical fixture/API call as tc_pfi_int_02, whose nonempty/unwrap and shape assertions are strictly stronger. |
| tc_pfi_int_02_result_shape | Keep | PFI orchestrator result must retain per-parameter width and per-objective R². |
| tc_2263_01_multiple_metrics_all_returned | Keep | Actual product orchestrator must retain metric order and corresponding payloads. |
| tc_2263_02_none_excluded_from_results | Keep | Product's explicit unavailable-result exclusion must not fabricate placeholders. |
| tc_2263_03_invalid_obj_idx_excluded | Keep | Orchestrator must forward requested objective index rather than silently substitute zero. |
| tc_2263_04_empty_metrics_vec_returns_empty | Delete | Orchestrator is direct standard iter/filter_map/collect; empty-vector iteration adds no guarantee beyond compiler/iterator behavior. |
| public_ridge_rejects_non_finite_features_and_objectives | Keep | Raw faer fitting boundary must reject nonfinite measurements before standardization/solve, distinct from sensitivity's finite-row selection. |
| missing_parameters_do_not_change_sensitivity_of_complete_rows | Keep | Both DataFrame metric adapters must exclude absent numeric parameter cells rather than zero-impute and alter fitted results. |
| nominal_categories_never_receive_spearman_or_ridge_scores | Keep | Relabeling/order/numeric-looking category identity must never alter numerical model or fabricate category scores. |
| numerical_sensitivity_matches_existing_primitives_and_keeps_nan_supported | Keep | DataFrame extraction/typed result must wire exact numeric primitives and distinguish invalid numeric data from unsupported categorical data. |

### `tdashboard-analysis::sensitivity::tree::common::split_tests::` (3 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| complete_case_selection_preserves_alignment_sampling_and_shuffle | Keep | Complete-case selection must precede seeded downsampling/shuffle with exact target alignment and method-specific invalid-row behavior. |
| pipeline_keeps_short_and_mismatched_input_rejection | Keep | Public shared pipeline must reject invalid shape/count before invoking fitter. |
| holdout_always_leaves_at_least_two_eval_rows | Keep | Four/five-row holdout must retain >=2 train/eval observations, avoiding spurious zero R² from singleton eval. |

### `tdashboard-analysis::sensitivity::tree::fanova::tests::` (2 originals; complete test module)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| analytic_two_split_tree_matches_hand_computation | Keep | Hand-computed leaf masses, total/main-effect variances and fractions guard actual custom marginalization math; fixed tree is mathematical input, not a mock testing itself. |
| cached_marginal_weights_match_naive_recomputation | Keep | Independent uncached traversal over trained 3D forest checks cached leaf/dimension indexing and bitwise aggregation beyond fixed analytic two-feature tree. |

Sensitivity continuation: **103 additional originals**, 67 Keep, 22 Delete,
14 Merge, six fully reviewed test-bearing files. The two verbose metric adapter
files were rewritten with the same surviving fixtures/assertions, consolidated
name/readiness checks and no self-comparison/compile-only scaffolding. Shared
production trait bounds already enforce Send+Sync; there is no new abstraction.

Count correction: the earlier checkpoint claimed 25 originals in
clustering::tests, but its complete table has **24** (one extra was counted in
the summary, not an omitted test). Clustering's total is therefore 46, not 47;
the earlier 637-definition reviewed count was 636. Runtime baseline/final counts
and deletion counts were unaffected. Current totals above use actual table rows.

### `tdashboard-analysis::surrogate_opt::progress::tests::` (4 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| default_is_not_cancelled_and_empty | Delete | Constructor delegates to derived Default on Arc/atomics/String; standard initial values are rechecked. Retained actual cancellation/counter/subtask tests also require correct initialization. |
| subtask_shares_cancellation_but_not_counters_or_stage | Keep | Custom subtask must share cancellation while isolating fit counters/stage from adaptive evaluation. |
| cancel_is_observed_through_clone | Keep | Public cancellation handle operations must be observable through another retained handle. |
| progress_counters_update | Keep | Actual stage/counter mutation and snapshots must preserve completed count rather than overwrite or increment wrong field. |

### `tdashboard-analysis::surrogate_opt::feasibility::tests::` (5 originals; complete file)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| gp_feasibility_matches_normal_cdf_exactly | Keep | Actual probability kernel must turn known constraint mean/variance into correct CDF argument; analytic model is mathematical input, not a mock tested alone. |
| p_feas_saturates_at_extremes | Keep | Extreme violated/satisfied constraints must not invert or overflow probabilities. |
| multiple_constraints_multiply | Keep | Joint probability must multiply independent constraints rather than add/average; receives empty neutral-element check. |
| no_variance_uses_hard_indicator | Keep | Variance-free model must use correct signed feasibility boundary. |
| empty_models_returns_one | Merge | Into multiple_constraints_multiply; retains unconstrained neutral probability without separate setup. |

### `tdashboard-analysis::surrogate_opt::ard::tests::` (3 originals; complete file)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| ard_rejects_absent_numeric_parameters | Keep | DataFrame ARD entry must reject missing measurements, not fabricate a trained sensitivity. |
| ard_importance_from_df_wires_through_gp | Keep | Public ARD extraction must align parameter identities with normalized importance and finite validation metric; asymmetric response detects swapped columns. |
| ard_importance_from_df_out_of_range_objective | Keep | Invalid requested objective must fail safely before fitting. |

### `tdashboard-analysis::surrogate_opt::acquisition::tests::` (10 originals; complete test module)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| normal_cdf_at_zero | Keep | Renamed normal_cdf_matches_known_probabilities; custom approximation must match independent midpoint/positive/negative quantiles. |
| normal_cdf_at_1_96 | Merge | Into normal_cdf_matches_known_probabilities; retains 0.975 literal and tolerance. |
| normal_cdf_negative_1 | Merge | Into normal_cdf_matches_known_probabilities; retains negative-tail literal. |
| ei_zero_when_sigma_tiny_and_mu_above_fbest | Keep | Degenerate-variance EI must not propose spurious positive improvement. |
| ei_grows_with_sigma_at_fixed_mu | Keep | Exploration term must reward uncertainty instead of ignore/invert sigma. |
| single_ei_candidate_targets_known_minimum_with_std | Keep | Actual acquisition optimization must target known surface minimum and return correctly converted score/std/bounds. |
| batch_3_candidates_pairwise_diverse | Keep | Constant-Liar refits must produce distinct candidates; receives repeated-batch determinism assertions using same actual trained model. |
| batch_3_deterministic_across_two_runs | Merge | Into batch_3_candidates_pairwise_diverse; retain two complete proposal calls, eliminating redundant GP fit/third proposal call. |
| maximize_steers_away_from_minimum | Keep | Direction sign conversion must steer acquisition toward higher response. |
| non_gp_model_returns_error | Keep | Missing posterior variance must produce actionable GP requirement, not fabricate EI uncertainty. |

### `tdashboard-analysis::surrogate_opt::ehvi::tests::` (8 originals; complete test module)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| single_candidate_in_box_with_stds | Keep | Actual EHVI result must retain feature bounds, nonnegative score, per-objective means/stds and merged seeded repeatability. |
| deterministic_across_two_runs | Merge | Into single_candidate_in_box_with_stds; preserve repeated parameter/score comparisons on identical analytic fixture. |
| batch_3_candidates_pairwise_diverse | Keep | Multiobjective Constant-Liar refits must avoid repeated candidates. |
| suggested_beats_worst_observed_point | Keep | Proposal optimizer must improve on a concrete baseline acquisition score, not only return a valid shape. |
| ehvi_matches_naive_clone_reference | Keep | Separate per-sample front-cloning reference detects in-place buffer restoration/accumulation errors across repeated probe points. |
| non_gp_models_return_error | Keep | Every objective requires usable posterior variance; no-variance models must reject. |
| mixed_min_max_valid | Keep | Real fitted batch path with mixed direction must retain finite nonnegative EHVI and valid bounds. |
| errors_on_empty_and_mismatch | Keep | Empty objectives, zero candidate budget and direction arity must fail before optimization. |

### `tdashboard-analysis::surrogate_opt::robustness::tests::` (14 originals; complete test module)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| same_seed_is_reproducible | Merge | Into result_shape_and_ordering; identical trained fixture retains exact sample/mean repeatability with two analyses. |
| zero_noise_collapses_to_nominal | Keep | Disabled input noise must yield nominal-only samples, zero dispersion and no clipping. |
| result_shape_and_ordering | Keep | Actual sampled result must preserve sample count/percentile order and absent constraints/spec metrics; consolidated repeatability reuses model. |
| feasibility_rate_present_with_constraints | Keep | Constraint model predictions must produce meaningful signed feasible rates, not just Some placeholder. |
| center_near_bound_reports_clipping | Keep | Perturbations beyond declared bound must report clipping. |
| dimension_mismatch_errors | Keep | Wrong center dimension must reject before prediction/indexing. |
| surface_slice_at_returns_expected_grid | Keep | Separate slice-at-center API must retain rectangular output and reject invalid dimensions/identical axes. |
| spec_metrics_none_without_limits | Merge | Into result_shape_and_ordering; same unconstrained/no-limits fixture retains absent success/sigma/Cpk. |
| success_rate_and_sigma_level_with_limits | Keep | Spec success fraction, finite empirical sigma clamping and one-sided Cpk must respond correctly to limits. |
| two_sided_limits_and_cpk_direction | Keep | Cpk must use closer side and reject reversed specification bounds. |
| distributions_share_scale_but_differ_in_shape | Keep | Normal/Uniform/Weibull must share chosen noise scale but retain different tails after project standardization. |
| weibull_shape_out_of_range_errors | Keep | Invalid shape must fail without generating unusable perturbations. |
| weibull_standardization_is_zero_mean_unit_std | Keep | In-house Weibull sampling/gamma-based standardization must recover unit scale across shapes, not a dependency RNG guarantee. |
| percentile_linear_interpolation | Merge | Into statistics::boxplot::tests::quantile_interpolates; test directly calls shared quantile, not robustness. Existing midpoint plus moved min/max endpoint expectations retain same generic policy. |

### `tdashboard-analysis::surrogate_opt::optimizers::cma_es::tests::` (6 originals; complete test module)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| stop_hook_preserves_uninterrupted_surrogate_trajectory | Delete | cma_es_minimize directly delegates to cma_es_minimize_until with ||false, exactly the compared call. Cannot detect changed historical trajectory; actual convergence/cancellation boundaries remain tested. |
| symmetric_eigen_known_2x2 | Delete | faer guarantees eigenvalues/orthogonality; retained 3D reconstruction checks project's eigenvector/value layout adapter more meaningfully. |
| symmetric_eigen_reconstructs_matrix | Keep | Conversion of faer columns/eigenvalues to nested arrays must preserve matching orientation for covariance updates. |
| next_gauss_has_zero_mean_unit_variance | Keep | CMA owns spare-sample Box-Muller implementation; distribution scaling/spare handling is not rand's standard distribution. |
| cma_es_minimizes_sphere_2d | Keep | In-house optimizer must converge to known nonzero target in normalized box. |
| cma_es_minimizes_elliptic_5d | Keep | Covariance adaptation must handle ill-conditioned multivariate objective, unlike isotropic sphere alone. |

### `tdashboard-analysis::surrogate_opt::optimizers::nsga2::tests::` (7 originals; complete test module)

| Original function | Decision | Specific reason |
| --- | --- | --- |
| dominates_requires_strict_improvement | Keep | Shared helper used here must not treat equality or tradeoff as domination; Pareto sorter has a separate hot-loop implementation. |
| non_dominated_sort_splits_fronts | Keep | Optimizer's own sorting must preserve true fronts and original population indices. |
| crowding_distance_boundaries_are_infinite | Keep | Selection must preserve boundary individuals while interior distance stays finite. |
| sbx_and_mutation_stay_in_unit_box | Keep | Custom genetic operators must clamp offspring/perturbations to valid normalized domain. |
| nsga2_minimizes_sphere_single_objective | Keep | In-house scalar NSGA-II configuration must converge, not merely return nondominated arrays. |
| config_selects_eta_by_objective_count | Keep | Deliberate objective-dependent crossover configuration must be selected, not universal scalar tuning. |
| nsga2_two_objective_front_spans_tradeoff | Keep | Multiobjective search must preserve both extremes/diversity of known tradeoff. |

### `tdashboard-analysis::surrogate_opt::tests::` (59 originals; complete 1877-line original file read through EOF)

| Original function | Decision | Specific reason / Merge destination |
| --- | --- | --- |
| lbfgs_finds_exact_quadratic_minimum | Keep | Analytic input isolates actual optimizer adapter, result/std conversion and best observation; receives absent unconstrained fields. |
| random_search_finds_quadratic_minimum_loosely | Keep | Independent optimizer dispatch must search correct direction/domain. |
| nsga2_finds_quadratic_minimum | Keep | Trained-model NSGA-II dispatch/normalization must reach known target, distinct from raw optimizer API. |
| cma_es_finds_quadratic_minimum | Keep | CMA model-consuming adapter must reach correct real parameter target. |
| cma_es_maximize_finds_quadratic_maximum_corner | Keep | CMA-specific maximize/start-point sign path must reach correct corner. |
| maximize_direction_finds_quadratic_maximum_corner | Keep | L-BFGS-specific maximize path must reach correct corner. |
| no_variance_model_reaches_box_corner | Keep | Variance-free model remains optimizable with absent predicted std. |
| slice_grid_passes_through_optimum_and_has_expected_shape | Keep | Actual slice generator must map parameter grid to known function and variance values without index/unit mismatch. |
| invalid_slice_params_are_ignored | Keep | Invalid optional same-axis slice must not invalidate an otherwise valid optimization. |
| gp_fitc_runs_and_finds_minimum_region | Keep | One-shot real FITC fit/normalization/optimization must work without validation workflow or analytic fixture hiding integration faults. |
| too_few_trials_returns_error | Keep | One-shot input minimum must reject before fitting. |
| non_finite_input_returns_error | Keep | One-shot fitting rejects corrupt measurements instead of silently cleaning/imputing. |
| trained_surrogate_is_send_sync | Delete | Compile-only type assertion provides no runtime safety evidence; actual background/parallel bounds are compiler-enforced. |
| validate_surrogate_deterministic_with_same_seed | Keep | Project seeded CV/holdout assignment and OOF aggregation must be reproducible. |
| validate_surrogate_minimum_size_dataset | Keep | Minimum sample validation must preserve fold/test counts, every OOF pair and finite metrics. |
| fit_and_optimize_on_trained_finds_quadratic_minimum | Keep | Real staged validation/final model and optimization must agree on target, fit metric and requested slice. |
| param_importance_reflects_ard_for_gp_and_none_for_others | Keep | Model-family-specific importance extraction must normalize/align GP importance and avoid inventing Ridge/LGBM/MoE ARD. |
| gp_vfe_trains_and_predicts_finite_with_std | Keep | Independent real VFE fit/deployment must retain uncertainty and finite metrics. |
| gp_moe_trains_and_predicts_finite_with_std | Keep | Independent mixture backend on suitable non-smooth data must deploy with finite uncertainty/metrics. |
| multi_opt_front_spans_full_tradeoff | Keep | Actual model-consuming front must cover known extremes; receives point/value correspondence, sort order, R² width and absent slices on same result. |
| multi_opt_front_sorted_by_first_objective | Merge | Into multi_opt_front_spans_full_tradeoff; identical analytic optimization now checks ordering without another NSGA-II run. |
| multi_opt_point_dimensions_match | Merge | Into multi_opt_front_spans_full_tradeoff; retains all per-point dimensions/exact analytic values and R² count. |
| multi_opt_maximize_objective_direction | Keep | Strengthened old any-high-f2 check (also satisfied by minimize front) to require nonempty front near jointly optimal x0=0 when maximizing f2. |
| multi_opt_slices_returned_for_each_objective | Keep | Requested multiobjective slices must exist per objective with correct grids. |
| multi_opt_no_slices_without_slice_params | Merge | Into multi_opt_front_spans_full_tradeoff; same None request retains no-slices policy. |
| multi_opt_error_on_single_objective | Keep | One-shot multiobjective entry must reject scalar problem. |
| multi_opt_error_on_ys_length_mismatch | Keep | Objective labels and value columns must agree. |
| multi_opt_error_on_too_few_trials | Keep | Multiobjective fitting minimum must fail before models are trained. |
| staged_multi_opt_error_on_single_trained | Keep | Separate staged API validates model count; reuses existing analytic fixture instead of expensive irrelevant GP fits before rejection. |
| staged_multi_opt_error_on_minimize_length_mismatch | Keep | Separate staged direction arity check likewise needs no real fit; existing analytic model fixture suffices. |
| staged_multi_opt_matches_one_shot_result | Keep | Distinct staged/one-shot fitting orchestration must preserve deployed fronts/parameters/metrics with real backend models. |
| fit_multi_surrogates_runs_end_to_end | Keep | Multi-fit API must deploy finite models usable by staged optimization, including front-focused fit orchestration. |
| fit_multi_surrogates_validates_lengths | Keep | Shared input labels/columns and observation counts must be validated before fitting. |
| lgbm_fit_validate_and_optimize_finds_minimum_region | Keep | Native piecewise-constant model must work with appropriate search and absent uncertainty, unlike gradient-capable GP path. |
| lgbm_multi_opt_returns_front | Keep | Native multiobjective deployment must retain per-point dimensions/finite values and useful front. |
| constrained_opt_pushes_x_toward_feasible_region | Keep | Actual constraint penalty must shift known optimum to boundary and expose correct predicted constraint/probability. |
| unconstrained_opt_finds_true_minimum_near_0_3 | Merge | Into lbfgs_finds_exact_quadratic_minimum; same input/spec retains optimum and additional absent constraint/probability fields. |
| constrained_fit_validation_succeeds | Keep | Real objective/constraint fitting must retain names/models and row-aligned observed constraint values. |
| suggest_candidates_constrained_p_feas_present | Keep | Real Constant-Liar constraint refits must retain feasible probability/predictions and repeatable proposals. |
| suggest_candidates_unconstrained_p_feas_none | Keep | Acquisition candidate schema must omit constraint fields without constraints (distinct from optimizer-result schema). |
| model_choice_uses_global_best_and_inclusive_absolute_tolerance | Keep | Explicit preference policy must compare global best, include 0.01 boundary and handle negative/nonfinite scores. |
| model_choice_rejects_all_invalid_and_empty_scores | Keep | No valid candidate must return failure rather than arbitrary default model. |
| eligible_gp_choice_uses_only_eligible_best_score | Keep | Eligibility subset must determine its own best/preference, not be biased by ineligible model scores. |
| explicit_gp_fit_rejects_overflow | Keep | Finite input whose normalization overflows must fail deployment. |
| explicit_single_gp_fits_support_ei_without_auto | Keep | Manual FITC/VFE must deploy requested GP/constraints/progress independently of general Auto choosing Ridge, and remain cancellable/EI-capable. |
| select_best_model_picks_gp_on_nonlinear_smooth | Merge | Into fit_surrogate_with_validation_auto_select_end_to_end; same 50-row fixture retains candidate count/GP winner and GP score beating Ridge while checking actual deployment. |
| select_best_model_picks_ridge_on_linear | Keep | Actual CV evaluation must apply near-best Ridge preference on linear data, not always pick GP. |
| fit_surrogate_with_validation_auto_select_end_to_end | Keep | Auto placeholder must be replaced by actually selected model and attached report; receives nonlinear score comparison without extra CV pass. |
| select_best_model_is_deterministic | Keep | Actual four-model CV ordering/scores must remain bit-identical with fixed seed. |
| subsample_returns_none_when_within_cap | Keep | Within-budget fit must avoid unnecessary sampling/correspondence changes. |
| subsample_single_keeps_both_extremes_and_caps_size | Keep | Scalar sampling must retain both objective tails and bounded unique source indices. |
| subsample_is_deterministic | Keep | Seeded fill selection on tied objectives must remain reproducible. |
| subsample_multi_keeps_pareto_front | Keep | Strengthened chance-prone any-front check to retain every front row when fixture front fits explicit elite quota. |
| fit_tracked_reports_progress_to_completion | Keep | Actual validation/final-fit work must complete planned progress/stage; receives pre-cancelled fresh-handle zero-work check using same request. |
| fit_tracked_cancel_before_start_returns_err | Merge | Into fit_tracked_reports_progress_to_completion; retains fresh cancelled handle, error and zero completed work without duplicate request setup. |
| fit_multi_tracked_cancel_returns_err | Keep | Separate multi-fit orchestration must reject pre-cancellation. |
| fit_with_validation_subsamples_large_data | Keep | Real single-fit cap must retain exact source-row/input/target correspondence. |
| multi_fit_keeps_shared_source_rows_through_subsampling | Keep | Models for different objectives must share retained rows and objective-specific targets. |
| multi_fit_rejects_non_finite_input_before_subsampling_and_ranking | Keep | Nonfinite data must reject before sampling could hide it or Pareto sorting could panic, both below/above cap. |

Surrogate continuation: **116 additional originals**, 98 Keep, 4 Delete,
14 Merge, nine fully reviewed test-bearing files/modules. All 704 baseline
analysis test definitions are now individually accounted for. Analytic model,
constraint and placeholder-report helper bodies were also read; retained tests
exercise actual project optimizers/acquisition/slices/probability calculations,
not assertions about mock return values alone.

### `tunny-dashboard::integration::csv_projection::` (11 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| parses_colibri_like_csv | Keep | Multi-objective Colibri headers must preserve objective order while sorting parameters and defaulting directions. |
| maps_images_per_trial | Keep | Image references retain row identities; receives quoted-comma and empty-image cases in the same fixture. |
| skips_empty_image_cells | Merge | Into maps_images_per_trial; third blank image must not produce a reference. |
| computes_param_bounds_from_observed_range | Keep | Multi-column observed extrema must survive the acquired-input projection. |
| dataframe_row_count_matches_data_rows | Delete | d3_io::csv_owned_load_preserves_fixture_columns_identity_and_nonfinite_values already checks exact source, projection and dataframe row counts. |
| categorical_param_falls_back_to_label_column | Delete | The same d3_io test checks missing categorical bounds, actual string/category values and absence of a numeric column, stronger than this row-count-only smoke. |
| non_prefixed_column_becomes_user_attr | Delete | The same d3_io test checks both numeric/string attribute names, columns and original observations. |
| handles_quoted_fields_with_commas | Merge | Into maps_images_per_trial; quoted comma stays in the second image path. |
| errors_on_empty_input | Keep | Shared rejection table retains empty bytes, objective-free data and header-only inputs. |
| errors_when_no_objective_columns | Merge | Into errors_on_empty_input; retains rejection of populated data without objectives. |
| errors_on_header_only | Merge | Into errors_on_empty_input; retains rejection of a schema without trials. |

### `tunny-dashboard::integration::d3_csv_attribute_boundary::` (1 original)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| csv_nonfinite_numeric_attributes_survive_the_authoritative_handoff | Keep | Explicit AttributeValue::from(f64) reconstruction must preserve infinities/NaN through Study authority and projection, distinct from parser-owned conversion. |

### `tunny-dashboard::integration::d3_io::` (8 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| recorded_nonfinite_numeric_parameters_remain_observations_not_missing_values | Keep | Inline/update journal numeric parameters retain nonfinite observations rather than becoming missing. |
| metadata_only_metric_names_replace_labels_without_losing_streaming_layout | Keep | Metadata-only labels replace objective names without reordering an already-streamed parameter layout. |
| repeated_objective_labels_keep_indexed_values_separate_from_parameters | Keep | Streaming duplicate objective labels and colliding parameter names retain role/index-specific values. |
| actual_optuna5_journal_retains_nonfinite_observations_without_conflating_null | Keep | Real upstream fixture tests full/streamed equivalence and intermediate/nonfinite/null/text distinctions. |
| journal_bulk_streaming_reload_preserve_domain_and_path_order | Keep | Reload and different batch boundaries preserve all-state records, schema evidence and completion-path ordering. |
| journal_sink_failure_retains_only_accepted_prefix_and_empty_reload_replaces | Keep | Rejected sink delivery must not become accepted state; subsequent empty reload replaces rather than appends. |
| csv_owned_load_preserves_fixture_columns_identity_and_nonfinite_values | Keep | Owned CSV handoff checks actual typed columns, category labels, nonfinite values, identities, empty cells and artifacts in both read modes. |
| sqlite_all_state_batches_reload_and_metadata_growth_keep_visible_schema | Keep | Actual SQLite reload grows all-state constraint evidence without exposing hidden-state features or mutating retained input. |

### `tunny-dashboard::integration::d4_analysis::` (8 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| explicit_input_shares_tables_and_results_but_not_conditions_or_sources | Keep | Same input shares pending results/tables, but conditions/source identity distinguish work; released cache does not invalidate retained results. |
| completion_cannot_publish_into_another_sessions_equal_numbered_slot | Keep | Equal numbered runs in distinct sessions must not accept each other's completion. |
| same_count_updates_schema_growth_and_completion_invalidate_without_mixing_inputs | Keep | Same-count edits, hidden-state schema growth and load completion invalidate results while old work retains immutable tables. |
| rerun_failure_reload_and_dependency_keys_cannot_relabel_previous_work | Keep | Superseded failure cannot publish, and reload cannot adopt a dependency from the previous generation. |
| comparison_keys_track_every_input_without_invalidating_independent_study | Keep | Updating one comparison input invalidates comparison keys but not independent results for another study. |
| subset_tables_keep_trial_keys_and_release_source_roots_after_work_finishes | Keep | Filtered tables retain trial provenance; completed work releases source roots while retained result payloads survive cache release. |
| journal_nonfinite_objectives_do_not_shift_into_finite_analysis | Keep | Both journal objective encodings retain per-objective alignment when PDP extraction filters nonfinite values. |
| optuna_numeric_categorical_choices_stay_nominal_while_int_and_float_are_supported | Keep | Real journal distributions must keep numeric-looking categorical choices out of numeric sensitivity without excluding Int/Float parameters. |

### `tunny-dashboard::integration::d4_preparation::` (11 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| empty_entropy_and_unconfigured_or_featureless_fit_are_not_ready_but_reports_and_heatmaps_are | Keep | Method-specific readiness must not become blanket empty-input rejection; entropy finite selection retains source map. |
| fit_preparation_owns_complete_finite_selection_constraints_and_trial_correspondence | Keep | Fit selection preserves complete/finite rows, constraint alignment, source identities and shared multi-objective filtering. |
| nominal_numeric_looking_categories_remain_ready_unsupported_ridge_data_not_a_numeric_fit | Keep | Preparation must allow unsupported-categorical metric presentation while rejecting numeric fit on category-only inputs. |
| model_preparation_owns_the_fitted_dependency_and_rejects_missing_or_retargeted_models | Keep | Actual fitted model binds fit provenance; Optimize waits without it and rejects models from another input. |
| compare_keeps_validation_order_and_rejects_a_best_row_lost_by_feature_filtering | Keep | Compare distinguishes invalid slice from lost best-row anchor and permits small datasets for per-model execution fallbacks. |
| front_dependent_methods_prepare_distinct_subsets_and_entropy_does_not_wait_for_a_front | Keep | Cluster feature filtering and MCDM objective subsets differ; only front-dependent methods wait for complete Pareto input. |
| sensitivity_preparation_preserves_pairwise_complete_case_rejection_and_heatmap_fallbacks | Keep | Pairwise Spearman, complete-case Ridge, tree/Sobol rejection and heatmap/ARD fallbacks retain distinct missingness policies. |
| pdp_preparation_maps_finite_rows_without_removing_empty_numerical_fallbacks | Keep | PDP preparation preserves finite source correspondence without prematurely rejecting a model's short-data fallback. |
| convergence_and_synchronous_feature_preparations_keep_each_inputs_maps_and_method_minimums | Keep | Per-study subsampling maps/infeasible masking and distinct PCA/linkage/SOM/history readiness thresholds survive preparation. |
| contour_preparation_rejects_degenerate_bounds_but_preserves_collinear_masked_results | Keep | Degenerate bounds reject, whereas collinear geometry reaches a correctly masked surface. |
| full_objective_preparation_preserves_direction_order_with_repeated_labels | Keep | Fit/MCDM full-objective flags preserve descriptor direction order despite repeated display labels. |

### `tunny-dashboard::integration::entry_dependencies::` (1 original)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| product_entries_use_the_approved_production_graph_and_binary_names | Keep | Cargo accepts many dependency graphs; this enforces ADR-0021's actual responsibility edges, executable names and lack of reverse test dependencies, not merely compilation. |

### `tunny-dashboard::integration::export_projection::` (13 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| tc_1101_01_csv_header_row | Merge | Into tc_1101_02_trial_id_column; retains ordered header alongside actual projected cell values. |
| tc_1101_02_trial_id_column | Keep | Selected row positions export stored trial IDs (0,5), not row indices, alongside parameter/objective values. |
| tc_1101_03_numeric_column_values | Merge | Into tc_1101_02_trial_id_column; preserves exact parameter/objective numeric serialization. |
| tc_1101_04_index_filtering | Keep | Mixed invalid/valid indices retain only selected valid rows; receives empty and invalid-only header results. |
| tc_1101_05_out_of_range_index_skipped | Merge | Into tc_1101_04_index_filtering; retains invalid-only and mixed-invalid selection checks. |
| tc_1101_06_csv_field_escaping | Delete | I/O writer_quotes_and_guards_text_fields already covers comma/quote/plain escaping through the actual writer, without a second direct helper check. |
| tc_1101_08_empty_indices_header_only | Merge | Into tc_1101_04_index_filtering; preserves the header-only empty selection. |
| tc_1101_09_format_f64_cases | Keep | Legacy formatting has its own ten-decimal rounding and nonfinite policy, unlike CsvWriter's numeric to_string path. |
| tc_1101_10_nonexistent_column_empty | Keep | Unknown selected columns produce an empty cell rather than dropping the selected row. |
| tc_1102_01_report_stats_numeric_columns | Delete | Key labels/JSON shape are constructed by the test helper; product numeric-column inclusion and min/max/count are already exercised by tc_1102_03_report_stats_correct_values. |
| tc_1102_02_report_stats_empty_df | Keep | Product column_stats must not invent numeric summaries for an empty schema. |
| tc_1102_03_report_stats_correct_values | Keep | Product column_stats must return the fixture's min/max/count, not just nonempty serialized keys. |
| tc_1102_04_report_stats_valid_json_structure | Delete | Only checks braces from serde_json::to_string on the test helper's map; no product serialization is called. |

GUI boundary continuation: **53 additional originals**, 39 Keep, 6 Delete,
8 Merge across seven fully read test-bearing files. Shared acquisition/projection
helpers, AnalysisSession, method preparation, analysis export and I/O CSV writer/
formatting implementations were consulted.

### `tunny-dashboard::integration::study_slice::` (9 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| journal_slice_retains_all_states_without_exposing_noncomplete_columns | Keep | Multi-study acquired slice retains all states, typed distributions/times/intermediate values, hidden schema evidence and distinct full/streamed completion order. |
| full_read_retains_all_state_authority_and_legacy_returned_row_order | Delete | Its legacy comparator is common::journal, the identical read_full -> manager -> project path. Explicit full row order is already asserted in journal_slice_retains_all_states_without_exposing_noncomplete_columns; d3_io covers full all-state authority/schema. |
| streaming_projection_retains_discovery_order_but_resets_layout_on_full_reload | Keep | Incremental discovery order must reset on reload and revisiting an older retained input must not inherit future columns. |
| metadata_only_final_batch_reclassifies_existing_projection_without_new_rows | Keep | Final hidden-state constraint growth changes feasibility/revision without replacing unchanged trial Arcs or altering retained inputs. |
| owned_sink_error_stops_delivery_and_preserves_accepted_prefix | Merge | Into d3_io::journal_sink_failure_retains_only_accepted_prefix_and_empty_reload_replaces; adds exact two-delivery stop and one-row visible prefix to its failure/reload checks. |
| typed_categories_and_supported_inline_display_match_legacy_projection | Keep | Both inline typed choices and encoded serialized distributions must agree between distinct full and streamed ingestion paths for mixed JSON categories. |
| empty_and_constraint_attribute_only_studies_preserve_projection_discovery | Keep | Empty studies and hidden empty constraint observations retain objective schema, constrained presence and completed publication. |
| unavailable_categories_retain_typed_absence_and_existing_display_fallback | Keep | Negative/fractional/out-of-range/null encoded categories remain explicitly unavailable while full/streamed display agrees. |
| shipped_journal_fixtures_preserve_streamed_numeric_table_values | Keep | Both shipped real-world fixtures retain values, IDs/numbers, column layouts and feasibility between distinct full and streamed ingestion paths. |

### `tunny-dashboard::integration::rdb_integration::` (3 originals, ignored)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| scan_rdb_task_lists_known_studies | Merge | Into load_single_study_task_loads_mo_constrained, which already performs the same live scan; retains exact seeded study names before selection. |
| load_single_study_task_loads_mo_constrained | Keep | Real PostgreSQL scan, GUI worker request and message adoption must populate both authoritative trials and visible rows. |
| study_fingerprint_url_is_stable_without_changes | Keep | Real server fingerprint query must produce stable comparison values rather than accidentally including volatile query state. |

### `tunny-dashboard::integration::issue258_terminal_attributes::` (6 originals)

ADR-0022 was read before judging these post-terminal replay cases. The ignored
cost probe has actual coalescing assertions; it is not merely timing output.

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| supported_before_terminal_and_inline_controls_agree | Keep | Reordered before-terminal controls and inline records preserve typed JSON, constraint growth and per-path order rather than accidentally depending on late replay. |
| recorded_terminal_attributes_agree_across_readers | Keep | Recorded terminal replacements agree across batch sizes/full reads; artifact scanner stays independent and final payload/feasibility is explicitly asserted. |
| replacements_shrink_constraint_evidence_without_losing_layout_or_prior_inputs | Keep | Replacing long constraints shrinks current evidence, preserves prior inputs/discovery order, merges attribute keys and corrects existing rows without appending. |
| late_ordinary_attributes_do_not_enable_other_terminal_mutations | Keep | Accepting late ordinary attributes must not also enable terminal parameter/state/time/intermediate mutations or create unknown trials. |
| final_correction_delivery_is_fallible_and_does_not_mutate_an_accepted_prefix | Keep | Rejecting an EOF correction preserves old payload/constraint observations in accepted authority, distinct from rejecting a new trial batch. |
| journal_postprocessing_cost_probe | Keep | Controlled 100k-update tail must coalesce into eleven deliveries/11k upserts, not publish per-record corrections; ignored opt-in timing probe retains deterministic count assertions. |

### `tunny-dashboard::integration::dataframe::` (33 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| distribution_metadata_and_column_identity_survive_late_columns_and_filtering | Keep | Late typed categories/discrete metadata survive feasible filtering despite parameter/objective/attribute name collisions. |
| constraint_schema_growth_reclassifies_history_and_matches_initial_load | Keep | Incremental width growth reclassifies past rows and matches rebuild; receives explicit materialized is_feasible NaN/infeasible values. |
| attribute_only_empty_constraints_remain_unverified_after_append | Keep | Present-but-empty constraint evidence remains constrained/unverified across append and feasible filtering. |
| metadata_only_constraint_growth_reclassifies_complete_history | Keep | Zero-row append with wider metadata must reclassify existing rows without changing row count. |
| tc_102_01_row_count_single_trial | Delete | Row count is checked by full/append equivalence helpers and multiple nontrivial fixtures; one-row construction adds no guarantee. |
| tc_102_02_param_column_values | Keep | Direct TrialRow constructor preserves actual parameter values, independent of append-vs-rebuild agreement. |
| tc_102_03_objective_column_values | Keep | Direct constructor preserves two objective positions and values. |
| tc_102_04_user_attr_numeric | Delete | d3_io::csv_owned_load_preserves_fixture_columns_identity_and_nonfinite_values asserts actual generic numeric attribute column values; local role-specific accessor tests add collisions. |
| tc_102_05_user_attr_string | Delete | The same d3_io fixture asserts actual generic string attribute column values; local accessor tests add parameter/attribute name collisions. |
| user_attribute_accessors_ignore_same_named_other_categories | Keep | Typed attribute iterators must not resolve same-named parameter/objective columns. |
| user_attribute_indices_survive_late_same_named_parameter | Keep | Late parameter insertion must not shift numeric attribute indices across subsequent appends. |
| user_attribute_string_stays_separate_when_parameter_becomes_categorical | Keep | Same-named parameter type conversion must not replace string attribute storage. |
| user_attribute_indices_survive_parameter_type_change | Keep | Numeric attribute indices stay valid when a preceding unrelated parameter converts to categorical. |
| tc_102_06_constraint_columns | Keep | Materialized constraint values, raw sum and feasibility flag retain their exact distinct semantics. |
| filter_feasible_keeps_only_feasible_rows | Keep | Filtering retains exact trial IDs, parameters/objectives and schemas, including the zero constraint boundary. |
| filter_feasible_without_constraints_returns_all_rows | Keep | Unconstrained fast path must retain all rows instead of treating absence as unverified. |
| filter_feasible_all_infeasible_returns_empty | Keep | Empty filtered result retains a usable zero-length numeric column. |
| tc_102_13_select_study_returns_result | Delete | One nonempty-row assertion is subsumed by exact multi-study and typed-value ingestion fixtures. |
| tc_102_14_select_study_multiple_studies | Delete | tc_2330_all_studies_resident_by_id_after_parse already checks both study selections with different exact counts through the same helper. |
| tc_2330_all_studies_resident_by_id_after_parse | Keep | Selecting each recorded study isolates its trial count; removed stale comment claiming the helper parses only once. |
| tc_102_e01_invalid_study_id_returns_err | Keep | Selecting an absent ID must return an error rather than another study/empty success. |
| tc_102_e02_all_running_returns_empty | Keep | A study containing only a running record must not leak that record into the COMPLETE-only dataframe. |
| tc_102_b02_study_with_no_complete_trials | Delete | study_slice::empty_and_constraint_attribute_only_studies_preserve_projection_discovery covers the same common::journal empty-study path alongside schema/publication guarantees. |
| tc_102_p01_load_50000_trials_at_scale | Keep | Large journal ingestion must not truncate/deduplicate after small batches; exact 50k retained rows are asserted, not a timing budget. |
| json_user_attributes_keep_null_distinct_from_missing_and_append_matches_rebuild | Keep | Late/mixed JSON attributes preserve explicit null vs absence, array metadata and numeric/string views across append. |
| append_trials_equals_from_trials_basic | Delete | append_trials_backfills_new_param_column and missing_numeric_params_match_initial_and_chunked_ingestion cover the same append/rebuild contents/ID/schema invariant with more discriminating missing/late columns. |
| missing_numeric_params_match_initial_and_chunked_ingestion | Keep | Different chunk boundaries preserve missing-vs-zero values, including append into an empty dataframe. |
| append_trials_backfills_new_param_column | Keep | Late numeric parameter plus numeric/string attributes backfill historical rows consistently with rebuild. |
| append_trials_flips_numeric_param_to_categorical | Keep | Actual conversion removes numeric storage and preserves the explicit empty display fallback for formerly unlabeled values. |
| append_trials_constraints_appear_mid_stream | Merge | Into constraint_schema_growth_reclassifies_history_and_matches_initial_load; retains explicit materialized NaN for old missing evaluations and zero for infeasible rows. |
| append_trials_objective_added_mid_stream_backfills_nan | Keep | A newly discovered objective backfills old rows with NaN rather than zero or shifted values. |
| append_trials_to_empty_dataframe | Delete | missing_numeric_params_match_initial_and_chunked_ingestion already starts with DataFrame::empty for every chunk size and compares complete contents. |
| append_trials_empty_rows_is_noop | Delete | Only asserts unchanged row count, already asserted by metadata_only_constraint_growth_reclassifies_complete_history on zero-row append; no contents/no-op guarantee is lost. |

Further GUI continuation: **51 additional originals**, 38 Keep, 10 Delete,
3 Merge across four fully read test-bearing files. No live PostgreSQL execution
is claimed. Removing one duplicate ignored scan reduces the ignored inventory
by one; it is not additional verification.

### `tunny-dashboard::integration::sqlite_projection::` (21 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| sqlite_generic_rdb_metadata_survives_row_ingestion_and_append_refresh | Keep | SQLite decoding -> authoritative projection/manual append preserves typed choices and discrete metadata, including an appended trial missing the parameter. |
| scan_study_list_reads_names_directions_and_metric_names | Keep | Actual SQLite scan retains study-level ordered directions/names/counts and constraint presence. |
| parse_single_study_reads_params_and_excludes_non_complete_trials | Keep | Actual database load retains numeric/categorical values, declared bounds and all-state extras while excluding noncomplete visible rows; receives missing-intermediate-table checks. |
| json_user_attributes_match_journal_values | Keep | Independent SQLite and Journal decoders agree on mixed JSON types, explicitly distinguishing null from absence. |
| parse_single_study_converts_infinite_objective_values | Keep | Both objective infinity tags preserve position/value and custom names; receives the same fixture's constraint values and missing evaluation checks. |
| parse_single_study_extracts_constraints | Merge | Into parse_single_study_converts_infinite_objective_values; same study/fixture/query retains both actual constraint columns and Unverified missing evaluation. |
| python_constraint_tokens_and_noncomplete_schema_are_ingested | Keep | Non-JSON numeric tokens and hidden-state evidence retain positions/width and distinguish positive violation from invalid/unverified observations. |
| constraint_json_positions_and_attribute_only_study_are_preserved | Keep | JSON null/invalid slots do not shift later constraints; replacing observations with empty arrays retains constrained presence. |
| parse_single_study_extras_without_intermediate_table_are_empty | Merge | Into parse_single_study_reads_params_and_excludes_non_complete_trials; its schema already omits the optional table and checks all-state extras count. |
| parse_single_study_extras_include_all_states_datetimes_and_intermediate_values | Keep | Optional table data maps infinity tags, sorted steps and exact timestamps across complete/pruned trials. |
| parse_single_study_missing_study_id_errors | Keep | Absent database study ID must not silently project another study or empty success. |
| scan_study_list_rejects_non_optuna_database | Keep | Non-Optuna SQLite schema returns a controlled scan error. |
| scan_study_list_empty_database_errors | Keep | Valid Optuna schema without studies is rejected rather than mistaken for a usable storage. |
| study_fingerprint_detects_complete_user_attribute_mutations | Keep | Insert/update JSON attributes change the fingerprint without trial counts/state changes and load the actual updated payload. |
| study_fingerprint_detects_complete_constraint_mutations | Keep | Constraint insert/value replacement changes fingerprint and visible feasibility without trial-table changes. |
| study_fingerprint_detects_noncomplete_constraint_schema_mutations | Keep | Hidden-state empty/one/two-wide evidence must alter fingerprint and reclassify visible COMPLETE rows. |
| study_fingerprint_equal_on_unchanged_db | Merge | Into study_fingerprint_changes_when_trial_added; adds stable second pre-mutation query using its existing schema. |
| study_fingerprint_changes_when_trial_added | Keep | Adding a RUNNING row changes total/max-ID fingerprint fields; receives unchanged query and missing optional-table count. |
| study_fingerprint_changes_when_state_updated | Keep | State-only edits change completed count without changing total/max-ID. |
| study_fingerprint_changes_when_intermediate_value_added | Keep | Intermediate insertion changes its count without changing trial/state fingerprint components. |
| study_fingerprint_works_when_intermediate_table_absent | Merge | Into study_fingerprint_changes_when_trial_added; its schema already omits the optional table, so retain explicit zero intermediate_count there. |

### `tunny-dashboard::integration::journal_projection::` (64 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| distribution_metadata_survives_inline_events_streaming_and_filtering | Keep | Both event encodings preserve discrete/continuous metadata and typed empty/numeric/string/null/missing categories through full/streamed projection and filtering. |
| missing_param_event_values_do_not_fabricate_numeric_or_category_groups | Keep | Missing op5 values must remain absent/NaN instead of fabricated zero or first-choice categories. |
| objective_nonfinite_positions_survive_op4_and_op6_and_are_excluded_from_reports | Keep | Invalid/nonfinite objective slots preserve dimensions/alignment while report outcomes exclude affected trials and render warnings. |
| python_nonfinite_constraints_and_all_trial_schema_match_metadata_only_batches | Keep | Noncomplete EOF evidence grows schema via a zero-row batch and reclassifies older rows without corrupting literal nonfinite text. |
| noncomplete_attribute_only_evidence_is_not_unconstrained | Keep | Running/pruned/failed empty observations establish constraint presence and a final metadata-only update even with zero width. |
| studies_without_complete_rows_still_infer_noncomplete_constraint_schema | Keep | Zero visible rows must still publish noncomplete schema and correct first/final batch flags. |
| python_nonfinite_inline_array_is_unverified_not_a_lost_trial | Keep | Nonfinite inline constraint arrays do not drop the trial or its ordinary attributes and remain unverified. |
| invalid_constraint_positions_and_schema_growth_match_streaming | Keep | JSON invalid slots preserve column positions and positive violation precedence across width growth in both read paths. |
| constraints_attribute_with_empty_or_invalid_value_establishes_study | Keep | Empty/null/scalar constraint attributes establish presence rather than unconstrained status. |
| scan_study_list_returns_names_only | Keep | Cheap journal scan returns ordered study names without claiming computed completion counts. |
| parse_single_study_alpha_returns_correct_trials | Keep | Selecting the first of multiple studies yields its own name/counts/parameter schema. |
| parse_single_study_beta_skips_alpha_trials | Keep | Selecting a later study excludes earlier-study parameter schema and rows. |
| parse_single_study_beta_trial_number_is_in_study_index | Keep | Full ingestion separates global trial IDs from per-study creation numbers. |
| streaming_beta_trial_number_is_in_study_index | Keep | Streamed new-row deliveries independently preserve global IDs vs study-local numbers. |
| parse_single_study_objective_values_correct | Keep | Full ingestion has an explicit ordered objective-value oracle, not only counts/equality with streaming. |
| parse_single_study_nonexistent_id_returns_error | Keep | Full ingestion must reject a missing ID in a populated multi-study log. |
| streaming_beta_matches_single_study_and_batches | Keep | Cross-study streaming retains exact boundaries/first-final flags, schema and ordered objective values. |
| streaming_emits_inline_completed_trials_inmem | Keep | Inline multi-objective completed rows emit without op5/op6, skip FAIL and retain metric labels. |
| streaming_single_batch_when_batch_size_large | Keep | Below-cap stream sets both first/final flags with exact new-row count and derived objective schema. |
| streaming_nonexistent_id_returns_error | Keep | Distinct streaming API rejects absent study instead of invoking an apparently successful sink. |
| streaming_objective_names_prefer_metric_names_over_derived | Keep | op6 route uses recorded custom labels consistently in batch and metadata, distinct from inline handling. |
| line_u32_field_basic | Delete | I/O unit extracts_fields_with_varied_positions_and_spacing and rejects_non_numeric_and_missing already check the same direct helper guarantee. |
| tc_101_01_create_study_basic | Keep | Trial-free projected study retains ordered mixed directions and its recorded name. |
| parse_single_study_collects_extras_for_all_states | Keep | Full ingestion extras retain exact timestamps/sorted steps/PRUNED metadata independently of COMPLETE-only table rows. |
| full_journal_retains_all_state_extras | Delete | parse_journal merely calls the same common::try_journal as parse_single_study_collects_extras_for_all_states; weaker count/presence assertions add nothing. |
| parse_single_study_streaming_stores_extras_for_all_states | Keep | Streaming EOF extras also retain unfinished RUNNING records and exact complete-trial intermediate/timestamp data. |
| tc_101_02_create_trial_complete | Delete | parse_single_study_alpha_returns_correct_trials already asserts exact full projected total/completed counts through the same helper. |
| tc_101_03_float_distribution_no_log | Delete | Only checks a parameter name, already covered with actual typed/discrete metadata by distribution_metadata_survives_inline_events_streaming_and_filtering and values by other full-ingestion fixtures. |
| tc_101_04_float_distribution_log_true | Delete | Only checks a parameter name, not log semantics; study_slice checks real log distribution metadata and distribution_from_json_string_log_true checks unchanged stored values. |
| tc_101_05_int_distribution_basic | Delete | Only checks name presence; distribution_metadata_survives_inline_events_streaming_and_filtering checks actual Int metadata and SQLite fixture asserts actual external Int values. |
| tc_101_07_categorical_distribution_string | Delete | Only checks name presence; typed-category/distribution fixtures exercise actual decoded category values and labels. |
| tc_101_10_multiple_studies | Keep | Interleaved study creation with already-completed trials preserves independent final counts. |
| tc_101_11_trial_id_sequential | Delete | Does not assert IDs; tc_101_e06_all_trials_not_complete already checks exact total/zero-completed metadata for still-running op4 records. |
| tc_101_12_user_attr_numeric | Keep | Pre-terminal op8 numeric attributes populate projected metadata discovery. |
| tc_101_13_user_attr_string | Keep | Pre-terminal op8 string attributes populate projected metadata discovery. |
| json_user_attrs_survive_inline_and_update_events_in_streaming | Keep | Inline and repeated op8 typed replacements preserve mixed JSON/null-vs-missing, with streamed state matching full state. |
| tc_101_14_constraints_expansion | Delete | Only asserts has_constraints, already covered by python_nonfinite_constraints_and_all_trial_schema_match_metadata_only_batches with exact widths/positions/classification. |
| tc_101_15_constraints_all_feasible | Delete | Does not assert feasibility, only presence/count; other complete constraint fixtures already assert both. |
| tc_101_16_multi_objective_values | Delete | Only checks dimension two, already explicitly checked with aligned values/report behavior by objective_nonfinite_positions_survive_op4_and_op6_and_are_excluded_from_reports. |
| tc_101_e01_incomplete_json_line_skipped | Keep | A truncated middle record must not suppress following valid completion. |
| tc_101_e02_non_json_line_skipped | Keep | Invalid text and invalid UTF-8 among valid records are tolerated, distinct from truncated JSON. |
| tc_101_e03_unknown_opcode_ignored | Keep | Valid JSON with unsupported opcode must not abort otherwise usable acquisition. |
| tc_101_e04_all_lines_invalid_returns_error | Keep | Entirely non-UTF8/non-JSON bytes reject instead of pretending to be an empty journal. |
| tc_101_e06_all_trials_not_complete | Keep | Incomplete-only study metadata reports zero completed but retains total created trials. |
| tc_101_e07_distributed_optimization_overwrite | Keep | Duplicate completions from different workers must not count the same trial twice. |
| tc_101_b01_empty_file | Keep | Empty input yields no scanned studies rather than the malformed-input error policy. |
| tc_101_b02_study_only_no_trials | Keep | Trial-free study remains discoverable with zero completed metadata. |
| tc_101_b03_categorical_boundary_indices | Delete | Only checks name presence, not boundary values; retained typed-category fixtures include first and last real choices. |
| tc_101_b07_minimal_journal | Delete | Same helper and weaker completed-count-only guarantee as parse_single_study_alpha_returns_correct_trials. |
| tc_101_p01_performance_50000_lines | Delete | dataframe::tc_102_p01_load_50000_trials_at_scale covers the same large completion-count/truncation risk. Added op5 records have no parameter-value assertion or separate scale-dependent guarantee here; smaller fixtures retain explicit parameter decoding oracles. |
| distribution_float_display_is_identity | Delete | distribution_from_json_string_with_attributes and distribution_from_json_string_log_true already assert identity through actual parsing. |
| distribution_int_display_is_stored_value | Keep | Nonzero distribution low must not be added as an offset to stored Int values. |
| distribution_int_display_rounds_float_noise | Keep | Int display absorbs small floating error rather than exposing fractional labels. |
| distribution_categorical_label | Keep | Direct label lookup maps all three indices to exact first/middle/last string choices. |
| distribution_from_json_string_with_attributes | Keep | Double-serialized Float distribution parses successfully and preserves stored external values. |
| distribution_from_json_string_log_true | Keep | Log-marked serialized Float must not exponentiate already-external values. |
| distribution_from_json_object_with_attributes | Keep | Nested Int attributes preserve bounds and external stepped values. |
| parse_real_log_format_param_values | Keep | Actual double-serialized multi-parameter event format reaches final metadata through full ingestion. |
| tc_inmem_01_basic | Keep | Full inline multi-objective storage preserves metric labels, study identity and total/completed counts. |
| tc_inmem_02_incomplete_state_not_counted | Keep | Inline FAIL contributes to total but not completed count in full metadata. |
| tc_inmem_03_user_attrs_inline | Keep | Inline numeric/string attribute keys reach full metadata independently of op8. |
| tc_inmem_04_single_objective_value_singular | Keep | Singular inline value with null values array retains completed rows and custom objective identity. |
| parse_real_inmem_log_file | Keep | Shipped inline fixture has independently known ten parameter names and two custom objectives, not just agreement between read paths. |
| parse_real_log_file | Keep | Both shipped journal studies retain known ten-parameter schemas and nonzero numeric display data. |

Final libtest boundary continuation: **85 additional originals**, 66 Keep,
15 Delete, 4 Merge across two fully read test-bearing files. All 189 GUI
integration libtest definitions now have individual rows; the native harness-free
entry still requires source review. SQLite test adapters/SQL fixture construction
and all Journal helpers were read, including the fact that parse_journal is a
test-only composition, not a separate legacy production parser.

Other-language diagnostic sources read in full:

- `egui-app/tests/probes/issue258_optuna_probe.py`: Keep as historical upstream
  corroboration for ADR-0022, not a product regression suite. Pinned Optuna API,
  optional Dashboard/Rustuna replay and sampler-order assertions document real
  upstream divergence, including artifact association and rejected recorded
  writes. Its MemoryJournal implements storage transport for actual Optuna calls,
  not assertions about a mock's hardcoded response.
- `egui-app/tests/probes/issue258_dashboard_probe.cjs`: Keep as the separate pinned
  browser-reader diagnostic. It transpiles actual upstream TypeScript and checks
  null failure, scalar updates, nonfinite evidence and lossy browser conversions;
  it is explicitly not a browser UI/typecheck or this product's acceptance oracle.

These probes were not executed: their exact pinned external environments/source
checkouts were not provisioned, and no dependency installation was performed.

### `tunny-dashboard::ui::widgets::artifact_animation::tests::` (17 originals)

All test bodies, helpers and native capture body in `artifact_animation/tests.rs`
were read in full, alongside widget orchestration and frame/prefix implementation.
The adjacent cache/export tests are recorded separately below; presentation has
no local test definitions. These checks exercise project-owned mapping, scheduling,
provenance and rendering behavior, not image decoder quality alone.

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| duration_normalizes_input_and_persistence_without_fps_conversion | Keep | Product duration deserialization clamps/rounds invalid and half-step values and preserves milliseconds, not inverse FPS. |
| playback_deadline_does_not_add_preparation_or_skip_unready_frames | Keep | Clock advances existing deadlines and never skips a frame whose preparation is not ready. |
| resident_cache_hit_starts_next_prefix_before_sleeping_until_deadline | Keep | Resident image/prefix readiness starts lookahead before an event-driven sleep, maintaining cadence without incidental worker redraws. |
| common_sampling_transport_endpoints_remap_and_persistence | Keep | Skip sequence retains endpoints, ordinal seeking/looping/remapping and persisted common skip controls. |
| common_sampling_changes_retarget_workers_and_preserve_full_observations | Keep | Changing skip cancels obsolete prefix jobs without rebuilding source data, losing artifactless observations or changing stable plot bounds. |
| source_resets_keep_obsolete_prefix_work_in_one_slot_until_completion | Keep | Repeated resets must retain a single cancelled prefix worker slot instead of spawning unbounded sorters. |
| repeated_source_changes_queue_only_latest_mapping_and_reject_old_completion | Keep | A deliberately blocked old mapping completion must not overwrite the latest requested source. |
| rapid_seek_and_reload_discard_obsolete_image_and_prefix_results | Keep | Rapid real-image seek/removal/reload retains matching texture/prefix identity and observations despite frame-index shifts. |
| export_sampling_preserves_endpoints_count_duration_and_all_prefix_observations | Keep | selected_count agrees with endpoint-preserving sequence and sparse artifacts do not remove skipped observations from exported prefixes. |
| mapping_uses_study_numbers_and_original_entry_indices_not_journal_ids_or_filtered_indices | Keep | Sparse global IDs, local trial numbers, original artifact slots, MIME/extension/header checks and unreadable images map only eligible study frames. |
| prefix_includes_artifactless_trials_excludes_future_and_restores_front_on_backseek | Keep | Prefix Pareto computation includes artifactless observations, honors maximize dimensions, excludes invalid/future rows and restores early fronts on backseek. |
| constrained_prefix_ranks_feasible_then_violation_then_unverified_and_invalid_never_on_front | Keep | Animation-specific rank assembly preserves feasible/violation/unverified precedence and dataframe-order ties rather than display-order ties. |
| history_uses_local_numbers_finite_selected_values_and_all_three_feasibility_states | Keep | Render series use local-number order, selected-objective missingness and all feasibility states without leaking future observations. |
| objective_identity_is_not_shadowed_by_a_same_named_parameter | Keep | Animation's objective extraction/bounds use objective-role identity rather than generic name lookup. |
| playback_pause_step_loop_and_persistent_instances_are_independent | Keep | Export success/cancel/error preserves playback/cursor, presentation snapshots are independent and persistence excludes transient playback. |
| shared_composition_draws_only_prefix_and_omits_disabled_values_and_transport | Keep | Actual egui text output honors presentation toggles, missing-value placeholders and export content excludes transport controls. |
| source_snapshot_and_same_path_rescan_reset_cursor_cache_and_owned_texture | Keep | Same-path file replacement/source switch invalidates owned textures/cache, streaming disables animation and slow playback sleeps rather than polling. |

Additional source-only harness paths (not libtest definition counts):

- `egui-app/tests/artifact_animation_native.rs::main` and
  `artifact_animation/tests.rs::native_capture_check`: Keep. A real full-app
  eframe/wgpu render -> screenshot/crop -> GIF/decode verifies image frame order,
  live/export sequence agreement, native capture dimensions/timing, full prefixes,
  instance independence and seek/pause restoration. Only the OS destination
  picker is bypassed; unit synthetic screenshots cannot replace this guarantee.
- The same main's `TUNNY_ANIMATION_PERF` branch ->
  `artifact_animation/performance.rs::run`: Keep as an opt-in measured production
  workload, not a default regression test. Its entire workload/Probe/report/seek
  implementation was read. It records cold/warm and event-driven/continuous UI
  timing/cache/memory, asserts every requested frame is observed and bounds seek
  completion; optional real journal/artifact inputs remain read-only. It was not
  run in this continuation and no performance budget is inferred from compilation.

### `tunny-dashboard::ui::widgets::artifact_animation::cache::tests::` (2 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| chronological_windows_pause_seek_loop_and_selected_export_order | Keep | Product prefetch windows prioritize demand, selected sequence, paused neighbors and wrapped chronology with small-budget capacity/quantized display-size limits. |
| shared_budget_reuses_overlap_instances_resize_and_invalidates_obsolete_work | Keep | Actual decode/cache clients share overlapping textures without rebuilding, enforce retained/peak budgets including pending work, pin live/export snapshots and invalidate obsolete source/size jobs. |

### `tunny-dashboard::ui::widgets::artifact_animation::export::tests::` (2 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| screenshot_requires_matching_export_and_frame_acknowledgment | Keep | Wrong screenshot tokens cannot publish/advance the pending frame; correct acknowledgment produces exact cropped dimensions/color and returns the initiating owner. Native happy-path capture does not exercise rejected tokens. |
| cancel_and_capture_timeout_clear_session_without_publishing | Keep | Both cancellation and missing screenshot timeout clear controller state while preserving a pre-existing destination and removing temporary output. |

Both files' entire implementations and fixture helpers were read. The shared
presentation implementation was also read in full; it has no local #[test]
definitions. All animation widget files are now body-reviewed, including its
two explicitly gated harness modes; no additional runtime-count change results
from these four Keep decisions.

### `tunny-dashboard::theme::color_compute::tests::` (12 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| contrasting_text_color_flips_between_light_and_dark_backgrounds | Keep | Cell labels must choose a legible contrasting foreground at both ends of the actual viridis palette. |
| rgba_key_roundtrips_through_key_to_color32 | Keep | Product adapters must preserve premultiplied color keys, not accidentally unpremultiply semi-transparent groups. |
| compute_point_alpha_empty_selected_returns_opaque | Keep | Consolidated slice-selection policy retains opaque empty/selected cases and dimmed nonselection. |
| compute_point_alpha_selected_returns_opaque | Merge | Into compute_point_alpha_empty_selected_returns_opaque; retains selected ID among multiple IDs. |
| compute_point_alpha_not_selected_returns_transparent | Merge | Into compute_point_alpha_empty_selected_returns_opaque; retains exact dimmed alpha for absent ID. |
| point_alpha_in_set_matches_slice_version | Keep | Independent HashSet adapter explicitly checks all three expected alpha outcomes, not merely equality with the slice implementation. |
| diverging_colormap_negative_one_is_blue | Keep | Consolidated diverging scale checks blue/white/red direction and neutral center. |
| diverging_colormap_zero_is_white | Merge | Into diverging_colormap_negative_one_is_blue; retains exact RGB white midpoint. |
| diverging_colormap_positive_one_is_red | Merge | Into diverging_colormap_negative_one_is_blue; retains opposite endpoint orientation. |
| diverging_colormap_intermediate_values_bounded | Delete | No boundedness assertion at all; just calls a cast-based color function on 21 finite values, with real endpoint/midpoint behavior retained elsewhere. |
| correlation_color_negative_is_reddish | Keep | Correlation scale intentionally reverses the other diverging scale's sign colors; receives both endpoint orientations. |
| correlation_color_positive_is_bluish | Merge | Into correlation_color_negative_is_reddish; retains blue positive correlation. |

### `tunny-dashboard::theme::colormap::tests::` (7 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| sample_categorical_single_is_midpoint | Merge | Into sample_categorical_spans_full_range; retains distinct singleton midpoint policy. |
| sample_categorical_spans_full_range | Keep | Product category sampling includes both endpoints, intermediate category and degenerate singleton. |
| interpolate_at_zero_returns_first_stop | Keep | Consolidated actual viridis endpoints and out-of-range clamping. |
| interpolate_at_one_returns_last_stop | Merge | Into interpolate_at_zero_returns_first_stop; retains exact last-stop RGB. |
| interpolate_at_half_returns_midpoint | Keep | Actual interpolation between two stops preserves expected independently specified RGB rounding. |
| interpolate_clamped_negative | Merge | Into interpolate_at_zero_returns_first_stop; retains lower clamp. |
| interpolate_clamped_above_one | Merge | Into interpolate_at_zero_returns_first_stop; retains upper clamp. |

### `tunny-dashboard::theme::colormap_name::tests::` (2 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| colormap_name_to_colormap_jet_boundaries | Keep | State Jet selection routes to the actual expected palette, rather than another valid colormap. |
| colormap_name_to_colormap_each_boundary | Delete | Discards all outputs; literal palette construction with guarded interpolation adds no independent behavior beyond actual endpoint/routing tests. |

### `tunny-dashboard::theme::tests::` (1 original, `theme/mod.rs`)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| strong_text_color_matches_the_body_text_color | Keep | Project Visuals must not let egui strong-text stroke precedence make titles unreadable on panel backgrounds. |

Theme continuation: **22 originals**, 11 Keep, 2 Delete, 9 Merge across four
test-bearing files. All six theme sources were read in full; chart_colors and
ui_colors define themed-color functions but no tests. The macro expansion was
also read and does not generate hidden test definitions.

### `tunny-dashboard::ui::widgets::history::edf_plot::tests::` (8 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| build_edf_points_basic_staircase | Keep | Explicit staircase coordinates retain normalization and right-continuous increments. |
| build_edf_points_handles_ties | Keep | Equal values form one jump with their combined probability mass. |
| build_edf_points_skips_nan_and_inf | Keep | Filtered finite denominator retains correct fractions; receives nonpositive log filtering with its own input. |
| build_edf_points_log_scale_drops_non_positive | Merge | Into build_edf_points_skips_nan_and_inf; retains exclusion of zero/negative values before normalization. |
| build_edf_points_empty_input_returns_empty | Keep | Shared empty-result cases retain initially empty, wholly nonfinite and wholly log-rejected inputs. |
| build_edf_points_all_non_finite_returns_empty | Merge | Into build_edf_points_empty_input_returns_empty; retains filtered-empty early return. |
| build_edf_points_all_dropped_by_log_filter_returns_empty | Merge | Into build_edf_points_empty_input_returns_empty; retains log-only empty case. |
| edf_plot_chart_default | Delete | Only reasserts derived Default for bool/usize, without exercising widget behavior or persistence. |

### `tunny-dashboard::ui::widgets::history::intermediate_values::tests::` (7 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| chart_default_has_log_scale_off | Delete | Only asserts derived bool Default. |
| build_curves_skips_trials_without_intermediate_values | Keep | Empty records are excluded from eligible counts and actual step/value data is preserved below the cap. |
| build_curves_keeps_raw_points_when_linear | Keep | Same fixture now asserts preservation of negative/zero values in linear mode and their removal in log mode. |
| build_curves_filters_non_positive_when_log_scale | Merge | Into build_curves_keeps_raw_points_when_linear; reuses the identical original input and preserves its exact remaining point. |
| build_curves_drops_curve_that_becomes_empty_under_log_scale | Keep | Wholly rejected curves disappear without changing the pre-log eligible count. |
| build_curves_subsamples_evenly_when_over_cap | Keep | Over-budget inputs must yield bounded nonempty curves and retain the first eligible trial. |
| build_curves_no_subsample_when_under_cap | Delete | build_curves_skips_trials_without_intermediate_values already asserts exact retained count/contents under the same below-cap branch; ten identical one-point rows add no new policy. |

### `tunny-dashboard::ui::widgets::history::optimization_history::tests::` (8 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| compute_best_values_minimize_decreasing | Keep | Consolidated explicit min/max running-best oracles retain deterioration plateaus and improvements. |
| compute_best_values_maximize_increasing | Merge | Into compute_best_values_minimize_decreasing; retains independent maximize fixture/expected values. |
| compute_moving_average_window3 | Keep | Explicit rolling values/count plus oversized-window behavior remain in one test. |
| compute_moving_average_window_larger_than_data | Merge | Into compute_moving_average_window3; retains clamp-to-input-length mean/count. |
| log_scale_toggle | Delete | Tests local boolean negation only; never constructs or calls the widget. |
| tc_cav_partition_history_no_constraints_all_feasible | Keep | Unconstrained adapter must classify every objective as feasible. |
| tc_cav_partition_history_mixed | Keep | Actual constrained partition retains nonconsecutive feasible rows and exact infeasible source coordinate/value. |
| tc_cav_partition_history_all_infeasible | Delete | No all-infeasible branch exists: mixed partition already checks infeasible membership and exclusion from feasible; repeated zero flags only repeat that per-row classification. |

### `tunny-dashboard::ui::widgets::history::timeline::tests::` (6 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| build_bars_skips_trials_without_start | Merge | Into build_bars_rebases_to_earliest_start; adds absent-start WAITING record and checks exact remaining IDs/count. |
| build_bars_rebases_to_earliest_start | Keep | Real timestamps map to shared-origin durations while missing starts do not create bars. |
| build_bars_extends_running_trial_to_max_known_timestamp | Keep | Incomplete trial's bar extends to the latest known timestamp instead of zero/current wall time. |
| build_bars_empty_when_no_trial_has_start | Keep | Missing all timestamps returns no bars rather than infinite coordinates. |
| select_time_unit_thresholds | Keep | Exact thresholds retain inclusive seconds/minutes and strict transitions to larger units. |
| bar_at_position_hits_within_range | Keep | Hit testing respects local trial-number Y and both horizontal/vertical exclusion. |

### `tunny-dashboard::ui::widgets::history::state_colors::tests::` (1 original)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| state_color_is_distinct_per_state | Keep | State-to-palette routing must not collapse different trial states to identical visual categories. |

History continuation: **30 originals**, 18 Keep, 5 Delete, 7 Merge across five
fully read test-bearing files. Relevant full widget implementations and pure
builders were read. Convergence was already recorded; history/mod.rs still needs
the final source census, not an inferred claim based on test-name search.

### `tunny-dashboard::io::rdb::tests::` (5 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| path_as_rdb_url_accepts_postgresql_and_mysql | Keep | Path adapter recognizes PostgreSQL/MySQL/driver suffixes; receives negative local/SQLite routing cases. |
| path_as_rdb_url_rejects_local_paths | Merge | Into path_as_rdb_url_accepts_postgresql_and_mysql; retains all rejected path examples. |
| scan_rdb_task_unreachable_host_returns_error | Delete | Only calls obsolete cfg(test)-only scan bridge, not production study_worker; remove the now-unused bridge. |
| load_single_study_task_unreachable_host_sends_error | Delete | Only exercises obsolete cfg(test)-only load bridge; no production path calls it. Remove that bridge and its exclusively used fixture_chunk helper. |
| scan_rdb_task_uses_normalized_url_as_path | Delete | Does not call scan task or assert a path, only RdbUrl::parse; I/O parse_postgres_short_scheme_normalizes already checks normalization. |

### `tunny-dashboard::io::sqlite::tests::` (3 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| is_sqlite_path_detects_extensions | Keep | Product routing accepts case-insensitive db/sqlite/sqlite3 and rejects other/no extensions. |
| scan_sqlite_task_nonexistent_path_returns_error | Delete | Only validates obsolete cfg(test)-only scan bridge rather than production storage acquisition; unused bridge removed. |
| load_single_study_task_nonexistent_path_sends_error | Delete | Only validates obsolete cfg(test)-only load bridge; unused bridge removed, actual storage failures remain in production worker tests to audit separately. |

### `tunny-dashboard::io::file::tests::` (7 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| read_existing_file_succeeds | Delete | Exercises cfg(test)-only std::fs::read delegation; no production reader is called. Remove unused read_journal_file helper/import. |
| read_nonexistent_file_returns_err | Delete | Same test-only std::fs wrapper, no independent project behavior. |
| write_atomic_creates_new_file | Keep | Actual external write creates/publishes correct bytes; receives overwrite and directory-cleanup assertions. |
| write_atomic_overwrites_existing_file | Merge | Into write_atomic_creates_new_file; second write replaces first contents at the same destination. |
| write_atomic_leaves_no_temp_file_behind | Merge | Into write_atomic_creates_new_file; exact directory contents still contain only destination after both writes. |
| write_atomic_preserves_original_on_bad_directory | Delete | No original file was created/checked; only missing-parent error, already exercised through export::save_helper_returns_error_on_write_failure on the same write_atomic path. |
| is_ghx_path_matches_case_insensitive | Keep | Product open routing recognizes .ghx without conflating unsupported binary .gh or storage files. |

### `tunny-dashboard::io::flat_csv::tests::` (7 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| is_csv_path_detects_extension | Keep | Product routing distinguishes case-insensitive CSV from journal/no extension. |
| build_artifact_map_only_includes_existing_files | Delete | Wrapper delegates unchanged to I/O resolve_csv_images; I/O csv_resolution_rebases_associations_and_rejects_missing_or_escaping_files checks real existence/association/root behavior. |
| is_safe_relative_filename_accepts_plain_names | Keep | Literal relative nested/current-directory forms must remain accepted; receives parent/absolute/Windows-prefix rejection cases. |
| is_safe_relative_filename_rejects_traversal_and_absolute | Merge | Into is_safe_relative_filename_accepts_plain_names; preserves every original rejection, including cfg(windows) UNC/drive paths. |
| build_artifact_map_rejects_parent_dir_traversal | Delete | Same I/O resolution test checks an actual outside file and ../ traversal through the identical delegated implementation. |
| build_artifact_map_rejects_absolute_path | Delete | Same I/O resolution test includes an existing absolute outside path. |
| load_csv_builds_study_and_artifacts | Delete | Only tests obsolete cfg(test)-only load_csv composition, not production loading. Real CSV acquisition and I/O resolution are independently retained; remove now-unreferenced load_csv/CsvLoadResult/write_temp helpers. |

### `tunny-dashboard::io::export::tests::` (1 original)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| save_helper_returns_error_on_write_failure | Keep | Product CSV write path propagates actual missing-directory write failure to its caller. |

### `tunny-dashboard::io::journal::tests::` (1 original)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| convert_minimize_direction | Keep | Actual scan-metadata adapter preserves ordered minimize/maximize directions and completed count across responsibility types. |

### `tunny-dashboard::io::artifacts::tests::` (4 originals, 1 POSIX-only)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| image_uri_uses_windows_file_loader_paths | Keep | Windows-specific drive and UNC paths require different file URI prefixes and slash normalization. |
| image_uri_uses_posix_file_loader_paths | Keep | POSIX-specific absolute paths retain correct file URI form; source-reviewed only on this Windows host. |
| image_uri_loads_a_local_png_through_egui | Keep | Actual egui file loader accepts generated URI for a real local image with a space in its filename. |
| task2128_artifacts_dir_scanned_message_channel | Delete | Constructs and sends its own AppMessage through std mpsc, then reads back the same fields; never calls scan_artifacts_dir or message adoption. |

I/O adapter continuation: **28 originals**, 11 Keep, 13 Delete, 4 Merge across
seven fully read test-bearing files. One newly found POSIX-only original increases
the identified lower bound to 2016; it is not counted in Windows registration.
Repository-wide reference search confirmed removed cfg(test)-only helpers had no
remaining callers (only an old integration doc comment named the obsolete task).
The removed study.rs module was itself cfg(test)-only and had no tests or remaining
callers. No production function body/public production API was changed. Actual
study_worker, session/report/export/capture and CSV builders remain pending until
individually recorded, regardless of files already opened during this continuation.

### `tunny-dashboard::io::session::tests::` (7 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| roundtrip_preserves_version_and_defaults | Merge | Into write_and_read_roundtrip_via_file; retains current version and empty widget-map assertions through actual file composition. |
| newer_version_is_rejected | Keep | Product loader rejects unsupported future version with actionable error. |
| missing_view_fields_fall_back_to_defaults | Keep | Real session schema permits omitted view fields; receives an unknown top-level field in the same deserialization fixture. |
| unknown_fields_are_ignored | Merge | Into missing_view_fields_fall_back_to_defaults; retains loading with unknown nested top-level data. |
| widget_settings_survive_roundtrip | Keep | Multiple nested widget selections/settings and canvas pan/zoom survive actual session serialization/deserialization. |
| view_settings_capture_apply_roundtrip | Keep | Product capture/apply transfers colormap, pins, filters and HV override into another AppState. |
| write_and_read_roundtrip_via_file | Keep | Actual serialization/atomic write/read composition preserves version/widgets; use unique tempfile directory instead of a shared named test file. |

### `tunny-dashboard::io::chart_capture::tests::` (9 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| gif_duration_is_exact_milliseconds_at_both_limits_and_default | Keep | Product GIF API passes milliseconds unchanged at both duration limits/default and rejects invalid granularity/range before writing. |
| crop_helper_returns_expected_image_bounds | Keep | Product logical-to-physical crop preserves output size at scale one and two. |
| crop_with_scale_factor | Merge | Into crop_helper_returns_expected_image_bounds; retains exact scale-two dimensions with the same original larger image geometry. |
| crop_out_of_bounds_returns_none | Keep | A wholly outside capture rectangle returns no image instead of indexing outside viewport. |
| encode_png_produces_valid_png_header | Delete | Direct image-crate PNG encoding wrapper check only reasserts encoder magic bytes; no product capture geometry/channel policy is asserted. |
| save_png_pipeline_treats_cancel_as_noop | Delete | Constructs local Ok(None) and matches it; never calls save_png_to_file or a dialog. |
| unsupported_capture_backend_returns_user_visible_error | Delete | Never exercises a backend or error: only asserts image PNG encoder's valid output is nonempty. |
| gif_stream_roundtrip_has_order_dimensions_timing_and_atomic_publication | Keep | Actual bounded stream preserves frame colors/order/delay, acknowledgments and old destination until final publication, with no temporary output left. |
| gif_cancel_interruption_bad_dimensions_and_write_error_do_not_publish | Keep | Distinct cancellation/disconnect/dimension/publish-error paths preserve destination and clean temporary output. |

### `tunny-dashboard::io::report_export::tests::` (8 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| d4_report_worker_uses_one_acquired_input_and_the_finite_front_policy | Keep | Actual report worker retains an acquired revision while authority refreshes, writes all selected formats and applies finite-only front/report counts. |
| storage_display_masks_rdb_password | Keep | Product storage display removes URL secrets; receives exact local path and absent-storage fallback. |
| storage_display_uses_plain_path_for_local_files | Merge | Into storage_display_masks_rdb_password; retains unchanged local display. |
| storage_display_none_when_no_storage | Merge | Into storage_display_masks_rdb_password; retains explicit no-storage label. |
| to_core_meta_falls_back_to_row_count_without_extras | Delete | Only tests cfg(test)-only metadata reconstruction used for fixtures, not actual generate_prepared_report path; no product fallback guarantee exists here. |
| build_and_write_report_flags_silently_overwritten_siblings | Keep | Actual write_report path records only preexisting non-primary siblings; now first checks clean creation, then pre-seeds both primary/sibling for overwrite. |
| build_and_write_report_no_note_when_nothing_overwritten | Merge | Into build_and_write_report_flags_silently_overwritten_siblings; first export checks actual paths exist and overwrite list is empty. |
| build_and_write_report_ignores_preexisting_primary_path | Merge | Into build_and_write_report_flags_silently_overwritten_siblings; second export's exact overwrite list excludes the seeded primary while including sibling. |

Session/capture/report continuation: **24 originals**, 13 Keep, 4 Delete,
7 Merge across three fully read test-bearing files. Their entire implementations
and fixtures were read. Report's cfg(test)-only metadata/build helper remains
solely setup for tests that exercise actual write_report behavior; it has no
standalone assertion of its own semantics.

### `tunny-dashboard::io::csv_export::tests::` (50 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| rank_variants_route_export_all_rows_and_serialize_independently | Keep | 2D/3D routing preserves distinct schemas, missing/nonfinite rows, direction, independent persisted axes and duplicate-axis rejection. |
| csv_export_filename_optimization_history | Keep | Product filename routing retains the actual optimization-history basename. |
| csv_export_filename_all_end_with_csv | Delete | Every arm feeds the same literal format suffix; fifteen ChartId inputs do not add distinct suffix behavior or verify actual routing names. |
| opt_history_csv_minimize_tracks_cumulative_min | Keep | Product history CSV contains the expected objective values and cumulative-best plateaus. |
| opt_history_csv_nan_objective_becomes_empty_field | Keep | Nonfinite objective and initial running best both export empty, not NaN/Infinity text. |
| opt_history_csv_returns_none_when_no_study | Keep | History builder rejects unavailable authority rather than exporting an apparent empty result. |
| convergence_csv_uses_index_times_step | Keep | CSV trial coordinate uses display sample step, not stored global trial IDs. |
| convergence_csv_returns_none_when_missing | Keep | No convergence computation means no export. |
| importance_csv_returns_none_when_computing | Merge | Into importance_csv_has_expected_columns; now asserts the computing guard with an actual cached result, avoiding the original no-cache confounder. |
| importance_csv_returns_none_when_no_cache | Merge | Into importance_csv_has_expected_columns; checks missing cache before installing the fixture. |
| importance_csv_has_expected_columns | Keep | Actual cache selection yields expected method/column schema and two values; receives comma escaping, formula guard and readiness checks. |
| categorical_csv_outputs_are_explicit_for_mixed_and_all_categorical_inputs | Keep | Both signed methods export nominal categories as unsupported, never zero scores, with one-objective cache indexing independent of selected study index. |
| unsupported_names_do_not_bypass_heatmap_dimension_validation | Keep | Unsupported-category metadata must not make a malformed numeric heatmap exportable. |
| importance_csv_quotes_param_name_with_comma | Merge | Into importance_csv_has_expected_columns; first actual parameter name contains comma and exact exported row is asserted. |
| importance_csv_guards_param_name_starting_with_equals | Merge | Into importance_csv_has_expected_columns; second actual parameter has formula prefix and exact guarded row is asserted. |
| sensitivity_csv_has_objective_columns_in_header | Keep | Multi-objective heatmap column identities and parameter row count reach CSV. |
| sensitivity_csv_returns_none_when_no_result | Keep | Missing heatmap result disables serialization. |
| trial_based_csv_has_trial_id_header | Delete | all_trials_csv_follows_user_attribute_toggle_without_changing_chart_csv already checks exact full trial-ID header and actual chart/trial-based CSV equivalence. |
| trial_based_csv_returns_none_when_no_study | Keep | Shared trial builder rejects unavailable study. |
| cluster_csv_returns_none_when_no_cluster_result | Keep | Actual selected study without matching cluster result cannot export assignment data. |
| cluster_csv_includes_cluster_id_column | Keep | Result cache lookup supplies correct per-row labels and cluster schema. |
| pareto_csv_includes_all_trials_with_rank | Merge | Into pareto_csv_uses_row_rank_not_trial_id; retains header/count/all three ranks using more discriminating sparse IDs. |
| pareto_csv_uses_row_rank_not_trial_id | Keep | All trial ranks stay row-aligned despite noncontiguous global IDs, not front-only membership. |
| pareto_csv_returns_none_when_no_pareto | Keep | Missing computed Pareto membership prevents exporting an uncomputed front. |
| pdp_csv_returns_none_when_no_result | Keep | PDP1d builder rejects missing result. |
| pdp_csv_has_correct_header | Keep | Actual result writes variable/grid/prediction and ordered confidence limits; receives missing-limit fallback. |
| pdp_csv_handles_missing_ci | Merge | Into pdp_csv_has_correct_header; clears both optional CI vectors and asserts empty fields on the same result. |
| pdp_2d_csv_returns_none_when_no_result | Keep | Separate PDP2d result path rejects missing data. |
| pdp_2d_csv_has_correct_header_and_grid | Keep | Distinct axes flatten a 2x2 grid with expected coordinates/value ordering. |
| negative_mcdm_settings_cannot_export_a_cached_uniform_ranking | Keep | Invalid weights must not reuse a valid uniform cache across rank/2D/3D/table export paths. |
| mcdm_rank_csv_has_correct_header_and_method_topsis | Keep | Actual ranking schema/method/count plus separate scatter/table schemas share one fixture. |
| mcdm_rank_csv_returns_none_when_no_study | Keep | Both rank and table builders reject missing source trial identities. |
| mcdm_scatter_csv_has_correct_header | Merge | Into mcdm_rank_csv_has_correct_header_and_method_topsis; still calls actual scatter builder and checks its distinct schema. |
| mcdm_table_csv_topsis_header | Merge | Into mcdm_rank_csv_has_correct_header_and_method_topsis; retains TOPSIS-specific table schema. |
| mcdm_table_csv_vikor_header | Keep | VIKOR exports separate S/R/Q columns rather than a TOPSIS score schema. |
| mcdm_table_csv_returns_none_when_no_study | Merge | Into mcdm_rank_csv_returns_none_when_no_study; same missing-state fixture checks both actual builders. |
| slice_csv_includes_param_obj_and_pareto | Keep | Slice export carries selected feature/objective values and row-aligned front membership. |
| multi_opt_csv_header_is_objectives_then_params | Merge | Into build_surrogate_opt_csv_prefers_multi_result; strengthens its substring header check to exact objective-then-parameter order. |
| multi_opt_csv_row_count_equals_front_size | Merge | Into build_surrogate_opt_csv_prefers_multi_result; retains header plus both front rows through public chart routing. |
| has_csv_data_true_when_multi_result_present | Merge | Into build_surrogate_opt_csv_prefers_multi_result; checks availability before installing a single-objective result, isolating multi-only readiness. |
| build_surrogate_opt_csv_prefers_multi_result | Keep | Chart dispatch prefers multi-objective front when both results are retained; receives exact schema/count/multi-only enablement. |
| violin_plot_csv_writes_group_value_density_rows | Keep | Ungrouped objective fallback produces one full KDE grid with exact value/group/scale schema. |
| violin_plot_csv_returns_none_when_no_study | Keep | Violin export rejects unavailable study before distribution preparation. |
| all_trials_csv_follows_user_attribute_toggle_without_changing_chart_csv | Keep | Table attribute toggles preserve namespaced collisions/mixed types and quoted multiline text without adding attributes to chart export. |
| all_trials_csv_distinguishes_json_null_missing_and_literal_text | Keep | Actual table serialization and list expansion preserve null/absence, JSON vs literal text and sparse entries. |
| violin_plot_csv_category_mode_writes_one_group_per_level | Keep | Raw categorical grouping emits both KDE curves with correct group identity/order and grid count. |
| distribution_csv_matches_prepared_chart_groups_scale_and_sparse_eligibility | Keep | Export uses the same resolved value/group/normalization/eligibility as chart preparation, preserving typed group identities and stale-selection fallback; it does not claim a separate statistics oracle. |
| constant_distribution_csv_exports_box_only | Keep | Normalized constant population exports zero-scale box statistics while omitting degenerate KDE. |
| decimal_constant_raw_violin_csv_omits_constant_group_but_exports_nonconstant_group | Keep | Raw decimal constants remain skipped despite floating variance noise, without hiding a valid neighboring group or its box summaries. |
| distribution_csv_preserves_identity_when_formula_guards_collide | Keep | Distinct nominal identities survive equal-looking sanitized labels in both box and violin export. |

### `tunny-dashboard::io::csv_export::clustering::pca_tests::` (2 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| pca_csv_retains_source_rows_after_filtering | Keep | Actual PCA CSV preserves sparse source indices and both signed projection coordinates exactly. |
| pca_csv_keeps_complete_projection_values_unchanged | Delete | Same complete value oracle as pca_csv_retains_source_rows_after_filtering; changing source indices to 0/1 adds no independent serialization path. |

CSV continuation: **52 originals**, 37 Keep, 3 Delete, 12 Merge across two
test-bearing files. All seven CSV builder modules, public dispatch/readiness and
fixture helpers were read in full. io/mod.rs and study_worker.rs were also read;
neither contains test definitions. No claim is made that pending GUI app/state
request/adoption tests have been reviewed just because these builders call them.

### `tunny-dashboard::state::analysis_job::tests::` (17 originals)

All 2640 source lines (production orchestration, cfg(test) fixtures and every test)
were read. Synthetic payloads deliberately isolate routing/provenance; the tested
operations are actual spawn/select/adopt/poll/message handlers, not a mock's return.
These guarantees are above the analysis-session integration boundary.

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| d5_notification_success_fanout_preserves_each_operations_payload | Keep | Per-operation delivery clone variants retain payload identity/values and clear the actual target widget busy fields. |
| d5_notification_failure_fanout_preserves_diagnostics_and_unavailable_slots | Keep | Failures retain diagnostics/variant and unavailable notifications rebind to the selected consumer slot. |
| d5_typed_completed_and_inflight_results_share_payload_and_trial_maps | Keep | Typed cluster/MCDM consumers share pending/completed handles while preserving filtered/source row correspondence and sentinel cluster labels. |
| d5_cluster_and_mcdm_reject_stale_success_error_and_cancellation | Keep | Entry adoption rejects revision/reload/source/study/rerun-stale typed outcomes of all three kinds without populating current caches. |
| d5_coloring_keeps_the_last_computation_not_the_table_selection | Keep | Coloring follows last computed result independently of table cache selection; externally retained payload lifetime survives interest eviction then releases. |
| d4_entry_rejects_same_count_revision_load_epoch_and_source_results | Keep | Entry adapter rejects pending results after load completion epoch, same-count edits and replaced source while releasing the correct busy flag. |
| d4_entry_rerun_rejects_predecessor_without_clearing_the_new_busy_slot | Keep | Obsolete rerun completion must not clear a newer busy request; current completion delivers its own diagnostic. |
| d4_entry_fit_retains_actual_input_and_condition_provenance | Keep | Actual numerical fit binds filtered original trial keys and dependency provenance; stale trained models cannot execute optimization after refresh. |
| d4_unavailable_preparation_is_keyed_and_never_executes_the_calculation | Keep | Featureless-fit rejection carries current input key, clears fitting and never invokes numerical closure. |
| d4_publication_invalidates_same_count_schema_results_without_resetting_controls | Keep | Hidden-state constraint publication invalidates stale cached results/busy flags but preserves selection, HV override and model controls. |
| d4_matching_widget_conditions_share_one_inflight_calculation | Keep | Actual public spawn path executes one numerical closure for matching separate widget requests and fans out correctly targeted failures. |
| d4_rejections_reach_both_current_widgets_in_either_completion_order | Keep | Both settings and preparation rejections reach all current widgets in either queued order rather than superseding another interested consumer. |
| d4_rejection_supersession_preserves_other_current_interests | Keep | Same/different-setting supersession preserves other current interests and prevents stale failures from clearing their busy state. |
| d4_cached_selection_rejects_queued_success_error_and_panic_from_previous_request | Keep | Cache hits in importance/Sobol/heatmap/PDP detach obsolete adoption interest before queued success/error/panic can corrupt selected cached B. |
| d4_cached_selection_detaches_only_its_consumer_of_shared_work | Keep | Detaching one consumer preserves shared work and correctly delivers to its still-interested table consumer. |
| d4_consumed_missing_model_requests_supersede_old_failures | Keep | Polling new model-dependent requests with no trained model supersedes old queued failures for all single/multi optimize/suggest slots. |
| d4_convergence_control_requests_supersede_work_before_the_busy_early_return | Keep | Actual render control handoff supersedes old convergence work before busy early return for both reference and indicator changes. |

### `tunny-dashboard::state::filter::tests::` (2 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| filter_5k_trials_at_scale | Delete | Reimplements filtering on test-local TrialRows and never calls AppState/set_filter or the analysis filter; only tests its own closure. |
| set_filter_excludes_out_of_range_trials | Keep | Product AppState set_filter/apply_filters updates selected trial identities using the actual derived dataframe. |

State continuation: **19 originals**, 18 Keep, 1 Delete across two fully read
test-bearing files. state/results.rs was read and is only an analysis re-export,
with no local tests. Remaining AppState/management/type/message files still require
individual review before the state layer is complete.

### `tunny-dashboard::state::types::tests::` (14 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| user_attribute_format_preserves_json_kind_and_missing | Keep | UI attribute formatting distinguishes absence/null/literal text and rounds actual numeric values without serializing all values as strings. |
| expanded_attribute_columns_follow_their_key_and_keep_scalar_rows | Keep | Product display-column expansion retains original scalar column and ordered element access without fabricating values for scalar rows. |
| colormap_name_all_has_nine_variants | Delete | Hardcoded literal-list length has no independent routing/selection guarantee and becomes maintenance churn when adding a map. |
| colormap_name_labels_not_empty | Delete | Literal nonempty label smoke does not check actual mapping identity; theme's actual palette routing tests remain. |
| gh_compute_prefs_capture_and_apply_roundtrip | Keep | Capture/apply transfers connection/method settings between fresh dialogs without carrying per-file study/journal paths. |
| gh_compute_prefs_apply_clamps_invalid_values | Keep | Invalid persisted sampler/concurrency counts clamp before becoming run configuration. |
| gh_methods_labels_restrictions_and_persisted_settings | Keep | Actual labels/core method mapping/objective restrictions and persisted adaptive settings survive serialization; apply now targets a fresh dialog rather than the same already-populated instance. |
| gh_compute_prefs_serde_tolerates_missing_fields | Keep | Stored partial settings use product defaults while preserving explicit executable path. |
| gh_opt_dialog_state_derives_defaults_from_path | Keep | Constructor derives per-file output/study names, objective direction count and manual product run defaults. |
| gh_opt_dialog_state_maximize_matches_objective_count | Delete | The same constructor path/count mapping is already checked as exact false/false vector in gh_opt_dialog_state_derives_defaults_from_path; changing count two to three adds no policy. |
| study_view_row_count_and_columns | Keep | Product view exposes current dataframe columns/counts and absence while preserving parameter names. |
| study_view_row_at_matches_columnar_values | Delete | row_at is cfg(test)-only legacy fixture reconstruction, never a production row API. Existing callers still use it as setup; remove only its self-test. |
| study_view_to_trial_rows_roundtrip | Delete | Sole caller of cfg(test)-only bulk fixture reconstruction; remove now-unreferenced to_trial_rows as well. |
| study_view_new_pads_mismatched_pareto_rank | Keep | Mismatched rank vectors become row-aligned provisional zeros rather than partly retaining invalid alignment. |

### `tunny-dashboard::state::journal_studies::tests::` (1 original)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| comparison_failure_releases_later_ready_views_and_reset_discards_pending_views | Keep | Failed earlier comparison releases already-ready later view in request order; reset discards queued old deliveries. |

### `tunny-dashboard::state::layout_state::tests::` (16 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| chart_id_all_contains_surrogate_opt | Delete | ChartId::all is cfg(test)-only fixture enumeration; checking its own list membership cannot verify production panel completeness. |
| surrogate_opt_label_is_correct | Merge | Into panel_item_label_returns_chart_label; retains exact product chart label. |
| panel_item_all_includes_all_charts_and_trial_table | Delete | PanelItem::all is cfg(test)-only and constructed from ChartId::all plus TrialTable; checks its own setup rather than product panel items. |
| panel_item_label_returns_chart_label | Keep | Product label delegation and standalone table label retain independently specified UI text. |
| panel_item_label_returns_trial_table | Merge | Into panel_item_label_returns_chart_label; retains exact Trial Table label. |
| panel_item_derives_clone_eq_hash | Delete | Only checks compiler-derived Clone/Eq/Hash and HashSet lookup of the inserted value. |
| drag_payload_new_widget_returns_item | Delete | Single-variant borrowed-field accessor has no selection/routing decision; this constructs and immediately unwraps its own payload rather than exercising D&D. |
| drag_payload_clone_works | Delete | Only checks derived Clone/Eq. |
| drag_payload_partial_eq_works | Delete | Only checks derived equality for two identical literals. |
| drag_payload_hash_works_in_set | Delete | Only checks derived hashing and standard HashSet membership. |
| layout_state_default_values | Keep | Manual layout defaults preserve hover-reveal closed panel and intended panel dimensions. |
| right_panel_state_default | Merge | Into layout_state_default_values, which already checks the same actual nested defaults. |
| canvas_layout_default_is_empty_and_identity | Keep | Manual canvas defaults must have usable identity zoom/pan and an empty ID allocator. |
| canvas_layout_add_assigns_incrementing_ids | Keep | Product placement keeps distinct IDs/positions even when identical widget content is placed twice. |
| canvas_layout_allows_duplicate_content | Merge | Into canvas_layout_add_assigns_incrementing_ids; now both placements use TrialTable with explicit identical-content/different-ID assertions. |
| canvas_layout_remove_by_id | Keep | Removing one of two identical widgets removes only its instance ID. |

Type/layout/collection continuation: **31 originals**, 15 Keep, 12 Delete,
4 Merge across three fully read test-bearing files. types/study.rs and
types/gh_dialog.rs were also read entirely to distinguish product logic from
cfg(test)-only compatibility fixtures; journal_studies.rs and layout_state.rs
were read in full. A scoped state run exposed the now-unused to_trial_rows helper,
removed after a repository reference check; no production API was removed.

### `tunny-dashboard::state::app_state::tests::` (23 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| app_state_new_initial_values | Keep | Consolidated manual startup state checks empty authority/selections/caches and expected user defaults. |
| app_state_clear_resets_selection | Keep | Actual clear resets selection/filter/highlight/cache/history while preserving loaded studies, artifacts, comparison and pins in one populated fixture. |
| app_state_clear_preserves_studies | Merge | Into app_state_clear_resets_selection; loaded study list is explicitly populated and retained. |
| app_state_new_colormap_defaults | Merge | Into app_state_new_initial_values; retains actual Viridis default. |
| task2110_comparison_fields_default | Merge | Into app_state_new_initial_values; retains false/empty comparison defaults. |
| task2110_artifacts_fields_default | Merge | Into app_state_new_initial_values; retains absent root/empty artifact map. |
| task2110_best_trial_history_default | Merge | Into app_state_new_initial_values; retains absent initial history. |
| clear_preserves_storage_artifacts_and_resets_history | Merge | Into app_state_clear_resets_selection; populated real artifact entry/root survives while populated history is cleared. |
| task2110_clear_preserves_comparison_fields | Merge | Into app_state_clear_resets_selection; preserves populated mode/color state. |
| app_state_default_includes_new_fields | Merge | Into app_state_new_initial_values; retains empty pins/no comparison base. |
| app_state_clear_preserves_pinned_trials | Merge | Into app_state_clear_resets_selection; retains populated pins/base identity. |
| merge_selected_with_pinned_preserves_union | Keep | Selected-row order and duplicate-free pinned-only append must survive view assembly. |
| toggle_pinned_trial_adds_and_removes_entry | Keep | Actual pin toggle returns add/remove result and updates list. |
| toggle_pinned_trial_rejects_21st_entry | Keep | Pin limit rejects new entry without corrupting retained twenty pins. |
| reset_comparison_session_clears_all_comparison_state | Keep | Product comparison reset clears mode/colors/base independent of ordinary study-switch clear. |
| reset_to_empty_drops_the_open_file_and_its_derived_state | Keep | File > New discards storage/selection/derived state; receives pin-drop and confirmation-dismissal checks. |
| reset_to_empty_drops_pinned_trials_unlike_clear | Merge | Into reset_to_empty_drops_the_open_file_and_its_derived_state; populated pins must drop, whereas app_state_clear_resets_selection retains them. |
| reset_to_empty_preserves_data_independent_settings | Keep | File > New preserves theme/colormap/indicator/Compute preferences rather than blindly replacing all state. |
| reset_to_empty_keeps_a_running_optimization_overlay | Keep | Running optimization remains cancellable after New, but finished overlay is discarded; both states checked in sequence. |
| reset_to_empty_drops_a_finished_optimization_overlay | Merge | Into reset_to_empty_keeps_a_running_optimization_overlay; retains explicit finished-run dismissal. |
| reset_to_empty_closes_its_own_confirmation | Merge | Into reset_to_empty_drops_the_open_file_and_its_derived_state; sets confirmation before reset and checks it closes. |
| export_and_pinning_logic_have_dedicated_regression_tests | Delete | Repeats exact pin toggle and twenty-pin-limit behavior of the two named retained tests; never exercises export. |
| comparison_state_transitions_are_covered | Delete | Actual reset assertions duplicate reset_comparison_session_clears_all_comparison_state; remaining spinner assertions only check local true/false literals. |

### `tunny-dashboard::state::messages::tests::` (3 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| app_message_error_variant | Delete | Constructs Error variant and matches its own literal string; compiler enum storage only. |
| pdp_result_one_dim | Delete | Asserts length of a vector just inserted into a struct literal; no product calculation. |
| message_handler_accepts_new_message_family | Delete | Never calls MessageHandler; matches self-constructed cfg(test)-only variant and checks literal nonempty text. |

### `tunny-dashboard::state::message_handler::tests::` (19 originals)

The entire test file and actual handler/study/clustering/GH implementations were
read before judging these. Legacy StudySelected/StudyChunkLoaded/ComparisonStudy*
branches are explicitly cfg(test)-only; production uses stamped Journal batches
and AnalysisSession results. Their self-tests do not validate the production path.
The compatibility helpers/variants still used as setup by pending app/UI tests
are retained; only this standalone self-testing is removed.

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| best_trial_history_set_for_single_objective_minimize | Delete | Only exercises cfg(test)-only StudySelected -> refresh_best_trial_history; actual analysis best-history math and acquired-input publication tests remain. |
| best_trial_history_set_for_single_objective_maximize | Delete | Same test-only route with opposite direction, not production publish_journal/session.history. |
| best_trial_history_none_for_multi_objective | Delete | Same test-only reconstruction gate; production history readiness is independently covered by d4_preparation. |
| clustering_done_updates_state_when_lengths_match | Delete | Direct legacy notification calls fixture_delivery_only and cfg(test)-only cache insertion; actual production SharedAnalysis adoption is tested by analysis_job typed sharing. |
| clustering_done_rejects_mismatched_label_length | Delete | Legacy handler is unreachable after fixture_delivery_only in production; validates a compatibility path, not the production typed handoff. |
| study_chunks_reclassify_history_and_keep_attribute_only_status | Delete | Only exercises cfg(test)-only handle_study_chunk reconstruction, not actual manager/projection publication; real all-state schema growth remains in boundary and stamped publication tests. |
| study_chunks_accumulate_rows_across_batches | Delete | Tests obsolete cfg(test)-only append route; real accepted-batch/manager/projection paths are independently retained. |
| convergence_waits_for_final_constraint_schema_and_rejects_old_snapshot | Delete | Uses test-only chunks, fixture_inputs fallback and cfg(test)-only source_df rejection; actual stamped epoch/revision adoption/readiness is covered by analysis_job/d4 boundaries. |
| study_chunk_handles_ragged_objectives_without_panic | Delete | Tests only legacy fixture chunk handling; real decoded nonfinite/ragged dimension preservation remains in Journal boundary/report tests. |
| study_selected_resets_cluster_widget_runtime_state | Delete | Only validates cfg(test)-only StudySelected handler; actual publication invalidation is checked by analysis_job::d4_publication_invalidates_same_count_schema_results_without_resetting_controls. |
| comparison_load_message_updates_state_entrypoint | Delete | Only executes cfg(test)-only ComparisonStudyLoaded vector assembly, not production complete_journal_comparison. |
| comparison_load_failed_message_sets_load_error | Delete | Only executes cfg(test)-only ComparisonStudyLoadFailed branch. |
| report_export_done_stores_paths_and_clears_generating | Keep | Actual report notification stores success/overwrite paths and clears generation/error; receives closed-dialog no-op. |
| report_export_done_without_dialog_is_noop | Merge | Into report_export_done_stores_paths_and_clears_generating; closes dialog then delivers notification and checks no resurrection/error. |
| error_during_report_generation_surfaces_in_dialog | Keep | Actual generic error path clears report generation and surfaces the same diagnostic in modal/load error. |
| gh_opt_finished_ok_formats_success_message | Keep | Actual GH completion message formats success counts and cancelled suffix from stop reason in a shared fixture table. |
| gh_opt_finished_ok_cancelled_appends_hint | Merge | Into gh_opt_finished_ok_formats_success_message; retains exact cancelled summary text/counts. |
| gh_opt_finished_err_sets_error_string | Keep | Actual fatal error is retained by overlay; receives no-active-run no-op. |
| gh_opt_finished_without_run_state_is_noop | Merge | Into gh_opt_finished_err_sets_error_string; removes overlay before delivery and asserts no resurrection/load error. |

AppState/message continuation: **45 originals**, 13 Keep, 17 Delete, 15 Merge
across three fully read test-bearing files. Product handler branches and public
message variants were not changed. types/process_dialog.rs was opened but remains
pending for complete body/cfg reconciliation; opening a partial file is not audit
completion.

### `tunny-dashboard::app::tests::` (51 originals)

All 3304 original lines (including helpers and both ignored bodies) were read,
alongside app message/consumer orchestration, files/run/reload and state managers.
The HTTP Compute stub in d6 only supplies an external evaluator response; assertions
exercise actual app preparation, recorded journal, progress and reload, not stub
behavior in isolation.

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| d7_live_filter_session_and_csv_export_keep_trial_identity | Keep | Real open/select/filter/session restore/background export preserves sparse global IDs vs study-local numbers through the entry. |
| d6_process_launch_progress_cancel_completion_and_reload | Keep | Real child-process evaluation updates progress/reload; cancel and explicit recording-only reuse preserve old bytes and reject mismatched directions. |
| d6_gh_launch_mock_compute_completion_and_reload | Keep | App constructs Compute definition/request, records returned objective, completes/reloads and explicitly reuses journal without unsafe implicit resume. |
| d5_pdp_1d_rerun_delivers_success_to_both_rebound_canvas_consumers | Keep | Shared helper now exercises both actual 1D/2D requests and both completion orders, preserving canvas result/busy/cache delivery. |
| d5_pdp_2d_rerun_delivers_success_to_both_rebound_canvas_consumers | Merge | Into d5_pdp_1d_rerun_delivers_success_to_both_rebound_canvas_consumers; retains check_pdp_rebound_consumers(true) unchanged. |
| d5_generic_failure_finishes_only_the_rebound_consumers_selected_slot | Keep | Actual app pump fans out rejections only to rebound consumers' selected busy slot, not unrelated importance work. |
| d5_trained_model_reuse_does_not_finish_another_instances_fit | Keep | Model output is shared by Arc/provenance without clearing another instance's pending fit or diagnostic. |
| d5_mcdm_rank_table_and_gallery_share_the_retained_result | Keep | Three actual widget routes select one job/payload and all render against retained result with independent busy controls. |
| d5_canvas_consumers_share_equal_conditions_and_keep_unequal_jobs_independent | Keep | Actual cluster consumers share equal settings, separately finish unequal work and release cache/session interests when removed. |
| d5_global_deliveries_do_not_finish_unrelated_canvas_requests | Keep | Global success/panic updates its own widget but cannot clear a canvas consumer's pending work. |
| journal_late_attributes_correct_the_live_prefix_before_completion_and_reload | Keep | Real parser worker coalesces terminal attributes/constraints into accepted corrections before Complete, preserving old input and reloaded final values. |
| d2_journal_live_open_select_reload_replace_and_headless_projection | Keep | Actual open/select/cached switch/reload/reopen preserves authority identity/generation, independent studies and real table/history rendering. |
| d2_stale_same_path_scan_and_old_loader_cannot_populate_replaced_source | Keep | Old-source same-path scan/batch/failure cannot install data or finish a newer load; obsolete sink is rejected. |
| d2_gui_reload_failure_preserves_the_correct_generation_before_and_after_acceptance | Keep | Failed reload retains old generation before first accepted batch but retains new accepted prefix afterward; comparison failure does not append a partial view. |
| d2_replaced_sqlite_callbacks_cannot_overwrite_a_completed_journal_load | Keep | Real SQLite batch/finish/comparison/error callbacks from replaced source cannot mutate a completed Journal projection/authority/extras. |
| d3_csv_and_sqlite_use_owned_managers_for_selection_and_reload | Keep | Real CSV direction confirmation and SQLite reload use owned managers, retain nonfinite attributes/all states and old immutable inputs. |
| d3_live_servers_use_the_owned_gui_handoff_and_reload | Keep | Ignored opt-in real PostgreSQL/MySQL GUI acquisition/select/reload validates server-specific handoff, all-state metadata and generation changes. |
| d2_streaming_gui_preserves_column_discovery_order_across_the_batch_boundary | Keep | Production 1000-row boundary retains discovered parameter/numeric/string attribute order and actual values. |
| d2_comparison_reload_preserves_request_order_and_full_read_rows_on_reverse_completion | Keep | Real intercepted reverse completion cannot reorder comparison rows/colors/slots; reload preserves full-read path ordering. |
| d2_comparison_cached_streaming_input_is_reused_but_forced_reload_uses_full_read_order | Keep | Comparison reuses exact cached streaming input but forced reload deliberately adopts full-read order/new generation. |
| d2_comparison_in_flight_then_primary_selection_shares_the_study_load | Keep | Shared helper now checks both request orders, one acquisition ticket/worker and shared table/front without orphaned reservations. |
| d2_primary_in_flight_then_comparison_request_shares_the_study_load | Merge | Into d2_comparison_in_flight_then_primary_selection_shares_the_study_load; retains assert_d2_overlapping_journal_load_is_shared(false) unchanged. |
| d2_shared_journal_failure_retry_and_forced_supersede_resolve_comparison_interests | Keep | Real fallible sink rejection releases ordered comparison interests; retry/forced ticket replacement rejects stale finish and completes current shared acquisition. |
| d2_cached_journal_selection_and_comparison_complete_with_a_full_gui_channel | Keep | Cached local activation must not block on a self-send or drain GUI backlog to make space; exact authority stamps/order remain. |
| artifacts_follow_storage_lifecycle_through_app_actions | Keep | Actual scan/manual selection/reload/source replacement/New and extensionless image rendering retain correct storage artifact ownership. |
| cli_root_overrides_adjacent_discovery_and_preserves_scans_across_studies | Keep | Explicit root overrides discovery, avoids study-switch rescans, refreshes recorded metadata and survives New/reopen. |
| artifact_race_late_journal_scan_cannot_replace_requested_storage | Keep | Real delayed scan after D&D/New cannot overwrite requested source, start old artifact scans or display old images. |
| artifact_race_manual_folder_during_reload_stays_authoritative | Keep | Explicit GUI folder chosen during held reload remains authoritative and post-reload scan sees newly appended metadata/files. |
| journal_scan_guard_accepts_normalized_rdb_source | Keep | App scan guard accepts semantically identical driver-suffixed/normalized RDB URLs rather than rejecting current source completion. |
| real_journal_artifacts_through_app_actions | Keep | Ignored user-fixture check validates known 501-entry real artifact population through automatic/manual scans and real loader/rendering. |
| designexplorer_worker_rebases_images_and_gui_selection_survives_late_messages | Keep | Actual CSV worker respects explicit root/rebasing, rejects legacy inference/traversal and retains GUI priority across reopen/format switch/reload/stale messages. |
| designexplorer_omission_uses_parent_and_rejects_other_source_messages | Delete | Only drives cfg(test)-only CsvArtifacts reconstruction rather than actual manager publication; real worker/root precedence remains in preceding test and I/O/boundary tests. |
| session_root_survives_study_activation_and_new_but_omission_still_clears | Delete | Activation uses obsolete cfg(test)-only chunks; root/New/omission semantics are already asserted through real app lifecycle/CLI-root tests and AppState reset. |
| unsupported_drop_message_guides_gh_binary_to_ghx | Keep | Consolidated diagnostics retain binary GH Save As hint, unsupported path/type and unknown path fallback. |
| unsupported_drop_message_lists_supported_types | Merge | Into unsupported_drop_message_guides_gh_binary_to_ghx; retains exact branch substrings. |
| unsupported_drop_message_handles_missing_paths | Merge | Into unsupported_drop_message_guides_gh_binary_to_ghx; retains unknown placeholder. |
| channel_send_receive_journal_parsed | Delete | Sends self-constructed enum through std mpsc and checks literal payload length; never invokes app pump. |
| channel_try_recv_empty_returns_error | Delete | Only standard empty mpsc behavior. |
| convergence_done_maps_to_compute_sync | Keep | Product direct-notification routing must select convergence synchronization rather than leave legacy busy state stuck. |
| surrogate_multi_messages_map_to_compute_sync | Keep | Actual multi-fit/opt success/failure variants route to intended synchronization families. |
| spawn_task_sends_message | Keep | Product background launcher executes closure and delivers its actual result. |
| spawn_task_captures_panic | Keep | Launcher must convert worker panic to a diagnostic delivery instead of silently abandoning busy state. |
| spawn_task_multiple_messages | Delete | Repeats spawn_task_sends_message twice and asserts only count; additional behavior is standard sender clone/channel transport, not project batching. |
| compute_window_title_no_path_returns_base_title | Keep | Consolidated product title oracle retains base/local/secret-masked/no-password URL strings. |
| compute_window_title_local_path_shows_full_path | Merge | Into compute_window_title_no_path_returns_base_title; retains full local title. |
| compute_window_title_rdb_url_masks_password | Merge | Into compute_window_title_no_path_returns_base_title; retains exact masked URL title. |
| compute_window_title_rdb_url_without_password_unchanged | Merge | Into compute_window_title_no_path_returns_base_title; retains password-free URL. |
| has_discardable_state_is_false_on_the_startup_state | Keep | Consolidated product confirmation policy distinguishes startup, scanned studies, open empty storage and canvas-only layout. |
| has_discardable_state_detects_a_scanned_storage | Merge | Into has_discardable_state_is_false_on_the_startup_state; retains populated list-only branch. |
| has_discardable_state_detects_a_storage_holding_no_studies | Merge | Into has_discardable_state_is_false_on_the_startup_state; retains open empty-file branch. |
| has_discardable_state_detects_canvas_items_without_data | Merge | Into has_discardable_state_is_false_on_the_startup_state; retains independent layout-only branch. |

### `tunny-dashboard::app::reload::tests::` (9 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| can_reload_requires_a_non_csv_source_and_an_idle_app | Keep | Actual reload guard distinguishes busy/no-study/CSV vs Journal/SQLite/RDB; receives capture's no-study rejection. |
| capture_returns_none_without_an_active_study | Merge | Into can_reload_requires_a_non_csv_source_and_an_idle_app; retains actual ReloadRestore::capture call. |
| reload_preserves_session_root_and_a_newer_gui_selection | Delete | Actual CLI root and held-reload manual folder tests already cover real preservation; this only assigns fields and calls capture/apply which do not own root state. |
| reload_rescans_explicit_root_on_activation_and_ignores_prior_scan | Delete | Uses obsolete cfg(test)-only chunk activation; finishing_reload_scans_new_and_existing_trial_artifacts_authoritatively plus real CLI/manual reload tests retain actual scan/stale rejection guarantee. |
| apply_drops_state_pointing_at_trials_that_disappeared | Keep | Reload restore drops disappeared trial IDs from selection/pins/highlight rather than treating IDs as row positions. |
| apply_keeps_state_that_still_resolves | Keep | New appended trials do not erase existing selection/pins/highlight/HV/comparison settings. |
| apply_reports_comparison_studies_in_their_original_order | Keep | Capture/apply preserves comparison request order for deterministic display/colors. |
| finishing_reload_scans_new_and_existing_trial_artifacts_authoritatively | Keep | Actual finish reload dispatch updates old/new/removed artifact associations and rejects an older scan completion. |
| finishing_reload_without_an_artifacts_folder_does_not_scan | Keep | No effective artifact folder means no extra async scan/channel message. |

App/reload continuation: **60 originals**, 42 Keep, 7 Delete, 11 Merge across
two fully read test-bearing files. state/types/process_dialog.rs was also finished
and contains no tests. Removed last callers of cfg(test)-only StudySelected,
StudyChunkLoaded and ComparisonStudyLoadFailed; their fixture branches/helpers
and unused test TrialRow.user_attrs field were then removed following reference
checks. Existing fixture initializers in three still-pending widget files only
lost that obsolete field; this does not mark their test bodies reviewed.

The cleanup caused clippy's match-like-matches lint after removing a cfg(test)
arm: the remaining JournalBatch bool match now uses equivalent matches!, with
identical production behavior. No API or product specification was changed.

### `tunny-dashboard::ui::help::help_url::tests::` (4 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| rank_variants_share_the_combined_manual | Keep | Product help routing preserves combined rank page and English/latest URL base, now including standalone table and overview URLs. |
| widget_url_points_at_the_widget_page | Merge | Into rank_variants_share_the_combined_manual; retains exact TrialTable URL. |
| overview_url_points_at_the_documentation_top_page | Merge | Into rank_variants_share_the_combined_manual; retains exact overview URL. |
| every_chart_has_a_slug_shaped_like_a_url_path_segment | Keep | Per-widget documentation routes must remain valid single lowercase path segments, not accidental slashes/spaces/empty links. |

### `tunny-dashboard::ui::widget_states::tests::` (4 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| widget_states_default_has_capture_slot | Delete | Only asserts Option::None supplied by two derived Default implementations. |
| surrogate_opt_state_default_has_expected_flags | Keep | Manual optimizer defaults retain independent 0/1 multi-front axes and clean staged fit/opt state. |
| surrogate_opt_adopt_compute_state_propagates_new_fields | Keep | Product adoption clears old busy/error/model outputs without overwriting per-widget objective/model/validation selections. |
| png_capture_state_transitions_are_covered | Delete | Manually assigns each purported transition and asserts its own field assignments; no capture/app logic runs. |

Help/widget-state continuation: **8 originals**, 4 Keep, 2 Delete, 2 Merge
across two fully read files. Entire WidgetStates source and help URL/launcher/mod
were read; launcher/mod have no local test definitions. No browser launch was
attempted merely to verify a fixed URL string.

### `tunny-dashboard::ui::panels::left_panel::tests::` (4 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| set_filter_updates_filter_ranges | Merge | Into state::filter::tests::set_filter_excludes_out_of_range_trials; asserts stored range while actual study selection changes. |
| set_filter_overwrites_existing | Merge | Into the same state filter test; retains updated range and explicitly checks resulting selected identities. |
| chart_id_all_returns_all_variants | Delete | Only checks cfg(test)-only enum enumeration's own literal length/membership, not production panel items. |
| chart_id_label_not_empty | Delete | Literal label smoke, no rendering or actual route identity; explicit retained label oracles remain. |

### `tunny-dashboard::ui::panels::right_panel::tests::` (2 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| right_panel_default_is_closed | Delete | state::layout_state::layout_state_default_values already checks the same actual nested closed default. |
| panel_item_all_count_matches | Delete | Tests cfg(test)-only list builder rather than production widget groups; now-unused PanelItem::all helper removed. |

### `tunny-dashboard::ui::panels::toolbar::tests::` (8 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| loading_state_clears_on_file_open_sequence | Delete | Constructs local true/None constants; never calls toolbar or open sequence. |
| error_cleared_on_click_simulation | Delete | Only asserts local Option::None; no click or error-clearing function. |
| toolbar_action_variants_compile_and_match | Delete | Constructs own enum vector, discards all matched payloads and asserts literal length; compilation only. |
| export_csv_action_targets_all_three_modes | Delete | Wraps enum literals then matches the wrapper, checking a literal three-element array count. |
| apply_toolbar_actions_handles_cancel_as_noop | Delete | Does not call toolbar action or cancel: ordinary write_csv_to_path success is already covered through real background export in app::d7_live_filter_session_and_csv_export_keep_trial_identity and atomic-write tests. |
| toolbar_emits_add_comparison_action | Delete | Constructs its own AddComparisonStudy action and checks supplied ID; never renders/emits it. |
| chip_remove_emits_remove_action | Delete | Same self-constructed enum check for removal. |
| successful_add_switches_to_comparison_mode | Delete | Manually sets comparison_mode then calls cfg(test)-only ComparisonStudyLoaded branch; real comparison app paths are retained. Its last exclusive fixture variant/handler was removed after reference checking. |

### `tunny-dashboard::ui::canvas::chart_cell::tests::` (3 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| cell_toolbar_shows_menu_for_chart_cells | Delete | Constructs SaveAsPng action and matches it without ever drawing a menu. |
| save_as_png_action_records_target_cell | Keep | Product capture-target recorder stores correct item and both file/clipboard destination choices. |
| copy_image_action_records_clipboard_dest | Merge | Into save_as_png_action_records_target_cell; retains exact clipboard destination/target with the same item fixture. |

### `tunny-dashboard::ui::canvas::viewport::tests::` (6 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| items_bbox_empty_returns_none | Merge | Into items_bbox_multiple_items; retains actual empty builder call. |
| items_bbox_single_item | Merge | Into items_bbox_multiple_items; retains exact singleton min/max geometry. |
| items_bbox_multiple_items | Keep | Consolidated empty/single/multiple bounds retain negative positions and enclosing extrema. |
| fit_view_wide_bbox_produces_zoom_below_1 | Keep | Product fit centers large bounds while zooming out, caps small fits at identity and retains finite degenerate fallback. |
| fit_view_small_bbox_caps_zoom_at_1 | Merge | Into fit_view_wide_bbox_produces_zoom_below_1; retains exact no-overzoom policy. |
| fit_view_degenerate_bbox_no_nan | Merge | Into fit_view_wide_bbox_produces_zoom_below_1; retains finite zoom/pan on zero bounds. |

### `tunny-dashboard::ui::canvas::minimap::tests::` (8 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| layout_respects_max_width | Merge | Into layout_preserves_aspect_ratio; retains wide-world width limit. |
| layout_respects_max_height | Merge | Into layout_preserves_aspect_ratio; retains tall-world height limit. |
| layout_preserves_aspect_ratio | Keep | Product map scaling retains world aspect ratio plus each independently constrained maximum. |
| layout_anchored_bottom_left | Keep | Minimap placement reserves intended lower-left corner/margins instead of overlapping lower-right fit action. |
| layout_degenerate_bounds_finite | Keep | Zero world extent must still produce finite map layout/scale. |
| world_map_roundtrip | Keep | Product world/screen conversions stay inverse for corners/interior/outside points. |
| map_world_roundtrip | Delete | Reverses the same affine inverse-composition property already exercised by world_map_roundtrip at min/center/max; no separate branch/identity oracle. |
| viewport_rect_always_inside_map_bounds | Keep | Actual layout/world_to_map keeps all viewport corners inside computed map extent with nonzero world origin, beyond algebraic inverse equality. |

Panels/canvas continuation: **31 originals**, 8 Keep, 14 Delete, 9 Merge
across six fully read test-bearing files. Their implementations and panel/canvas
module declarations were read in full. Main canvas/view files still require
final source/cfg census; no test-name search is used to claim them audited.
Scoped canvas check: 8 passed. Clippy --workspace --all-targets --locked -- -D
warnings passed after these edits. Newly unused test-only PanelItem::all and
ComparisonStudyLoaded transport/handler/imports were removed; production enums
and behavior remain unchanged.

### `tunny-dashboard::ui::widgets::common::beta_notice_modal::tests::` (4 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| shows_when_nothing_acknowledged | Keep | Consolidated exact-version acknowledgment policy retains no acknowledgment, current acknowledgment, upgrade and downgrade. |
| hides_when_current_version_acknowledged | Merge | Into shows_when_nothing_acknowledged; retains matching-version suppression. |
| shows_again_after_version_change | Merge | Into shows_when_nothing_acknowledged; retains upgrade/downgrade redisplay. |
| default_state_is_closed_and_unchecked | Delete | Only checks compiler-derived bool defaults. |

### `tunny-dashboard::ui::widgets::common::about_modal::tests::` (1 original)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| default_state_is_closed | Delete | Only checks compiler-derived bool Default, without opening/rendering About. |

### `tunny-dashboard::ui::widgets::common::range_math::tests::` (13 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| normalize01_min_maps_to_zero | Keep | Consolidated product normalization preserves both endpoints, zero-span midpoint and lower/upper clamping. |
| normalize01_max_maps_to_one | Merge | Into normalize01_min_maps_to_zero; exact maximum outcome retained. |
| normalize01_degenerate_range_returns_half | Merge | Into normalize01_min_maps_to_zero; zero-span policy retained. |
| normalize01_clamps_out_of_range | Merge | Into normalize01_min_maps_to_zero; both clamp cases retained. |
| value_range_empty_is_none | Merge | Into value_range_basic; empty iterator remains distinguishable from observations. |
| value_range_basic | Keep | Product aggregation retains actual extrema, empty-input result, NaN ignore and caller-owned infinity policy. |
| value_range_ignores_nan | Merge | Into value_range_basic; original mixed-NaN input/oracle retained. |
| value_range_includes_infinite | Merge | Into value_range_basic; retains included positive infinity. |
| finite_value_range_ignores_non_finite | Keep | Separate finite-only range retains mixed finite extrema and no-finite results. |
| finite_value_range_empty_is_none | Merge | Into finite_value_range_ignores_non_finite; empty-input case retained. |
| finite_value_range_all_non_finite_is_none | Merge | Into finite_value_range_ignores_non_finite; filtered-empty case retained. |
| expand_degenerate_equal_expands_by_one | Keep | Product zero-span expansion remains exactly +/-1, without changing nondegenerate spans. |
| expand_degenerate_distinct_unchanged | Merge | Into expand_degenerate_equal_expands_by_one; nondegenerate identity retained. |

Small common-widget continuation: **18 originals**, 5 Keep, 2 Delete, 11 Merge
across three fully read files. convergence_card.rs was also read in full and has
no local test definitions.

### `tunny-dashboard::ui::widgets::common::log_scale::tests::` (3 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| format_log_tick_restores_original_scale | Keep | Product magnitude-dependent tick strings retain zero/fixed/scientific display policy. |
| log10_grid_spacer_places_decades_as_majors | Keep | Actual grid builder emits powers-of-ten majors and nearby minors, but suppresses minors over wide spans. |
| log10_grid_spacer_thins_to_majors_for_wide_range | Merge | Into log10_grid_spacer_places_decades_as_majors; retains independent wide-range call and all-major assertion. |

### `tunny-dashboard::ui::widgets::common::axis_labels::tests::` (8 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| rotated_corners_put_text_end_highest_and_start_lowest | Keep | Actual rotated text bounds preserve correct anchor orientation and extrema consistency. |
| rotated_corners_ry_range_matches_highest_and_lowest | Merge | Into rotated_corners_put_text_end_highest_and_start_lowest; identical corner calculation retains both extrema/range equalities. |
| stride_is_one_when_labels_have_room | Merge | Into stride_relaxes_as_the_spacing_grows; retains large-spacing identity. |
| stride_thins_out_when_rotated_labels_would_still_overlap | Merge | Into stride_relaxes_as_the_spacing_grows; retains cramped-label thinning. |
| stride_relaxes_as_the_spacing_grows | Keep | Product dynamic stride thins cramped labels and restores names when zoom increases spacing. |
| horizontal_labels_need_their_full_width_of_spacing | Keep | Horizontal and rotated labels need different occupied-width policies, not a single shared stride. |
| stride_of_single_label_is_one | Merge | Into degenerate_spacing_falls_back_to_a_single_label; retains singleton guard with original horizontal geometry. |
| degenerate_spacing_falls_back_to_a_single_label | Keep | Zero/NaN spacing cannot create step_by(0); single-label input remains stride one. |

### `tunny-dashboard::ui::widgets::common::rdb_url_modal::tests::` (7 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| classify_accepts_postgresql_and_mysql_loopback | Keep | Consolidated product classifier checks normalized URL and implicit-plaintext notice for both protocols/normalization/whitespace. |
| classify_normalizes_short_scheme_and_driver_suffix | Merge | Into classify_accepts_postgresql_and_mysql_loopback; retains short/suffixed input with exact expected normalized URL. |
| classify_trims_whitespace | Merge | Into classify_accepts_postgresql_and_mysql_loopback; retains padded input and exact trimmed output. |
| classify_rejects_unsupported_scheme | Keep | Product UI classification rejects local/SQLite/empty input as Invalid rather than connectable. |
| classify_notifies_when_sslmode_disable_is_not_explicit | Merge | Into classify_accepts_postgresql_and_mysql_loopback; now exact PlaintextImplicit outcomes rather than a helper hiding implicit/explicit distinction. |
| classify_skips_notice_when_sslmode_disable_is_explicit | Keep | Explicit plaintext opt-in suppresses warning and permits acknowledged remote host. |
| classify_blocks_urls_rejected_by_tls_precondition | Keep | Remote unacknowledged plaintext and unsupported required TLS are blocked with actionable diagnostic. |

### `tunny-dashboard::ui::widgets::common::csv_import_modal::tests::` (3 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| from_meta_sorts_bounds_and_defaults_to_minimize | Keep | Actual settings adapter sorts parameter bounds, retains initial directions and applies edited direction/ranges. |
| apply_to_overwrites_directions_and_bounds | Merge | Into from_meta_sorts_bounds_and_defaults_to_minimize; same metadata fixture retains exact edited values/maximize outcome. |
| bounds_valid_rejects_inverted_or_nonfinite | Keep | Invalid edited bounds must disable import, including reversed interval and nonfinite endpoint. |

### `tunny-dashboard::ui::widgets::common::license_modal::tests::` (6 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| empty_filter_matches_all | Keep | Consolidated product search policy handles empty/name/license/no-match with actual mixed-case metadata. |
| filter_matches_name | Merge | Into empty_filter_matches_all; partial-name match retained. |
| filter_matches_license | Merge | Into empty_filter_matches_all; license-name/case-normalization match retained. |
| filter_rejects_non_match | Merge | Into empty_filter_matches_all; negative search retained. |
| app_entry_carries_the_embedded_mit_license | Keep | Actual shipped app license includes MIT and warranty disclaimer, not merely an empty generated label. |
| default_state_is_closed | Delete | Only checks derived bool/String Default. |

### `tunny-dashboard::licenses::tests::` (4 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| app_version_matches_cargo_package | Delete | APP_VERSION is defined by the same env! expression used as expected value, with no independent version oracle. |
| app_repository_is_an_https_url | Delete | Literal constant prefix smoke, not a version/link/caller behavior check. |
| licenses_are_collected | Keep | Actual build-generated license array must not silently fall back to empty and entries retain name/version. |
| entries_have_name_and_version | Merge | Into licenses_are_collected; retains every generated entry's required fields. |

Additional common/metadata continuation: **31 originals**, 15 Keep, 3 Delete,
13 Merge across six fully read test-bearing files. Entire implementations and
fixture helpers were read, plus plot_nav/modal/new_confirm sources (no local
tests) and build.rs. The build script's include-generated license body contains
only a static data array, not hidden generated test definitions. No dependency
license file content was treated as project test source.

### `tunny-dashboard::ui::widgets::common::report_modal::tests::` (8 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| default_state_has_html_only | Keep | Manual report defaults select HTML/English/top-ten with no in-progress/completed export. |
| selected_formats_rejects_empty_selection | Keep | Consolidated format policy rejects empty selection, accepts Markdown-only and retains stable HTML/Markdown/JSON order. |
| selected_formats_preserves_html_md_json_order | Merge | Into selected_formats_rejects_empty_selection; enables all three on the same state and retains exact ordered result. |
| default_file_name_sanitizes_unsafe_characters | Keep | Consolidated filename builder sanitizes unsafe names, retains safe names, supplies empty-name fallback and chooses first selected format extension. |
| default_file_name_falls_back_when_fully_sanitized | Merge | Into default_file_name_sanitizes_unsafe_characters; preserves empty/all-unsafe fallback. |
| default_file_name_for_derives_extension_from_first_format | Merge | Into default_file_name_sanitizes_unsafe_characters; retains JSON/Markdown-first/empty selection extensions. |
| export_paths_derives_sibling_extensions_from_base_path | Keep | Actual format mapping derives all sibling paths and appends an extension to extensionless base paths. |
| export_paths_replaces_extension_even_without_dot_in_base | Merge | Into export_paths_derives_sibling_extensions_from_base_path; retains exact extensionless JSON output. |

### `tunny-dashboard::ui::widgets::common::heatmap::tests::` (1 original)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| normalize_clamps_to_unit_range | Delete | normalize delegates unchanged to range_math::normalize01; its endpoint/clamp/degenerate policies are already checked directly. |

### `tunny-dashboard::cli::tests::` (11 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| parses_no_args_as_app_launch | Keep | Empty command line launches default app without source/root and with notice enabled. |
| parses_version_commands | Keep | All three product version aliases select PrintVersion, not a startup path. |
| parses_input_option | Keep | Both input aliases preserve supplied source path. |
| parses_no_beta_notice_flag | Keep | Verification-only flag disables notice with/without an input file. |
| ignores_finder_process_serial_number | Keep | LaunchServices token must not abort startup or interfere with a following input option. |
| rejects_positional_input | Keep | Unsupported positional source is rejected instead of silently changing source selection. |
| rejects_missing_input_path | Keep | Missing source value returns a controlled parse error. |
| artifact_options_are_independent_and_order_independent | Keep | Both root aliases work alone/before/after input while retaining independent source and absolute artifact root. |
| artifact_requires_one_directory_argument | Keep | Missing/option-like/duplicate root arguments reject with meaningful diagnostics. |
| artifact_rejects_missing_paths_and_files | Keep | Existing regular file and absent path must not be accepted as artifact roots; diagnostic identifies offending path. |
| relative_artifact_directory_uses_launch_cwd | Keep | Relative root resolves against launch CWD and unknown-argument usage advertises artifact option. |

Report/heatmap/CLI continuation: **20 originals**, 15 Keep, 1 Delete, 4 Merge
across three fully read test-bearing files. cluster_controls.rs was read in full
and has no local tests; its real queueing paths are exercised by widget/app tests,
not inferred complete merely from the absence of local definitions.

### `tunny-dashboard::ui::widgets::stats::histogram::tests::` (4 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| histogram_chart_default_values | Keep | Manual histogram defaults retain automatic Sturges rule and twenty manual bins with no cached result. |
| cache_key_changes_with_rule | Delete | Constructs two tuple literals outside production key construction; at most checks the rule discriminant, never actual cache invalidation. |
| cache_key_changes_with_manual_bins | Delete | Only checks tuple Eq for different literal bin counts; never calls histogram show/key builder. |
| cache_key_changes_with_row_count | Delete | Only checks tuple Eq for different literal lengths; no production update occurs. |

### `tunny-dashboard::ui::widgets::stats::correlation_matrix::tests::` (3 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| correlation_matrix_chart_default_values | Keep | Manual correlation defaults select Pearson and both parameter/objective groups. |
| cache_key_changes_with_method | Delete | Constructs two tuple literals with method discriminants, not the actual key/cache path. |
| cache_key_changes_with_column_group_toggles | Delete | Standard tuple inequality on literal bools cannot verify production group-based invalidation. |

### `tunny-dashboard::ui::widgets::stats::violin_plot::tests::` (1 original)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| density_scale_gives_every_curve_the_same_max_width | Keep | Product renderer's peak-normalization keeps differing real KDE curves at the same visual maximum width. |

### `tunny-dashboard::ui::widgets::stats::box_plot::tests::` (1 original)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| value_combo_updates_statistics_in_same_frame_and_same_count_refreshes | Keep | Actual accessibility-driven combo click updates computed values in the same frame and same-count new snapshots refresh renderer cache. |

### `tunny-dashboard::ui::widgets::stats::distribution::tests::` (9 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| candidates_defaults_stale_selections_and_numeric_value | Keep | Product value/group candidates distinguish objective/discrete/continuous/CSV-numeric roles and stale selections fall back on new schema. |
| identity_presence_alignment_sparse_groups_and_normalization | Keep | Typed category identities, absence, finite filtering and whole-population normalization preserve sparse groups and numeric discrete order. |
| constant_singleton_all_skipped_and_no_group | Keep | Singleton/constant/nonfinite-only populations preserve normalized box fallback while skipping invalid KDE with the correct group count. |
| decimal_constant_raw_violin_is_skipped_and_reported_in_empty_ui | Keep | Decimal constants do not pass KDE eligibility due to floating variance noise; actual rendered empty UI reports the skipped group/reason. |
| same_name_same_count_snapshot_refresh_and_widget_independence | Keep | Real DistributionCache refreshes on dataframe identity change despite same count and independent box/violin settings remain separate. |
| same_named_objective_and_parameter_remain_distinct | Keep | Value identity selects objective or parameter role despite identical display name. |
| generated_category_labels_cannot_collide_with_literal_labels | Keep | Recursive qualification distinguishes typed/literal/generated labels, including number/string/boolean/null/empty collisions. |
| csv_numeric_parameters_are_value_only_and_categories_keep_parsed_presence | Keep | Real CSV ingestion does not invent discrete groups from ordinary numeric parameters and preserves empty categorical presence. |
| both_widgets_expose_equivalent_controls_and_violin_reports_all_skipped | Keep | Actual rendered box/violin control labels and sparse-category skipped diagnostics remain consistent. |

Statistics widgets: **18 originals**, 13 Keep, 5 Delete across five fully read
test-bearing files and their complete production implementations.

**Out-of-scope product issue discovered, not fixed:** HistogramChart::show uses
(study_name, selected_col, rule, manual_bins, row_count) as its cache key, and
CorrelationMatrixChart::show uses (study_name, method, column toggles, row_count).
Neither includes dataframe identity/revision/content, and WidgetStates input
invalidation does not clear those private caches. Thus a same-name/same-count
refreshed input with changed values can display stale histogram/correlations.
The removed literal-tuple tests never exercised that production condition and
could not catch it. Fixing product cache invalidation is outside this test-only
contract; no replacement test was added to compensate counts or change behavior.

### `tunny-dashboard::ui::widgets::surrogate::anchor::tests::` (2 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| center_choice_default_is_best_trial | Delete | Only asserts compiler-derived enum Default; real widget manual defaults retain anchor policy. |
| center_choice_serde_round_trip_keeps_variant_names | Keep | Actual persisted anchor JSON schema retains independently specified BestTrial/Pinned representations. |

### `tunny-dashboard::ui::widgets::surrogate::compare::tests::` (3 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| surrogate_compare_chart_default_values | Delete | Only compiler-derived field defaults, no product constructor or deserialization decisions. |
| adopt_compute_state_propagates_and_keeps_selection | Keep | Product comparison adoption clears busy/error while retaining objective and slice selections. |
| model_kinds_includes_moe_only_when_requested | Keep | Expensive optional MoE is excluded/included according to explicit product request, not silently always fitted. |

### `tunny-dashboard::ui::widgets::surrogate::robustness::tests::` (5 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| robustness_chart_default_values | Keep | Manual noise/spec/sample defaults and clean fit/cache state retain the intended analysis setup. |
| noise_dist_kind_default_is_normal | Delete | Derived enum Default, also already included in actual robustness chart default oracle. |
| noise_dist_labels_cover_all_choices | Delete | Nonempty literal-label smoke has no independent distribution routing/value guarantee. |
| cache_key_changes_with_distribution_shape_and_specs | Keep | Calls actual product key constructor, ensuring distribution/shape/LSL/USL/fit-generation distinguish calculations; unlike removed literal tuple tests it exercises project construction. |
| adopt_compute_state_propagates_and_keeps_selection | Keep | Product adoption clears old fitting/error without overwriting per-instance objective/noise selections. |

### `tunny-dashboard::ui::widgets::surrogate::response_surface::tests::` (2 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| response_surface_chart_default_values | Keep | Manual surface grid/uncertainty/observation defaults and nonidentity isometric camera retain usable initial view. |
| adopt_compute_state_propagates_and_keeps_selection | Keep | Product surface adoption clears fit state but preserves objective and parameter-axis choice. |

Surrogate widget continuation: **12 originals**, 8 Keep, 4 Delete across four
test-bearing files. Complete anchor/compare/response-surface/robustness source,
robustness cache/render and all test bodies were read. Other surrogate_opt widget
submodules remain pending until separately recorded.

### `tunny-dashboard::ui::widgets::scatter::rank_plot::tests::` (10 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| compute_rank_percentiles_ascending_minimize | Keep | Consolidated rank mapping retains actual ascending/reversed source order and opposite objective direction. |
| compute_rank_percentiles_descending_minimize_reorders | Merge | Into compute_rank_percentiles_ascending_minimize; original reversed-array oracle retained. |
| compute_rank_percentiles_maximize_flips_order | Merge | Into compute_rank_percentiles_ascending_minimize; original maximize oracle retained. |
| compute_rank_percentiles_ties_are_averaged | Keep | Equal objective values receive averaged normalized ranks, not arbitrary source ordering. |
| compute_rank_percentiles_nan_is_worst | Keep | Consolidated nonfinite/singleton/empty policy retains finite-only denominator, sole-finite best and invalid worst. |
| compute_rank_percentiles_single_element_is_best | Merge | Into compute_rank_percentiles_nan_is_worst; sole finite observation remains best. |
| compute_rank_percentiles_single_nan_is_worst | Merge | Into compute_rank_percentiles_nan_is_worst; sole invalid observation remains worst. |
| compute_rank_percentiles_empty_is_empty | Merge | Into compute_rank_percentiles_nan_is_worst; empty request remains empty. |
| rank_colormap_input_puts_best_at_high_end | Keep | Product display intentionally reverses best/worst colormap orientation with intermediate rank mapping. |
| rank_plot_chart_default | Keep | Manual 2D axes start at distinct parameters 0/1, not a collapsed diagonal. |

### `tunny-dashboard::ui::widgets::scatter::pca_biplot::tests::` (7 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| pca_coloring_uses_retained_source_rows | Keep | Actual drawing color lookup follows PCA source-row mapping, not compacted projection indices. |
| defaults_are_param_space_with_loadings_and_no_color | Keep | Manual PCA initial space/loading/color policy remains usable. |
| space_disc_is_stable_and_distinct | Merge | Into to_core_maps_each_variant; retains pairwise distinct cache tags to prevent cross-space cache reuse, without pinning private numeric values. |
| to_core_maps_each_variant | Keep | Product UI-to-analysis conversion retains actual requested feature space. |
| color_range_ignores_non_finite | Keep | Consolidated color-range policy excludes nonfinite observations and preserves missing/all-invalid fallback. |
| color_range_none_column_is_zero_zero | Merge | Into color_range_ignores_non_finite; missing column fallback retained. |
| color_range_all_non_finite_is_zero_zero | Merge | Into color_range_ignores_non_finite; all-invalid fallback retained. |

### `tunny-dashboard::ui::widgets::scatter::cluster_scatter_3d::tests::` (1 original)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| cluster_scatter_3d_default_axes | Keep | Manual distinct 0/1/2 axes, initial infeasible visibility and nonidentity camera retain a meaningful initial 3D view. |

Rank/PCA/3D-cluster continuation: **18 originals**, 10 Keep, 1 Delete,
7 Merge across three fully read files, including complete implementations.

Additional out-of-scope cache issue: ClusterScatter3D's range_cache key contains
only selected axes and trial_count, while clear_runtime_state and input
invalidation do not clear that private range cache. A changed same-count input
can therefore retain obsolete 3D axis ranges. This is recorded, not fixed under
the test-only contract, alongside Histogram/Correlation cache issues above.

### `tunny-dashboard::ui::widgets::surrogate::surrogate_opt::labels::tests::` (2 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| verdict_returns_correct_category | Keep | Product quality verdict maps actual CV score ranges to distinct text/colors. |
| labels_cover_all_choices | Delete | Literal-label nonempty smoke does not verify actual selected model/optimizer routing. |

### `tunny-dashboard::ui::widgets::surrogate::surrogate_opt::fit::tests::` (2 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| fit_click_builds_pending_fit_from_selections | Delete | Reimplements button body inside test and checks own pending literal; never renders/clicks production fit UI. |
| multi_fit_click_builds_pending_multi_fit | Delete | Same copied test-local branching/assignment for multi-fit, not actual control handoff. |

### `tunny-dashboard::ui::widgets::surrogate::surrogate_opt::optimize::tests::` (7 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| improvement_delta_minimize_positive | Keep | Consolidated actual delta function retains positive/negative/zero outcomes under both objective directions. |
| improvement_delta_minimize_no_improvement | Merge | Into improvement_delta_minimize_positive; original worse-prediction sign check retained. |
| improvement_delta_maximize_positive | Merge | Into improvement_delta_minimize_positive; original maximize delta oracle retained. |
| improvement_delta_maximize_no_improvement | Merge | Into improvement_delta_minimize_positive; original negative maximize check retained. |
| improvement_delta_exact_zero | Merge | Into improvement_delta_minimize_positive; both equal-value direction cases retained. |
| result_arrival_switches_spinner_off | Delete | Assigns result and optimizing=false locally, then asserts those assignments; no arrival/handler occurs. |
| adopt_compute_state_keeps_selections | Keep | Actual adoption carries a populated single result/error and clears busy while retaining objective/model/optimizer; removed unrelated Arc compile-only assertion/import. |

### `tunny-dashboard::ui::widgets::surrogate::surrogate_opt::tests::` (6 originals, mod.rs)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| optimize_click_requires_matching_trained | Delete | Test-local Option::None map chain never calls trained_matches or actual Optimize control. |
| multi_trained_matches_correct | Keep | Actual multi-model selection guard accepts exact match/rejects wrong model/objective set/count; same fitted fixture now also tests adoption of populated multi-trained payload. |
| multi_trained_matches_wrong_model | Merge | Into multi_trained_matches_correct; changes selected model after success using same actual fitted pair. |
| multi_trained_matches_wrong_objectives | Merge | Into multi_trained_matches_correct; mismatched objective identity still rejects. |
| multi_trained_matches_wrong_length | Merge | Into multi_trained_matches_correct; mismatched objective count still rejects. |
| adopt_compute_state_propagates_multi_trained | Merge | Into multi_trained_matches_correct; retains actual populated Arc vector adoption/busy clear without two more unnecessary GP fits. |

Surrogate optimizer continuation: **17 originals**, 4 Keep, 5 Delete,
8 Merge across four fully read test-bearing files. Complete optimizer module,
fit/optimize/labels bodies and front_view/tables/suggest were read; the latter
three have no local tests. No copied button simulation was replaced with a
count-compensating new test.

### `tunny-dashboard::ui::widgets::common::cluster_table::tests::` (6 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| cluster_table_default_hides_unclustered | Keep | Manual table policy initially excludes unclustered rows. |
| cluster_counts_aggregates_per_label | Keep | Actual cluster summary counts each label including the unclustered sentinel. |
| visible_indices_excludes_unclustered_by_default | Keep | Consolidated row builder preserves hide/show policy, cluster-then-source ordering and wholly hidden result. |
| visible_indices_includes_unclustered_when_requested | Merge | Into visible_indices_excludes_unclustered_by_default; same fixture retains visible sentinel rows last. |
| visible_indices_sorts_by_cluster_then_trial | Merge | Into visible_indices_excludes_unclustered_by_default; independent shuffled labels retain exact ordering. |
| visible_indices_empty_when_all_unclustered_and_hidden | Merge | Into visible_indices_excludes_unclustered_by_default; all-hidden empty result retained. |

### `tunny-dashboard::ui::widgets::decision::comparison_table::tests::` (6 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| defaults_show_params_and_highlight_best | Keep | Manual comparison table defaults include parameters/best highlight but not user attributes. |
| best_pin_index_minimize_picks_smallest | Keep | Consolidated product best-cell resolver retains min/max, finite filtering and empty/all-invalid results. |
| best_pin_index_maximize_picks_largest | Merge | Into best_pin_index_minimize_picks_smallest; same numeric fixture retains opposite objective direction. |
| best_pin_index_ignores_non_finite | Merge | Into best_pin_index_minimize_picks_smallest; mixed nonfinite input retained. |
| best_pin_index_all_non_finite_returns_none | Merge | Into best_pin_index_minimize_picks_smallest; no finite winner retained. |
| best_pin_index_empty_returns_none | Merge | Into best_pin_index_minimize_picks_smallest; empty population retained. |

### `tunny-dashboard::ui::widgets::decision::radar_comparison::tests::` (5 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| defaults_are_objectives_only_and_outward_better | Keep | Manual radar comparison policy starts objectives-only and direction-aware outward improvement. |
| normalize_normal_case | Keep | Consolidated radius normalization retains fractional values, flipped direction, degenerate midpoint and clipping. |
| normalize_flip_case | Merge | Into normalize_normal_case; original flipped fractions retained. |
| normalize_degenerate_range_is_mid_radius | Merge | Into normalize_normal_case; both flip choices retain midpoint on zero span. |
| normalize_clamps_out_of_range_values | Merge | Into normalize_normal_case; exact lower/upper clamps retained. |

### `tunny-dashboard::ui::widgets::common::radar_chart::tests::` (5 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| axis_scale_outer_maps_to_front_max | Keep | Consolidated scale/fraction policy retains actual front envelope/margin and degenerate-center behavior. |
| axis_scale_handles_degenerate_front | Merge | Into axis_scale_outer_maps_to_front_max; singleton front still expands around value and maps to midpoint. |
| value_fraction_zero_span_is_center | Merge | Into axis_scale_outer_maps_to_front_max; direct zero-span fraction fallback retained. |
| build_objectives_first_then_params_and_collects_front | Keep | Actual radar data builder preserves objective/parameter order/roles, finite front envelope and selected/front row alignment. |
| build_skips_axis_without_front_values | Keep | Missing numeric axis disappears without misaligning remaining front vectors. |

### `tunny-dashboard::ui::widgets::common::process_def_modal::tests::` (3 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| from_then_to_definition_round_trips | Keep | Actual GUI flatten/rebuild preserves command, arguments, directory, extractors, constraints and optional hooks. |
| new_starter_needs_a_program | Keep | Product starter deliberately stays invalid until user supplies an executable, preventing accidental empty command launch. |
| to_definition_drops_blank_names | Keep | Actual form rebuild drops blank rows without losing ordered named parameters/objectives and produces valid definition. |

### `tunny-dashboard::ui::widgets::common::ghx_opt_modal::tests::` (3 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| nsga2_total_evaluations_matches_runner_evenization | Keep | GH dialog's actual budget function honors minimum/even population and initial generation; it is independent code from process dialog. |
| default_state_has_no_maximize_flags_set | Delete | Only calls same GhOptDialogState::new fixture and checks false/false already checked by state::types::gh_opt_dialog_state_derives_defaults_from_path. |
| rendered_method_labels_and_hints_are_explicit_and_safe | Keep | Actual egui modal render exposes correct method-specific restrictions/budget/BO explanation without misleading adaptive/auto-model terms. |

### `tunny-dashboard::ui::widgets::common::process_opt_modal::tests::` (1 original)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| nsga2_total_evaluations_matches_runner_evenization | Keep | Separate process-dialog budget implementation must match minimum/even population and initial generation; not merely a call to the GH helper. |

Decision/table/process continuation: **29 originals**, 16 Keep, 1 Delete,
12 Merge across seven fully read files and their complete production builders.
No shared production abstraction was introduced just to eliminate repeated
dialog budget code; only independent tests with real behavior were retained.

### `tunny-dashboard::ui::widgets::common::trial_table::tests::` (8 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| user_attributes_default_on_and_old_widget_state_loads_on | Keep | Manual attribute visibility policy also applies when loading a session with omitted new fields. |
| get_display_rows_empty_selected_returns_all | Delete | Only calls cfg(test)-only row reconstruction, not actual show_all visible-cache/filter path. |
| get_display_rows_filters_by_trial_id | Delete | Same test-only duplicated selector; cannot verify production rendering/selection. |
| get_display_rows_nonexistent_id_excluded | Delete | Same test-only helper, never actual visible row computation. |
| get_display_rows_keeps_pinned_rows_visible | Delete | Same cfg(test)-only selector; product selected/pin union and real table/card rendering tests remain. |
| pin_icon_reflects_current_state | Delete | Reimplements literal conditional locally and asserts its own output, not actual pin rendering. |
| pin_limit_error_is_surfaceable_to_ui | Delete | Repeats actual AppState pin-limit test without exercising any UI surfacing. |
| pin_row_then_change_selection_row_stays_visible | Delete | Calls same cfg(test)-only selector with another pin; no actual row pin/selection transition occurs. |

### `tunny-dashboard::ui::widgets::common::trial_detail_modal::tests::` (8 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| expanded_array_child_key_is_indented | Keep | Actual egui key layout indents child attribute entries relative to their parent. |
| nearest_within_returns_closest_in_threshold | Keep | Consolidated product hit resolver retains within/outside/empty and closer-of-multiple candidate behavior. |
| nearest_within_none_outside_threshold | Merge | Into nearest_within_returns_closest_in_threshold; outside-threshold rejection retained. |
| nearest_within_empty_is_none | Merge | Into nearest_within_returns_closest_in_threshold; no candidates retained. |
| nearest_within_picks_strictly_closest | Merge | Into nearest_within_returns_closest_in_threshold; original two-in-range closer candidate retained. |
| fmt_opt_formats_and_handles_none | Keep | Product numeric detail text retains four decimals and missing-value marker. |
| user_attribute_rows_preserve_types_missing_values_and_order | Keep | Actual detail row builder preserves mixed JSON/string/numeric formatting, missing values and deterministic attribute order. |
| user_attribute_rows_expand_only_present_array_elements | Keep | Actual expansion retains parent/child kinds, numeric alignment, short-array and scalar behavior without invented children. |

### `tunny-dashboard::ui::widgets::common::artifact_gallery::sections::tests::` (11 originals)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| artifact_trials_with_index_filters_and_sorts | Keep | Product eligible-artifact selector preserves sorted IDs and requested original entry index, excluding empty/missing slots. |
| restrict_to_current_study_keeps_only_study_trials | Keep | Storage-wide artifact IDs stay study-scoped; receives no-study empty behavior on same map. |
| restrict_to_current_study_empty_without_study | Merge | Into restrict_to_current_study_keeps_only_study_trials; removes current context and checks actual result empty. |
| filter_ids_by_selection_empty_returns_all | Keep | Consolidated selection policy retains empty=all and exact selected-ID filtering with unknown ID. |
| filter_ids_by_selection_keeps_only_selected | Merge | Into filter_ids_by_selection_empty_returns_all; original selection/input retained. |
| paginate_clamps_range | Keep | Product pagination preserves final short page, out-of-range empty and zero page-size guard. |
| cluster_sections_groups_and_puts_unclustered_last | Keep | Actual cluster grouping maps sparse source IDs, preserves member order and moves sentinel last; receives missing-artifact exclusion after map removal. |
| cluster_sections_excludes_trials_without_artifacts | Merge | Into cluster_sections_groups_and_puts_unclustered_last; after full grouping checks remove all but one association and retain exact one-member result. |
| cluster_sections_selects_requested_artifact_index | Keep | Actual cluster cards use requested entry rather than first artifact, skipping trials missing that index. |
| mcdm_ordered_respects_rank_and_topn | Keep | Actual ranking preserves global score/rank/ID correspondence and top-N while sparse associations do not renumber overall rank. |
| mcdm_ordered_skips_trials_without_artifacts | Merge | Into mcdm_ordered_respects_rank_and_topn; same ranking fixture retains only middle-ranked artifact and original rank two. |

### `tunny-dashboard::ui::widgets::common::artifact_gallery::card::tests::` (5 originals, card_tests.rs)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| journal_cards_display_local_numbers_and_keep_global_links_after_study_switch | Keep | Actual rendering/click/detail/table paths show local numbers despite global IDs/gaps and retain source artifact/highlight/pin targets through study switches. |
| cluster_render_keeps_numbers_through_filtering_and_global_click_and_pin_targets | Keep | Actual filtered cluster table click/pin uses global identity while displaying correct local number. |
| trial_table_header_is_trial_number_in_every_mode | Keep | All/cluster/MCDM rendered headers use correct semantic label and fit initial width. |
| card_and_anchor_missing_number_use_original_row_not_global_id | Keep | Missing number fallback uses original source row for actual card/anchor label, not sparse global ID. |
| single_study_card_number_is_unchanged | Delete | Earlier real multi-study card rendering already explicitly checks Trial 0 as well as gapped Trial 2; single-study count does not select a different card renderer path. |

### `tunny-dashboard::ui::widgets::common::artifact_gallery::card::layout_tests::` (1 original, card.rs)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| card_columns_fits_width_and_min_one | Keep | Product card grid computes fitting columns and never returns zero columns in cramped areas. |

### `tunny-dashboard::ui::widgets::common::artifact_gallery::tests::` (2 originals, mod.rs)

| Original function | Decision | Regression / retained destination |
| --- | --- | --- |
| study_scoped_selector_and_switching_in_every_mode | Keep | Actual gallery render clamps artifact entry bound per study (not storage/selection), preserves valid choices and handles empty/missing entries in every mode. |
| adopt_cluster_runtime_clears_stuck_computing | Keep | Product gallery runtime adoption releases its own cluster busy field. |

Trial/gallery continuation: **35 originals**, 20 Keep, 8 Delete, 7 Merge
across six fully read test-bearing files, including complete table/modal/gallery
production paths and rendering helpers. Repository references showed row_at was
now used only by the removed test-only get_display_rows helpers, so those helpers,
their fixture/imports and cfg(test)-only StudyView::row_at were removed together.
Scoped common widgets: 55 passed; all-target clippy passed after these edits.

Additional out-of-scope cache issue: TrialTable::show_all keys its visible row
cache only by selected IDs, pinned IDs and row count, not dataframe identity or
the row-to-global-ID mapping. Same-count source/row-order changes with unchanged
selection/pins can therefore retain wrong visible row indices. Product input
invalidation does not clear this private cache. Recorded, not fixed; deleted
test-only selectors never exercised this production cache.

## Validation actually executed (first pass)

Windows, existing LightGBM native libraries. No commit or dependency changes.

| Command | Result |
| --- | --- |
| `cargo test --workspace --locked --profile ci-test -- --list` (baseline) | Success; 2866 registered cases; five doc harnesses with zero cases; native harness skipped without opt-in. |
| `cargo test --workspace --locked --profile ci-test` (first-pass checkpoint) | Success: **2798 passed, 0 failed, 16 ignored**, plus zero-case doc tests. Native harness initially skipped by its environment gate. |
| `cargo fmt --all -- --check` | Success. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Success. |
| `git diff --check` | Success for the code edits. |
| `TUNNY_NATIVE_CAPTURE_CHECK=1 cargo test -p tunny-dashboard --locked --profile ci-test --test artifact_animation_native` (PowerShell environment assignment, removed in finally) | Success: native full-app wgpu preview/GIF, three frames, matching live/GIF order, 684x544, 330 ms, complete prefix observations, two independent instances, seek/pause restoration, image aspect ratios and content-only crop. |

Baseline registered counts come from listing, **not a baseline passing test run**.
The first-pass workspace test log is locally retained at
`C:\Users\dev\.local\share\opencode\tool-output\tool_163880b00001rP103XTv9dBozH`;
baseline list at `tool_1638404510011w06ttbP2xlVn0` in the same directory.
The 16 ignored cases were not explicitly run. No live PostgreSQL/MySQL, licensed
Rhino.Compute, other-platform test execution or performance-probe claim is made.
Passing execution of unreviewed tests is not source-audit evidence.

## Continuation validation checkpoint (not final completion)

- `cargo fmt --all`: applied formatting to test edits only.
- `cargo test -p tdashboard-study -p tdashboard-io -p tdashboard-mcp -p tdashboard-optimization-runner --locked --profile ci-test`: passed after Study/MCP/runner/I/O audit edits: I/O 100 passed/3 ignored; MCP 15 unit +4 integration passed/2 ignored; runner 97 passed; Study 2+9 passed; three zero-case doc harnesses passed.
- This scoped run precedes the MCDM continuation edits and is not final validation
  for those edits. All three exact workspace checks remain required after the
  final repository-wide changes.

After the code edits through the earlier 636-definition checkpoint:

| Command | Actual result |
| --- | --- |
| `cargo test --workspace --locked --profile ci-test` | Success: **2714 passed, 0 failed, 16 ignored** (2730 registered cases). Analysis 616/3 ignored; I/O 100/3; MCP 15+4/2; runner 97; Study 2+9; GUI 843/2 ignored in each unit target; boundary harness counts unchanged. Five doc harnesses: zero cases, success. Default native entry skipped without environment opt-in. |
| `cargo fmt --all -- --check` | Success after formatting test-only edits. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Success, including examples/benchmarks/test targets; compilation is not their body-audit evidence. |
| `git diff --check` | Success. Git also warns about future LF-to-CRLF normalization; no whitespace error. |
| `$env:TUNNY_NATIVE_CAPTURE_CHECK='1'; cargo test -p tunny-dashboard --locked --profile ci-test --test artifact_animation_native` (environment removed in finally) | Success: native full-app wgpu, 3 frames, live/GIF order identical, 684x544 at scale 1, 330 ms, full prefix observations, independent widgets, seek/pause restoration, PNG/JPEG aspect ratios and content-only crop. |

Current workspace log:
`C:\Users\dev\.local\share\opencode\tool-output\tool_1661fe027001xbCBY23b4pmgy1`.
The 16 ignored cases were not explicitly run. Unix-only tests were body-reviewed
but cannot execute on this Windows host. Native opt-in execution is verified;
its source audit still belongs to the pending GUI integration review. There was
also a successful earlier 523-definition workspace checkpoint (2737 passed,
16 ignored), not substituted for the latest run. No commit was made.

## Historical remaining work (completed by the final census below)

- Reconcile every recorded original definition with the final source census;
  all Windows-baseline bodies now have decisions.
- Independently reconcile source attributes/macros and module paths with runtime
  names, including platform-/feature-excluded tests, ignored bodies and the
  harness-free native entry.
- Read every fixture producer/probe, verification example and potential doc-test
  fence before concluding the other-language/doc/manual scope is complete.
- Reconsider cross-module duplication only after reading the supposed equivalent
  tests; do not delete based on matching names or test counts.

Latest analysis-complete checkpoint (all current source changes are test-only):

| Command | Actual result |
| --- | --- |
| `cargo test -p tdashboard-analysis --locked --profile ci-test` | Success: **532 passed, 0 failed, 2 ignored** (534 registered cases); zero-case doc harness passed. |
| `cargo test --workspace --locked --profile ci-test` | Success: **2630 passed, 0 failed, 15 ignored** (2645 registered cases). Analysis 532/2 ignored; all other harness counts unchanged from preceding checkpoint. Five doc-test harnesses have zero cases. Native default entry skipped by its environment gate. |
| `cargo fmt --all -- --check` | Success. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Success. |
| `git diff --check` | Success (line-ending normalization warnings only). |

Workspace log:
`C:\Users\dev\.local\share\opencode\tool-output\tool_1664023a9001eO1A7a9EMhhHik`.
One ignored PDP workload was deleted as redundant, so 16 -> 15 ignored is not
additional execution evidence. Remaining ignored tests were not explicitly run.
The interactive native opt-in is executable and passed at the preceding recorded
checkpoint; it was not rerun after these later analysis test-only edits. Its
body remains pending with GUI integration audit. No production implementation,
dependency, changelog, ADR or commit change was made. The final workspace checks
must run again after the rest of the repository audit and any resulting edits.

Consequently, the user's all-tests enumeration/audit completion criterion is
**not met**, despite successful validation of the implemented local pruning.

## GUI boundary / native continuation validation (not repository completion)

After all code edits in this continuation, including removal of the obsolete
Journal import found by the scoped run:

| Command | Actual result |
| --- | --- |
| `cargo test --workspace --locked --profile ci-test` | Success: **2585 passed, 0 failed, 14 ignored** (2599 registered cases); five doc-test harnesses with zero cases. Default native entry skips without environment opt-in. |
| `cargo fmt --all -- --check` | Success. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Success, no remaining unused-import warning. |
| `git diff --check` | Success; Git warns about future LF-to-CRLF normalization only. |
| `TUNNY_NATIVE_CAPTURE_CHECK=1 cargo test -p tunny-dashboard --locked --profile ci-test --test artifact_animation_native` (PowerShell env, removed in finally) | Success: full-app native wgpu, 3/3 selected frames, identical live/GIF order, 684x544 at scale 1, 428 responsive UI passes, complete prefixes, independent instances, restored seek/pause, PNG/JPEG aspect ratios and 330 ms frames/content-only crop. |

Workspace output:
`C:\Users\dev\.local\share\opencode\tool-output\tool_1676091be001TxnlFCRjtPUiVl`.
Scoped boundary runs also passed: first seven harnesses 39 passed; dataframe,
Study slice, d3_io and terminal attributes 43 passed/3 ignored including the RDB
harness; final Journal/SQLite 66 passed. The obsolete import warning in that last
scoped run was fixed before the clean workspace/clippy checks above.

This continuation audited **210 additional original definitions**, retaining 164,
deleting 31 and merging 15 (46 fewer integration cases, one of them ignored).
All 189 GUI integration libtest originals and the native/workload harness bodies
are now recorded, plus all 21 animation widget unit definitions and both issue258
upstream probes. **779 GUI unit definitions and final source/cfg/example/bench/doc
census remain pending.** No missing coverage is described as a specification
blocker, no external dependencies were installed, and no commit was made.

## Theme-complete continuation checkpoint (latest, still partial)

After the additional theme test-only merges/deletions:

| Command | Actual result |
| --- | --- |
| `cargo test --workspace --locked --profile ci-test` | Success: **2563 passed, 0 failed, 14 ignored**, 2577 registered cases. Each GUI unit target: 832 passed/2 ignored. Other harness counts match the preceding checkpoint. Five zero-case doc harnesses pass; default native entry skips. |
| `cargo fmt --all -- --check` | Success after `cargo fmt --all`. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Success. |
| `git diff --check` | Success; line-ending normalization warnings only. |
| `TUNNY_NATIVE_CAPTURE_CHECK=1 cargo test -p tunny-dashboard --locked --profile ci-test --test artifact_animation_native` (env removed in finally) | Success after the theme edits: 3/3 frames, matching live/GIF order, 684x544, 330 ms, 70 responsive UI passes, prefixes/independent instances/seek restoration/aspect ratios/content crop verified. |

Latest workspace log:
`C:\Users\dev\.local\share\opencode\tool-output\tool_16766bf0a0010CopB4aVuRVRm1`.
The latest native pass is additional opt-in execution, not one of the 14 ignored
libtest cases. Live DB/licensed compute/Unix-only/pinned upstream environments
remain unexecuted. All current production code and dependencies remain unchanged.

Across this continuation, **232 additional originals** have individual decisions:
175 Keep, 33 Delete, 24 Merge. This removes 57 distinct definitions / 68 registered
cases (11 theme definitions had duplicate lib/bin executions). Source-reviewed
total: **1258**; remaining Windows-baseline GUI unit definitions: **757**, plus
the pending final source/cfg/example/benchmark/documentation census. Next review
groups are GUI app/state, I/O adapters/export, canvas/panels/routing and other
widgets. No unresolved design judgment prevents continuing that review, and this
checkpoint is explicitly **not repository-wide audit completion**.

## Dendrogram, SOM and shared 3D continuation

All three source files were read through EOF, including production calculations,
cache use, drawing, selection handoff and test fixtures. This continuation reviews
**37 additional original definitions: 26 Keep, 0 Delete, 11 Merge**. It removes
11 definitions / 22 duplicated lib/bin cases. The earlier PCA cache-tag decision
is corrected from Delete to Merge: exact numeric tags are private, but distinctness
has an independent regression guarantee not supplied by core-space conversion.
Only that existing assertion is retained in the surviving conversion test.

### `tunny-dashboard::ui::widgets::scatter::dendrogram::tests::` (9 originals)

| Original test | Decision | Concrete reason / destination |
| --- | --- | --- |
| dendrogram_chart_default_values | Keep | Manual initial cluster count and parameter-space policy are not derive guarantees. |
| dendrogram_space_disc_is_distinct | Keep | Hand-written tags must distinguish all spaces or the widget can reuse linkage for the wrong features. |
| feature_names_variants | Keep | Parameter-only, objective-only and combined selection must use the correct columns and order. |
| cut_threshold_normal_case | Keep | Known merge distances yield a 3.5 cut line; also hosts original out-of-range boundaries. |
| cut_threshold_k_one_is_none | Merge | Into cut_threshold_normal_case; preserves the one-cluster boundary using the original two-leaf fixture. |
| cut_threshold_k_at_least_n_is_none | Merge | Into cut_threshold_normal_case; preserves k=n and k>n rejection, including checked subtraction. |
| compute_node_labels_propagates_matching_children | Keep | Child cluster colors propagate below the cut while the mixed root stays uncolored. |
| leaf_assignments_returns_none_without_cache | Keep | Export must report unavailable linkage rather than fabricate assignments or panic. |
| linkage_and_export_preserve_source_rows_after_filtering | Keep | Real feature preparation/linkage/export retain sparse source indices after excluding a NaN row. |

### `tunny-dashboard::ui::widgets::scatter::som_map::tests::` (12 originals)

| Original test | Decision | Concrete reason / destination |
| --- | --- | --- |
| som_map_chart_default_values | Keep | Manual training size/epoch/display defaults and initial selection/cache state are application policy. |
| som_space_disc_is_distinct | Keep | Hand-written tags must invalidate the trained map when objectives enter the feature space. |
| feature_names_params_only | Keep | Both feature modes retain correct ordered columns; hosts the combined-mode assertions. |
| feature_names_params_and_objectives | Merge | Into feature_names_params_only; verifies objectives follow parameters without duplicating fixture setup. |
| training_matrix_reports_the_source_row_of_every_kept_row | Keep | Shared GUI adapter must retain the original row offset after finite-row filtering. |
| group_rows_by_node_translates_bmu_back_to_view_rows | Keep | BMU membership translates training-row positions back to sparse displayed rows. |
| selected_trials_maps_nodes_to_trial_ids_without_duplicates | Keep | Actual node selection maps to global trial IDs in source-row order, not node traversal order. |
| selected_trials_is_empty_without_a_trained_map | Merge | Into selected_trials_maps_nodes_to_trial_ids_without_duplicates; retains absent-map behavior with the same view fixture. |
| node_at_maps_corners_to_the_first_and_last_node | Keep | Edge clicks clamp to valid node indices and outside-grid clicks are rejected. |
| format_node_value_keeps_hit_counts_whole | Keep | Hosts integer, magnitude-dependent decimals, zero and scientific notation display policies. |
| format_node_value_trims_decimals_as_the_magnitude_grows | Merge | Into format_node_value_keeps_hit_counts_whole; retains all original magnitude expectations. |
| format_node_value_falls_back_to_exponent_for_extreme_magnitudes | Merge | Into format_node_value_keeps_hit_counts_whole; retains zero and both tiny/large exponent cases. |

### `tunny-dashboard::ui::widgets::scatter::scatter_3d::tests::` (16 originals)

| Original test | Decision | Concrete reason / destination |
| --- | --- | --- |
| arcball_camera_default_is_identity | Keep | Manual front-facing pose and default zoom are project policy. |
| apply_zoom_clamps_to_min | Keep | Hosts both camera zoom limits, preserving original starting zooms and scroll directions. |
| apply_zoom_clamps_to_max | Merge | Into apply_zoom_clamps_to_min; retains the upper clamp without repeating camera setup. |
| normalize_to_clip_min_maps_to_minus_one | Keep | Hosts known clip-space endpoint and zero-width-range behavior. |
| normalize_to_clip_max_maps_to_plus_one | Merge | Into normalize_to_clip_min_maps_to_minus_one; retains the positive endpoint assertion. |
| normalize_to_clip_equal_range_returns_zero | Merge | Into normalize_to_clip_min_maps_to_minus_one; retains the degenerate-range guard. |
| axis_angle_to_quat_zero_axis_returns_identity | Keep | Custom zero-axis handling must avoid division by zero and provide the identity fallback. |
| quat_mul_identity_preserves_quat | Keep | Custom Hamilton multiplication must preserve all four components under identity multiplication. |
| rotate_by_quaternion_identity_is_noop | Keep | Custom point rotation must preserve coordinates at the initial camera pose. |
| rotate_90_deg_around_z_maps_x_to_y | Keep | Independent geometric oracle catches rotation direction, axis or scale mistakes. |
| axis_segments_3d_returns_three_axes_subdivided | Keep | All rendered axes span -1 to +1 with other coordinates zero; also hosts the zero-subdivision guard. |
| axis_segments_3d_clamps_zero_subdivisions_to_one | Merge | Into axis_segments_3d_returns_three_axes_subdivided; retains the minimum three-segment result. |
| pan_by_drag_accumulates_offset | Keep | Screen pan must accumulate signed deltas on both axes rather than replace the previous offset. |
| pick_nearest_3d_returns_id_within_threshold | Keep | Actual hit adapter returns trial/row identity and rejects distant clicks. |
| rotate_by_drag_changes_rotation_from_identity | Merge | Into rotate_by_drag_preserves_unit_quaternion_length; checks the original horizontal drag changes the pose before repeated drags. |
| rotate_by_drag_preserves_unit_quaternion_length | Keep | Repeated camera updates must remain normalized; the merged nonidentity assertion prevents a no-op implementation passing. |

Current cumulative review: **1821 original definitions across 194 files**;
**195 Windows GUI unit originals remain pending**, as does final source/cfg/manual
census. Historical checkpoints above describe their respective earlier edits only.

### `tunny-dashboard::ui::widgets::scatter::rank_plot_3d::tests::` (6 originals)

| Original test | Decision | Concrete reason / destination |
| --- | --- | --- |
| shared_ranks_filtering_colors_and_cache_invalidation | Keep | Known tied/nonfinite ranks and sparse rendered rows exercise direction/objective/axis/colormap/source invalidation and rank reuse. |
| canvas_fills_remaining_area_and_legend_stays_inside_on_resize | Keep | Actual resized canvas and rank legend must fill available space without clipping labels. |
| rendered_points_click_opens_detail_and_study_changes_clamp | Keep | Real chart dispatch, hover, click/detail context and study-change repair cover the complete interaction path. |
| selectors_only_offer_distinct_axes_and_rebuild_points_immediately | Keep | AccessKit interaction verifies duplicate axes are unavailable and choosing a new parameter rebuilds points in that frame. |
| sampling_is_capped_and_does_not_change_full_population_ranks | Keep | A population beyond the drawing cap retains full-population ranks and uncapped CSV output; missing columns produce no points. |
| defaults_clamping_and_serialization | Keep | Custom duplicate/out-of-range axis repair preserves distinct valid axes across persisted selections; not merely a serde roundtrip. |

The complete 728-line source and helpers were read.

### `tunny-dashboard::ui::widgets::scatter::observed_contour::tests::` (1 original)

| Original test | Decision | Concrete reason / destination |
| --- | --- | --- |
| nearest_point_within_threshold | Keep | Custom contour hit selection chooses the nearer observation even when another point is also within threshold, and rejects distant clicks. |

The complete observed-contour implementation was read, including masked-grid
orientation, raw mesh construction, 2D/3D point identity and hover/click routing.

### `tunny-dashboard::ui::widgets::scatter::cluster_scatter::tests::` (13 originals)

| Original test | Decision | Concrete reason / destination |
| --- | --- | --- |
| cluster_space_labels | Keep | Hand-written feature-space labels distinguish objective, variable and combined modes. |
| cluster_scatter_default_k | Keep | Manual initial cluster settings and inactive runtime/display state remain application policy. |
| compute_obj_axes_2d_empty_trials | Merge | Into compute_obj_axes_2d_single_objective; retains empty-view behavior. |
| compute_obj_axes_2d_single_objective | Keep | A single objective maps to X with a zero Y fallback; also hosts the empty case. |
| cache_key_updated_on_data_change | Delete | Does not change data or call cache invalidation; repeats default cache fields already checked by cluster_scatter_default_k. |
| adopt_runtime_state_clears_stuck_computing | Keep | Completion adoption clears a stale spinner/pending state while preserving item-specific drawing state and accepting later errors. |
| adopt_runtime_state_preserves_display_cache | Merge | Into adopt_runtime_state_clears_stuck_computing; retains original cached points and nonzero display key. |
| adopt_runtime_state_propagates_error | Merge | Into adopt_runtime_state_clears_stuck_computing; resets item computing before error adoption and retains the original error assertion. |
| validate_cluster_request_rejects_manual_k_too_small | Keep | Hosts mode-dependent validation: manual k=1 fails, while elbow ignores k=999. |
| validate_cluster_request_accepts_elbow_mode | Merge | Into validate_cluster_request_rejects_manual_k_too_small; retains the ignored manual-k check in elbow mode. |
| cache_key_normalizes_unused_field_per_mode | Keep | Actual key constructor must normalize inactive settings to avoid unnecessary recomputation while retaining the active setting. |
| cluster_matrix_filters_missing_numeric_params_and_preserves_targets | Delete | Calls only cfg(test)-only build_cluster_matrix, an obsolete adapter with no production caller; real preparation is exercised by the retained d4_preparation boundary suite. |
| mixed_type_cluster_matrix_keeps_categories_constant_and_filters_missing_numbers | Delete | Same unused cfg(test)-only adapter, not live cluster preparation; core categorical/finite preparation and real execution boundaries remain independently tested. |

The obsolete test-only matrix adapter was removed with its two callers. All
cluster-scatter production/settings/cache/matrix sources were read; production
ClusterMatrix and its real users are unchanged.

### `tunny-dashboard::ui::widgets::scatter::parallel_coords::tests::` (37 originals)

| Original test | Decision | Concrete reason / destination |
| --- | --- | --- |
| pcp_horizontal_labels_fit_clip | Keep | Hosts real horizontal and rotated label clipping checks at their original different widths/counts. |
| pcp_rotated_labels_fit_clip | Merge | Into pcp_horizontal_labels_fit_clip; retains the seven-axis rotated layout at width 650 and scale 1.25. |
| pcp_pointer_movement_does_not_change_labels | Keep | Moving the pointer across/away from the chart must not alter labels or create brushes. |
| pcp_galleys_refresh_after_atlas_reset_and_dpi_change | Keep | Project drawing must not retain stale text UVs after font/atlas/DPI/theme changes, for both label orientations. |
| pcp_galleys_refresh_after_full_atlas | Keep | Actual atlas pressure exercises the independent automatic-reset path, checking fresh UVs and clipping. |
| pcp_resize_show_hide_and_long_label_band | Keep | Reusing the chart across resize/visibility transitions and very long names preserves label identity, layout and freshness. |
| pcp_cramped_layout_has_ordered_finite_geometry | Keep | Very small chart sizes must not invert axes or create nonfinite tessellated vertices. |
| pcp_brush_create_move_hover_and_clear_unchanged | Keep | Real pointer interaction creates, translates, highlights and clears brushes while publishing selected trial IDs. |
| normalize_min_maps_to_zero | Delete | Exact duplicate of shared range_math::tests::normalize01_min_maps_to_zero through a forwarding wrapper; real brush/drawing tests retain caller coverage. |
| normalize_max_maps_to_one | Delete | Same wrapper; shared normalization test already checks the positive endpoint. |
| normalize_equal_min_max_returns_half | Delete | Same wrapper; shared normalization test already checks the degenerate midpoint. |
| normalize_clamps_out_of_range | Delete | Same wrapper; shared normalization test already checks both out-of-range clamps. |
| normalized_to_screen_y_zero_maps_to_bottom | Keep | Hosts both Y-coordinate endpoints, independently specifying the inverted screen direction. |
| normalized_to_screen_y_one_maps_to_top | Merge | Into normalized_to_screen_y_zero_maps_to_bottom; retains top endpoint. |
| build_axis_order_concatenates_params_then_objectives | Keep | Actual parameter/objective ordering helper must not swap or omit feature groups. |
| parallel_coords_chart_default | Keep | Manual initial visibility/brush/color policy, including the merged infeasible visibility assertion. |
| ordered_brush_range_forward_drag | Keep | Hosts both drag directions with ordered expected endpoints. |
| ordered_brush_range_reverse_drag | Merge | Into ordered_brush_range_forward_drag; retains upward dragging behavior. |
| axis_visibility_filter | Delete | Tests a local HashMap/filter copy, never the production helper; actual visible_axis_indices and rendering tests remain. |
| axis_reorder_logic | Delete | Only edits its own Vec with copied logic; no production reordering path is invoked. |
| axis_visibility_all_hidden | Delete | Local copy, not the actual helper; retained visible_axis_indices checks all-hidden behavior. |
| axis_visibility_default_true_for_unknown | Delete | Local copy, not the actual helper; retained visible_axis_indices checks unregistered defaults. |
| tc_cav_parallel_coords_show_infeasible_default_true | Merge | Into parallel_coords_chart_default; preserves initial infeasible visibility. |
| multi_axis_brush_applies_and_filter | Keep | Intersecting multiple axis brushes must select only rows satisfying all ranges. |
| shifted_brush_range_moves_within_bounds | Keep | Hosts translation, both boundary clamps and width preservation against original independent expected values. |
| shifted_brush_range_clamps_at_top | Merge | Into shifted_brush_range_moves_within_bounds; retains top clamp with preserved width. |
| shifted_brush_range_clamps_at_bottom | Merge | Into shifted_brush_range_moves_within_bounds; retains bottom clamp with preserved width. |
| shifted_brush_range_preserves_width | Merge | Into shifted_brush_range_moves_within_bounds; retains original width assertion. |
| trial_passes_brushes_no_active_brush_passes | Keep | An explicitly inactive axis brush must not filter trials. |
| trial_passes_brushes_missing_value_with_active_brush_excluded | Keep | A short/missing column under an active brush must exclude that row rather than select it. |
| visible_axis_indices_default_all_visible | Keep | Hosts actual helper defaults, hidden-axis order preservation and all-hidden behavior. |
| visible_axis_indices_filters_hidden_and_preserves_order | Merge | Into visible_axis_indices_default_all_visible; retains sparse original indices. |
| visible_axis_indices_all_hidden_is_empty | Merge | Into visible_axis_indices_default_all_visible; marks all axes hidden and retains empty output. |
| feasible_color_range_excludes_infeasible_outliers | Keep | Hosts real feasible-only color range, unconstrained input, empty-feasible fallback and nonfinite exclusion. |
| feasible_color_range_no_constraints_uses_all | Merge | Into feasible_color_range_excludes_infeasible_outliers; retains unconstrained outlier inclusion. |
| feasible_color_range_all_infeasible_falls_back | Merge | Into feasible_color_range_excludes_infeasible_outliers; retains explicit fallback range. |
| feasible_color_range_skips_non_finite | Merge | Into feasible_color_range_excludes_infeasible_outliers; retains NaN/Inf filtering. |

Complete PCP production, layout/brush helpers, render fixtures and all 806 original
test-file lines were read. Additional out-of-scope finding (source analysis):
trial_passes_brushes accepts a NaN value under an active brush because normalization
returns NaN and both out-of-range comparisons are false. Existing missing-value
coverage exercises a short column, not this case. No production fix or replacement
coverage was added under the test-pruning contract.

### `tunny-dashboard::ui::widgets::scatter::scatter_matrix::tests::` (38 originals)

| Original test | Decision | Concrete reason / destination |
| --- | --- | --- |
| matrix_labels_horizontal_text_options | Keep | Actual chart labels must refresh after text-option atlas reset; hosts both orientations with independent contexts. |
| matrix_labels_rotated_text_options | Merge | Into matrix_labels_horizontal_text_options; preserves original rotated width and reset/UV checks. |
| matrix_labels_horizontal_atlas_pressure | Keep | Actual atlas exhaustion followed by another widget's glyph allocations must not leave stale matrix label coordinates. |
| matrix_labels_rotated_atlas_pressure | Merge | Into matrix_labels_horizontal_atlas_pressure; retains the rotated pressure/reset path. |
| matrix_labels_horizontal_dpi | Keep | Retains actual 1x-to-2x label refresh in both orientations. |
| matrix_labels_rotated_dpi | Merge | Into matrix_labels_horizontal_dpi; retains rotated label scale/freshness assertions. |
| matrix_labels_horizontal_theme | Keep | Actual labels refresh text color after a dark-to-light theme transition in both orientations. |
| matrix_labels_rotated_theme | Merge | Into matrix_labels_horizontal_theme; retains rotated theme checks. |
| resolve_color_objective_none_returns_first | Keep | Hosts explicit selection, unset/unknown fallback and no-objective behavior using the real resolver. |
| resolve_color_objective_existing_name_returns_it | Merge | Into resolve_color_objective_none_returns_first; retains nonfirst objective selection. |
| resolve_color_objective_unknown_name_falls_back_to_first | Merge | Into resolve_color_objective_none_returns_first; retains disappeared-objective fallback. |
| resolve_color_objective_empty_names_returns_none | Merge | Into resolve_color_objective_none_returns_first; retains both empty-name-list calls. |
| tc_cav_scatter_matrix_show_infeasible_default_true | Merge | Into scatter_matrix_default_mode; preserves manual initial infeasible visibility. |
| tc_cav_split_feasibility_no_constraints_all_feasible | Merge | Into unverified_is_not_partitioned_as_infeasible; retains unconstrained index sequence and empty other partitions. |
| tc_cav_split_feasibility_mixed | Merge | Into unverified_is_not_partitioned_as_infeasible; retains feasible/infeasible source positions. |
| unverified_is_not_partitioned_as_infeasible | Keep | Hosts every original feasibility partition scenario and the actual detail-row Unverified label. |
| tc_cav_split_feasibility_all_infeasible | Merge | Into unverified_is_not_partitioned_as_infeasible; retains empty-feasible and all-infeasible indices. |
| scatter_matrix_default_mode | Keep | Manual initial matrix mode, sort, selected cell and infeasible visibility policy. |
| downsample_cap_keeps_all_when_under_cap | Keep | Hosts uncapped input preservation, capped ordered nonempty sample and zero-cap behavior of the shared drawing helper. |
| downsample_cap_limits_when_over_cap | Merge | Into downsample_cap_keeps_all_when_under_cap; preserves the 100000-entry cap/order/first-item assertions. |
| downsample_cap_zero_is_empty | Merge | Into downsample_cap_keeps_all_when_under_cap; retains zero-cap input. |
| compute_histogram_bins_count | Keep | Known finite bins conserve counts; hosts original empty-data and zero-bin guards. |
| compute_histogram_extreme_endpoints | Merge | Into compute_histogram_extreme_asymmetric_intermediates; retains both f64 extremes and expected edge bins. |
| compute_histogram_extreme_asymmetric_intermediates | Keep | Known asymmetric/reflected extreme values must be binned correctly despite overflowing unscaled spans. |
| compute_histogram_extreme_mixed_counts | Keep | Overflow-safe histogram retains exact varied bin counts across multiple bin widths while excluding nonfinite inputs. |
| compute_histogram_near_constant_with_nonfinite | Keep | Hosts near-degenerate, constant finite and constant/nonfinite inputs with both odd/even center-bin policies. |
| compute_histogram_all_in_same_bin | Merge | Into compute_histogram_near_constant_with_nonfinite; retains ten equal values and exact centered counts. |
| compute_histogram_empty_data | Merge | Into compute_histogram_bins_count; retains output length and all-zero counts. |
| compute_histogram_mixed_finite_and_nonfinite | Keep | Ordinary finite ranges exclude NaN and both infinities without shifting endpoints or intermediate bins. |
| compute_histogram_constant_with_nonfinite | Merge | Into compute_histogram_near_constant_with_nonfinite; retains both original constant/nonfinite fixtures and bin widths. |
| compute_histogram_all_nonfinite | Keep | Hosts both mixed-all-nonfinite and NaN-only empty-finite populations. |
| compute_histogram_all_nan | Merge | Into compute_histogram_all_nonfinite; retains the two-NaN-only case. |
| compute_histogram_zero_bins | Merge | Into compute_histogram_bins_count; retains every original zero-bin fixture, including empty, constant and nonfinite populations. |
| compute_correlation_perfect_positive | Delete | Repeats retained analysis::math::stats::tests::tc_2261_02_pearson_proportional_positive through an adapter; does not exercise the adapter's differing-length/NaN presentation policy. |
| compute_correlation_perfect_negative | Delete | Repeats retained analysis::math::stats::tests::tc_2261_03_pearson_perfect_negative through the same adapter. |
| compute_correlation_range_bounded | Delete | A weak generic coefficient bound misses arbitrary constant outputs and does not exercise display-specific fallback/clamping; exact numerical guarantees remain at the core owner. |
| data_to_screen_min_maps_to_left_bottom | Keep | Hosts both screen corners with independently expected X/Y orientation. |
| data_to_screen_max_maps_to_right_top | Merge | Into data_to_screen_min_maps_to_left_bottom; retains the opposite screen corner using the same rect. |

All matrix implementation, stats/draw helpers, render fixtures and eight
macro-generated cases were read. This module contributes **15 Keep, 3 Delete,
20 Merge**, removing 23 definitions / 46 registered GUI cases. Cumulative review:
**1916 originals, 199 files, 100 Windows GUI originals still pending**. Final
source/cfg/example/benchmark/documentation census and workspace checks remain due.

### `tunny-dashboard::ui::widgets::importance::sensitivity_heatmap::tests::` (3 originals)

| Original test | Decision | Concrete reason / destination |
| --- | --- | --- |
| categorical_heatmap_ui_never_draws_a_scored_category_cell | Keep | Actual mixed/category-only heatmap rendering must present unsupported categories explicitly without invented numeric scores. |
| sensitivity_heatmap_default | Delete | Only rechecks derived Default fields and ImportanceMetric's default; no manual heatmap policy or rendering path is exercised. |
| adopt_compute_state_copies_computing_flag | Keep | Real canvas-state adoption must synchronize the spinner without overwriting the local selected method. |

### `tunny-dashboard::ui::widgets::decision::mcdm_scatter_chart_3d::tests::` (8 originals)

| Original test | Decision | Concrete reason / destination |
| --- | --- | --- |
| ranked_hash_changes_when_order_changes | Delete | Same two ranking orders are exercised through actual cache staleness by cache_stale_after_ranking_change. |
| ranked_hash_stable_for_same_order | Delete | Deterministic equal-input hash equality adds no guarantee beyond retained same-ranking cache reuse. |
| cache_stale_after_ranking_change | Keep | Same-count changed ranking must invalidate colors; also hosts the preceding unchanged-ranking cache hit. |
| cache_not_stale_for_same_ranking | Merge | Into cache_stale_after_ranking_change; retains the original ranking/hash/key equality before the changed-order assertion. |
| mcdm_scatter_3d_default_axes | Keep | Manual distinct objective axes, initial infeasible visibility and absent display cache remain project policy. |
| val_range_basic | Keep | Hosts real shared 3D range computation, empty fallback and equal-value expansion. |
| val_range_empty_returns_fallback | Merge | Into val_range_basic; retains exact (-1, 1) fallback. |
| val_range_equal_values_expands | Merge | Into val_range_basic; retains both-sided expansion around 5. |

### `tunny-dashboard::ui::widgets::decision::mcdm_scatter_chart::tests::` (21 originals)

| Original test | Decision | Concrete reason / destination |
| --- | --- | --- |
| test_scatter_chart_default_values | Keep | Manual initial objective axes, display/error/cache state remain usable. |
| test_cache_stale_when_no_key | Merge | Into test_cache_not_stale_same_key; verifies cache absence before initializing the same real key. |
| test_cache_stale_when_trial_count_changes | Merge | Into test_cache_not_stale_same_key; preserves changed row-count invalidation. |
| test_cache_not_stale_same_key | Keep | Hosts absent/hit/changed-count transitions of the actual staleness predicate. |
| test_axis_options_vikor_has_scores | Keep | VIKOR selector must offer objective axes and all three method-specific scores. |
| test_axis_options_topsis | Keep | TOPSIS must offer its score, not VIKOR scores; hosts objective-free selector behavior. |
| test_axis_options_promethee | Keep | PROMETHEE selector must offer positive/negative/net flow axes. |
| test_axis_options_empty_objectives | Merge | Into test_axis_options_topsis; retains the exact single remaining score option. |
| test_extract_objective0 | Keep | Hosts real objective-column mapping, independent second-column values and out-of-range rejection. |
| test_extract_objective1 | Merge | Into test_extract_objective0; preserves second objective mapping with the same two-column view/result. |
| test_extract_vikor_q | Keep | Actual axis adapter returns raw Q rather than display scores and rejects unknown identifiers. |
| test_extract_topsis_score | Keep | Actual method/axis dispatch must return TOPSIS scores. |
| test_extract_phi_plus | Keep | Actual PROMETHEE axis dispatch must return positive, not negative, flow. |
| test_extract_unknown_axis_error | Merge | Into test_extract_vikor_q; retains unknown-axis rejection with the same method/view. |
| test_extract_out_of_range_objective | Merge | Into test_extract_objective0; retains the original one-objective view and Objective5 failure. |
| test_build_rank_map_basic | Keep | Sparse nonidentity ranking maps to zero-based ranks with an unranked sentinel; hosts full-population reverse order. |
| test_build_rank_map_all_trials | Merge | Into test_build_rank_map_basic; retains complete reverse ranking endpoints. |
| test_compute_scatter_points_basic | Keep | Actual objective scatter coordinates retain differing/reversed X/Y values and correct metadata/count. |
| test_compute_scatter_points_rank0_gets_best_color | Keep | Best rank uses the high colormap endpoint while points outside Top N use the unranked gray. |
| test_compute_scatter_points_empty_trials | Keep | Empty input short-circuits before invalid objective extraction and reports zero points. |
| test_compute_scatter_points_vikor_axis | Keep | Score-space scatter accepts both raw Q and S axes with a nonempty view. |

All three complete implementations and 2D points/hit-test/render helpers were
read. This continuation contributes **32 originals: 19 Keep, 3 Delete, 10 Merge**.
Additional out-of-scope cache issue: both MCDM scatter cache keys track ranked
indices but not changed score values. Rerunning the same method with changed
weights can preserve ranking order while changing score-axis coordinates; the
existing cache can then retain obsolete coordinates. No product fix was made.

Validation after the scatter-matrix edits, before this later MCDM/heatmap batch:
workspace tests passed (GUI 530 passed/2 ignored in each entry target, 532 total);
all-target clippy, formatting check and diff check passed. Native opt-in also
passed: 3/3 frames, 684x544, 330 ms, 453 responsive UI passes, matching live/GIF
order and all prefix/seek/independence/aspect/crop checks. Workspace log:
`C:\Users\dev\.local\share\opencode\tool-output\tool_168de7e4f001vXtK13wXPpRewF`.
These checks must be repeated after subsequent code edits; they are not final
repository-wide audit evidence. **68 baseline definitions and final census remain.**

### `tunny-dashboard::ui::widgets::pareto::pareto_2d::tests::` (7 originals)

| Original test | Decision | Concrete reason / destination |
| --- | --- | --- |
| pareto_scatter_2d_default | Keep | Hosts manual initial axes, absent brush and enabled surrogate overlay policies. |
| blank_click_clears_selection_per_policy | Delete | Only clears a locally constructed Vec; no pointer event or actual chart selection path is invoked. |
| brush_state_default_is_none | Merge | Into pareto_scatter_2d_default; retains both absent brush endpoints. |
| surrogate_front_points_normal_order | Keep | Hosts known predicted-front values, swapped axis mapping and unknown-axis rejection from one result fixture. |
| surrogate_front_points_swapped_axes | Merge | Into surrogate_front_points_normal_order; retains both reversed coordinates. |
| surrogate_front_points_unknown_axis_returns_empty | Merge | Into surrogate_front_points_normal_order; retains unavailable-axis behavior. |
| pareto_scatter_2d_show_surrogate_front_default_true | Merge | Into pareto_scatter_2d_default; retains overlay default. |

### `tunny-dashboard::ui::widgets::pareto::pareto_3d::tests::` (5 originals)

| Original test | Decision | Concrete reason / destination |
| --- | --- | --- |
| pareto_3d_chart_default_objectives | Keep | Manual distinct axes, tilted camera and initial infeasible visibility produce a usable initial view. |
| tc_cav_pareto3d_show_infeasible_default_true | Merge | Into pareto_3d_chart_default_objectives; preserves initial visibility. |
| tc_cav_pareto_front_returns_pareto_color | Keep | Hosts rank-dependent palette/radius and selection-dimming policy of the actual helper. |
| tc_cav_non_pareto_returns_non_pareto_color | Merge | Into tc_cav_pareto_front_returns_pareto_color; retains the nonfront palette branch. |
| tc_cav_dimmed_point_uses_alpha_60 | Merge | Into tc_cav_pareto_front_returns_pareto_color; retains alpha 60. |

### `tunny-dashboard::ui::widgets::pdp::slice_chart::tests::` (7 originals)

| Original test | Decision | Concrete reason / destination |
| --- | --- | --- |
| test_compute_plot_points_empty | Keep | Hosts empty input, missing parameter and invalid objective selection without fabricated points. |
| test_compute_plot_points_pareto_classification | Keep | Sparse front/nonfront membership preserves independently specified X/Y coordinates. |
| test_compute_plot_points_single_obj_minimize | Keep | Hosts both objective directions with original distinct expected best-point fixtures. |
| test_compute_plot_points_single_obj_maximize | Merge | Into test_compute_plot_points_single_obj_minimize; retains maximizing the middle point at value 9. |
| test_compute_plot_points_unknown_param_returns_empty | Merge | Into test_compute_plot_points_empty; retains original two-row view and unknown y parameter. |
| test_compute_plot_points_obj_idx_out_of_bounds | Merge | Into test_compute_plot_points_empty; retains one-objective view and invalid index 1. |
| test_compute_plot_points_skips_nan_obj | Keep | A missing objective becomes NaN and must not shift sparse surviving nonfront coordinates. |

All three full source files were read, including brush interaction/cache drawing
and slice hover/log-coordinate paths. This batch contributes **19 originals:
8 Keep, 1 Delete, 10 Merge**. Pareto3D has the same out-of-scope range-cache issue
as ClusterScatter3D: its key only includes selected axes and trial count, allowing
stale axis ranges after a same-count input replacement. No production fix was made.
Latest scoped MCDM check before this batch: **39 passed, 0 failed**.
Current cumulative review is **1967 originals / 205 files**, with **49 baseline
definitions and final source/cfg/manual census pending**.

### `tunny-dashboard::ui::widgets::pdp::pdp_chart::tests::` (19 originals)

| Original test | Decision | Concrete reason / destination |
| --- | --- | --- |
| d4_pdp_cache_reuse_still_queues_the_selection_for_adoption | Keep | Actual cache hit must still publish an adoption request to supersede earlier asynchronous interests. |
| adopt_compute_state_propagates_result_and_clears_computing | Keep | Canvas adoption transfers result/cache and clears a stale spinner without changing local parameter/objective selection. |
| r2_quality_good_above_0_8 | Keep | Hosts every original Good/Fair/Poor threshold fixture, including negative R-squared. |
| r2_quality_fair_between_0_6_and_0_8 | Merge | Into r2_quality_good_above_0_8; preserves Fair values .7 and .61. |
| r2_quality_poor_at_or_below_0_6 | Merge | Into r2_quality_good_above_0_8; preserves .6, zero and negative values. |
| pdp_chart_default | Keep | Manual initial 1D/Ridge/overlay/feasibility and idle result/cache policy. |
| pdp_chart_default_show_observed_false | Merge | Into pdp_chart_default; retains disabled observed-data overlay. |
| extract_observed_normal | Keep | Hosts exact original observed coordinates, missing parameter and invalid objective index. |
| extract_observed_missing_param | Merge | Into extract_observed_normal; retains missing y column with the original view. |
| extract_observed_out_of_range_obj | Merge | Into extract_observed_normal; retains objective index 5 rejection. |
| classify_observed_matches_scatter_rules | Keep | Feasibility overrides rank; explicit Unverified classification/label and actual palette mappings remain checked together. |
| observed_kind_colors_match_scatter_palette | Merge | Into classify_observed_matches_scatter_rules; retains all three original palette branches. |
| cache_key_same_inputs_produce_same_key | Delete | Compares the same pure string-formatting function with identical literal inputs; not a cache lookup/invalidation regression. |
| cache_key_different_model_produces_different_key | Keep | Hosts model/parameter/feasible-setting distinctions in the actual key builder, not literal tuple comparison. |
| cache_key_different_param_produces_different_key | Merge | Into cache_key_different_model_produces_different_key; retains parameter-only change. |
| cache_key_different_feasible_flag_produces_different_key | Merge | Into cache_key_different_model_produces_different_key; retains feasibility-only change. |
| pdp_overlay_uses_filtered_rows_when_selection_exists | Keep | Hosts real overlay selection, unfiltered fallback and pinned-row exemption with original coordinate assertions. |
| pdp_overlay_falls_back_to_all_rows_without_selection | Merge | Into pdp_overlay_uses_filtered_rows_when_selection_exists; retains original two-row unfiltered view. |
| pinned_row_remains_in_observed_overlay | Merge | Into pdp_overlay_uses_filtered_rows_when_selection_exists; retains original three-row selected/pinned/unselected contrast. |

### `tunny-dashboard::ui::widgets::pdp::pdp_2d::tests::` (25 originals)

| Original test | Decision | Concrete reason / destination |
| --- | --- | --- |
| normalize_value_midpoint | Delete | Thin forwarder to shared range_math::normalize01, already exercised by normalization and real brush/coordinate tests; no PDP-specific behavior. |
| normalize_value_clamps_below_zero | Delete | Exact shared normalization clamp case through a forwarding wrapper. |
| normalize_value_clamps_above_one | Delete | Exact shared normalization clamp case through a forwarding wrapper. |
| normalize_value_equal_range_returns_half | Delete | Exact shared degenerate-range case through a forwarding wrapper. |
| value_range_of_correct | Keep | Hosts grid flattening/range extraction and its caller-specific empty fallback (0, 1). |
| value_range_of_empty_returns_default | Merge | Into value_range_of_correct; retains exact empty fallback. |
| check_params_different_true_for_different | Keep | Hosts Run eligibility for distinct, identical and either-empty parameter selection. |
| check_params_different_false_for_same | Merge | Into check_params_different_true_for_different; retains identical-axis rejection. |
| check_params_different_false_for_empty | Merge | Into check_params_different_true_for_different; retains both empty-axis positions. |
| surface_quads_count_matches_grid_cells | Merge | Into surface_quads_corners_span_clip_space; retains rectangular 3x4 grid producing six cells. |
| surface_quads_corners_span_clip_space | Keep | Known quad corners/mean specify row/column/value axes; hosts count, insufficient-grid and ragged-row cases. |
| surface_quads_empty_for_single_row_or_col | Merge | Into surface_quads_corners_span_clip_space; retains single row, single column and empty grid. |
| surface_quads_skips_ragged_rows | Merge | Into surface_quads_corners_span_clip_space; retains original short-middle-row fixture and exact two valid cells. |
| pdp2d_default_camera_is_tilted | Keep | Manual tilted view and uncertainty/observed/feasibility defaults make the initial surface legible. |
| band_grids_computes_95_ci | Keep | Known variance-to-sigma conversion and ±1.96 factor; hosts zero/negative variance and ragged truncation. |
| band_grids_negative_variance_does_not_produce_nan | Merge | Into band_grids_computes_95_ci; retains finite equal-to-mean bounds for slight negative numerical variance. |
| band_grids_truncates_to_shorter_rows | Merge | Into band_grids_computes_95_ci; retains shorter row/column output lengths. |
| push_tri_appends_three_vertices_and_indices | Keep | Actual raw triangle construction supplies three vertices and valid triangle indices, not dependency tessellation. |
| push_edge_zero_length_adds_nothing | Merge | Into push_edge_builds_finite_quad; retains empty mesh on degenerate edge before ordinary construction. |
| push_edge_builds_finite_quad | Keep | Nearly horizontal edge geometry remains finite/bounded rather than producing miter spikes. |
| extract_observed_3d_returns_all_rows_without_selection | Keep | Hosts exact row indices and 3D coordinates plus front/nonfront classification from the same two-row view. |
| extract_observed_3d_filters_by_selection_and_pinned | Keep | Real 3D overlay retains selected and pinned observations but excludes unselected/unpinned rows. |
| extract_observed_3d_missing_column_returns_empty | Keep | Missing parameter or objective rejects overlay rather than fabricating zeros. |
| extract_observed_3d_skips_non_finite_rows | Keep | Nonfinite feature row is excluded without changing surviving source identity/coordinates. |
| extract_observed_3d_classifies_by_pareto_rank | Merge | Into extract_observed_3d_returns_all_rows_without_selection; uses the original nonuniform rank vector and preserves both classifications. |

Both complete PDP implementations, math/mesh/observed helpers and all test
fixtures were read. This batch contributes **44 originals: 19 Keep, 5 Delete,
20 Merge**. Cumulative review: **2011 originals / 207 files**, with **5 expected
Windows-baseline definitions pending reconciliation**, plus final source/cfg/manual
census. The expected baseline total is not substituted for a fresh exact census.

### `tunny-dashboard::ui::chart::poll_chart::tests::` (4 originals)

| Original test | Decision | Concrete reason |
| --- | --- | --- |
| mcdm_dispatch_propagates_negative_weight_failure_before_empty_front | Keep | Real asynchronous dispatch of every method rejects negative criteria before the empty-front fallback, consumes the pending request and publishes the correct error. |
| convergence_mixed_constraints_exclude_unverified_and_violations_preserving_trial_axis | Keep | Actual worker dispatch preserves original sampling slots/IDs across studies while masking unverified/violating rows for every indicator. |
| convergence_attribute_only_and_all_unverified_have_empty_verified_front | Keep | Entirely unverified studies produce no verified front/reference and do not contaminate another study's verified history. |
| convergence_genuinely_unconstrained_sampling_is_unchanged | Keep | Absence of constraint evidence must not mask rows; distinct 120/3-row studies retain original sampling behavior. |

### `tunny-dashboard::ui::chart::poll_chart::compute::categorical_tests::` (1 original)

| Original test | Decision | Concrete reason |
| --- | --- | --- |
| heatmap_transports_only_numerical_scores_and_explicit_unsupported_names | Keep | Actual GUI message adapter carries numeric-only scores and categorical unsupported names for mixed/category-only Spearman and Ridge inputs. |

Complete test files, poll_chart routing/convergence implementation, heatmap
adapter and MCDM dispatch/calculation were read. Scoped poll_chart execution:
**5 passed, 0 failed**; latest PDP scoped execution: **23 passed, 0 failed**.
All **2016 identified original Rust definitions** now have individual decisions.
This is still not final completion until source/cfg/manual census, quantitative
ledger reconciliation and final post-edit workspace checks are complete.

## Final source, manual and documentation census

An independent uncapped inventory read every tracked Rust file from `git show
HEAD:<path>` and extracted original `#[test]` definitions (including intervening
ignore/cfg attributes), plus the eight original matrix-label macro invocations.
Comparing that complete function-name multiset with the module-qualified decision
tables gave **2016 source definitions / 209 files, 2016 Rust decision rows,
no missing names and no extra names**. Repeated basename multiplicities were
preserved rather than deduplicated. No duplicate original identity exists within
a module-qualified section. Three Python producer rows are not Rust definitions;
they explain why counting every table row indiscriminately would produce 2019.
Package totals independently match: analysis 704, I/O 114, runner 113, Study 11,
GUI 1052 (863 unit-source + 189 integration), MCP 22.

Macro/alternate-attribute discovery found only the matrix test macro, the themed
color macro and Condition tuple macro; the latter two generate no test definitions.
There are no rstest/test_case/property-test/async-test/bench-attribute cases or
cfg_attr-generated tests. The two kmeans cfg_attr sites only condition existing
should-panic expectations on debug assertions. Platform exclusions are the already
reviewed 11 Unix runner tests and one POSIX GUI URI test. All six workspace
members' Cargo target/feature metadata was checked: the only workspace feature,
I/O test-support, exposes an already-reviewed fixture control, not extra tests.
The native Animation target is explicitly harness=false and has its own reviewed
entry path; it is not omitted because it lacks #[test].

All Rust documentation fences were enumerated: eight `text` fences across MCP
main, RDB/report examples, optimizer formulas and feasibility formulas. They are
not executable Rust tests. Five final rustdoc harnesses confirm zero doc-tests.
The sole include! is generated license-array data; build.rs emits no tests.
Historical Markdown cross-validation snippets are manual external-oracle records,
not additional Cargo/rustdoc test definitions. No dependency environment was
installed to rerun historical Python/Node oracles.

The complete bodies/helpers of **all 26 original example/benchmark Rust files**
were read, not sampled. They are separate from the 2016 libtest definitions:

- Keep `crates/analysis/examples/verify_boxplot.rs::main`: numpy/matplotlib oracle data for quantiles, fences and outliers.
- Keep `crates/analysis/examples/verify_correlation.rs::main`: scipy oracle data for Pearson/Spearman, ties and pairwise missing observations.
- Keep `crates/analysis/examples/verify_distribution_fit.rs::main`: scipy MLE parameters, likelihood and AIC across distribution families.
- Keep `crates/analysis/examples/verify_entropy.rs::main`: positive-matrix pymcdm oracle, distinct from Rust's custom nonpositive handling.
- Keep `crates/analysis/examples/verify_hierarchical.rs::main`: scipy Ward merge distances, source leaf order and standardized/raw comparisons.
- Keep `crates/analysis/examples/verify_histogram.rs::main`: numpy bin-rule/edge/count oracle across distributions and manual bin limits.
- Delete `crates/analysis/examples/verify_hv_2d_bug_repro.rs::main`: its claim that (0.6,0.6) is dominated by {(0.2,0.8),(0.8,0.2)} is false. The correct HV changes from .28 to .32 at reference (1,1), so its printed equal-HV expectation is not a valid regression oracle. The retained hypervolume example and unit tests cover genuinely dominated points and dimensional dispatch.
- Keep `crates/analysis/examples/verify_hypervolume.rs::main`: pymoo oracle data for genuine dominated/duplicate inputs, 2D/3D direct HV and direction/reference history behavior.
- Keep `crates/analysis/examples/verify_indicators.rs::main`: external IGD+/epsilon and definition-based R2 oracles; the copied lattice generator supplies inputs, not expected results.
- Keep `crates/analysis/examples/verify_kmeans.rs::main`: sklearn partition/inertia and elbow data independent of stochastic label identities.
- Keep `crates/analysis/examples/verify_pareto.rs::main`: pymoo nondomination-rank oracle across directions/dimensions and duplicated points.
- Keep `crates/analysis/examples/verify_pca.rs::main`: numpy/sklearn centering/standardization/eigenspace oracle with materially different feature scales.
- Keep `crates/analysis/examples/verify_promethee.rs::main`: pymcdm flow oracle with explicitly supplied preference thresholds.
- Keep `crates/analysis/examples/verify_ridge.rs::main`: sklearn coefficient/R-squared oracle with explicit population-standardization and intercept policy.
- Keep `crates/analysis/examples/verify_sobol.rs::main`: analytical quadratic oracle and separately documented Ishigami surrogate-approximation diagnostic, not a claimed exact Ishigami match.
- Keep `crates/analysis/examples/verify_spearman.rs::main`: scipy preprocessing oracle for ties, NaN/Inf and insufficient finite pairs.
- Keep `crates/analysis/examples/verify_topsis.rs::main`: pymcdm score/ranking/ideal oracle with mixed directions and raw weights.
- Keep `crates/analysis/examples/verify_vikor.rs::main`: pymcdm S/R/Q and mixed-direction oracle outputs; stale defect comments are reported below, not mistaken for current failures.
- Keep `crates/analysis/examples/report_svg_demo.rs::main`: browser-level light/dark visual review of all five chart families; unit mark-count assertions do not replace visual review.
- Keep `crates/io/examples/rdb_smoke.rs::main`: manual backend/storage E2E metadata, rows, extras and fingerprints with caller-supplied storage.
- Keep `egui-app/examples/report_smoke.rs::main`: actual storage -> Study authority -> analysis -> three report formats with caller-selected language/output.
- Keep `egui-app/examples/mem_probe.rs::main`: dhat whole-resident/peak heap diagnostic including retained Study managers, not a mock allocation test.
- Keep `crates/analysis/benches/sobol_bench.rs::bench_sobol`: Criterion objective-count scaling, not an additional numerical oracle.
- Keep `crates/analysis/benches/sensitivity_bench.rs::bench_sensitivity`: Criterion whole per-objective multi-method execution scaling.
- Keep `crates/analysis/benches/permutation_bench.rs::bench_permutation`: Criterion feature-count scaling of permutation computation.
- Keep `egui-app/benches/journal_bench.rs::{bench_synthetic,bench_real_fixture}`: Criterion full scan/acquired-projection cost on large synthetic and real journals, separate from correctness gates.

These are **22 original examples (21 retained, 1 deleted)** and **four retained
benchmark files**. All five tracked Python/CJS files are the already-reviewed
three fixture producers and two issue258 probes. No other tracked script test
files were found. Compilation of retained examples/benches is verified by final
all-target clippy, not described as execution of external numerical/visual oracles.

Additional out-of-scope documentation issue: verify_vikor's comments still claim
a maximize accumulator defect, although the retained core mixed-direction tests
exercise the corrected implementation. Historical hypervolume.md references the
now-removed old repro as historical execution evidence. Those historical reports
were not rewritten as part of test pruning.

## Final post-edit validation

All code edits, including manual-repro removal, preceded these checks:

| Command | Actual result |
| --- | --- |
| `cargo test --workspace --locked --profile ci-test` | Success: **1861 passed, 0 failed, 14 ignored**, 1875 registered cases. GUI lib/bin each 481 passed/2 ignored; analysis 532/2 ignored; I/O 100/3; runner 97; Study 2+9; MCP 15+4/2 ignored. All integration harness counts match the final table. Five zero-case doc harnesses passed. |
| `cargo fmt --all -- --check` | Success. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Success, including all retained examples/benchmarks. |
| `git diff --check` | Success; LF-to-CRLF normalization warnings only, no whitespace errors. |
| `TUNNY_NATIVE_CAPTURE_CHECK=1 cargo test -p tunny-dashboard --locked --profile ci-test --test artifact_animation_native` (PowerShell environment removed in finally) | Success after final code edits: native full-app wgpu, 3/3 selected frames, identical live/GIF order, 684x544, 330 ms, 434 responsive UI passes, full prefix observations, independent instances, seek/pause restoration, PNG/JPEG aspect ratios and content-only crop. |

Final workspace output:
`C:\Users\dev\.local\share\opencode\tool-output\tool_168ebae2300157LTKAHKfjBKDq`.
The 14 ignored cases were body-reviewed but not explicitly executed. Unix/POSIX
paths cannot execute on this Windows host; live PostgreSQL/MySQL, licensed
Compute, caller-provided workload/artifact fixtures and pinned external-oracle
environments were not executed. These limits do not represent unreviewed tests.

Final result: **2016/2016 original libtest definitions reviewed**, **1404 Keep,
276 Delete, 336 Merge**. Removing 612 distinct definitions reduces duplicated
Windows registrations **2866 -> 1875** (991 fewer); GUI unit originals execute
twice, so registration reduction is not the distinct-definition count. One
additional invalid manual example was deleted. Product behavior, public APIs and
dependencies are unchanged; only tests/test-only support and this audit record
were changed. No replacement tests were added to restore coverage/counts, and no
commit was created.

## GitHub tracking for out-of-scope findings

All findings above were rechecked against the implementation and all 50 existing
open/closed issues before filing. The following new issues track separate fixes;
their bodies explicitly distinguish source analysis from unexecuted reproduction
proposals. The broader ownership issue #282 is related, not a duplicate bug fix.

| Finding recorded above | Tracking issue |
| --- | --- |
| Histogram same-name/same-count input cache, including bins/statistics/fits | [#283](https://github.com/Tunny-gh/Tunny-Dashboard/issues/283) |
| Correlation Matrix same-count input/schema cache | [#286](https://github.com/Tunny-gh/Tunny-Dashboard/issues/286) |
| ClusterScatter3D and Pareto3D stale axis ranges | [#285](https://github.com/Tunny-gh/Tunny-Dashboard/issues/285) |
| TrialTable visible row indices after trial-ID mapping changes | [#284](https://github.com/Tunny-gh/Tunny-Dashboard/issues/284) |
| MCDM 2D/3D score coordinates after score changes with unchanged ranking | [#290](https://github.com/Tunny-gh/Tunny-Dashboard/issues/290) |
| PCP NaN observations passing active brushes | [#287](https://github.com/Tunny-gh/Tunny-Dashboard/issues/287) |
| Stale maximize-defect explanations in verify_vikor | [#288](https://github.com/Tunny-gh/Tunny-Dashboard/issues/288) |
| Historical hypervolume report retaining the invalid dominated-point interpretation / old repro reference | [#289](https://github.com/Tunny-gh/Tunny-Dashboard/issues/289) |

Cluster/Pareto 3D share the Range3DCache input-dependency defect; MCDM 2D/3D share
score extraction and ranking-only invalidation, so each pair is one fix unit.
Histogram and Correlation Matrix have separate private caches and separate issues.
Historical HV output should be annotated, not rewritten as a successful old run.
This follow-up changes only tracking links; product/test fixes belong to the issues.
