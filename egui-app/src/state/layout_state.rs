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

// ----------------------------------------
// PanelItem — キャンバスに配置できるウィジェットの統合型
// 🔵 ユーザーヒアリング（チャートとテーブルをD&D対象化）より
// ----------------------------------------
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
// GridCell — グリッドの1セルの状態
// 🔵 ユーザーヒアリング（セル結合・D&D配置）より
// ----------------------------------------
#[derive(Debug, Clone)]
pub struct GridCell {
    /// 配置されたウィジェット。None = 空スロット。
    pub content: Option<PanelItem>,

    /// 横方向のセル結合数（1 = 結合なし）
    pub col_span: u8,

    /// 縦方向のセル結合数（1 = 結合なし）
    pub row_span: u8,

    /// このセルが別のセルに結合されている場合、その結合元の (row, col)。
    /// Some の場合は描画をスキップ（結合元セルが描画担当）。
    pub merged_into: Option<(usize, usize)>,
}

impl Default for GridCell {
    fn default() -> Self {
        Self::new_empty()
    }
}

impl GridCell {
    /// 空セルを生成する（content=None、span=1、結合なし）
    pub fn new_empty() -> Self {
        Self {
            content: None,
            col_span: 1,
            row_span: 1,
            merged_into: None,
        }
    }

    /// このセルが描画担当かどうか（merged_into=None のとき true）
    pub fn is_active(&self) -> bool {
        self.merged_into.is_none()
    }
}

// ----------------------------------------
// GridLayout — キャンバス全体のグリッドレイアウト
// 🔵 ユーザーヒアリング（自由に行列を追加）より
// ----------------------------------------
#[derive(Debug, Clone)]
pub struct GridLayout {
    pub rows: usize,
    pub cols: usize,
    /// セルの2次元配列 [row][col]
    pub cells: Vec<Vec<GridCell>>,
}

impl Default for GridLayout {
    fn default() -> Self {
        // デフォルト 2×2 の空グリッド
        let cells = vec![
            vec![GridCell::new_empty(), GridCell::new_empty()],
            vec![GridCell::new_empty(), GridCell::new_empty()],
        ];
        Self {
            rows: 2,
            cols: 2,
            cells,
        }
    }
}

impl GridLayout {
    /// アイテムをセルに配置する。
    /// 既に別セルに配置されている場合は元セルをクリアする（移動動作）。
    pub fn place(&mut self, row: usize, col: usize, item: PanelItem) {
        // 既存配置の削除
        for r in 0..self.rows {
            for c in 0..self.cols {
                if self.cells[r][c].content.as_ref() == Some(&item) {
                    self.cells[r][c].content = None;
                }
            }
        }
        // 新しい配置（結合先セルへは配置しない）
        if let Some(cell) = self.cells.get_mut(row).and_then(|r| r.get_mut(col)) {
            if cell.merged_into.is_none() {
                cell.content = Some(item);
            }
        }
    }

    /// 現在グリッドに配置済みのアイテム参照リストを返す（右パネルのグレーアウト用）
    pub fn placed_items(&self) -> Vec<&PanelItem> {
        self.cells
            .iter()
            .flatten()
            .filter_map(|c| c.content.as_ref())
            .collect()
    }

    /// 行を末尾に追加する
    pub fn add_row(&mut self) {
        self.rows += 1;
        self.cells.push(vec![GridCell::new_empty(); self.cols]);
    }

    /// 列を末尾に追加する
    pub fn add_col(&mut self) {
        self.cols += 1;
        for row in &mut self.cells {
            row.push(GridCell::new_empty());
        }
    }

    /// 末尾行が削除可能かチェックする（削除はしない）。
    pub fn can_remove_last_row(&self) -> bool {
        if self.rows <= 1 {
            return false;
        }
        let last = self.rows - 1;
        self.cells[last]
            .iter()
            .all(|c| c.content.is_none() && c.merged_into.is_none())
    }

    /// 末尾列が削除可能かチェックする（削除はしない）。
    pub fn can_remove_last_col(&self) -> bool {
        if self.cols <= 1 {
            return false;
        }
        let last = self.cols - 1;
        self.cells.iter().all(|row| {
            row.get(last)
                .map(|c| c.content.is_none() && c.merged_into.is_none())
                .unwrap_or(true)
        })
    }

    /// 末尾行を削除する（コンテンツが空の場合のみ許可）。削除できた場合 true を返す。
    pub fn try_remove_last_row(&mut self) -> bool {
        if self.rows <= 1 {
            return false;
        }
        let last = self.rows - 1;
        let can_remove = self.cells[last]
            .iter()
            .all(|c| c.content.is_none() && c.merged_into.is_none());
        if can_remove {
            self.cells.pop();
            self.rows -= 1;
            true
        } else {
            false
        }
    }

