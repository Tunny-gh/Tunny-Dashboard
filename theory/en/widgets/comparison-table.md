# Comparison Table

The Comparison Table widget lays out **pinned trials** as columns and their objectives, parameters, and user attributes as rows, with the best value in each row highlighted. It is the numeric counterpart to [Radar Comparison](radar-comparison.md): where the radar shows the qualitative *shape* of a handful of candidates at a glance, the table shows their exact values side by side, ready to quote or export.

## Workflow

1. Pin the candidate trials to compare (📌 in the Trial Table or the trial detail modal). Each pinned trial becomes one column.
2. Read down each row to compare a single objective, parameter, or user attribute across every pinned trial; the best cell in each row is highlighted.
3. Unpin trials to narrow the comparison, or pin more to widen it — the table always reflects the current pin set.

## Reading the table

- **Best-cell highlighting is per row and direction-aware.** For an objective row, the highlighted cell is the minimum or the maximum depending on that objective's optimization direction — not always "the biggest number." Parameter and user-attribute rows have no optimization direction, so they are shown without a best-cell highlight.
- **Categorical parameters are shown as their label**, not coerced into a number — a row for a categorical parameter simply lists each pinned trial's chosen category.

## Caveats

- **Values are shown raw, with no normalization.** Unlike [Radar Comparison](radar-comparison.md), which min-max normalizes every axis to make units and scales comparable on one chart, this table intentionally shows the real numbers — it is the place to check exact figures, not silhouettes. If you need a scale-free comparison of overall shape across many axes at once, use the Radar Comparison widget instead.
- **The table is the numeric ground truth; the radar is the qualitative shape.** Use them together: skim candidate profiles on the radar to shortlist, then confirm the actual numbers behind that impression here.
