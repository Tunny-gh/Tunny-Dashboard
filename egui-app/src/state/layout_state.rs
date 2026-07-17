#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ChartId {
    ParetoScatter2D,
    ParetoScatter3D,
    ParallelCoordinates,
    ScatterMatrix,
    ImportanceChart,
    PdpChart,
    PdpChart2D,
    OptimizationHistory,
    ConvergenceIndicators,
    SensitivityHeatmap,
    ClusterScatter,
    ClusterScatter3D,
    McdmRankChart,
    McdmScatterChart,
    McdmScatterChart3D,
    SliceChart,
    ObservedContour,
    SurrogateOpt,
    Robustness,
    Histogram,
    BoxPlot,
    CorrelationMatrix,
    ArtifactGallery,
    RadarComparison,
    ComparisonTable,
    PcaBiplot,
    SomMap,
    Dendrogram,
    ResponseSurface3D,
    IntermediateValues,
    Timeline,
    EdfPlot,
    RankPlot,
    SurrogateCompare,
}

impl ChartId {
    /// Enumeration of all charts. Used only by help/panel exhaustiveness tests.
    #[cfg(test)]
    pub fn all() -> &'static [ChartId] {
        &[
            ChartId::ParetoScatter2D,
            ChartId::ParetoScatter3D,
            ChartId::ParallelCoordinates,
            ChartId::ScatterMatrix,
            ChartId::ImportanceChart,
            ChartId::PdpChart,
            ChartId::PdpChart2D,
            ChartId::OptimizationHistory,
            ChartId::ConvergenceIndicators,
            ChartId::SensitivityHeatmap,
            ChartId::ClusterScatter,
            ChartId::ClusterScatter3D,
            ChartId::McdmRankChart,
            ChartId::McdmScatterChart,
            ChartId::McdmScatterChart3D,
            ChartId::SliceChart,
            ChartId::ObservedContour,
            ChartId::SurrogateOpt,
            ChartId::Robustness,
            ChartId::Histogram,
            ChartId::BoxPlot,
            ChartId::CorrelationMatrix,
            ChartId::ArtifactGallery,
            ChartId::RadarComparison,
            ChartId::ComparisonTable,
            ChartId::PcaBiplot,
            ChartId::SomMap,
            ChartId::Dendrogram,
            ChartId::ResponseSurface3D,
            ChartId::IntermediateValues,
            ChartId::Timeline,
            ChartId::EdfPlot,
            ChartId::RankPlot,
            ChartId::SurrogateCompare,
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
            ChartId::PdpChart2D => "PDP Chart 2D",
            ChartId::OptimizationHistory => "Optimization History",
            ChartId::ConvergenceIndicators => "Convergence Indicators",
            ChartId::SensitivityHeatmap => "Sensitivity Heatmap",
            ChartId::ClusterScatter => "Cluster Scatter 2D",
            ChartId::ClusterScatter3D => "Cluster Scatter 3D",
            ChartId::McdmRankChart => "MCDM Ranking",
            ChartId::McdmScatterChart => "MCDM Scatter Chart 2D",
            ChartId::McdmScatterChart3D => "MCDM Scatter Chart 3D",
            ChartId::SliceChart => "Slice Chart",
            ChartId::ObservedContour => "Observed Contour",
            ChartId::SurrogateOpt => "Surrogate Optimizer",
            ChartId::Robustness => "Robustness",
            ChartId::Histogram => "Histogram",
            ChartId::BoxPlot => "Box Plot",
            ChartId::CorrelationMatrix => "Correlation Matrix",
            ChartId::ArtifactGallery => "Artifact Gallery",
            ChartId::RadarComparison => "Radar Comparison",
            ChartId::ComparisonTable => "Comparison Table",
            ChartId::PcaBiplot => "PCA Biplot",
            ChartId::SomMap => "SOM Map",
            ChartId::Dendrogram => "Dendrogram",
            ChartId::ResponseSurface3D => "Response Surface 3D",
            ChartId::IntermediateValues => "Intermediate Values",
            ChartId::Timeline => "Timeline",
            ChartId::EdfPlot => "EDF",
            ChartId::RankPlot => "Rank Plot",
            ChartId::SurrogateCompare => "Compare Surrogates",
        }
    }

    /// Supplementary description (legend) shown next to the chart title. `None` if
    /// there isn't one.
    pub fn subtitle(&self) -> Option<&'static str> {
        match self {
            ChartId::ScatterMatrix => Some(
                "Lower-left: scatter plots / Upper-right: Pearson correlation / Diagonal: histograms",
            ),
            ChartId::Robustness => Some("MC noise propagation"),
            ChartId::RadarComparison => Some("Pinned-trial profiles"),
            ChartId::ComparisonTable => Some("Pinned trials side by side"),
            ChartId::PcaBiplot => Some("Scores + loadings"),
            ChartId::SomMap => Some("Topology-preserving map"),
            ChartId::ResponseSurface3D => Some("Surrogate slice viewer"),
            ChartId::IntermediateValues => Some("Learning curves per trial"),
            ChartId::Timeline => Some("Trial execution timeline"),
            ChartId::EdfPlot => Some("Empirical distribution of objective values"),
            ChartId::RankPlot => Some("Param pairs colored by objective rank"),
            ChartId::SurrogateCompare => Some("CV metrics & prediction overlay"),
            _ => None,
        }
    }
}

