# Artifact Animation followup performance evaluation

Date: 2026-10-09

## Later sampling-policy amendment

The measurements below are historical: sampling was export-only when they were
collected. A subsequent explicit user requirement makes Frame skip common to
widget playback and GIF export, with the same selected chronological sequence.
The implementation now applies transport, lookahead, and image windows to that
sequence while retaining every prefix observation. The original measurement
claims are preserved, not relabeled as new benchmarks of nonzero shared skip.
Selected-preview/GIF regression and native verification cover the amended policy.

The amended full-app wgpu harness plays the selected preview before exporting
and compares the observed image-index sequence with decoded GIF order. It passed
2 selected of 3 frames with skip 1 (684 x 544, scale 1, 381 UI passes) and
12 selected of 64 frames with skip 5 (513 x 408, scale 0.75, 841 UI passes).
Both used exactly 330 ms GIF delays, retained full prefix observations, and
restored the initiating selected seek/pause position. These synthetic checks are
not new real-workload performance measurements.

## Conditions and method

Baseline: `294ca71220823f7997d46742eba1bd038217b26d` on an isolated detached
worktree, with only the workload-probe hooks added. The original comparison below
used the followup working tree before the P2 review correction. The corrected
event-driven comparison is recorded separately below. Both use Rust 1.97.1, the optimized
`ci-test` profile, the production Animation widget, and Optimization History
enabled. Baseline uses 10 FPS; final uses 100 ms, so requested timing is equal.

The authorized private journal/artifact store was read-only. Its eligible
sequence contained 1000 PNGs; the checked source dimensions were 656 x 614.
No source files, paths, names, optimization values, or screenshots of private
data accompany this report. The short-sequence runs truncate the frame sequence
to 64 after mapping; they do not truncate the Study observations or fixed axes.

The Windows host exposes an Intel i7-13700 and 16 logical processors. Display
adapters include a GT 1030, UHD 770, and Remote Display Adapter; the actual wgpu
adapter was not logged. Results apply to this desktop session, not all hardware.

Two distinct measurements were used:

- **Headless egui readiness:** real PNG I/O/decoding, production controls,
  prefix preparation, image resources, and content rendering into egui shapes.
  The probe manually requests every successive frame, sleeps 1 ms between UI
  passes, and resets the playback deadline after each request. The playing flag
  enables chronological prefetch. These are accelerated readiness measurements,
  **not GUI FPS or GPU upload benchmarks**. A second pass and near-return/distant
  seeks reuse the same context and caches.
- **Native eframe/wgpu playback:** real windows and normal playback deadlines,
  not a modeled sum of dwell and preparation times. The probe records readiness
  transitions for all frames across two chronological passes and asserts the
  frame count. Widgets are floating windows rather than the full application
  canvas. Separate full-app native screenshot/GIF checks exercise that path.
  The probe requests continuous redraws to sample native readiness transitions;
  the full application's idle repaint scheduling can differ. Ready production
  playback requests its next deadline, and worker completions wake preparation
  waits; slow display durations do not require a constant 50 Hz poll.

700 x 650 point widgets at scale 1 were used for the main runs. The additional
550 x 500 point, scale 1.5 run exercises layout/DPI-aware preparation. At that
scale, floating windows can overlap within the fixed 1500 x 900 native viewport;
baseline and final use the same arrangement. Two-widget runs use independent
widget state with the same image sequence.

Timing excludes compilation and mapping from pass duration. Mapping plus first
ready is separately recorded. First-pass UI CPU and loading-pass counters include
startup; second-pass counters do not. The first pass has one fewer inter-frame
interval than the looping pass. Cache counters are cumulative, not per-pass.
Repeated reads benefit from the OS file cache; this is not a cold-disk experiment.

## Results: accelerated readiness

