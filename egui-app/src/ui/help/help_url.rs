use crate::state::layout_state::{ChartId, PanelItem};

/// Base URL of the online documentation site.
const DOCS_BASE: &str = "https://tunny.hrntsm.com";

/// Documentation channel. `latest` always resolves to the newest published version.
const DOCS_VERSION: &str = "latest";

/// Builds the URL of the documentation top page (Overview).
///
/// Links always point at the English pages; the documentation site carries its
/// own language switcher, so the app does not track a help language.
pub fn overview_url() -> String {
    format!("{DOCS_BASE}/dashboard/{DOCS_VERSION}/overview")
}

/// Builds the URL of the documentation page for a widget.
pub fn widget_url(item: &PanelItem) -> String {
    format!(
        "{DOCS_BASE}/dashboard/{DOCS_VERSION}/widgets/{slug}",
        slug = doc_slug(item)
    )
}

/// Maps a panel item to the page slug used under `/dashboard/<version>/widgets/`.
///
/// Some widgets have no page of their own and point at the closest one: the 3D
/// variants share their 2D counterpart's page, and PDP 2D shares the PDP page.
pub fn doc_slug(item: &PanelItem) -> &'static str {
    match item {
        PanelItem::TrialTable => "trial-table",
        PanelItem::Chart(id) => chart_slug(id),
    }
}

fn chart_slug(id: &ChartId) -> &'static str {
    match id {
        ChartId::OptimizationHistory => "optimization-history",
        ChartId::ConvergenceIndicators => "convergence",
        ChartId::IntermediateValues => "intermediate-values",
        ChartId::Timeline => "timeline",
        ChartId::EdfPlot => "edf-plot",
        ChartId::ParetoScatter2D => "pareto-2d",
        ChartId::ParetoScatter3D => "pareto-3d",
        ChartId::ParallelCoordinates => "parallel-coords",
        ChartId::ImportanceChart => "importance-chart",
        ChartId::SensitivityHeatmap => "sensitivity-heatmap",
        ChartId::ScatterMatrix => "scatter-matrix",
        ChartId::SliceChart => "slice-chart",
        ChartId::ObservedContour => "observed-contour",
        ChartId::RankPlot => "rank-plot",
        ChartId::Histogram => "histogram",
        ChartId::BoxPlot => "box-plot",
        ChartId::CorrelationMatrix => "correlation-matrix",
        ChartId::PdpChart => "pdp-chart",
        ChartId::PdpChart2D => "pdp-chart",
        ChartId::ResponseSurface3D => "response-surface-3d",
        ChartId::SurrogateCompare => "compare-surrogates",
        ChartId::SurrogateOpt => "surrogate-optimizer",
        ChartId::Robustness => "robustness",
        ChartId::ClusterScatter => "cluster-scatter",
        ChartId::ClusterScatter3D => "cluster-scatter",
        ChartId::PcaBiplot => "pca-biplot",
        ChartId::SomMap => "som-map",
        ChartId::Dendrogram => "dendrogram",
        ChartId::McdmRankChart => "mcdm-ranking",
        ChartId::McdmScatterChart => "mcdm-scatter",
        ChartId::McdmScatterChart3D => "mcdm-scatter",
        ChartId::RadarComparison => "radar-comparison",
        ChartId::ComparisonTable => "comparison-table",
        ChartId::ArtifactGallery => "artifact-gallery",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widget_url_points_at_the_widget_page() {
        assert_eq!(
            widget_url(&PanelItem::TrialTable),
            "https://tunny.hrntsm.com/dashboard/latest/widgets/trial-table"
        );
    }

    #[test]
    fn overview_url_points_at_the_documentation_top_page() {
        assert_eq!(
            overview_url(),
            "https://tunny.hrntsm.com/dashboard/latest/overview"
        );
    }

    #[test]
    fn every_chart_has_a_slug_shaped_like_a_url_path_segment() {
        for id in ChartId::all() {
            let slug = chart_slug(id);
            assert!(!slug.is_empty(), "empty slug for {id:?}");
            assert!(
                slug.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "slug {slug:?} for {id:?} is not a lowercase kebab-case path segment"
            );
        }
    }
}