// ----------------------------------------
// PanelItem — a unified type for widgets that can be placed on the canvas
// 🔵 From user interviews (making charts and tables D&D targets)
// ----------------------------------------
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PanelItem {
    /// Wraps an existing chart
    Chart(ChartId),
    /// The trial list table, made into a widget from the old BottomPanel
    TrialTable,
}

impl PanelItem {
    /// Display label (used in the right panel list, cell headers, etc.)
    pub fn label(&self) -> &'static str {
        match self {
            PanelItem::Chart(id) => id.label(),
            PanelItem::TrialTable => "Trial Table",
        }
    }

    /// Supplementary description (legend) shown next to the cell title. `None` if
    /// there isn't one.
    pub fn subtitle(&self) -> Option<&'static str> {
        match self {
            PanelItem::Chart(id) => id.subtitle(),
            PanelItem::TrialTable => None,
        }
    }

    /// List of all available items (in the order shown in the right panel). Used
    /// only by exhaustiveness tests.
    #[cfg(test)]
    pub fn all() -> Vec<PanelItem> {
        let mut items: Vec<PanelItem> = ChartId::all()
            .iter()
            .map(|id| PanelItem::Chart(id.clone()))
            .collect();
        items.push(PanelItem::TrialTable);
        items
    }
}

// ----------------------------------------
// DragPayload — a unified type for D&D payloads
// 🔵 From user interviews (D&D move) + existing PanelItem D&D
// ----------------------------------------
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DragPayload {
    /// New placement from the right panel
    NewWidget(PanelItem),
}

impl DragPayload {
    /// Extracts the PanelItem from the payload
    pub fn item(&self) -> &PanelItem {
        match self {
            DragPayload::NewWidget(item) => item,
        }
    }
}

// ----------------------------------------
// RightPanelState — the right panel's open/closed and size state
// 🔵 From user interviews (hamburger menu)
// ----------------------------------------
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RightPanelState {
    /// Open/closed state, toggled by the hamburger menu
    pub is_open: bool,
    /// Panel width (resizable by drag)
    pub width: f32,
    /// The X coordinate of the panel's left edge as actually drawn in the most
    /// recent frame.
    /// Icon tiles can be drawn wider than `width` and get shifted left by egui's
    /// constrain, so the hover-close check uses this measured value rather than the
    /// configured value. `None` before drawing (not yet set).
    #[serde(skip)]
    pub last_rendered_left_x: Option<f32>,
}

impl Default for RightPanelState {
    fn default() -> Self {
        Self {
            is_open: false,
            width: 200.0,
            last_rendered_left_x: None,
        }
    }
}

// ----------------------------------------
// CanvasItem — a single widget freely placed on the canvas
// Has a unique ID so the same widget can be placed multiple times.
// Coordinates and size are in world coordinates (values on an infinite plane).
// ----------------------------------------
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CanvasItem {
    /// Unique instance ID (supports multiple placements)
    pub id: u64,
    /// The placed widget
    pub content: PanelItem,
    /// Top-left position in world coordinates
    pub x: f32,
    pub y: f32,
    /// Size in world coordinates
    pub w: f32,
    pub h: f32,
}

// ----------------------------------------
// CanvasLayout — state of the freely-placed canvas
// The order of items is the z-order (last is frontmost). pan/zoom is the viewport
// transform.
// ----------------------------------------
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CanvasLayout {
    pub items: Vec<CanvasItem>,
    /// Next ID to assign
    pub next_id: u64,
    /// Translation of the viewport transform (screen-coordinate offset)
    pub pan_x: f32,
    pub pan_y: f32,
    /// Scale of the viewport transform (default 1.0)
    pub zoom: f32,
}

impl Default for CanvasLayout {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            next_id: 0,
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: 1.0,
        }
    }
}

