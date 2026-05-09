# Widget Help アーキテクチャ設計

**作成日**: 2026-05-08
**関連要件定義**: [requirements.md](../../spec/widget-help/requirements.md)
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *要件定義書・ユーザヒアリングより*

全18ウィジェットのセルツールバーに「?」ボタンを追加し、クリックでモーダルを開いて Theory 情報（概要・選び方ガイド・各手法詳細）をタブ切替で表示する。コンテンツは `theory/en/` の Markdown を `include_str!` で埋め込み、軽量 Markdown→egui レンダラで描画する。

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存 artifact_modal パターン・ユーザヒアリングより*

- **パターン**: 既存モーダルパターンの拡張（artifact_modal 準拠）
- **選択理由**:
  - artifact_modal で実績のある egui::Window パターンを再利用
  - WidgetStates への状態追加で最小の変更量
  - app.rs ループ内でレンダリングし、全ウィジェットからアクセス可能

## 新規モジュール構成 🔵

**信頼性**: 🔵 *既存プロジェクト構造・ユーザヒアリングより*

```
egui-app/src/
├── ui/
│   ├── help/                         # 新規: ヘルプシステム
│   │   ├── mod.rs                    # 公開インターフェース
│   │   ├── help_modal.rs             # モーダルUI（egui::Window + タブ）
│   │   ├── help_content.rs           # ChartId → HelpContent のルックアップ
│   │   ├── md_renderer.rs            # 軽量 Markdown → egui レンダラ
│   │   └── help_types.rs             # 型定義（HelpTab, HelpContent 等）
│   ├── grid_canvas.rs                # 変更: show_cell_toolbar に ? ボタン追加
│   ├── widget_states.rs              # 変更: HelpModalState フィールド追加
│   └── widgets/
│       └── (既存ファイル、変更なし)
├── app.rs                            # 変更: モーダル render 呼び出し追加
└── theme/
    └── ui_colors.rs                  # 変更: HELP_BTN_TEXT 色追加（任意）
```

## 新規 Rust 型定義 🔵

**信頼性**: 🔵 *要件定義書 REQ-006, REQ-010〜015, REQ-020〜021 より*

### HelpModalState（widget_states.rs に追加）

```rust
/// ヘルプモーダルの状態 🔵 *要件定義 REQ-003〜007・artifact_modal パターンより*
#[derive(Default)]
pub struct HelpModalState {
    /// モーダルが開いているか
    pub open: bool,
    /// 現在のタブインデックス（0 = Overview）
    pub active_tab: usize,
    /// 紐づく PanelItem
    pub item: Option<PanelItem>,
}
```

### HelpContent（help_types.rs）

```rust
/// ウィジェットごとのヘルプコンテンツ定義 🔵 *要件定義 REQ-010〜021 より*
pub struct HelpContent {
    /// モーダルのタイトル（例: "Importance Chart"）
    pub title: &'static str,
    /// タブ定義
    pub tabs: &'static [HelpTabDef],
}

/// ヘルプモーダルのタブ定義 🔵 *要件定義 REQ-010〜015 より*
pub struct HelpTabDef {
    /// タブラベル（例: "Overview", "Sobol"）
    pub label: &'static str,
    /// タブの内容（Markdown テキスト）
    pub markdown: &'static str,
}
```

### PanelItem 拡張メソッド 🔵

```rust
impl PanelItem {
    /// ヘルプコンテンツを返す 🔵 *要件定義 REQ-030〜036 より*
    pub fn help_content(&self) -> HelpContent {
        match self {
            PanelItem::Chart(ChartId::ImportanceChart) => HelpContent {
                title: "Importance Chart",
                tabs: &[
                    HelpTabDef { label: "Overview", markdown: include_str!("../../../../theory/en/sensitivity-analysis/overview.md") },
                    HelpTabDef { label: "Spearman", markdown: include_str!("../../../../theory/en/sensitivity-analysis/spearman.md") },
                    HelpTabDef { label: "Ridge", markdown: include_str!("../../../../theory/en/sensitivity-analysis/ridge.md") },
                    // ... 他の感度分析手法
                ],
            },
            PanelItem::Chart(ChartId::McdmRankChart) => HelpContent { /* ... */ },
            PanelItem::TrialTable => HelpContent { /* ... */ },
            // ... 全18ウィジェット
        }
    }
}
```

