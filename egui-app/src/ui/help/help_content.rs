use crate::state::layout_state::{ChartId, PanelItem};
use crate::ui::help::help_types::{HelpContent, HelpTabDef};

/// Returns help content for a given panel item.
pub fn get_help_content(item: &PanelItem) -> HelpContent {
    match item {
        PanelItem::Chart(id) => chart_help(id),
        PanelItem::TrialTable => HelpContent {
            title: "Trial Table",
            tabs: &[HelpTabDef {
                label: "Usage Guide",
                markdown: include_str!("../../../../theory/en/widgets/trial-table.md"),
            }],
        },
    }
}

fn chart_help(id: &ChartId) -> HelpContent {
    match id {
        ChartId::ImportanceChart => HelpContent {
            title: "Importance Chart",
            tabs: &[
                HelpTabDef {
                    label: "Overview",
                    markdown: include_str!("../../../../theory/en/sensitivity-analysis/overview.md"),
                },
                HelpTabDef {
                    label: "Spearman",
                    markdown: include_str!("../../../../theory/en/sensitivity-analysis/spearman.md"),
                },
                HelpTabDef {
                    label: "Ridge",
                    markdown: include_str!("../../../../theory/en/sensitivity-analysis/ridge.md"),
                },
                HelpTabDef {
                    label: "Sobol",
                    markdown: include_str!("../../../../theory/en/sensitivity-analysis/sobol.md"),
                },
                HelpTabDef {
                    label: "MDI",
                    markdown: include_str!("../../../../theory/en/sensitivity-analysis/mdi.md"),
                },
                HelpTabDef {
                    label: "RF-ANOVA",
                    markdown: include_str!("../../../../theory/en/sensitivity-analysis/rfanova.md"),
                },
                HelpTabDef {
                    label: "Permutation",
                    markdown: include_str!(
                        "../../../../theory/en/sensitivity-analysis/permutation.md"
                    ),
                },
                HelpTabDef {
                    label: "SHAP",
                    markdown: include_str!("../../../../theory/en/sensitivity-analysis/shap.md"),
                },
            ],
        },
        ChartId::SensitivityHeatmap => HelpContent {
            title: "Sensitivity Heatmap",
            tabs: &[
                HelpTabDef {
                    label: "Overview",
                    markdown: include_str!("../../../../theory/en/sensitivity-analysis/overview.md"),
                },
                HelpTabDef {
                    label: "Spearman",
                    markdown: include_str!("../../../../theory/en/sensitivity-analysis/spearman.md"),
                },
                HelpTabDef {
                    label: "Ridge",
                    markdown: include_str!("../../../../theory/en/sensitivity-analysis/ridge.md"),
                },
                HelpTabDef {
                    label: "Sobol",
                    markdown: include_str!("../../../../theory/en/sensitivity-analysis/sobol.md"),
                },
            ],
        },
        ChartId::McdmRankChart => HelpContent {
            title: "MCDM Ranking",
            tabs: &[
                HelpTabDef {
                    label: "Overview",
                    markdown: include_str!("../../../../theory/en/mcdm/overview.md"),
                },
                HelpTabDef {
                    label: "TOPSIS",
                    markdown: include_str!("../../../../theory/en/mcdm/topsis.md"),
                },
                HelpTabDef {
                    label: "VIKOR",
                    markdown: include_str!("../../../../theory/en/mcdm/vikor.md"),
                },
                HelpTabDef {
                    label: "PROMETHEE",
                    markdown: include_str!("../../../../theory/en/mcdm/promethee.md"),
                },
            ],
        },
        ChartId::McdmScatterChart => HelpContent {
            title: "MCDM Scatter Chart",
            tabs: &[
                HelpTabDef {
                    label: "Overview",
                    markdown: include_str!("../../../../theory/en/mcdm/overview.md"),
                },
                HelpTabDef {
                    label: "TOPSIS",
                    markdown: include_str!("../../../../theory/en/mcdm/topsis.md"),
                },
                HelpTabDef {
                    label: "VIKOR",
                    markdown: include_str!("../../../../theory/en/mcdm/vikor.md"),
                },
                HelpTabDef {
                    label: "PROMETHEE",
                    markdown: include_str!("../../../../theory/en/mcdm/promethee.md"),
                },
            ],
        },
        ChartId::McdmTable => HelpContent {
            title: "MCDM Table",
            tabs: &[
                HelpTabDef {
                    label: "Overview",
                    markdown: include_str!("../../../../theory/en/mcdm/overview.md"),
                },
                HelpTabDef {
                    label: "TOPSIS",
                    markdown: include_str!("../../../../theory/en/mcdm/topsis.md"),
                },
                HelpTabDef {
                    label: "VIKOR",
                    markdown: include_str!("../../../../theory/en/mcdm/vikor.md"),
                },
                HelpTabDef {
                    label: "PROMETHEE",
                    markdown: include_str!("../../../../theory/en/mcdm/promethee.md"),
                },
                HelpTabDef {
                    label: "Entropy Weight",
                    markdown: include_str!("../../../../theory/en/mcdm/entropy-weight.md"),
                },
            ],
        },
        ChartId::AhpRankChart => HelpContent {
            title: "AHP Ranking",
            tabs: &[
                HelpTabDef {
                    label: "Overview",
                    markdown: include_str!("../../../../theory/en/mcdm/overview.md"),
                },
                HelpTabDef {
                    label: "AHP",
                    markdown: include_str!("../../../../theory/en/mcdm/ahp.md"),
                },
            ],
        },
        ChartId::AhpTable => HelpContent {
            title: "AHP Table",
            tabs: &[
                HelpTabDef {
                    label: "Overview",
                    markdown: include_str!("../../../../theory/en/mcdm/overview.md"),
                },
                HelpTabDef {
                    label: "AHP",
                    markdown: include_str!("../../../../theory/en/mcdm/ahp.md"),
                },
            ],
        },
        ChartId::ClusterScatter => HelpContent {
            title: "Cluster Scatter",
            tabs: &[
                HelpTabDef {
                    label: "Overview",
                    markdown: include_str!("../../../../theory/en/clustering/overview.md"),
                },
                HelpTabDef {
                    label: "k-means",
                    markdown: include_str!("../../../../theory/en/clustering/kmeans.md"),
                },
                HelpTabDef {
                    label: "Elbow",
                    markdown: include_str!("../../../../theory/en/clustering/elbow.md"),
                },
            ],
        },
        ChartId::PdpChart => HelpContent {
            title: "PDP Chart",
            tabs: &[
                HelpTabDef {
                    label: "Overview",
                    markdown: include_str!("../../../../theory/en/sensitivity-analysis/pdp.md"),
                },
                HelpTabDef {
                    label: "Ridge",
                    markdown: include_str!("../../../../theory/en/surrogate-models/ridge.md"),
                },
                HelpTabDef {
                    label: "Random Forest",
                    markdown: include_str!("../../../../theory/en/surrogate-models/random-forest.md"),
                },
                HelpTabDef {
                    label: "Kriging",
                    markdown: include_str!("../../../../theory/en/surrogate-models/kriging.md"),
                },
                HelpTabDef {
                    label: "Sparse Kriging",
                    markdown: include_str!(
                        "../../../../theory/en/surrogate-models/sparse-kriging.md"
                    ),
                },
            ],
        },
        ChartId::PdpChart2D => HelpContent {
            title: "PDP Chart 2D",
            tabs: &[
                HelpTabDef {
                    label: "Overview",
                    markdown: include_str!("../../../../theory/en/sensitivity-analysis/pdp.md"),
                },
                HelpTabDef {
                    label: "Ridge",
                    markdown: include_str!("../../../../theory/en/surrogate-models/ridge.md"),
                },
                HelpTabDef {
                    label: "Random Forest",
                    markdown: include_str!("../../../../theory/en/surrogate-models/random-forest.md"),
                },
                HelpTabDef {
                    label: "Kriging",
                    markdown: include_str!("../../../../theory/en/surrogate-models/kriging.md"),
                },
                HelpTabDef {
                    label: "Sparse Kriging",
                    markdown: include_str!(
                        "../../../../theory/en/surrogate-models/sparse-kriging.md"
                    ),
                },
            ],
        },
        ChartId::SliceChart => HelpContent {
            title: "Slice Chart",
            tabs: &[
                HelpTabDef {
                    label: "Usage Guide",
                    markdown: include_str!("../../../../theory/en/widgets/slice-chart.md"),
                },
                HelpTabDef {
                    label: "L-BFGS",
                    markdown: include_str!("../../../../theory/en/optimization/lbfgs.md"),
                },
            ],
        },
        ChartId::ParetoScatter2D => HelpContent {
            title: "Pareto Scatter 2D",
            tabs: &[HelpTabDef {
                label: "Usage Guide",
                markdown: include_str!("../../../../theory/en/widgets/pareto-2d.md"),
            }],
        },
        ChartId::ParetoScatter3D => HelpContent {
            title: "Pareto Scatter 3D",
            tabs: &[HelpTabDef {
                label: "Usage Guide",
                markdown: include_str!("../../../../theory/en/widgets/pareto-3d.md"),
            }],
        },
        ChartId::ParallelCoordinates => HelpContent {
            title: "Parallel Coordinates",
            tabs: &[HelpTabDef {
                label: "Usage Guide",
                markdown: include_str!("../../../../theory/en/widgets/parallel-coords.md"),
            }],
        },
        ChartId::ScatterMatrix => HelpContent {
            title: "Scatter Matrix",
            tabs: &[HelpTabDef {
                label: "Usage Guide",
                markdown: include_str!("../../../../theory/en/widgets/scatter-matrix.md"),
            }],
        },
        ChartId::OptimizationHistory => HelpContent {
            title: "Optimization History",
            tabs: &[HelpTabDef {
                label: "Usage Guide",
                markdown: include_str!("../../../../theory/en/widgets/optimization-history.md"),
            }],
        },
        ChartId::HvHistory => HelpContent {
            title: "Hypervolume History",
            tabs: &[HelpTabDef {
                label: "Usage Guide",
                markdown: include_str!("../../../../theory/en/widgets/hv-history.md"),
            }],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::help::help_types::HelpModalState;

    #[test]
    fn all_panel_items_return_non_empty_help_content() {
        let items: Vec<PanelItem> = ChartId::all()
            .iter()
            .map(|id| PanelItem::Chart(id.clone()))
            .chain(std::iter::once(PanelItem::TrialTable))
            .collect();

        for item in &items {
            let content = get_help_content(item);
            assert!(!content.title.is_empty(), "title empty for {item:?}");
            assert!(!content.tabs.is_empty(), "no tabs for {item:?}");
            for tab in content.tabs {
                assert!(!tab.label.is_empty(), "empty label in {item:?}");
                assert!(!tab.markdown.is_empty(), "empty markdown in {item:?} tab '{}'", tab.label);
                assert!(
                    !tab.markdown.contains("TODO: English content pending."),
                    "placeholder found in {item:?} tab '{}'",
                    tab.label
                );
            }
        }
    }

    #[test]
    fn help_modal_state_default_is_closed() {
        let state = HelpModalState::default();
        assert!(!state.open);
        assert_eq!(state.active_tab, 0);
        assert!(state.item.is_none());
    }
}
