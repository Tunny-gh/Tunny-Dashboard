# ADR-0026: Retain partial ALL-objective importance results

Status: Accepted
Date: 2026-10-25

## Context

[Issue #304](https://github.com/Tunny-gh/Tunny-Dashboard/issues/304) adds ALL-objective
comparison to the Importance Chart. Some objectives may produce successful
results while others fail or are unavailable. The user agreed to retain useful
successful comparisons rather than require every objective to succeed.

## Decision

In ALL mode, retain successful objective results and explicitly identify failed
or unavailable objectives. Never represent failure or unavailability as a numeric
zero; genuine zero-valued results remain valid scores.

CSV may export only successful, available results. When the export is partial,
the UI must explicitly indicate this to the user.

## Alternatives

Withhold all results and CSV export unless every objective succeeds. Rejected
because this discards useful successful comparisons when another objective has
no available result.

## Consequences

Users can compare and export the available results despite partial failure, but
must be able to recognize omitted objectives and the partial nature of an export.
This changes neither the importance metrics nor their scores and introduces no
composite score. This ADR records the agreed policy before implementation; it
does not assert that ALL mode has been implemented.
