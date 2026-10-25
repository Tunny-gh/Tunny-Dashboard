# Timeline interval correction (#301)

Base: `858dba7`. Date: 2026-10-25.

## Cause and evidence

A user-supplied local Journal contains one study with 2,500 COMPLETE trials,
all encoded as inline op4 records, with no subsequent op6 records. Read-only
aggregation found valid start/completion timestamps for every trial, 2,500
distinct starts and completions, strictly increasing starts and completions,
and no overlapping adjacent executions. No private records, source identifiers,
or copies of that file are included in this repository.

The recorded elapsed start range is 0–35.867141 seconds; the final completion
is 35.883207 seconds. Actual durations range from 0.003394 to 0.063313 seconds,
with median 0.0157155 seconds. These match the reported trial count and time
scale, but not the claim that every source start is zero.

`crates/io/src/io/journal/parser/state.rs::process_create_trial` previously
initialized inline completion timestamps to `None`, losing all 2,500 recorded
completions. Projection forwarded those missing values unchanged.
`build_timeline_bars` then extended every missing completion, regardless of
state, to the latest known timestamp. Here that was the last start, not the last
recorded completion. CSV used the same bar builder, so agreement with CSV alone
could not detect the ingestion error.

For example, trial number 1250 has a recorded elapsed interval of
16.929795–16.945814 seconds; the old calculation yielded
16.929795–35.867141 seconds. Trial 0 became 0–35.867141 seconds, and the final
trial became a zero-duration interval at 35.867141 seconds.

This explains a triangular envelope with increasing left endpoints and a common
right endpoint. It does **not** reproduce the issue description literally as
common zero starts and increasing right endpoints. No original screenshot/build
comparison or native visual reproduction is claimed. The floating horizontal
bar API preserves its explicit start offset; egui_plot does not implicitly stack
these bars. No writer correction is indicated: the source already records valid
timestamps. The separate inline-completion-loss finding in the October 17
terminal-attribute investigation is corrected by this change.

## Corrected behavior

- Retain valid recorded inline completion timestamps, using the existing naive
  datetime parser. Missing, null, and invalid completion values remain absent.
- Omit COMPLETE/PRUNED/FAIL trials with missing completions from Timeline and
  its CSV. Missing starts remain omitted. Only RUNNING retains the existing
  latest-known-timestamp fallback for a missing completion. WAITING without a
  completion does not acquire an execution interval.
- Keep actual starts, time units, floating-bar rendering, and CSV columns.
  CSV availability is false when no trial has an exportable interval.
- No timezone, plotting-library, domain terminology, or ADR change is needed.
  The separate documentation repository receives EN/JA v0.2 Timeline overrides;
  historical v0.1 pages remain unchanged.

## Verification

`tests/fixtures/timeline-intervals.jsonl` is entirely synthetic. The regression
suite in `src/io/csv_export/timeline_tests.rs` drives full and streaming Journal
acquisition through StudyManager, projection, bar construction, and the public
chart CSV route. It covers sequential intervals, overlaps, inline and op6
completion, valid PRUNED/FAIL intervals, absent/null/invalid terminal completions,
RUNNING fallback, missing starts, and empty export availability.

Headless production `TimelineChart::show` checks inspect the actual egui
rectangle shapes and inverse-transform both endpoints and trial positions.
The 2,500-trial synthetic overview and stored zoom-bounds checks verify all
intervals, CSV agreement, and a visible zoomed bar larger than one pixel in both
dimensions. The opt-in local reference check also passed with the supplied
Journal, read directly in memory. No export or screenshot containing its data
was written. These are headless geometry checks, **not native rasterization or
manual UI verification**. Overview bars can legitimately be subpixel; zoom is
needed to distinguish such short executions.

Commands (Windows, Rust 1.99.1, existing LightGBM prerequisites):

```text
cargo test -p tunny-dashboard --lib timeline_ --locked --profile ci-test
# Set TUNNY_TIMELINE_JOURNAL only in the local process environment, then:
cargo test -p tunny-dashboard --lib timeline_optional_reference_journal_overview_and_zoom --locked --profile ci-test -- --ignored
cargo test --workspace --locked --profile ci-test
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

All passed. The first workspace invocation hit the terminal's 120-second limit;
the same command completed successfully with a 600-second limit. Existing
environment-dependent/diagnostic tests remain ignored; the local reference
check is explicitly ignored unless requested with `--ignored` and its input
environment variable. Native UI verification remains unperformed.