| Sequence / instances | Baseline first / loop | Final first / loop |
| --- | --- | --- |
| 1000 / 1, repeated runs | 4.900–4.923 s / 4.927–4.952 s | 3.218–3.250 s / 3.344–3.347 s |
| 1000 / 2 | 4.774 s / 4.768 s | 2.914–2.942 s / 2.927–2.928 s |
| 64 / 2 | 0.297 s / 0.303 s | 0.189 s / 0.196 s |

The final one-widget first-pass mean readiness interval was 3.221–3.253 ms
(p95 3.555–3.565 ms), versus baseline approximately 4.9 ms (p95 about 5.2 ms).
Mapping plus first ready remained about 1.8–1.9 s: initial eligibility validation
still decodes all candidate files separately in each widget. Runtime caching
does not claim to eliminate that initial scan.

For the final one-widget 1000-frame runs, runtime decode starts were 1033 after
the first pass and 2033 after the second: approximately the requested sequence
plus the forward/wrapping window, not repeated whole-window rebuilds. There were
816 / 1824 cumulative evictions, zero obsolete results at the pass boundaries,
and 1999 / 3999 successful image lookups. Lookup counts include polling while a
prefix is pending; they are **not a hit-rate denominator**. The two-widget final
run started 1033–1034 / 2033–2034 decodes total, not twice that number. Baseline has no
image cache and prepares each advancing image separately in each widget.
These runtime counts exclude mapping validation and the probe's one dimension
check.

Final cache-window update/refill/poll/upload bookkeeping took 153–156 ms total
over both one-widget accelerated passes, or about 2.3–2.4% of elapsed pass time.
Two widgets took 310–312 ms over 5.842–5.869 s (about 5.3%). This measurement includes
texture registration and lock acquisition, not only list maintenance; it excludes
decoder threads and prefix sorting. Two-widget second-pass UI CPU rose from
537 ms to 595–597 ms, while elapsed readiness fell from 4.768 s to 2.927–2.928 s. There is
some CPU cost to management, but no observed net throughput regression or
gratuitous window-rebuild decode amplification on these runs.

Near-return seeks in the final full-sequence run took approximately 0.17–1.39 ms,
versus baseline about 2.97–3.58 ms. The distant, evicted-prefix/image seek took
4.77–4.93 ms versus baseline 3.11 ms: a cache miss can be slower because it waits
for bounded obsolete work and performs display-size preparation. It is not a
universal seek-speed improvement. Rapid-seek regression tests separately check
that obsolete images/prefixes cannot become the displayed frame.

## Results: prior native continuous-redraw diagnostic

| Sequence / instances / scale | Baseline first / loop mean interval | Final first / loop mean interval | Final loop p95 |
| --- | --- | --- | --- |
| 1000 / 1 / 1 | 187.437 / 187.437 ms | 100.036 / 100.001 ms | 125.539 ms |
| 64 / 2 / 1 | 187.498 / 187.484 ms | 100.712 / 100.109 ms | 124.929 ms |
| 64 / 2 / 1.5 | 187.510 / 187.476 ms | 100.707 / 100.094 ms | 125.100 ms |

The 1000-frame native pass durations were 187.250 / 187.437 s baseline and
99.937 / 100.001 s final. The observed session's presentation intervals are
quantized near 31.25 ms, so a 100 ms mean does not imply every frame is exactly
100 ms on screen. Baseline added preparation/UI passes before starting each new
dwell; final retains deadlines for ready prefetched frames and freezes the clock
on a genuine preparation miss instead of dropping a frame.

The 1000-frame final native loop had **zero loading UI passes**, versus baseline
1000. Final first-pass loading passes were 62, including mapping/startup; baseline
had 1060. Short final native loops also had zero loading passes versus baseline
64. Full-sequence runtime decodes remained 1033 / 2033 with 816 / 1824 evictions
and zero obsolete results. The short native run started 65 decodes and did not
start more on the loop: the shared short sequence remained resident.

A separate **synthetic** eight-frame/two-widget accelerated diagnostic exercised
both image and prefix hits: 27.2 ms first pass / 13.7 ms loop, eight total image
decodes, no evictions or obsolete results, and zero loop loading passes. Near
seeks took 0.16–0.26 ms. This is not additional private-workload or GUI evidence.