## 軽量 Markdown→egui レンダラ 🔵

**信頼性**: 🔵 *要件定義 REQ-015（プレーンテキスト数式）・ユーザヒアリングより*

### 対応 Markdown 要素

| 要素 | egui 表現 | 優先度 |
|------|-----------|--------|
| `# Heading` | `ui.heading(text)` | Must Have |
| `## Heading` | `ui.strong(text)` | Must Have |
| `**bold**` | `ui.strong(text)` | Must Have |
| `- item` | `ui.label("• text")` | Must Have |
| `` `code` `` | `ui.code_editor` or monospace `RichText` | Must Have |
| `| table |` | egui TableBuilder | Should Have |
| ``` ```code``` ``` | `ui.code_editor` (readonly) | Should Have |
| `` $formula$ `` | プレーンテキスト（そのまま表示） | Must Have |
| `[link](url)` | `ui.hyperlink()` | Won't Have |
| `> blockquote` | `ui.colored_label` | Won't Have |

### レンダラインターフェース

```rust
/// 軽量 Markdown→egui レンダラ 🔵 *ユーザヒアリング: プレーンテキスト表現より*
pub fn render_markdown(ui: &mut egui::Ui, markdown: &str) {
    // ScrollArea 内で描画
    egui::ScrollArea::vertical().show(ui, |ui| {
        for line in markdown.lines() {
            render_markdown_line(ui, line);
        }
    });
}

fn render_markdown_line(ui: &mut egui::Ui, line: &str) {
    if line.starts_with("# ") {
        ui.heading(&line[2..]);
    } else if line.starts_with("## ") {
        ui.strong(RichText::new(&line[3..]).size(16.0));
    } else if line.starts_with("- ") || line.starts_with("* ") {
        ui.horizontal(|ui| {
            ui.label("•");
            render_inline(ui, &line[2..]);
        });
    } else if line.starts_with("|") {
        // テーブル行の処理
        render_table_row(ui, line);
    } else if line.starts_with("```") {
        // コードブロック開始/終了
    } else {
        render_inline(ui, line);
    }
    ui.end_row();
}
```

## セルツールバーへの「?」ボタン追加 🔵

**信頼性**: 🔵 *要件定義 REQ-001, REQ-002・grid_canvas.rs 実装より*

### 変更箇所: `grid_canvas.rs` の `show_cell_toolbar`

**現在のレイアウト**:
```
[Move (56px)] [8px spacer] [title (strong)] [spacer to right] [x (16px)]
```

**変更後のレイアウト**:
```
[Move (56px)] [8px spacer] [title (strong)] [spacer to right] [? (16px)] [4px] [x (16px)]
```

### 関数シグネチャ変更

```rust
/// 変更前
fn show_cell_toolbar(
    ui: &mut egui::Ui,
    row: usize, col: usize,
    item: PanelItem,
    title: &'static str,
) -> bool  // close clicked

/// 変更後 🔵 *要件定義 REQ-001〜003 より*
fn show_cell_toolbar(
    ui: &mut egui::Ui,
    row: usize, col: usize,
    item: PanelItem,
    title: &'static str,
) -> CellToolbarAction  // close または help clicked

