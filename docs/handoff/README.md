# Handoff Notes

Implementation records for work sessions: what was done, why, and what state
things were left in — enough for the next person (or agent) picking up the
work to get oriented without re-reading the whole diff.

This is distinct from [`docs/reports/`](../reports/), which holds one-off
audits and investigations, and from [`docs/planning/`](../planning/), which
holds forward-looking strategy.

This README is the entry point: check the **Open Items** column before
opening any individual file.

## Convention

- One file per handoff, named `YYYY-MM-DD_short-topic.md` (same dated-prefix
  style as `docs/reports/`).
- Required for any work that involved a non-trivial implementation decision —
  see [CONTRIBUTING.md](../../CONTRIBUTING.md#documentation) for exactly when.
- Each file covers three things:
  1. **Decision** — what was decided and why, including rejected alternatives.
  2. **What changed** — what was actually implemented.
  3. **Open Items** — what's left, or `None.` if fully wrapped up.
- Adding a handoff file and updating the index row below is one edit, not two
  separate steps — an entry missing from the index is as good as not existing.

## Index

| Date | Topic | Open Items |
| ---- | ----- | ---------- |
| 2026-08-13 | [Documentation reorganization](2026-08-13_docs-reorganization.md) | CHANGELOG/handoff rules are prose-only, not CI-enforced |
