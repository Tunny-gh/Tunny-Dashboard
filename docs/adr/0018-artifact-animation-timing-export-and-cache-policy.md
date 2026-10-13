# ADR-0018: Artifact Animation timing, export sampling, and cache policy

Status: Accepted
Date: 2026-10-09

## Context

[PR #246](https://github.com/Tunny-gh/Tunny-Dashboard/pull/246) introduced Artifact
Animation with editable FPS, all-frame GIF export, and the one-prepared-frame
resource policy in [ADR-0017](0017-artifact-animation-frame-resource-ownership.md).
The followup agreements choose explicit frame timing and export sampling, and
trade repeated image preparation against a shared, bounded runtime cache.

The original sampling decision was export-only. A subsequent explicit user
agreement requires widget preview to show the same selected chronological
sequence as GIF export. The shared sampling decision below replaces that earlier
policy; timing, analysis semantics, and resource ownership remain unchanged.

## Decision

### Shared display duration

Use `display_duration_ms` as the common UI playback and GIF frame duration,
replacing the editable FPS setting. Provide a logarithmic slider and numeric
input for 20 through 10000 ms in multiples of 10 ms, defaulting to 330 ms.
FPS is a computed reference only (`1000 / display_duration_ms`), not a second
editable timing setting. Milliseconds express the intended display duration
directly and the 10 ms increments align with GIF delay units.

### Shared preview and GIF sampling

Provide an integer skip count `N >= 0`, bounded by the eligible frame count.
Starting with the first eligible image, select every `N + 1`th eligible frame.
Append the final eligible frame if it is absent, without duplicating it.
`N = 0` selects all eligible frames. Widget playback and GIF export use this same
sequence. Play/Pause, Previous/Next, seeking, frame labels/counts, prefix lookahead,
image prefetch, and looping all follow the selected sequence without renumbering
Study trials. The persisted setting is common; the UI calls it Frame skip.

When skip changes, retain the currently displayed image if it is selected;
otherwise move to the nearest selected predecessor and clamp to a valid frame.
Discard obsolete preparation and do not display or capture an old frame under
the new selection. Reuse still-valid cached resources rather than rebuild them.

Sampling selects images, not analysis observations: at a displayed/exported frame's trial
number `t`, prefix charts still include all observations with trial number
`<= t`, including observations without images and those between sampled frames.
Show the selected frame count and estimated duration per pass. There is no target-seconds
setting or automatic timing/sampling adjustment. This keeps export size and
duration under explicit user control while retaining the final eligible state
and making preview representative of export.
The protected shared-renderer export interaction in
[ADR-0016](0016-artifact-animation-export-interaction.md) remains unchanged.

### Shared bounded Animation resources

Prepare images with awareness of display size and DPI. Use a bounded runtime
image cache and prefetch, with a global 256 MiB budget across Animation widgets
for estimated retained image, texture, and prefetch resources. This is resource
accounting, not a limit on total process RSS. Do not expose a budget UI.
Re-reading images after eviction is acceptable.

Keep instance cursor, timing, presentation, and prefix state independent while
sharing compatible owned image resources. Bound concurrent work and
cancel/discard obsolete preparation.

Ready frames follow the playback deadline rather than automatically adding
preparation time to every dwell. If a frame is not ready, wait and give it its
display dwell once prepared; do not silently drop live frames or display/capture
an image with a different trial's prefix.

Invalidate Animation resources on Study or source changes, reload, and artifact
rescan. Detection of silent file updates is not guaranteed; retained images may
remain until invalidation or eviction.

This supersedes ADR-0017's one-prepared-image retention policy and its implication
that re-reading avoids stale cache content. Preserve its independent ownership
and deadlock avoidance: Animation does not share Gallery's mutable URI cache,
background-poll egui's globally locked URI loaders, or change Gallery loading
behavior. Continue using the existing image dependency and native egui
Image/texture rendering. ADR-0017's bounded prefix snapshot caching against the
immutable Study snapshot, off-thread preparation, and one-cropped-frame encoder
acknowledgment flow remain in force.

### Timeline-aware cache window

Choose a moving cache window that follows the chronological order of eligible
frames rather than a fixed cache range. During playback, orient the window toward
upcoming frames; while paused, retain neighboring frames around the current
frame. Seeking changes the target and invalidates obsolete prefetch work. When
playback loops, prefetch wraps to the beginning of the eligible sequence. During
playback and export, follow the common selected sequence rather than preparing
skipped frames.
These priorities remain subject to the shared 256 MiB resource budget.

The intent is to spend bounded resources on frames likely to be needed next,
balancing memory use with playback and seek responsiveness. Changing the cache
range must not itself become a source of slowdown: avoid gratuitous eviction,
re-preparation, and prefetch churn. No exact window width, frame count, or internal
algorithm has been agreed. Hysteresis, batched refill, or reuse of still-useful
resources may be used if justified by evaluation; none is a mandated algorithm.

## Alternatives

- **Editable FPS or separate UI/GIF timing.** Rejected in favor of one explicit
  display duration, avoiding competing timing controls and preview/export drift.
- **Target duration with automatic timing or sampling adjustment.** Rejected
  because it hides how many images are selected and how long each is displayed.
- **Keep sampling export-only.** Replaced at the user's explicit request because
  full-frame preview does not show the sampled GIF sequence.
- **Sample prefix analysis along with images.** Rejected because skipped images
  and artifactless trials must not remove observations from the analysis.
- **Retain only the currently prepared image.** Replaced to allow bounded reuse
  and prefetch rather than repeated preparation on every revisit.
- **Retain every image, or allocate 256 MiB per widget.** Rejected because memory
  would grow with sequence length or the number of Animation widgets.
- **Keep a fixed cache range regardless of the timeline target.** Not selected
  because it spends the bounded budget on frames that may no longer be useful
  after playback advances or a seek changes the target. A moving window instead
  accepts cache-management overhead that must be evaluated and controlled.
- **Share Gallery's URI cache.** Still rejected for the lock-order deadlock and
  independent cache-eviction concerns recorded in ADR-0017.

## Consequences

UI and GIF share directly controlled duration and sampling; selected images
change in both preview and export without changing prefix analysis. The appended
final frame can add one frame beyond the regular sampling cadence, so count and
duration must reflect the actual selected sequence.

Display-aware caching and prefetch allow reuse within a shared resource budget,
at the cost of eviction, re-reading, and possible preparation latency. The budget
does not promise a process-memory ceiling or immediate visibility of silently
modified files. Gallery cache behavior remains independent.

Timeline-aware caching is a responsiveness tradeoff, not a general performance
guarantee. Memory savings alone do not justify a slowdown caused by unnecessary
range changes. The [performance evaluation](../reports/2026-10-09_artifact-animation-followup-performance.md)
records the baseline comparison, native playback/capture checks, cache-management
overhead, reuse and eviction behavior, and limitations. Exact cache mechanics
remain implementation choices subject to measured evaluation, not additional
human requirements.
