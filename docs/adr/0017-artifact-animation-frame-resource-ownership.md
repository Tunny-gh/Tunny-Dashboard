# ADR-0017: Artifact Animation frame resource ownership

Status: Accepted
Date: 2026-10-08

## Context

[Issue #245](https://github.com/Tunny-gh/Tunny-Dashboard/issues/245) requires
bounded asynchronous preparation and export without stale images or a second
styled renderer. Artifact Gallery uses egui_extras' file/image loaders. Native
capture verification exposed a lock-order deadlock when those globally locked
loaders were polled from a worker while egui checked pending images on the UI
thread. Gallery paging can also evict shared URI caches during an export.

## Decision

Reuse artifact associations, the existing image dependency's PNG/JPEG decoding,
and native egui Image/texture rendering. Decode one animation image off the UI
thread and retain its owned TextureHandle until that prepared frame is no longer
needed. Do not background-poll egui's shared URI loaders or change Gallery's
loading behavior. Owned texture handles release old frames without evicting
Gallery's cache.

Cache a bounded number of prefix rank snapshots against the immutable Study
snapshot, using the existing non-dominated sorter off the UI thread. Seeking to
an evicted prefix may recompute it; rendering an unchanged frame does not sort
again. Export hands one cropped frame at a time to the background encoder and
waits for its acknowledgment before advancing.

## Alternatives

- **Poll Gallery's URI loader from workers.** Rejected after native verification
  demonstrated the loader/context lock-order deadlock.
- **Decode through the URI loader on the UI thread.** Rejected because large
  images can block the UI, and shared cache eviction complicates snapshot lifetime.
- **Retain every image and prefix.** Rejected because memory grows with the whole
  animation, including potentially quadratic prefix storage.
- **Create a generalized rendering/loading subsystem.** Rejected as unnecessary
  for this widget and outside the issue's scope.

## Consequences

Animation uses the same standard rendering primitives as the Gallery without
sharing its mutable URI cache. Re-reading a frame avoids stale cache content at
the cost of decode work on backward seeks. Prefix-cache misses can incur sorting
latency, but preparation runs off-thread and the UI waits for the correct image
and analysis instead of capturing stale content. Memory retains only bounded
prefix snapshots and in-flight frame resources, not the entire image sequence.