impl CanvasLayout {
    /// Adds a new item at world coordinates `(x, y)`. Assigns and returns a unique ID.
    pub fn add(&mut self, content: PanelItem, x: f32, y: f32, w: f32, h: f32) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.items.push(CanvasItem {
            id,
            content,
            x,
            y,
            w,
            h,
        });
        id
    }

    /// Removes the item with the given ID.
    pub fn remove(&mut self, id: u64) {
        self.items.retain(|it| it.id != id);
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LayoutState {
    pub left_panel_width: f32,
    /// Layout of the freely-placed canvas
    #[serde(default)]
    pub canvas: CanvasLayout,
    /// State of the right panel
    pub right_panel: RightPanelState,
    /// Open/closed state of the left panel (auto-controlled by hover)
    #[serde(default)]
    pub left_panel_open: bool,
}

impl Default for LayoutState {
    fn default() -> Self {
        Self {
            left_panel_width: 240.0,
            canvas: CanvasLayout::default(),
            right_panel: RightPanelState::default(),
            left_panel_open: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- PanelItem tests ---

    // --- ChartId::SurrogateOpt tests ---

    #[test]
    fn chart_id_all_contains_surrogate_opt() {
        let all = ChartId::all();
        assert!(all.contains(&ChartId::SurrogateOpt));
    }

    #[test]
    fn surrogate_opt_label_is_correct() {
        assert_eq!(ChartId::SurrogateOpt.label(), "Surrogate Optimizer");
    }

    // --- PanelItem tests ---

    #[test]
    fn panel_item_all_includes_all_charts_and_trial_table() {
        let items = PanelItem::all();
        // ChartId::all() + TrialTable
        assert_eq!(items.len(), ChartId::all().len() + 1);
        assert_eq!(items.last(), Some(&PanelItem::TrialTable));
    }

    #[test]
    fn panel_item_label_returns_chart_label() {
        let item = PanelItem::Chart(ChartId::ParetoScatter2D);
        assert_eq!(item.label(), "Pareto Scatter 2D");
    }

    #[test]
    fn panel_item_label_returns_trial_table() {
        let item = PanelItem::TrialTable;
        assert_eq!(item.label(), "Trial Table");
    }

    #[test]
    fn panel_item_derives_clone_eq_hash() {
        use std::collections::HashSet;
        let a = PanelItem::TrialTable;
        let b = a.clone();
        assert_eq!(a, b);
        let mut set = HashSet::new();
        set.insert(a);
        assert!(set.contains(&PanelItem::TrialTable));
    }

    // --- DragPayload tests ---

    #[test]
    fn drag_payload_new_widget_returns_item() {
        let item = PanelItem::Chart(ChartId::ParetoScatter2D);
        let payload = DragPayload::NewWidget(item.clone());
        assert_eq!(payload.item(), &item);
    }

    #[test]
    fn drag_payload_clone_works() {
        let a = DragPayload::NewWidget(PanelItem::TrialTable);
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn drag_payload_partial_eq_works() {
        let a = DragPayload::NewWidget(PanelItem::Chart(ChartId::OptimizationHistory));
        let b = DragPayload::NewWidget(PanelItem::Chart(ChartId::OptimizationHistory));
        assert_eq!(a, b);
    }

    #[test]
    fn drag_payload_hash_works_in_set() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(DragPayload::NewWidget(PanelItem::TrialTable));
        assert!(set.contains(&DragPayload::NewWidget(PanelItem::TrialTable)));
    }

    // --- LayoutState tests ---

    #[test]
    fn layout_state_default_values() {
        let layout = LayoutState::default();
        assert_eq!(layout.left_panel_width, 240.0);
        assert!(!layout.right_panel.is_open); // hover-reveal: default closed
        assert_eq!(layout.right_panel.width, 200.0);
    }

    #[test]
    fn right_panel_state_default() {
        let rp = RightPanelState::default();
        assert!(!rp.is_open); // hover-reveal: default closed
        assert_eq!(rp.width, 200.0);
    }

    // --- CanvasLayout tests ---

    #[test]
    fn canvas_layout_default_is_empty_and_identity() {
        let c = CanvasLayout::default();
        assert!(c.items.is_empty());
        assert_eq!(c.next_id, 0);
        assert_eq!(c.pan_x, 0.0);
        assert_eq!(c.pan_y, 0.0);
        assert_eq!(c.zoom, 1.0);
    }

    #[test]
    fn canvas_layout_add_assigns_incrementing_ids() {
        let mut c = CanvasLayout::default();
        let id0 = c.add(PanelItem::TrialTable, 10.0, 20.0, 360.0, 280.0);
        let id1 = c.add(
            PanelItem::Chart(ChartId::ParetoScatter2D),
            30.0,
            40.0,
            360.0,
            280.0,
        );
        assert_eq!(id0, 0);
        assert_eq!(id1, 1);
        assert_eq!(c.items.len(), 2);
        assert_eq!(c.items[0].x, 10.0);
        assert_eq!(c.items[0].y, 20.0);
    }

    #[test]
    fn canvas_layout_allows_duplicate_content() {
        let mut c = CanvasLayout::default();
        c.add(PanelItem::TrialTable, 0.0, 0.0, 100.0, 100.0);
        c.add(PanelItem::TrialTable, 50.0, 50.0, 100.0, 100.0);
        assert_eq!(c.items.len(), 2);
        assert_ne!(c.items[0].id, c.items[1].id);
    }

    #[test]
    fn canvas_layout_remove_by_id() {
        let mut c = CanvasLayout::default();
        let id0 = c.add(PanelItem::TrialTable, 0.0, 0.0, 100.0, 100.0);
        let id1 = c.add(PanelItem::TrialTable, 0.0, 0.0, 100.0, 100.0);
        c.remove(id0);
        assert_eq!(c.items.len(), 1);
        assert_eq!(c.items[0].id, id1);
    }
}
