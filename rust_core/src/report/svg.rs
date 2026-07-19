//! SVG chart drawing primitives for embedding in HTML reports.
//!
//! Functions defined here are **pure string generators**: they have no
//! external crate dependencies and perform no statistical/aggregation
//! analysis whatsoever (the caller, `builder.rs`, is assumed to already
//! pass in a computed data series). All colors are referenced via CSS
//! custom properties defined by [`super::theme`] (`var(--foo)`), and no raw
//! hex color codes are ever emitted inside the SVG (so light/dark themes
//! are followed automatically).
//!
//! # Shared conventions
//!
//! - Sizing is `viewBox`-based, and `width="100%"` makes the chart
//!   responsive (it scales to follow the page width).
//! - Ticks are determined by the 1/2/5 × 10^n "nice number" algorithm
//!   ([`nice_ticks`]).
//! - Grid lines are hairlines (1px, `var(--grid)`); axis labels use muted
//!   ink (`var(--ink-muted)`).
//! - Every data mark gets a `<title>` child element so it works as a
//!   browser-native tooltip (this supports the no-JS design policy).
//! - `<text>` elements specify `font-family="inherit"` so no font name is
//!   embedded in the SVG (the font is inherited from the host HTML page's
//!   CSS). Numeric labels get `font-variant-numeric: tabular-nums`.
//! - Charts with only a single series omit the legend (the `<title>`
//!   doubles as the series name).
//! - Drawing elements are written directly into the output buffer via
//!   `write!` ([`std::fmt::Write`]), avoiding a disposable `String`
//!   allocation per element.
//!
//! # Submodule layout
//!
//! - [`primitives`]: shared drawing/formatting/tick helpers.
//! - [`line`]: `line_chart` (best-so-far / HV history).
//! - [`scatter`]: `scatter_chart` (Pareto front).
//! - [`hbar`]: `hbar_chart` (parameter importance).
//! - [`histogram`]: `histogram` (objective value distribution).
//! - [`heatmap`]: `heatmap` (correlation heatmap).

mod hbar;
mod heatmap;
mod histogram;
mod line;
mod primitives;
mod scatter;

#[cfg(test)]
mod tests;

pub use hbar::{hbar_chart, HBarItem};
pub use heatmap::heatmap;
pub use histogram::{histogram, HistBin};
pub use line::{line_chart, LinePoint};
pub(crate) use primitives::escape_xml;
pub use scatter::{scatter_chart, ScatterPoint};

// The following are only needed so that `tests` (a descendant module) can
// resolve these names via `use super::*;` — they are not part of this
// module's own code.
#[cfg(test)]
use crate::report::theme;
#[cfg(test)]
use primitives::{fmt_sig4, nice_ticks, nice_ticks_integer, truncate_label};