Final full-sequence native cache management totaled 767 ms across both passes
(about 0.38% of playback time). Loop UI CPU was 1953 ms versus baseline 2443 ms.
Short two-widget loop UI CPU was 186 ms versus baseline 170 ms; additional
management is measurable there, but timing and loading stalls still improve.

## Budget, tuning, and limitations

The estimated retained-resource peak stayed below the shared 256 MiB budget:
255.8 MiB headless / 255.2 MiB native for the 1000-frame sequence. The short
native sequence retained about 68.4 MiB across **both** widgets at scale 1 and
54.3 MiB at scale 1.5. The long sequence therefore exercises eviction rather
than accidentally benchmarking an unlimited all-frame cache.

Accounting conservatively estimates RGBA image plus texture storage at eight
bytes per prepared pixel and reserves target-size capacity for pending prefetch.
It is not total RSS, GPU allocation telemetry, or a cap on transient full-size
decoder/screenshot/encoder buffers, Study data, or prefix snapshots. Headless
egui drops upload deltas without a GPU, so its RSS cannot validate native texture
residency. Windows process peak working sets in the final runs were about
27–29 MiB headless and 218 MiB native. Baseline native full-sequence peak was
also about 219 MiB. Renderer/startup allocations dominate that peak; memory
samples after window/context teardown are not steady-state cache measurements.
Dedicated GPU memory was not measured.

## P2 review correction: event-driven readiness and playback

Review found that prefix lookahead was selected **before** a resident image and
cached current prefix established `ReadyFrame` in the same UI pass. With no other
repaint source, the widget could sleep until its next deadline without starting
the following prefix. Preparation then introduced an avoidable loading wait.
The target selection now runs **after** readiness establishment, preserving the
existing one-worker prefix bound without adding an unconditional extra pass.

The regression test warms all three synthetic images while paused, retains only
the next displayed prefix in its prefix cache, and transitions to that frame at
an expired deadline. It verifies that lookahead starts in that same pass, follows
only egui-requested immediate passes, worker wakeups, or deadlines, and reaches
the following frame without resetting cadence or decoding another image. With
the original ordering restored temporarily, the test failed at the same-pass
lookahead assertion; with the correction it passed.

The native probe now defaults to **event-driven** operation. It requests one
startup pass because its Play handoff occurs after drawing; it does not request
unconditional redraws afterward. Widget timers, worker wakeups, and ordinary
framework/window events remain active. `TUNNY_ANIMATION_PERF_CONTINUOUS=1` is an
explicit diagnostic option, and output distinguishes both modes. The prior
continuous-redraw figures above did **not** establish correctness of the idle
cache-hit transition and must not be used for that conclusion.

The same read-only 1000-PNG workload, optimized `ci-test` profile, 700 x 650 point
widgets, scale 1, and enabled History were used for baseline/final event-driven
runs. Baseline hooks still set 10 FPS; the corrected widget uses 100 ms. Baseline
was built from the isolated `294ca71` worktree with a distinct verified probe
artifact. Shared package artifacts were cleaned before rebuilding the final
workspace to prevent cross-worktree build-script/executable reuse.

| Event-driven sequence / instances | Baseline first / loop mean | Corrected first / loop mean | Corrected loop p95 |
| --- | --- | --- | --- |
| 1000 / 1 | 187.435 / 187.469 ms | 100.038 / 99.999 ms | 125.870 ms |
| 64 / 2 | 186.978 / 187.491 ms | 100.715 / 100.095 ms | 126.476 ms |

The 1000-frame durations were 187.248 / 187.469 s baseline and 99.938 / 100.000 s
corrected. Corrected loop loading passes were zero, versus baseline 1000; the
two-widget short loop likewise had zero versus 64. Both complete sequences were
observed without dropped transitions. Full-sequence runtime image decode starts
were 1033 / 2033, evictions 816 / 1824, obsolete completions zero, and the retained
estimate peaked at 255.2 MiB. The short shared sequence started 66 decodes
(including one obsolete completion) and started none on the loop.

