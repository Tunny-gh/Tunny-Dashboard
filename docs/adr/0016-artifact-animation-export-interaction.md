# ADR-0016: Artifact Animation export interaction

Status: Accepted
Date: 2026-10-08

## Context

[Issue #245](https://github.com/Tunny-gh/Tunny-Dashboard/issues/245) requires
Artifact Animation GIF export to preserve the live content's presentation using
the same rendering path. Reusing viewport screenshots requires protecting the
capture presentation from layout changes and occlusion while frames are captured.
The issue requires a responsive UI and snapshot-based export but does not specify
whether the initiating widget remains independently usable during export.

The user selected option A in the export-interaction agreement: temporarily pause
the initiating widget and use a protected capture presentation rather than keep
its live playback and settings independently usable throughout export.

## Decision

During GIF export, pause the initiating widget's playback and settings. Render a
protected, fixed-size, content-only snapshot presentation through the same render
path as the live animation. Do not maintain an independently styled exporter.

Keep progress and cancellation responsive, and perform heavy encoding off the UI
thread. Restore the initiating widget's original state on completion,
cancellation, or error.

This decision settles the export interaction only; the remaining requirements
and scope of Issue #245 are unchanged.

## Alternatives

- **Keep live playback and settings independently usable during export.** Not
  selected: this requires separate live and export presentation/state ownership
  while protecting screenshot capture. Temporarily pausing the initiating widget
  simplifies dependable reuse of the existing screenshot approach.
- **Maintain a separately styled export renderer.** Excluded by the issue's
  shared-rendering requirement because it can diverge from the live presentation.

## Consequences

The initiating widget is temporarily unavailable for playback and settings
changes, in exchange for a stable capture presentation. Progress and cancellation
remain responsive, and every export termination path must restore the original
widget state. The capture presentation and live animation share rendering rather
than maintaining parallel styling implementations.