/// ツールバーアクション 🔵
pub enum CellToolbarAction {
    None,
    Close,
    Help(PanelItem),
}
```

### 「?」ボタン描画

```rust
// 閉じるボタンの直前に追加 🔵 *要件定義 REQ-002 より*
let help_resp = ui.add_sized(
    egui::vec2(16.0, 16.0),
    egui::Button::new(
        egui::RichText::new("?").small().color(CLOSE_BTN_TEXT)
    ).frame(false),
);
if help_resp.clicked() {
    return CellToolbarAction::Help(item.clone());
}
if help_resp.hovered() {
    help_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
}
ui.add_space(4.0); // ? と x の間のスペース
```

## ヘルプモーダル描画 🔵

**信頼性**: 🔵 *要件定義 REQ-004〜007・artifact_modal パターンより*

### レンダリング（app.rs の update 内）

```rust
// TunnyApp::update() 内
// artifact_modal と同パターン
crate::ui::help::show_help_modal(ctx, &mut self.widget_states.help_modal);
```

### show_help_modal（help_modal.rs）

```rust
pub fn show_help_modal(ctx: &egui::Context, state: &mut HelpModalState) {
    if !state.open {
        return;
    }

    let item = match &state.item {
        Some(i) => i.clone(),
        None => return,
    };

    let content = item.help_content();
    let title = format!("{} — Help", content.title);

    let mut still_open = true;
    egui::Window::new(title)
        .open(&mut still_open)
        .resizable(true)
        .min_width(500.0)
        .default_width(650.0)
        .min_height(400.0)
        .default_height(500.0)
        .show(ctx, |ui| {
            // タブバー
            let active = state.active_tab.min(content.tabs.len() - 1);
            ui.horizontal(|ui| {
                for (i, tab) in content.tabs.iter().enumerate() {
                    if ui.selectable_label(active == i, tab.label).clicked() {
                        state.active_tab = i;
                    }
                }
            });
            ui.separator();

            // 選択タブの Markdown コンテンツを描画
            if let Some(tab) = content.tabs.get(active) {
            crate::ui::help::md_renderer::render_markdown(ui, tab.markdown);
            }
        });

    if !still_open {
        state.open = false;
        state.active_tab = 0;
    }
}
```

## WidgetStates への追加 🔵

**信頼性**: 🔵 *既存 WidgetStates 構造・artifact_modal パターンより*

```rust
// widget_states.rs に追加
pub struct WidgetStates {
    // ... 既存フィールド ...

    /// ヘルプモーダル状態 🔵
    pub help_modal: HelpModalState,
}
```

`HelpModalState` は `#[derive(Default)]` で `open: false, active_tab: 0, item: None` になるため、既存コードへの影響なし。

## Theory フォルダ再構成 🔵

**信頼性**: 🔵 *要件定義 REQ-040〜043・ユーザヒアリングより*

### 現在の構造
```
theory/
├── README.md
├── sensitivity-analysis/
│   ├── spearman.md, ridge.md, sobol.md, mdi.md, ...
│   └── README.md
├── mcdm/
│   ├── topsis.md, vikor.md, promethee.md, ...
│   └── README.md
├── clustering/
│   ├── kmeans.md, elbow.md
│   └── README.md
├── surrogate-models/
│   └── ridge.md, random-forest.md, kriging.md, sparse-kriging.md
└── optimization/
    └── lbfgs.md
```

### 変更後の構造
```
theory/
├── README.md                    # 言語別リンク（en/ ja/ への案内）
├── en/
│   ├── README.md
│   ├── sensitivity-analysis/
│   │   ├── overview.md          # README 相当の概要
│   │   ├── spearman.md
│   │   ├── ridge.md
│   │   ├── sobol.md
│   │   ├── mdi.md
│   │   ├── rfanova.md
│   │   ├── permutation.md
│   │   ├── shap.md
│   │   └── pdp.md
│   ├── mcdm/
│   │   ├── overview.md
│   │   ├── topsis.md
│   │   ├── vikor.md
│   │   ├── promethee.md
│   │   ├── entropy-weight.md
│   │   └── ahp.md
│   ├── clustering/
│   │   ├── overview.md
│   │   ├── kmeans.md
│   │   └── elbow.md
│   ├── surrogate-models/
│   │   ├── overview.md
│   │   ├── ridge.md
│   │   ├── random-forest.md
│   │   ├── kriging.md
│   │   └── sparse-kriging.md
│   ├── optimization/
│   │   └── lbfgs.md
│   └── widgets/                 # 新規: 使い方ガイド
│       ├── pareto-2d.md
│       ├── pareto-3d.md
│       ├── parallel-coords.md
│       ├── scatter-matrix.md
│       ├── optimization-history.md
│       ├── hv-history.md
│       ├── slice-chart.md
│       └── trial-table.md
└── ja/
    └── (現在の theory/ の内容をそのまま移動)
```

