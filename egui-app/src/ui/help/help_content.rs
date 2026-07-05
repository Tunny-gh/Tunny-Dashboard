use crate::state::layout_state::{ChartId, PanelItem};
use crate::ui::help::help_types::{HelpContent, HelpLanguage};

pub fn get_help_html(item: &PanelItem, lang: HelpLanguage) -> &'static str {
    get_help_content_inner(item).html(lang)
}

pub fn get_widget_name(item: &PanelItem) -> &'static str {
    get_help_content_inner(item).widget_name
}

fn get_help_content_inner(item: &PanelItem) -> HelpContent {
    match item {
        PanelItem::TrialTable => trial_table_help(),
        PanelItem::Chart(id) => chart_help(id),
    }
}

fn trial_table_help() -> HelpContent {
    HelpContent {
        widget_name: "trial-table",
        html_en: include_str!(concat!(
            env!("OUT_DIR"),
            "/help/en/widgets/trial-table.html"
        )),
        html_ja: include_str!(concat!(
            env!("OUT_DIR"),
            "/help/ja/widgets/trial-table.html"
        )),
    }
}

fn chart_help(id: &ChartId) -> HelpContent {
    match id {
        ChartId::ImportanceChart => HelpContent {
            widget_name: "importance-chart",
            html_en: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/en/sensitivity-analysis/overview.html"
            )),
            html_ja: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/ja/sensitivity-analysis/overview.html"
            )),
        },
        ChartId::SensitivityHeatmap => HelpContent {
            widget_name: "sensitivity-heatmap",
            html_en: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/en/sensitivity-analysis/overview.html"
            )),
            html_ja: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/ja/sensitivity-analysis/overview.html"
            )),
        },
        ChartId::McdmRankChart => HelpContent {
            widget_name: "mcdm-rank-chart",
            html_en: include_str!(concat!(env!("OUT_DIR"), "/help/en/mcdm/overview.html")),
            html_ja: include_str!(concat!(env!("OUT_DIR"), "/help/ja/mcdm/overview.html")),
        },
        ChartId::McdmScatterChart => HelpContent {
            widget_name: "mcdm-scatter-chart",
            html_en: include_str!(concat!(env!("OUT_DIR"), "/help/en/mcdm/overview.html")),
            html_ja: include_str!(concat!(env!("OUT_DIR"), "/help/ja/mcdm/overview.html")),
        },
        ChartId::ClusterScatter => HelpContent {
            widget_name: "cluster-scatter",
            html_en: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/en/clustering/overview.html"
            )),
            html_ja: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/ja/clustering/overview.html"
            )),
        },
        ChartId::PdpChart => HelpContent {
            widget_name: "pdp-chart",
            html_en: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/en/sensitivity-analysis/pdp.html"
            )),
            html_ja: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/ja/sensitivity-analysis/pdp.html"
            )),
        },
        ChartId::PdpChart2D => HelpContent {
            widget_name: "pdp-chart-2d",
            html_en: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/en/sensitivity-analysis/pdp.html"
            )),
            html_ja: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/ja/sensitivity-analysis/pdp.html"
            )),
        },
        ChartId::SliceChart => HelpContent {
            widget_name: "slice-chart",
            html_en: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/en/widgets/slice-chart.html"
            )),
            html_ja: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/ja/widgets/slice-chart.html"
            )),
        },
        ChartId::ParetoScatter2D => HelpContent {
            widget_name: "pareto-2d",
            html_en: include_str!(concat!(env!("OUT_DIR"), "/help/en/widgets/pareto-2d.html")),
            html_ja: include_str!(concat!(env!("OUT_DIR"), "/help/ja/widgets/pareto-2d.html")),
        },
        ChartId::ParetoScatter3D => HelpContent {
            widget_name: "pareto-3d",
            html_en: include_str!(concat!(env!("OUT_DIR"), "/help/en/widgets/pareto-3d.html")),
            html_ja: include_str!(concat!(env!("OUT_DIR"), "/help/ja/widgets/pareto-3d.html")),
        },
        ChartId::ParallelCoordinates => HelpContent {
            widget_name: "parallel-coords",
            html_en: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/en/widgets/parallel-coords.html"
            )),
            html_ja: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/ja/widgets/parallel-coords.html"
            )),
        },
        ChartId::ScatterMatrix => HelpContent {
            widget_name: "scatter-matrix",
            html_en: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/en/widgets/scatter-matrix.html"
            )),
            html_ja: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/ja/widgets/scatter-matrix.html"
            )),
        },
        ChartId::OptimizationHistory => HelpContent {
            widget_name: "optimization-history",
            html_en: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/en/widgets/optimization-history.html"
            )),
            html_ja: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/ja/widgets/optimization-history.html"
            )),
        },
        ChartId::ConvergenceIndicators => HelpContent {
            widget_name: "convergence",
            html_en: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/en/widgets/convergence.html"
            )),
            html_ja: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/ja/widgets/convergence.html"
            )),
        },
        ChartId::ObservedContour => HelpContent {
            widget_name: "observed-contour",
            html_en: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/en/widgets/observed-contour.html"
            )),
            html_ja: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/ja/widgets/observed-contour.html"
            )),
        },
        ChartId::SurrogateOpt => HelpContent {
            widget_name: "surrogate-optimizer",
            html_en: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/en/widgets/surrogate-optimizer.html"
            )),
            html_ja: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/ja/widgets/surrogate-optimizer.html"
            )),
        },
        ChartId::Robustness => HelpContent {
            widget_name: "robustness",
            html_en: include_str!(concat!(env!("OUT_DIR"), "/help/en/widgets/robustness.html")),
            html_ja: include_str!(concat!(env!("OUT_DIR"), "/help/ja/widgets/robustness.html")),
        },
        ChartId::ClusterScatter3D => HelpContent {
            widget_name: "cluster-scatter-3d",
            html_en: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/en/clustering/overview.html"
            )),
            html_ja: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/ja/clustering/overview.html"
            )),
        },
        ChartId::McdmScatterChart3D => HelpContent {
            widget_name: "mcdm-scatter-chart-3d",
            html_en: include_str!(concat!(env!("OUT_DIR"), "/help/en/mcdm/overview.html")),
            html_ja: include_str!(concat!(env!("OUT_DIR"), "/help/ja/mcdm/overview.html")),
        },
        ChartId::ArtifactGallery => HelpContent {
            widget_name: "artifact-gallery",
            html_en: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/en/widgets/artifact-gallery.html"
            )),
            html_ja: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/ja/widgets/artifact-gallery.html"
            )),
        },
        ChartId::Histogram => HelpContent {
            widget_name: "histogram",
            html_en: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/en/statistics/histogram.html"
            )),
            html_ja: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/ja/statistics/histogram.html"
            )),
        },
        ChartId::BoxPlot => HelpContent {
            widget_name: "box-plot",
            html_en: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/en/statistics/box-plot.html"
            )),
            html_ja: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/ja/statistics/box-plot.html"
            )),
        },
        ChartId::CorrelationMatrix => HelpContent {
            widget_name: "correlation-matrix",
            html_en: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/en/statistics/correlation-matrix.html"
            )),
            html_ja: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/ja/statistics/correlation-matrix.html"
            )),
        },
        ChartId::RadarComparison => HelpContent {
            widget_name: "radar-comparison",
            html_en: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/en/widgets/radar-comparison.html"
            )),
            html_ja: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/ja/widgets/radar-comparison.html"
            )),
        },
        ChartId::ComparisonTable => HelpContent {
            widget_name: "comparison-table",
            html_en: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/en/widgets/comparison-table.html"
            )),
            html_ja: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/ja/widgets/comparison-table.html"
            )),
        },
        ChartId::PcaBiplot => HelpContent {
            widget_name: "pca-biplot",
            html_en: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/en/clustering/pca-biplot.html"
            )),
            html_ja: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/ja/clustering/pca-biplot.html"
            )),
        },
        ChartId::SomMap => HelpContent {
            widget_name: "som-map",
            html_en: include_str!(concat!(env!("OUT_DIR"), "/help/en/clustering/som.html")),
            html_ja: include_str!(concat!(env!("OUT_DIR"), "/help/ja/clustering/som.html")),
        },
        ChartId::Dendrogram => HelpContent {
            widget_name: "dendrogram",
            html_en: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/en/clustering/hierarchical.html"
            )),
            html_ja: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/ja/clustering/hierarchical.html"
            )),
        },
        ChartId::ResponseSurface3D => HelpContent {
            widget_name: "response-surface-3d",
            html_en: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/en/widgets/response-surface-3d.html"
            )),
            html_ja: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/ja/widgets/response-surface-3d.html"
            )),
        },
        ChartId::IntermediateValues => HelpContent {
            widget_name: "intermediate-values",
            html_en: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/en/widgets/intermediate-values.html"
            )),
            html_ja: include_str!(concat!(
                env!("OUT_DIR"),
                "/help/ja/widgets/intermediate-values.html"
            )),
        },
        ChartId::Timeline => HelpContent {
            widget_name: "timeline",
            html_en: include_str!(concat!(env!("OUT_DIR"), "/help/en/widgets/timeline.html")),
            html_ja: include_str!(concat!(env!("OUT_DIR"), "/help/ja/widgets/timeline.html")),
        },
        ChartId::EdfPlot => HelpContent {
            widget_name: "edf-plot",
            html_en: include_str!(concat!(env!("OUT_DIR"), "/help/en/widgets/edf-plot.html")),
            html_ja: include_str!(concat!(env!("OUT_DIR"), "/help/ja/widgets/edf-plot.html")),
        },
        ChartId::RankPlot => HelpContent {
            widget_name: "rank-plot",
            html_en: include_str!(concat!(env!("OUT_DIR"), "/help/en/widgets/rank-plot.html")),
            html_ja: include_str!(concat!(env!("OUT_DIR"), "/help/ja/widgets/rank-plot.html")),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_panel_items_return_non_empty_html_en() {
        let items: Vec<PanelItem> = ChartId::all()
            .iter()
            .map(|id| PanelItem::Chart(id.clone()))
            .chain(std::iter::once(PanelItem::TrialTable))
            .collect();

        for item in &items {
            let html = get_help_html(item, HelpLanguage::En);
            assert!(!html.is_empty(), "EN html empty for {item:?}");
        }
    }

    #[test]
    fn all_panel_items_return_non_empty_html_ja() {
        let items: Vec<PanelItem> = ChartId::all()
            .iter()
            .map(|id| PanelItem::Chart(id.clone()))
            .chain(std::iter::once(PanelItem::TrialTable))
            .collect();

        for item in &items {
            let html = get_help_html(item, HelpLanguage::Ja);
            assert!(!html.is_empty(), "JA html empty for {item:?}");
        }
    }

    #[test]
    fn trial_table_en_and_ja_differ() {
        let en = get_help_html(&PanelItem::TrialTable, HelpLanguage::En);
        let ja = get_help_html(&PanelItem::TrialTable, HelpLanguage::Ja);
        assert_ne!(en, ja, "EN and JA html should be different for TrialTable");
    }
}
