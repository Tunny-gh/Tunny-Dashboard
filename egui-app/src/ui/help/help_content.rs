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
        ChartId::HvHistory => HelpContent {
            widget_name: "hv-history",
            html_en: include_str!(concat!(env!("OUT_DIR"), "/help/en/widgets/hv-history.html")),
            html_ja: include_str!(concat!(env!("OUT_DIR"), "/help/ja/widgets/hv-history.html")),
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
