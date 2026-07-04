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
    Histogram,
    BoxPlot,
    CorrelationMatrix,
    ArtifactGallery,
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
            ChartId::Histogram,
            ChartId::BoxPlot,
            ChartId::CorrelationMatrix,
            ChartId::ArtifactGallery,
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
            ChartId::Histogram => "Histogram",
            ChartId::BoxPlot => "Box Plot",
            ChartId::CorrelationMatrix => "Correlation Matrix",
            ChartId::ArtifactGallery => "Artifact Gallery",
        }
    }

    /// チャートタイトルの隣に表示する補足説明（凡例）。なければ None。
    pub fn subtitle(&self) -> Option<&'static str> {
        match self {
            ChartId::ScatterMatrix => Some(
                "Lower-left: scatter plots / Upper-right: Pearson correlation / Diagonal: histograms",
            ),
            _ => None,
        }
    }
}

// ----------------------------------------
// PanelItem — キャンバスに配置できるウィジェットの統合型
// 🔵 ユーザーヒアリング（チャートとテーブルをD&D対象化）より
// ----------------------------------------
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PanelItem {
    /// 既存チャートをラップ
    Chart(ChartId),
    /// 旧 BottomPanel をウィジェット化したトライアル一覧テーブル
    TrialTable,
}

impl PanelItem {
    /// 表示ラベル（右パネルの一覧・セルヘッダー等に使用）
    pub fn label(&self) -> &'static str {
        match self {
            PanelItem::Chart(id) => id.label(),
            PanelItem::TrialTable => "Trial Table",
        }
    }

    /// セルタイトルの隣に表示する補足説明（凡例）。なければ None。
    pub fn subtitle(&self) -> Option<&'static str> {
        match self {
            PanelItem::Chart(id) => id.subtitle(),
            PanelItem::TrialTable => None,
        }
    }

    /// 利用可能な全アイテムのリスト（右パネルに表示する順序）
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
// DragPayload — D&D ペイロードの統合型
// 🔵 ユーザーヒアリング（D&D移動）+ 既存 PanelItem D&D より
// ----------------------------------------
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DragPayload {
    /// 右パネルからの新規配置
    NewWidget(PanelItem),
}

impl DragPayload {
    /// ペイロードから PanelItem を取得する
    pub fn item(&self) -> &PanelItem {
        match self {
            DragPayload::NewWidget(item) => item,
        }
    }
}

// ----------------------------------------
// RightPanelState — 右パネルの開閉・サイズ状態
// 🔵 ユーザーヒアリング（ハンバーガーメニュー）より
// ----------------------------------------
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RightPanelState {
    /// ハンバーガーメニューで切り替える開閉状態
    pub is_open: bool,
    /// パネル幅（ドラッグでリサイズ可）
    pub width: f32,
    /// 直近フレームで実際に描画されたパネル左端の X 座標。
    /// アイコンタイルが `width` より広く描画され egui の constrain により
    /// 左へシフトされることがあるため、ホバー閉じ判定には設定値ではなく
    /// この実測値を用いる。描画前（未設定）は None。
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
// CanvasItem — キャンバス上に自由配置された1ウィジェット
// 同じウィジェットを複数配置できるよう固有 ID を持つ。
// 座標・サイズはワールド座標（無限平面上の値）。
// ----------------------------------------
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CanvasItem {
    /// 固有インスタンス ID（複数配置対応）
    pub id: u64,
    /// 配置されたウィジェット
    pub content: PanelItem,
    /// ワールド座標の左上位置
    pub x: f32,
    pub y: f32,
    /// ワールド座標でのサイズ
    pub w: f32,
    pub h: f32,
}

// ----------------------------------------
// CanvasLayout — 自由配置キャンバスの状態
// items の順序が z-order（末尾が最前面）。pan/zoom はビューポート変換。
// ----------------------------------------
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CanvasLayout {
    pub items: Vec<CanvasItem>,
    /// 次に採番する ID
    pub next_id: u64,
    /// ビューポート変換の平行移動（画面座標オフセット）
    pub pan_x: f32,
    pub pan_y: f32,
    /// ビューポート変換のスケール（既定 1.0）
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
    /// 新規アイテムをワールド座標 `(x, y)` に追加する。固有 ID を採番して返す。
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

    /// 指定 ID のアイテムを削除する。
    pub fn remove(&mut self, id: u64) {
        self.items.retain(|it| it.id != id);
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LayoutState {
    pub left_panel_width: f32,
    /// 自由配置キャンバスのレイアウト
    #[serde(default)]
    pub canvas: CanvasLayout,
    /// 右パネルの状態
    pub right_panel: RightPanelState,
    /// 左パネルの開閉状態（ホバーで自動制御）
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
