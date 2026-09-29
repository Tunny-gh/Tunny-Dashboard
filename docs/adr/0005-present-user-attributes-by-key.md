# ADR-0005: Present trial user attributes by key

Status: Accepted
Date: 2026-09-29

## Context

ADR-0004 displayed a numeric and a text attribute with the same key as two
columns. Issue #200 extends trial user attributes to all JSON value kinds and
requires one logical presentation per key, including when its type varies
between trials. The display rule in ADR-0004 conflicts with that requirement.

## Decision

Present each trial user attribute key once by default in the detail modal,
All Trials table, and its CSV output. In All Trials, an optional list expansion
adds `key[0]`, `key[1]`, and subsequent columns after the base `key` column,
up to the longest array stored under that key. The trial detail modal can
likewise expand the current trial's arrays into indexed rows. The base value
remains visible for scalar values used under the same key. All Trials CSV follows
the expansion setting, retaining the original array in the base key column.
Preserve the underlying JSON value kind and keep numeric attribute data
available to existing numeric analysis. A missing attribute remains distinct
from an explicitly present JSON null.

## Alternatives

- **Keep a separate display column per value kind.** This follows ADR-0004 but
  makes one logical attribute span multiple sparse columns as more JSON kinds
  are supported.
- **Convert every attribute to text in the data layer.** This simplifies display
  but breaks consumers of numeric attribute columns and loses type information.

## Consequences

ADR-0004's type-suffixed display columns are retired. Presentation and export
must select the value for a key and trial without inferring JSON types from
text, while numeric analysis continues to use numeric values only. List
expansion is off by default because wide arrays add many table columns.