Full-sequence cache management totaled 767.5 ms over 199.938 s, approximately
0.384%. Loop UI CPU was 1904.6 ms versus baseline 2424.5 ms. The short two-widget
case used 71.9 ms of cumulative management over 12.751 s, approximately 0.564%,
with loop UI CPU 160.4 ms versus baseline 209.0 ms. Moving target selection did
not introduce an additional image-window rebuild or a net management slowdown.

An event-driven 64-frame run of the **uncorrected followup** also averaged
100.112 ms on its loop with zero loading passes on this host. Ordinary framework
events still supplied incidental passes: the native short case alone did not
expose the otherwise-idle race and shows no meaningful before/after timing gain.
The isolated regression, not that incidental native schedule, proves that the
corrected widget no longer depends on an extra pass to start lookahead. Native
means/p95 remain session-dependent; these are single paired measurements, not
confidence intervals or exact physical-display timing guarantees.

A corrected two-widget accelerated readiness run completed 1000 frames in
2.358 / 2.298 s, versus the prior followup's approximately 2.94 / 2.94 s and the
original baseline's approximately 4.77 / 4.77 s. It started 1005 / 2004 runtime
decodes with 768 / 1776 evictions, one obsolete completion at the pass boundaries,
and 218.9 ms cumulative management (about 4.7% of accelerated elapsed time).
Loop loading passes dropped to 501 and loop UI CPU to 471.9 ms. This remains a
headless readiness diagnostic, not GUI FPS. The distant seek in this run took
6.40 ms; miss/obsolete-work waits remain a limitation rather than a universal
seek-speed improvement. Process peak working sets were about 32.7 MiB headless
and 218.6 MiB native; dedicated GPU memory and post-teardown allocation were not
used as validation of the retained-resource estimate.

### Prior resumed verification (before the P2 correction)

After resuming the preserved work, the real-workload two-widget accelerated
probe again completed both 1000-frame passes in 2.935 / 2.939 s, with 1033 / 2033
decode starts, 816 / 1824 evictions, and 312.5 ms cumulative cache management.
The 64-frame native two-widget loop averaged 100.026 ms (p95 124.871 ms), had
zero loading passes, and reused its 65 prepared images without additional loop
decodes. These are followup sanity checks, not replacements for the original
paired baseline comparisons above.

Startup varied more in this session: mapping plus first ready was 4.710 s in the
accelerated run and 2.018 s in the subsequent native run. Headless peak working
set was 44.2 MiB; native peak was 218.5 MiB, with substantially different
post-teardown private allocation. OS caching, background activity, and renderer
allocation were not controlled. This reinforces the distinction between retained
resource accounting and process memory, and limits any startup or RSS claim.

The evaluated implementation uses two shared image preparation slots, an
adaptive forward window capped at 32 upcoming frames with two recent frames,
and four neighbors on either side while paused. It reuses overlaps, uses
20% eviction headroom, and never evicts pinned live/export textures.
Window capacity also accounts for competing clients and in-flight reservations;
oversized desired windows shrink rather than repeatedly filling and evicting
one another. The initiating live window is suspended during export, so its
prefetch does not compete with the selected export sequence. Prefix work
is bounded to one live worker per widget with one-frame lookahead; existing
bounded prefix snapshots retain full-objective analysis.
Eligibility mapping and prefix worker slots survive source-reset cancellation
until completion, so repeated invalidations cannot accumulate obsolete workers.
Only the latest mapping request is queued; canceled completions are discarded.
These are tunable implementation choices, **not additional product requirements**.
Prepared-image
dimensions are bucketed upward in 64-pixel increments, capped at 2048 pixels
per side and further limited when many clients compete. Very large/high-DPI
presentations can therefore be softer than their full-resolution sources.

