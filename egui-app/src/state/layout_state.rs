use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChartId {
    ParetoScatter2D,
    ParetoScatter3D,
    ParallelCoordinates,
    ScatterMatrix,
    ImportanceChart,
    PdpChart,
    OptimizationHistory,
    HvHistory,
    SensitivityHeatmap,
    ClusterScatter,
}

impl ChartId {
    pub fn all() -> &'static [ChartId] {
        &[
            ChartId::ParetoScatter2D,
            ChartId::ParetoScatter3D,
            ChartId::ParallelCoordinates,
            ChartId::ScatterMatrix,
            ChartId::ImportanceChart,
            ChartId::PdpChart,
            ChartId::OptimizationHistory,
            ChartId::HvHistory,
            ChartId::SensitivityHeatmap,
            ChartId::ClusterScatter,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            ChartId::ParetoScatter2D => "Pareto Scatter 2D",
            ChartId::ParetoScatter3D => "Pareto Scatter 3D",
            ChartId::ParallelCoordinates => "Parallel Coordinates",
            ChartId::ScatterMatrix => "Scatter Matrix",
            ChartId::ImportanceChart => "Importance Chart",
            ChartId::PdpChart => "PDP Chart",
            ChartId::OptimizationHistory => "Optimization History",
            ChartId::HvHistory => "Hypervolume History",
            ChartId::SensitivityHeatmap => "Sensitivity Heatmap",
            ChartId::ClusterScatter => "Cluster Scatter",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LayoutMode {
    MultiObjective,
    VariableSpace,
    ConvergenceAnalysis,
    FreeLayout,
}

#[derive(Debug, Clone)]
pub struct LayoutState {
    pub left_panel_width: f32,
    pub bottom_panel_height: f32,
    pub layout_mode: LayoutMode,
    pub visible_charts: HashSet<ChartId>,
}

impl LayoutState {
    pub fn toggle_chart(&mut self, id: ChartId) {
        if self.visible_charts.contains(&id) {
            self.visible_charts.remove(&id);
        } else {
            self.visible_charts.insert(id);
        }
    }

    pub fn is_chart_visible(&self, id: &ChartId) -> bool {
        self.visible_charts.contains(id)
    }
}

impl Default for LayoutState {
    fn default() -> Self {
        let mut visible = HashSet::new();
        visible.insert(ChartId::ParetoScatter2D);
        visible.insert(ChartId::ParallelCoordinates);
        visible.insert(ChartId::OptimizationHistory);
        Self {
            left_panel_width: 240.0,
            bottom_panel_height: 200.0,
            layout_mode: LayoutMode::MultiObjective,
            visible_charts: visible,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_state_default_values() {
        let layout = LayoutState::default();
        assert_eq!(layout.left_panel_width, 240.0);
        assert_eq!(layout.bottom_panel_height, 200.0);
        assert_eq!(layout.layout_mode, LayoutMode::MultiObjective);
        assert!(layout.visible_charts.contains(&ChartId::ParetoScatter2D));
        assert!(layout.visible_charts.contains(&ChartId::ParallelCoordinates));
        assert!(layout.visible_charts.contains(&ChartId::OptimizationHistory));
    }

    #[test]
    fn toggle_chart_adds_hidden_chart() {
        let mut layout = LayoutState::default();
        assert!(!layout.visible_charts.contains(&ChartId::HvHistory));
        layout.toggle_chart(ChartId::HvHistory);
        assert!(layout.visible_charts.contains(&ChartId::HvHistory));
    }

    #[test]
    fn toggle_chart_removes_visible_chart() {
        let mut layout = LayoutState::default();
        assert!(layout.visible_charts.contains(&ChartId::ParetoScatter2D));
        layout.toggle_chart(ChartId::ParetoScatter2D);
        assert!(!layout.visible_charts.contains(&ChartId::ParetoScatter2D));
    }

    #[test]
    fn toggle_chart_twice_restores_state() {
        let mut layout = LayoutState::default();
        let initial = layout.visible_charts.contains(&ChartId::ScatterMatrix);
        layout.toggle_chart(ChartId::ScatterMatrix);
        layout.toggle_chart(ChartId::ScatterMatrix);
        assert_eq!(layout.visible_charts.contains(&ChartId::ScatterMatrix), initial);
    }

    #[test]
    fn layout_mode_variants() {
        let mode = LayoutMode::FreeLayout;
        assert_ne!(mode, LayoutMode::MultiObjective);
        assert_ne!(mode, LayoutMode::VariableSpace);
    }
}