    /// 末尾列を削除する（コンテンツが空の場合のみ許可）。削除できた場合 true を返す。
    pub fn try_remove_last_col(&mut self) -> bool {
        if self.cols <= 1 {
            return false;
        }
        let last = self.cols - 1;
        let can_remove = self.cells.iter().all(|row| {
            row.get(last)
                .map(|c| c.content.is_none() && c.merged_into.is_none())
                .unwrap_or(true)
        });
        if can_remove {
            for row in &mut self.cells {
                row.pop();
            }
            self.cols -= 1;
            true
        } else {
            false
        }
    }

    /// セルを右方向に結合する。成功した場合 true を返す。
    pub fn expand_right(&mut self, row: usize, col: usize) -> bool {
        let new_end_col = col + self.cells[row][col].col_span as usize;
        if new_end_col >= self.cols {
            return false;
        }
        if self.cells[row][new_end_col].merged_into.is_some() {
            return false;
        }
        self.cells[row][new_end_col].merged_into = Some((row, col));
        self.cells[row][col].col_span += 1;
        true
    }

    /// セルを下方向に結合する。成功した場合 true を返す。
    pub fn expand_down(&mut self, row: usize, col: usize) -> bool {
        let new_end_row = row + self.cells[row][col].row_span as usize;
        if new_end_row >= self.rows {
            return false;
        }
        if self.cells[new_end_row][col].merged_into.is_some() {
            return false;
        }
        self.cells[new_end_row][col].merged_into = Some((row, col));
        self.cells[row][col].row_span += 1;
        true
    }

    /// セルの右方向結合を1段階縮小する（col_span が 1 以下なら false を返す）。
    pub fn shrink_right(&mut self, row: usize, col: usize) -> bool {
        if self.cells[row][col].col_span <= 1 {
            return false;
        }
        let end_col = col + self.cells[row][col].col_span as usize - 1;
        // 末端被結合セルを解放
        self.cells[row][end_col].merged_into = None;
        self.cells[row][col].col_span -= 1;
        true
    }

    /// セルの下方向結合を1段階縮小する（row_span が 1 以下なら false を返す）。
    pub fn shrink_down(&mut self, row: usize, col: usize) -> bool {
        if self.cells[row][col].row_span <= 1 {
            return false;
        }
        let end_row = row + self.cells[row][col].row_span as usize - 1;
        // 末端被結合セルを解放
        self.cells[end_row][col].merged_into = None;
        self.cells[row][col].row_span -= 1;
        true
    }
}

// ----------------------------------------
// RightPanelState — 右パネルの開閉・サイズ状態
// 🔵 ユーザーヒアリング（ハンバーガーメニュー）より
// ----------------------------------------
#[derive(Debug, Clone)]
pub struct RightPanelState {
    /// ハンバーガーメニューで切り替える開閉状態
    pub is_open: bool,
    /// パネル幅（ドラッグでリサイズ可）
    pub width: f32,
}

