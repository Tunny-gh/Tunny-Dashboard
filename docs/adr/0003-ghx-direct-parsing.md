# ADR-0003: Parse .ghx directly for Grasshopper integration

Status: Accepted
Date: 2026-09-17

## Context

The core goal of Phase 2B is drag-and-drop of a Grasshopper definition onto the
Dashboard to run an optimization via Rhino.Compute. The MVP form of this goal,
decided here, is drag-and-drop of a `.ghx` file; drag-and-drop of a plain `.gh`
file is a later, manifest-based fallback (see Alternatives). To run the optimization
the Dashboard must know the problem definition — which sliders and Gene Pools
are variables and their ranges, how the Tunny component's inputs are wired, and
the Tunny settings — and it must be able to inject the values Rhino.Compute
needs. The choice between direct parsing and a Tunny-side manifest was made in
2026-07.

## Decision

The MVP parses the `.ghx` file (the XML serialization of GH_Archive) directly on
the Dashboard side. From that XML the Dashboard extracts variable sliders and
Gene Pools (name, range, precision), wire connections, and Tunny settings, and
injects RH_IN and RH_OUT groups directly into the ghx XML.

## Alternatives

- **Embed a problem-definition manifest into the `.gh` file (a Tunny-side
  feature) as the MVP approach.** Rejected as the MVP: it requires a Tunny
  release before any Dashboard work can be tested, which couples the two
  projects' release cycles. It is retained only as a fallback, for supporting
  drag-and-drop of plain `.gh` files and for future changes to the GH_Archive
  format.

## Consequences

- The Dashboard owns parsing and is coupled to the GH_Archive format. The
  manifest fallback mitigates this: if the format changes, the Tunny side can
  supply a stable definition instead.
- Rhino.Compute is Windows-only, while the Dashboard is cross-platform. How to
  reconcile the two remains an open question.
