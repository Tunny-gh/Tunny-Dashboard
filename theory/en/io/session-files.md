# Session Files

## Overview

A session file (plain JSON, saved as `*-session.json`) captures **how the data is being viewed**, not the data itself. It persists three layers:

- **Canvas layout** — which widgets are placed, their positions/sizes, z-order, pan/zoom, and panel state.
- **Per-widget settings** — every user-adjustable knob of every placed widget (selected axes/columns, cluster count, MCDM weights, histogram bin rule, normalization and log-scale toggles, feasibility filters, 3D camera poses, …). Each canvas item keeps its own independent copy.
- **Global view settings** — colormap, filter ranges, pinned trials, hypervolume reference point, convergence indicator, help language.

The optimization data (journal / SQLite / CSV) and everything derived from it are deliberately **not** stored.

---

## The state-separation principle

The application state divides into three categories, and the session boundary follows directly from it:

| Category | Examples | In session? |
| -------- | -------- | ----------- |
| Data | trials, objective values, study metadata | No — belongs to the storage file |
| Derived state | Pareto ranks, cluster assignments, fitted surrogates, MCDM scores, caches | No — a pure function of (data, settings) |
| View state | layout, widget settings, colormap, filters | **Yes** |

Derived state is excluded because it is fully reproducible: after a session is restored, each widget's normal polling path recomputes its results from the current data and the restored settings. Persisting it would only risk showing stale results if the data has changed, and would bloat the file with model weights and caches.

Because no data is stored, a session can be applied to a **different dataset**: open another study and the same dashboard configuration is reused. Column references degrade the same way they do when switching studies inside the app — name-based references (axis and column selections) fall back to a default column when the name does not exist, and index-based references clamp to the available range.

---

## Format and schema evolution

The file is a single JSON document:

```json
{ "version": 1, "layout": { ... }, "widgets": { "<item-id>": { ... } }, "view": { ... } }
```

Compatibility follows the *tolerant reader* pattern:

- **Unknown fields are ignored** on load, so a file written by a newer minor revision still opens.
- **Missing fields take their defaults**, so a file written by an older revision also opens; newly introduced settings simply start at their default values.
- The `version` number is therefore only raised for **breaking** changes (a field changing meaning or type), and a file with a version newer than the application understands is rejected with an explicit error instead of being misread.

Runtime-only fields (compute results, caches, trained models, modal state) are excluded at the type level: they are marked as skipped during serialization, so they can never leak into the file and are reset to defaults on load.

---

## Characteristics

- A session restore never triggers I/O on the optimization data; it only replaces layout and settings, which makes it instantaneous and safe to apply while data is loaded.
- Pinned-trial IDs and filter ranges are stored as-is. Against a different dataset they may reference trials or ranges that do not exist; consumers ignore missing IDs, and filters for absent columns have no effect.
- The session file is human-readable JSON, so dashboards can be diffed and version-controlled alongside a project.

---

## Where It Is Used in the App

- **Save Session / Load Session toolbar buttons**: save the current dashboard to a session JSON file, or restore one while keeping the currently loaded data.