Uncached distant seeks, resize-induced variants, very large multi-objective
prefix sorting, storage latency, and cancellation of a decoder already inside
its non-interruptible decode can still incur waits. No silent-file-update
detection or total-process-memory guarantee is implied. Full-sequence native
runs were performed once per version; short native runs and accelerated runs
provide repeat observations, not a controlled statistical confidence interval.

## Native GIF evidence

The full-app wgpu screenshot → crop → background encode → decode harness passed:

- All 3 frames, 684 x 544 output, scale 1, exactly 330 ms per frame, 44 UI passes.
- 12 selected of 64 frames with skip 5, 513 x 408 output, scale 0.75, exactly
  330 ms per frame, 116–120 UI passes in repeated checks. This includes the appended last frame.
- All 64 frames, 684 x 544 output, scale 1, exactly 330 ms per frame, 771 UI
  passes. The retained synthetic GIF's first frame was also visually inspected:
  image, Trial annotation, objective value, Pareto, and History are present;
  transport controls and surrounding panels are absent.

Both check source-image colors in output order, constant dimensions, content-only
crop, PNG/JPEG aspect ratios, two independent instances, and original seek/pause
restoration. Unit tests cover 20/330/10000 ms encode/decode delays, invalid timing,
sampling endpoints/counts and prefix preservation, budget/pinned texture reuse,
resize, invalidation, rapid seeks, cancellation, and atomic publication.
The native harness bypasses the OS file picker; mouse-driven save/cancel dialogs
and arbitrary hardware/viewer playback timing are not certified by these checks.

## Synthetic reproduction only

The repository probe creates deterministic synthetic PNGs and Study observations
when no input paths are provided. Ensure the temporary parent exists first:

```powershell
Test-Path -LiteralPath (Join-Path $env:TEMP "opencode")
$env:TUNNY_ANIMATION_PERF = "1"
$env:TUNNY_ANIMATION_COUNT = "1000"
$env:TUNNY_ANIMATION_INSTANCES = "1"
$env:TUNNY_ANIMATION_WIDTH = "700"
$env:TUNNY_ANIMATION_HEIGHT = "650"
$env:TUNNY_ANIMATION_SCALE = "1"
cargo test -p tunny-desktop --locked --profile ci-test --test artifact_animation_native

# Event-driven native playback, using the same sequence and requested timing:
$env:TUNNY_ANIMATION_PERF_NATIVE = "1"
cargo test -p tunny-desktop --locked --profile ci-test --test artifact_animation_native
```

Repeat with count 64 / instances 2, then width 550 / height 500 / scale 1.5.
Native runs are event-driven by default. Set
`TUNNY_ANIMATION_PERF_CONTINUOUS=1` only to reproduce the older continuous-redraw
diagnostic, and remove that variable before event-driven comparisons.
Remove the probe variables before ordinary test or GIF-capture runs. Synthetic
files live only in a temporary subdirectory under the verified parent and are
removed on completion; synthetic metrics need not match the private workload.

For a baseline comparison, create a detached worktree at the baseline SHA under
a verified temporary parent, **never check it out over the active branch**. Port
only `performance.rs` and the opt-in harness hooks; the baseline configuration
hook sets `fps = 10` and its stats hook describes the original one-frame policy.
Do not port caching, timing, sampling, or renderer changes. Keep compiler,
profile, count, size, scale, and instance count identical. Provision the existing
native LightGBM dependency for that worktree. Use separate build target
directories, or distinct explicitly verified probe artifacts: shared Cargo
targets can reuse the wrong worktree's executable/build-script paths. Executable
output must identify the baseline one-frame policy versus final cache counters.

Native GIF checks are independent of the workload probe:

```powershell
$env:TUNNY_NATIVE_CAPTURE_CHECK = "1"
$env:TUNNY_NATIVE_CAPTURE_FRAMES = "64"
$env:TUNNY_NATIVE_CAPTURE_SKIP = "5"
$env:TUNNY_NATIVE_CAPTURE_SCALE = "0.75"
cargo test -p tunny-desktop --locked --profile ci-test --test artifact_animation_native
```
