# Widget Help - Context Note

**Generated**: 2026-05-08

## Technical Stack

| Layer | Technology |
|-------|-----------|
| UI Framework | eframe + egui |
| GPU Rendering | wgpu + egui-wgpu |
| Charts | egui_plot |
| Build | cargo |
| Testing | cargo test |

## Project Structure (Relevant Files)

```
egui-app/src/
├── ui/
│   ├── chart_registry.rs     # Widget dispatch + show_cell_chart/show_chart
│   ├── grid_canvas.rs        # Grid cell rendering + cell toolbar (move/title/close)
│   ├── render_chart.rs       # render_chart() dispatcher
│   ├── poll_chart.rs         # Async work dispatcher
│   ├── widget_states.rs      # WidgetStates struct (all widget UI state)
│   └── widgets/
│       ├── mod.rs            # Widget module declarations (19 widgets)
│       ├── importance_chart.rs
│       ├── sensitivity_heatmap.rs
│       ├── pdp_chart.rs
│       ├── pdp_2d.rs
│       ├── mcdm_chart.rs
│       ├── mcdm_scatter_chart.rs
│       ├── ahp_chart.rs
│       ├── cluster_scatter.rs
│       ├── pareto_2d.rs
│       ├── pareto_3d.rs
│       ├── parallel_coords.rs
│       ├── scatter_matrix.rs
│       ├── optimization_history.rs
│       ├── hv_history.rs
│       ├── slice_chart.rs
│       ├── tradeoff_navigator.rs
│       ├── convergence_card.rs
│       ├── trial_table.rs
│       └── artifact_modal.rs  # Existing modal pattern
├── state/
│   ├── layout_state.rs       # ChartId enum (17 variants), PanelItem, GridLayout
│   └── messages.rs           # AppMessage enum
└── theme.rs                  # Color constants, tunny_light_visuals()

theory/
├── README.md                 # Overview of all analysis methods
├── sensitivity-analysis/     # Spearman, Ridge, Sobol, MDI, RF-ANOVA, SHAP, PDP, etc.
├── mcdm/                     # TOPSIS, VIKOR, PROMETHEE, Entropy Weight, AHP
├── clustering/               # k-means, elbow method
├── surrogate-models/         # Ridge, Random Forest, Kriging, Sparse Kriging
└── optimization/             # L-BFGS
```

## Key Architecture Patterns

### Cell Toolbar (grid_canvas.rs)
- Each grid cell has a toolbar: [Move button] [title] [x close button]
- Toolbar uses `egui::Frame` with theme colors
- Help button (?) should be added to this toolbar

### Modal Pattern (artifact_modal.rs)
- State: `open: bool` flag
- Render: `egui::Window::new().open(&mut still_open).show(ctx, ...)`
- Clean lifecycle with automatic close

### ChartId Dispatch (layout_state.rs)
- 17 ChartId variants: ParetoScatter2D/3D, ParallelCoordinates, ScatterMatrix, ImportanceChart, PdpChart, PdpChart2D, OptimizationHistory, HvHistory, SensitivityHeatmap, ClusterScatter, McdmRankChart, McdmScatterChart, McdmTable, AhpRankChart, AhpTable, SliceChart
- Plus PanelItem::TrialTable

### Widget Render Pattern
- All widgets: `pub fn show(&mut self, ui: &mut egui::Ui, ...)`
- Control bars: horizontal layouts with ComboBoxes, buttons, toggles
- No existing help/tooltip/info mechanism

## Development Rules

- No WASM support needed (native-only desktop app)
- No TypeScript/JavaScript
- egui idioms: RichText, Frame, ComboBox, selectable_label
- Theme constants from `crate::theme` module

## Constraints

- egui has no native LaTeX rendering - use plain text formulas
- Help content must be embedded in binary (no runtime file loading for help text)
- Theory folder content currently in Japanese only - needs English translation
- Theory restructuring: `theory/ja/` (move current), `theory/en/` (new English)