## Widget→HelpContent 対応表 🔵

**信頼性**: 🔵 *要件定義 REQ-030〜036・Theory フォルダ構造より*

| PanelItem | 概要タブ | 詳細タブ |
|-----------|---------|---------|
| ChartId::ImportanceChart | sensitivity-analysis/overview.md | spearman, ridge, sobol, mdi, rfanova, permutation, shap |
| ChartId::SensitivityHeatmap | sensitivity-analysis/overview.md | spearman, ridge, sobol |
| ChartId::PdpChart | sensitivity-analysis/pdp.md | ridge, random-forest, kriging, sparse-kriging |
| ChartId::PdpChart2D | sensitivity-analysis/pdp.md | ridge, random-forest, kriging, sparse-kriging |
| ChartId::McdmRankChart | mcdm/overview.md | topsis, vikor, promethee |
| ChartId::McdmScatterChart | mcdm/overview.md | topsis, vikor, promethee |
| ChartId::McdmTable | mcdm/overview.md | topsis, vikor, promethee |
| ChartId::AhpRankChart | mcdm/ahp.md | — |
| ChartId::AhpTable | mcdm/ahp.md | — |
| ChartId::ClusterScatter | clustering/overview.md | kmeans, elbow |
| ChartId::ParetoScatter2D | widgets/pareto-2d.md | — |
| ChartId::ParetoScatter3D | widgets/pareto-3d.md | — |
| ChartId::ParallelCoordinates | widgets/parallel-coords.md | — |
| ChartId::ScatterMatrix | widgets/scatter-matrix.md | — |
| ChartId::OptimizationHistory | widgets/optimization-history.md | — |
| ChartId::HvHistory | widgets/hv-history.md | — |
| ChartId::SliceChart | widgets/slice-chart.md | — |
| PanelItem::TrialTable | widgets/trial-table.md | — |

## 非機能要件の実現方法

### パフォーマンス 🟡

**信頼性**: 🟡 *NFR-001（100ms 以内）から妥当な推測*

- **レスポンスタイム**: `include_str!` はコンパイル時埋め込みのため、実行時 I/O ゼロ。モーダル表示は egui::Window の生成のみで 1ms 以下
- **メモリ**: 25ファイル × 平均5KB ≈ 125KB の文字列リテラル。バイナリサイズへの影響は微小
- **Markdown レンダラ**: 行単位のパースで O(n)。ScrollArea により可視範囲のみ描画

### 保守性 🔵

**信頼性**: 🔵 *NFR-020, NFR-021・ユーザヒアリングより*

- **コンテンツ更新**: theory/en/ の .md ファイルを編集 → `cargo build` で反映。コード変更不要
- **新規ウィジェット追加**: `PanelItem::help_content()` に match arm を追加 + .md ファイル配置

## 技術的制約 🔵

**信頼性**: 🔵 *CLAUDE.md・既存実装より*

- egui に Markdown レンダラなし → 軽量パーサを自前実装（見出し・リスト・テーブル・コードブロック対応）
- LaTeX 数式のネイティブ表示不可 → プレーンテキスト表現のみ
- include_str! のパスは Cargo.toml 基準（クレートルートからの相対パス）
- モーダルは egui::Window で描画（モーダル外クリックでは閉じない仕様）

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **ヒアリング記録**: [design-interview.md](design-interview.md)
- **要件定義**: [requirements.md](../../spec/widget-help/requirements.md)

## 信頼性レベルサマリー

- 🔵 青信号: 28件 (93%)
- 🟡 黄信号: 2件 (7%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質 — 既存 artifact_modal パターンの再利用、egui 標準機能のみで実装可能
