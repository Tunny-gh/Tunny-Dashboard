//! Brush (range-selection) logic for the parallel coordinates chart.

use super::layout::normalize_value;

/// Drag kind for PCP brush interaction.
pub(super) enum BrushDrag {
    /// Creating a new range (the legacy behavior). Drags a range from the
    /// `drag_start` anchor to the current pointer position.
    Create,
    /// Translating an existing range. `grab_norm_y` is the normalized Y of the
    /// pointer at drag start, `orig_range` is the `(lo, hi)` before the move.
    /// The delta is added to slide the whole range.
    Move {
        grab_norm_y: f32,
        orig_range: (f32, f32),
    },
}

/// Orders the pair as (min, max) regardless of drag direction.
pub fn ordered_brush_range(start: f32, end: f32) -> (f32, f32) {
    (start.min(end), start.max(end))
}

/// Translates an existing brush range by `delta` (normalized units).
/// Clamps at the edges to stay within [0, 1] while preserving the width.
pub fn shifted_brush_range(orig: (f32, f32), delta: f32) -> (f32, f32) {
    let (lo, hi) = orig;
    let width = hi - lo;
    // Limit delta by whichever edge (low or high) would hit the boundary first.
    let clamped_delta = delta.clamp(-lo, 1.0 - hi);
    let new_lo = lo + clamped_delta;
    (new_lo, new_lo + width)
}

/// Checks whether a single trial (row index `t_idx`) satisfies all active
/// brush ranges under an AND condition. Fails (false) if an axis with a
/// missing value has an active brush.
pub fn trial_passes_brushes(
    t_idx: usize,
    brush_ranges: &std::collections::HashMap<String, Option<(f32, f32)>>,
    cols: &[Option<&[f64]>],
    col_ranges: &[(f64, f64)],
    all_names: &[String],
) -> bool {
    for (axis_idx, axis_name) in all_names.iter().enumerate() {
        let Some(Some((lo, hi))) = brush_ranges.get(axis_name.as_str()) else {
            continue; // no active brush on this axis
        };
        let Some(val) = cols
            .get(axis_idx)
            .and_then(|c| c.as_ref())
            .and_then(|c| c.get(t_idx))
            .copied()
        else {
            return false; // missing value but brush is active → excluded
        };
        let Some((mn, mx)) = col_ranges.get(axis_idx).copied() else {
            return false;
        };
        let norm = normalize_value(val, mn, mx);
        if norm < *lo || norm > *hi {
            return false; // outside brush range
        }
    }
    true
}

/// Filters trials against all brush ranges under an AND condition and
/// returns the trial_id list (TASK-2242). Computed from column slices
/// (borrowed from view) and the parallel trial_ids array, so no row cloning
/// is needed (MEM-003).
pub fn filter_trials_by_brushes(
    trial_ids: &[u32],
    brush_ranges: &std::collections::HashMap<String, Option<(f32, f32)>>,
    cols: &[Option<&[f64]>],
    col_ranges: &[(f64, f64)],
    all_names: &[String],
) -> Vec<u32> {
    (0..trial_ids.len())
        .filter_map(|t_idx| {
            if trial_passes_brushes(t_idx, brush_ranges, cols, col_ranges, all_names) {
                trial_ids.get(t_idx).copied()
            } else {
                None
            }
        })
        .collect()
}
