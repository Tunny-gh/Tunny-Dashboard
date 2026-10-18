# ADR-0023: Include supported attribute changes in RDB fingerprints

Status: Accepted
Date: 2026-10-18

## Context

The public RDB fingerprint API describes change detection, but its count/state
aggregates omit existing-trial attribute changes. There is no active Dashboard
fingerprint consumer: the GUI uses manual Reload. During the #259 design
discussion, the user explicitly chose to include supported attribute changes in
the public contract anyway, rather than narrow the API to structural progress.
This decision is independent of crate extraction and database snapshot work.

## Decision

Include COMPLETE trial user-attribute insertion/in-place updates and all-state
`constraints` insertion/in-place updates in fingerprint detection. Preserve the
existing count/state/intermediate fields and add an attribute digest. Match
current parser coverage, not the future all-state user-attribute model.

Read the two relevant attribute sets ordered by trial ID and key, through the
existing backend callback. Reuse FNV-1a and standard facilities: include the
attribute kind, trial ID, length-framed key and stored JSON text. Hash stored text
without JSON parsing/canonicalization. Conservative formatting-only changes may
trigger rereads; equality remains a change-detection hint, not semantic identity
or a collision-free guarantee. Do not persist or compare fingerprints across
implementations as a stable serialization format.

## Alternatives

- **Document attributes as excluded because there is no GUI poller.** Rejected by
  the user: absence of an internal consumer does not narrow the public contract.
- **Count attribute rows or use maximum row IDs.** Rejected because in-place
  updates preserve both and must be detected.
- **Parse/canonicalize JSON or fully parse the Study for every fingerprint.** Not
  needed for conservative change detection; adds parsing/allocation work to avoid
  harmless extra rereads. Reuse the existing lightweight hash instead.
- **Add triggers, version tables, or a new polling service.** Rejected: would
  require external schema changes or unrelated architecture instead of correcting
  the existing public API.

## Consequences

Each query now adds two ordered attribute scans. Client hashing is proportional
to matching payload bytes and does not collect the full result; database joins,
sorting, driver buffering and network transfer still have costs. There is no
measured latency or memory bound. The 64-bit non-cryptographic hash has collision
risk and is not suitable as an adversarial integrity check. Raw-text formatting
changes can produce false-positive refresh hints.
Adding a public struct field can require downstream struct literals to change;
backward compatibility is not preserved for this correction.

All-state constraint evidence remains governed by ADR-0007. No new automatic
refresh, semantic equivalence guarantee, external read snapshot, or coverage of
unrelated fingerprint collision candidates is introduced. #251 must carry this
contract if it migrates the fingerprint API, but #259 is not a prerequisite for
every extraction path. Runtime evidence and verification belong in the
[investigation report](../reports/2026-10-18_rdb-attribute-fingerprint-investigation.md).
