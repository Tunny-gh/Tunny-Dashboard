# 2026-09-17: Violin Plot

## Decision

Added a **Violin Plot** chart (issue #175) to the Statistics group, next to
Histogram and Box Plot. It draws Gaussian kernel density estimates (KDEs) as
filled violins, either for several numeric columns at once or split by a
categorical parameter.

Decisions made, with rejected alternatives:

- **Two modes in one widget (the "B2" model).** With no category selected, all
  numeric columns of the chosen `Source` (Objectives / Parameters) are drawn
  side by side, one violin per column, at x = column index. With a category
  selected, one numeric column plus one categorical parameter are chosen and
  one violin is drawn per category level, at x = category index. Rejected:
  separate widgets per mode, and a single scatter-like mode — the two modes
  share all of the computation, caching, and drawing, so two widgets would
  duplicate the same code and fragment the UI.
- **Category candidates are string parameters only**, i.e. `param_names` for
  which `DataFrame::get_string_column` returns `Some`. Numeric candidates use
  `numeric_column().is_some()`, which already excludes categorical columns.
  Rejected: inferring categories by low cardinality from numeric columns —
  it would silently reinterpret continuous parameters as categories.
- **Gaussian KDE with a robust variant of Scott's rule, auto bandwidth only.**
  `compute_violin` uses `h = 1.06 * spread * n^(-1/5)` with
  `spread = min(sd, IQR/1.34)` (population sd, sample IQR via the existing
  `quantile`). The textbook normal-reference Scott bandwidth uses `sd` alone;
  the IQR term only makes it robust to skewed/heavy-tailed samples. There is no
  manual bandwidth slider; a bad manual value either over-smooths every
  distribution or turns it into spikes, and the automatic rule is what a violin
  plot is expected to do. When the robust term is zero but `sd > 0` (a non-constant
  sample whose type-7 IQR collapses to 0, e.g. `[1,1,1,1,2]`), `spread` falls
  back to `sd`, so the sample is still drawn as a narrow spike. Only constant
  input (or a non-positive/non-finite `h`) returns `None`, and those groups are
  reported as skipped.
- **Equal-max-width normalization.** Every violin's density is divided by that
  violin's own maximum so all violins reach the same half-width (0.4 x-units).
  Without it, a narrow, tall-peaked group and a wide, low group are not
  comparable, and groups with very different densities make the plot
  unreadable. Rejected: a shared global max — it makes the sparsest group
  nearly invisible.
- **Non-convex fill via a custom `PlotItem` triangle-strip mesh.** `egui_plot`
  0.36 can build `PlotItem`s from outside the crate (`PlotItem`,
  `PlotItemBase::new`, `PlotGeometry`, `PlotBounds` are all public), so a
  custom `ViolinItem` was implemented rather than contorting existing items.
  Rejected alternatives: `Polygon` fills with `Shape::convex_polygon`, which
  produces wrong shading on a multi-modal (non-convex) violin; `FilledArea`
  only fills between two x-indexed lines with the same x values, while a
  violin's left and right edges share *y* values. The custom item emits the
  same "vertices = right edge then left edge, two triangles per grid cell"
  strip that `FilledArea` uses internally, plus the outline and per-group
  median segment, and computes its own `PlotBounds` from the generated points
  so auto-bounds still work.
- **50,000-sample cap.** `compute_violin` deterministically subsamples larger
  inputs (uniformly spaced indices) before computing bandwidth and density, so
  the O(n × grid) evaluation is bounded regardless of trial count.
  `ViolinCurve::n` reports the number actually used.
- **Synchronous computation.** The KDE is cheap enough (128 grid points,
  ≤ 50k samples) to run in the render path and is cached on the widget keyed
  by `(study_name, source, normalize, selected_numeric, category, row_count)`,
  matching Histogram / Box Plot. Rejected: offloading to the async worker
  pipeline (used by the surrogate/PDP charts) — it would add a spinner and
  message plumbing for no perceptible gain.
- **`Normalize [0,1]` is applied before splitting by category.** In category
  mode the whole selected column is min-max normalized first, then the rows
  are grouped, so every level shares the same scale. Normalizing each group
  independently would erase the between-group differences the plot exists to
  show. (In no-category mode each column is normalized independently, exactly
  like the Box Plot.)

## What changed

- `rust_core/src/statistics/violin.rs` (new): `ViolinCurve`,
  `compute_violin`, and focused unit tests (None cases, non-finite filtering,
  grid/range, density validity, unimodal/bimodal shapes, Scott bandwidth,
  subsample cap). Exported from `statistics/mod.rs`. Review follow-up: the
  bandwidth test asserts the sd-winning and IQR-winning branches separately
  (rather than restating `min`), a regression test covers the zero-IQR
  non-constant fallback, and a density-integral test checks the KDE sums to 1.
- `egui-app/src/ui/widgets/stats/violin_plot.rs` (new): `ViolinSource`,
  `ViolinPlotChart` (source / normalize / category / numeric selection + cache),
  and the `ViolinItem` custom `PlotItem`. Uses the shared x-label band helpers
  and `unified_nav` + wheel zoom like Box Plot. Review follow-up: category-mode
  `build_curves` (one curve per level, whole-column normalization before split)
  and `density_scale` (equal max width) now have direct tests. A further review
  follow-up fixed the frame logic: it now derives the column lists *after* the
  Source ComboBox has been drawn (previously they were computed from the
  pre-click source while the cache key already used the new one, so switching to
  a source with an overlapping column name could cache the wrong columns for the
  rest of the session); an AccessKit-driven regression test clicks the source
  combo and asserts the cache holds the newly selected source's columns. The
  empty-state message now reads "Need at least 2 finite, non-identical values to
  estimate a distribution.", which is accurate for constant columns as well as
  for columns with fewer than two values; an AccessKit regression test pins it.
- `egui-app/src/state/types/study.rs`: `StudyView::string_column` convenience
  accessor.
- `egui-app/src/ui/chart/poll_chart/compute.rs` + `poll_chart.rs`:
  `categorical_param_names` helper.
- Integration: `ChartId::ViolinPlot` (enum, `all()`, `label()`, render arm,
  poll early-return), right-panel icon + Statistics group entry, help URL
  slug, `WidgetStates.violin_plot`, and the ChartId count test (34 → 35).
- `egui-app/assets/widget_icons/violin_plot.svg` (new): minimal solid-white
  violin silhouette, tinted per group like the other icons.
- `egui-app/src/io/csv_export.rs` + `csv_export/distribution.rs`:
  `build_violin_plot_csv` writes `group,value,density` (one row per group and
  grid point), reproducing the widget's selection fallbacks. Review follow-up:
  a category-mode test exercises the string-column grouping path.
- `CHANGELOG.md`: `[Unreleased]` entry.

## Open Items

- **UI not visually verified.** The chart compiles and all unit tests pass,
  but the app was not launched, so the violin silhouettes, label band, and
  median markers have not been inspected on screen.
- **Toolchain-drift clippy fixes included.** Rust 1.98 added
  `clippy::chunks_exact_to_as_chunks` and started flagging trait imports made
  redundant by glob imports; clippy `-D warnings` failed on `main` because of
  them. Three unrelated files (`contour/mod.rs`, the report markdown/html
  submodules) were touched to unblock the required lint run. They are
  mechanical and should not conflict with other work.
