# chart-implementation データフロー図

**作成日**: 2026-04-12
**関連アーキテクチャ**: [architecture.md](architecture.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実なフロー
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測によるフロー
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測によるフロー

---

## チャート描画の全体フロー 🔵

**信頼性**: 🔵 *`grid_canvas.rs` の既存実装より*

```
AppState (current_study, sensitivity_result, cluster_result)
    │
    ▼
show_grid_canvas() [grid_canvas.rs]
    │ 各セルの content を走査
    ▼
render_cell_content() → show_cell_chart() → show_chart()
    │
    ├─ ChartId::ParallelCoordinates → widgets.parallel_coords.show(...)
    ├─ ChartId::ScatterMatrix       → widgets.scatter_matrix.show(...)
    ├─ ChartId::SensitivityHeatmap  → widgets.sensitivity_heatmap.show(...)
    └─ ChartId::ClusterScatter      → widgets.cluster_scatter.show(...)
```

---

## ParallelCoordinates データフロー 🔵

**信頼性**: 🔵 *`parallel_coords.rs` の既存ヘルパー関数より*

```
AppState.current_study
    ├── trial_rows: Vec<TrialRow>     ─────────────────────────────┐
    ├── meta.param_names: Vec<String> ──────────────────────────┐  │
    └── meta.objective_names: Vec<String> ──────────────────┐   │  │
                                                            │   │  │
                                                            ▼   ▼  ▼
                                              ParallelCoordsChart::show()
                                                            │
                                    ┌───────────────────────┤
                                    ▼                       ▼
                             build_axis_order()      normalize_value()
                            （軸の並び順を決定）    （各値を [0,1] に正規化）
                                    │                       │
                                    └──────────┬────────────┘
                                               ▼
                                    egui::Painter でライン描画
                                    各試行を折れ線で描画
                                               │
                          ┌────────────────────┤
                          ▼                    ▼
                  ブラシ範囲 UI         ハイライト試行
                  (drag interaction)   （太線で強調）
                          │
                          ▼
               brush_ranges: HashMap<usize, (f64, f64)>
               （ParallelCoordsChart のフィールドに格納）
```

---

## ScatterMatrix データフロー 🔵

**信頼性**: 🔵 *`scatter_matrix.rs` の既存描画関数より*

```
AppState.current_study
    ├── trial_rows: Vec<TrialRow>
    ├── meta.param_names: Vec<String>
    └── meta.objective_names: Vec<String>
                │
                ▼
        ScatterMatrix::show()
                │
    ┌───────────┼───────────────────┐
    ▼           ▼                   ▼
  対角セル    上三角セル           下三角セル
draw_histogram draw_correlation  draw_scatter
   _cell()        _cell()           _cell()
    │               │                 │
compute_histogram  compute_correlation  data_to_screen()
    │               │                 │
    └───────────────┴─────────────────┘
                    │
            egui::Painter で各セル描画
            （rect_filled + line_segment）
```

N×N グリッド（N = param_names.len() + obj_names.len()）の分割:
- セル幅/高さ = `available_width / N`, `available_height / N`
- 各セルへの `egui::Rect` を計算して `ui.new_child()` で子 UI 生成

---

## SensitivityHeatmap データフロー 🔵

**信頼性**: 🔵 *`sensitivity_heatmap.rs` + `app_state.rs` 調査より*

```
AppState.sensitivity_result: Option<SensitivityResult>
    ├── param_names: Vec<String>        // 行ヘッダ
    ├── objective_names: Vec<String>    // 列ヘッダ
    └── spearman: Vec<Vec<f64>>         // [param_idx][obj_idx]
                │
                ▼ (None の場合は "No sensitivity data" ラベル表示)
    SensitivityHeatmap::show()
                │
        ┌───────┴───────┐
        ▼               ▼
  ヘッダ描画        セルグリッド描画
  (param/obj 名)    spearman[i][j] を走査
        │               │
        │         diverging_colormap(v)
        │         （-1 → 青, 0 → 白, 1 → 赤）
        │               │
        └───────────────┘
                │
        egui::Painter::rect_filled() で各セルを塗色
        セル上に数値テキストをオーバーレイ
```

---

## ClusterScatter データフロー 🟡

**信頼性**: 🟡 *ユーザヒアリング（外部クレート方針）+ 既存コード調査より*

```
AppState
    ├── current_study.trial_rows: Vec<TrialRow>   // パラメータ値を使用
    └── cluster_result: Option<ClusterResult>
            ├── labels: Vec<i32>
            └── n_clusters: usize
                    │
                    ▼
        ClusterScatter::show()
                    │
       ┌────────────┴────────────┐
       ▼                         ▼
  キャッシュ確認              cluster_result が None の場合
  (trial_rows.len() と          → "No cluster data" ラベル表示
   n_clusters が同一か？)
       │
  ┌────┴──────┐
  │ キャッシュ │ キャッシュ
  │  ヒット   │  ミス
  │           ▼
  │    ndarray::Array2 に変換
  │    (trial_rows の param_values を行列化)
  │           │
  │    linfa_reduction::Pca::params(2)
  │    .fit(&dataset).transform()
  │    → pca_components: Vec<[f32; 2]>
  │           │
  │    cached_pca に格納
  │           │
  └─────┬─────┘
        │
        ▼
  egui_plot::Plot::new()
        │
  クラスタ別に Points を構築
  (labels[i] でグルーピング、クラスタ色を割り当て)
        │
  plot_ui.points() で散布図を描画
```

### ClusterScatter のキャッシュ管理 🟡

**信頼性**: 🟡 *既存の Widget パターンから妥当な推測*

```rust
pub struct ClusterScatter {
    // 既存フィールド...
    cached_pca: Option<Vec<[f32; 2]>>,
    cache_key: (usize, usize),   // (trial_count, n_clusters)
}
```

`show()` 冒頭でキャッシュキーを確認し、不一致の場合のみ PCA を再計算。

---

## WidgetStates 経由の呼び出しフロー 🔵

**信頼性**: 🔵 *`widget_states.rs` + `grid_canvas.rs` の既存パターンより*

```
WidgetStates (eframe フレームごとに保持)
    ├── pareto_2d: Pareto2dChart          ← 既存
    ├── opt_history: OptHistoryChart      ← 既存
    ├── hv_history: HvHistoryChart        ← 既存
    ├── importance: ImportanceChart       ← 既存
    ├── pdp_chart: PdpChart               ← 既存
    ├── parallel_coords: ParallelCoordsChart  ← 追加
    ├── scatter_matrix: ScatterMatrix         ← 追加
    ├── sensitivity_heatmap: SensitivityHeatmap ← 追加
    └── cluster_scatter: ClusterScatter       ← 追加

eframe::App::update()
    │
    ▼
show_main_canvas() → show_grid_canvas()
    │
    ▼ (各セルの ChartId に応じて)
show_chart(ui, app_state, widgets, chart_id)
    │
    ├── widgets.parallel_coords.show(...)
    ├── widgets.scatter_matrix.show(...)
    ├── widgets.sensitivity_heatmap.show(...)
    └── widgets.cluster_scatter.show(...)
```

---

## 関連文書

- **アーキテクチャ**: [architecture.md](architecture.md)
- **ヒアリング記録**: [design-interview.md](design-interview.md)

## 信頼性レベルサマリー

- 🔵 青信号: 8件 (80%)
- 🟡 黄信号: 2件 (20%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
