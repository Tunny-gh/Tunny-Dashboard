# ADR-0027: Index Journal records for Study selection

Status: Accepted
Date: 2026-11-27

## Context

Selecting a Study in an open Journal required parsing every record in the
file. A Study near the end of a large Journal offered little early feedback
even when it accounted for a small share of the records.

## Decision

Build an in-memory index when the desktop opens or reloads a Journal. Keep
record locations and global trial IDs alongside the immutable Journal text.
Route each selection through the existing replay logic with that Study's
records in original order. Study creation and deletion determine which
references are valid and which trial creations consume global IDs.

The index belongs to the acquisition that produced the Study list. Reload
creates a fresh snapshot. Replay retains streaming batch boundaries, late
attribute and metric-name updates, and the existing full-read ordering.

## Alternatives

- Decode all Studies on open. This would retain decoded trial payloads for
  Studies the user might never select and add more work before the list appears.
- Write an index alongside the Journal. Persistent offsets would require
  invalidation across app starts and coordination with external writers.

## Consequences

Opening a Journal includes the cost of scanning record headers and keeping
their locations in memory. First-time Study selection decodes only that
Study's records. The existing storage-wide Artifact scan has its own purpose
and lifetime under ADR-0022.