impl Default for RightPanelState {
    fn default() -> Self {
        Self {
            is_open: true,
            width: 200.0,
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
    pub layout_mode: LayoutMode,
    /// フリーグリッドレイアウト（visible_charts の置き換え）
    pub grid: GridLayout,
    /// 右パネルの状態
    pub right_panel: RightPanelState,
}

impl Default for LayoutState {
    fn default() -> Self {
        Self {
            left_panel_width: 240.0,
            layout_mode: LayoutMode::MultiObjective,
            grid: GridLayout::default(),
            right_panel: RightPanelState::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    // --- GridCell tests ---

    #[test]
    fn grid_cell_new_empty_default_values() {
        let cell = GridCell::new_empty();
        assert!(cell.content.is_none());
        assert_eq!(cell.col_span, 1);
        assert_eq!(cell.row_span, 1);
        assert!(cell.merged_into.is_none());
    }

    #[test]
    fn grid_cell_is_active_when_not_merged() {
        let cell = GridCell::new_empty();
        assert!(cell.is_active());
    }

    #[test]
    fn grid_cell_is_not_active_when_merged() {
        let mut cell = GridCell::new_empty();
        cell.merged_into = Some((0, 0));
        assert!(!cell.is_active());
    }

    #[test]
    fn grid_cell_default_equals_new_empty() {
        let a = GridCell::default();
        let b = GridCell::new_empty();
        assert_eq!(a.col_span, b.col_span);
        assert_eq!(a.row_span, b.row_span);
        assert_eq!(a.content, b.content);
        assert_eq!(a.merged_into, b.merged_into);
    }

    // --- GridLayout tests ---

    #[test]
    fn grid_layout_default_is_2x2() {
        let g = GridLayout::default();
        assert_eq!(g.rows, 2);
        assert_eq!(g.cols, 2);
        assert_eq!(g.cells.len(), 2);
        assert_eq!(g.cells[0].len(), 2);
    }

    #[test]
    fn grid_layout_place_sets_content() {
        let mut g = GridLayout::default();
        g.place(0, 0, PanelItem::TrialTable);
        assert_eq!(g.cells[0][0].content, Some(PanelItem::TrialTable));
    }

    #[test]
    fn grid_layout_place_clears_old_position() {
        let mut g = GridLayout::default();
        g.place(0, 0, PanelItem::TrialTable);
        g.place(1, 1, PanelItem::TrialTable);
        assert_eq!(g.cells[0][0].content, None);
        assert_eq!(g.cells[1][1].content, Some(PanelItem::TrialTable));
    }

    #[test]
    fn grid_layout_placed_items_returns_contents() {
        let mut g = GridLayout::default();
        g.place(0, 0, PanelItem::TrialTable);
        g.place(1, 0, PanelItem::Chart(ChartId::ParetoScatter2D));
        let items = g.placed_items();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn grid_layout_add_row_increases_rows() {
        let mut g = GridLayout::default();
        g.add_row();
        assert_eq!(g.rows, 3);
        assert_eq!(g.cells.len(), 3);
        assert_eq!(g.cells[2].len(), 2);
    }

    #[test]
    fn grid_layout_add_col_increases_cols() {
        let mut g = GridLayout::default();
        g.add_col();
        assert_eq!(g.cols, 3);
        assert_eq!(g.cells[0].len(), 3);
    }

    #[test]
    fn grid_layout_try_remove_last_row_when_empty() {
        let mut g = GridLayout::default();
        let result = g.try_remove_last_row();
        assert!(result);
        assert_eq!(g.rows, 1);
    }

    #[test]
    fn grid_layout_try_remove_last_row_blocked_by_content() {
        let mut g = GridLayout::default();
        g.place(1, 0, PanelItem::TrialTable);
        let result = g.try_remove_last_row();
        assert!(!result);
        assert_eq!(g.rows, 2);
    }

    #[test]
    fn grid_layout_try_remove_last_row_not_below_1() {
        let mut g = GridLayout::default();
        g.try_remove_last_row(); // 2→1
        let result = g.try_remove_last_row(); // 1→拒否
        assert!(!result);
        assert_eq!(g.rows, 1);
    }

    #[test]
    fn grid_layout_try_remove_last_col_when_empty() {
        let mut g = GridLayout::default();
        let result = g.try_remove_last_col();
        assert!(result);
        assert_eq!(g.cols, 1);
    }

    #[test]
    fn grid_layout_expand_right_merges_cell() {
        let mut g = GridLayout::default();
        let result = g.expand_right(0, 0);
        assert!(result);
        assert_eq!(g.cells[0][0].col_span, 2);
        assert_eq!(g.cells[0][1].merged_into, Some((0, 0)));
    }

    #[test]
    fn grid_layout_expand_right_fails_at_boundary() {
        let mut g = GridLayout::default();
        g.expand_right(0, 0); // col_span=2, reaches boundary
        let result = g.expand_right(0, 0); // already at boundary
        assert!(!result);
    }

    #[test]
    fn grid_layout_expand_down_merges_cell() {
        let mut g = GridLayout::default();
        let result = g.expand_down(0, 0);
        assert!(result);
        assert_eq!(g.cells[0][0].row_span, 2);
        assert_eq!(g.cells[1][0].merged_into, Some((0, 0)));
    }

    #[test]
    fn grid_layout_shrink_right_releases_cell() {
        let mut g = GridLayout::default();
        g.expand_right(0, 0); // col_span=2, cells[0][1].merged_into=Some((0,0))
        let result = g.shrink_right(0, 0);
        assert!(result);
        assert_eq!(g.cells[0][0].col_span, 1);
        assert_eq!(g.cells[0][1].merged_into, None);
    }

    #[test]
    fn grid_layout_shrink_right_fails_when_not_merged() {
        let mut g = GridLayout::default();
        let result = g.shrink_right(0, 0);
        assert!(!result);
        assert_eq!(g.cells[0][0].col_span, 1);
    }

    #[test]
    fn grid_layout_shrink_down_releases_cell() {
        let mut g = GridLayout::default();
        g.expand_down(0, 0); // row_span=2, cells[1][0].merged_into=Some((0,0))
        let result = g.shrink_down(0, 0);
        assert!(result);
        assert_eq!(g.cells[0][0].row_span, 1);
        assert_eq!(g.cells[1][0].merged_into, None);
    }

    // --- LayoutState tests ---

    #[test]
    fn layout_state_default_has_grid_and_right_panel() {
        let layout = LayoutState::default();
        assert_eq!(layout.left_panel_width, 240.0);
        assert_eq!(layout.layout_mode, LayoutMode::MultiObjective);
        assert_eq!(layout.grid.rows, 2);
        assert_eq!(layout.grid.cols, 2);
        assert!(layout.right_panel.is_open);
        assert_eq!(layout.right_panel.width, 200.0);
    }

    #[test]
    fn right_panel_state_default() {
        let rp = RightPanelState::default();
        assert!(rp.is_open);
        assert_eq!(rp.width, 200.0);
    }

    #[test]
    fn layout_mode_variants() {
        let mode = LayoutMode::FreeLayout;
        assert_ne!(mode, LayoutMode::MultiObjective);
        assert_ne!(mode, LayoutMode::VariableSpace);
    }
}
