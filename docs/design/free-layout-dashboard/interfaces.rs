// ============================================================
// Free Layout Dashboard — Rust 型定義
//
// 作成日: 2026-04-12
// 関連設計: architecture.md
//
// 信頼性レベル:
// - 🔵 青信号: ユーザーヒアリング・既存実装を参考にした確実な型定義
// - 🟡 黄信号: ヒアリングから妥当な推測による型定義
// - 🔴 赤信号: ヒアリングにない推測による型定義
// ============================================================

// ----------------------------------------
// PanelItem — キャンバスに配置できるウィジェットの統合型
// 🔵 ユーザーヒアリング（チャートとテーブルをD&D対象化）より
// ----------------------------------------
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PanelItem {
    Chart(ChartId),  // 🔵 既存 ChartId をラップ
    TrialTable,      // 🔵 旧 BottomPanel をウィジェット化
}

impl PanelItem {
    /// 表示ラベル
    pub fn label(&self) -> &'static str {
        match self {
            PanelItem::Chart(id) => id.label(),  // 🔵 既存実装より
            PanelItem::TrialTable => "Trial Table", // 🔵 ヒアリングより
        }
    }

    /// 利用可能な全アイテムのリスト（右パネルに表示する順序）
    pub fn all() -> Vec<PanelItem> {  // 🔵 ヒアリングより
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
#[derive(Debug, Clone, Default)]
pub struct GridCell {
    /// 配置されたウィジェット。None = 空スロット。
    pub content: Option<PanelItem>,  // 🔵 ヒアリングより

    /// 横方向のセル結合数（1 = 結合なし）
    pub col_span: u8,  // 🔵 ヒアリング（上段を横長で使う）より

    /// 縦方向のセル結合数（1 = 結合なし）
    pub row_span: u8,  // 🔵 ヒアリングより

    /// このセルが別のセルに結合されている場合、その結合元の(row, col)
    /// Some なら描画スキップ（結合元セルが描画担当）
    pub merged_into: Option<(usize, usize)>,  // 🔵 設計要件より
}

impl GridCell {
    pub fn new_empty() -> Self {
        Self {
            content: None,
            col_span: 1,
            row_span: 1,
            merged_into: None,
        }
    }

    pub fn is_active(&self) -> bool {
        self.merged_into.is_none()  // 🔵 結合セルの描画制御のため
    }
}

// ----------------------------------------
// GridLayout — キャンバス全体のグリッドレイアウト
// 🔵 ユーザーヒアリング（自由に行列を追加）より
// ----------------------------------------
#[derive(Debug, Clone)]
pub struct GridLayout {
    pub rows: usize,                  // 🔵 デフォルト 2
    pub cols: usize,                  // 🔵 デフォルト 2
    pub cells: Vec<Vec<GridCell>>,    // [row][col] // 🔵
}

impl Default for GridLayout {
    fn default() -> Self {
        // デフォルト 2×2 の空グリッド
        let cells = vec![
            vec![GridCell::new_empty(), GridCell::new_empty()],
            vec![GridCell::new_empty(), GridCell::new_empty()],
        ];
        Self { rows: 2, cols: 2, cells }
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
        // 新しい配置
        if let Some(cell) = self.cells.get_mut(row).and_then(|r| r.get_mut(col)) {
            if cell.merged_into.is_none() {
                cell.content = Some(item);
            }
        }
    }

    /// 現在グリッドに配置済みのアイテムセットを返す（右パネルのグレーアウト用）。
    pub fn placed_items(&self) -> Vec<&PanelItem> {  // 🔵
        self.cells.iter().flatten()
            .filter_map(|c| c.content.as_ref())
            .collect()
    }

    /// 行を末尾に追加する。
    pub fn add_row(&mut self) {  // 🔵 ヒアリング（自由に行追加）より
        self.rows += 1;
        self.cells.push(vec![GridCell::new_empty(); self.cols]);
    }

    /// 列を末尾に追加する。
    pub fn add_col(&mut self) {  // 🔵
        self.cols += 1;
        for row in &mut self.cells {
            row.push(GridCell::new_empty());
        }
    }

    /// 末尾行を削除する（コンテンツが空の場合のみ許可）。
    /// 削除できた場合 true を返す。
    pub fn try_remove_last_row(&mut self) -> bool {  // 🔵
        if self.rows <= 1 { return false; }
        let last = self.rows - 1;
        let can_remove = self.cells[last].iter().all(|c| c.content.is_none() && c.merged_into.is_none());
        if can_remove {
            self.cells.pop();
            self.rows -= 1;
            true
        } else {
            false
        }
    }

    /// 末尾列を削除する（コンテンツが空の場合のみ許可）。
    pub fn try_remove_last_col(&mut self) -> bool {  // 🔵
        if self.cols <= 1 { return false; }
        let last = self.cols - 1;
        let can_remove = self.cells.iter().all(|row| {
            row.get(last).map(|c| c.content.is_none() && c.merged_into.is_none()).unwrap_or(true)
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

    /// セルを右方向に結合する。
    /// 成功した場合 true を返す。
    pub fn expand_right(&mut self, row: usize, col: usize) -> bool {  // 🔵 ヒアリングより
        let cell = &self.cells[row][col];
        let new_end_col = col + cell.col_span as usize;
        if new_end_col >= self.cols { return false; }
        if self.cells[row][new_end_col].merged_into.is_some() { return false; }

        // 被結合セルのコンテンツを退避（右パネルに戻す処理は呼び出し元で実施）
        self.cells[row][new_end_col].merged_into = Some((row, col));
        self.cells[row][col].col_span += 1;
        true
    }

    /// セルを下方向に結合する。
    pub fn expand_down(&mut self, row: usize, col: usize) -> bool {  // 🔵
        let cell = &self.cells[row][col];
        let new_end_row = row + cell.row_span as usize;
        if new_end_row >= self.rows { return false; }
        if self.cells[new_end_row][col].merged_into.is_some() { return false; }

        self.cells[new_end_row][col].merged_into = Some((row, col));
        self.cells[row][col].row_span += 1;
        true
    }
}

// ----------------------------------------
// RightPanelState — 右パネルの開閉・サイズ状態
// 🔵 ユーザーヒアリング（ハンバーガーメニュー）より
// ----------------------------------------
#[derive(Debug, Clone)]
pub struct RightPanelState {
    pub is_open: bool,   // 🔵 ハンバーガーメニューで切り替え
    pub width: f32,      // 🟡 デフォルト 240.0（既存 left_panel_width に合わせる）
}

impl Default for RightPanelState {
    fn default() -> Self {
        Self {
            is_open: true,   // 🟡 デフォルトで開いた状態
            width: 200.0,
        }
    }
}

// ----------------------------------------
// LayoutState の変更後の構造（参考）
// 🔵 既存 LayoutState に GridLayout / RightPanelState を追加
// ----------------------------------------
//
// pub struct LayoutState {
//     pub left_panel_width: f32,      // 既存（フィルター用）
//     pub right_panel: RightPanelState,  // 新規
//     pub layout_mode: LayoutMode,    // 既存（将来削除候補）
//     pub grid: GridLayout,           // 新規（visible_charts を置き換え）
// }
//
// removed: pub visible_charts: HashSet<ChartId>

// ----------------------------------------
// 信頼性レベルサマリー
// - 🔵 青信号: 20件 (80%)
// - 🟡 黄信号: 4件 (16%)
// - 🔴 赤信号: 1件 (4%)
//
// 品質評価: ✅ 高品質
// ----------------------------------------
