# chart-implementation アーキテクチャ設計

**作成日**: 2026-04-12
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *既存コード調査・ユーザヒアリングより*

現在 `grid_canvas.rs` の `show_chart()` で "not yet implemented" と表示されている4つのチャートを実装する。対象は以下の通り（ParetoScatter3D は今回スコープ外）:

1. **ParallelCoordinates** — 平行座標プロット
2. **ScatterMatrix** — 散布図行列
3. **SensitivityHeatmap** — 感度分析ヒートマップ
4. **ClusterScatter** — クラスタリング散布図（k-means + PCA 2D投影）

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存実装パターン（pareto_2d, opt_history 等）より*

- **パターン**: immediate mode widget パターン（egui 0.30）
- 各チャートは `struct XxxWidget` + `fn show(&mut self, ui: &mut egui::Ui, ...)` の形式
- `WidgetStates` が全ウィジェットをフィールドとして保持し、`grid_canvas.rs` の `show_chart()` から呼び出す

## コンポーネント構成

### 既存スタブ（実装済み構造体・ヘルパー） 🔵

**信頼性**: 🔵 *既存コード調査より*

| ファイル | 構造体 | 実装済み要素 | 未実装 |
|----------|--------|--------------|--------|
| `egui-app/src/ui/widgets/parallel_coords.rs` | `ParallelCoordsChart` | normalize_value, normalized_to_screen_y, build_axis_order, denormalize_brush_range, ordered_brush_range | `show()` |
| `egui-app/src/ui/widgets/scatter_matrix.rs` | `ScatterMatrix` | draw_scatter_cell, draw_histogram_cell, draw_correlation_cell, compute_histogram, compute_correlation, correlation_color, data_to_screen | `show()` |
| `egui-app/src/ui/widgets/sensitivity_heatmap.rs` | `SensitivityHeatmap` | diverging_colormap | `show()` |
| `egui-app/src/ui/widgets/cluster_scatter.rs` | `ClusterScatter` | cluster_labels_valid, ClusterStats, ClusteringResult | `show()`, k-means, PCA |

### データソース 🔵

**信頼性**: 🔵 *`app_state.rs` 調査より*

```
AppState
├── current_study: Option<StudyContext>
│   ├── trial_rows: Vec<TrialRow>       // 全試行データ
│   ├── meta.objective_names: Vec<String>
│   └── meta.param_names: Vec<String>
├── sensitivity_result: Option<SensitivityResult>
│   ├── param_names: Vec<String>
│   ├── objective_names: Vec<String>
│   └── spearman: Vec<Vec<f64>>         // [param][obj]
└── cluster_result: Option<ClusterResult>
    ├── labels: Vec<i32>               // 各試行のクラスタ番号
    └── n_clusters: usize
```

### 追加依存クレート（ClusterScatter 用） 🔵

**信頼性**: 🔵 *ユーザヒアリング「外部クレート使用」より*

`egui-app/Cargo.toml` に追加:
```toml
linfa = "0.7"
linfa-clustering = "0.7"   # k-means
linfa-reduction = "0.7"    # PCA
ndarray = "0.15"
```

k-means は `app_state.rs` の `ClusterResult` が既に `labels` と `n_clusters` を持つため、ClusterScatter ウィジェット側では PCA 2D投影のみを内部計算する（もしくは `ClusterResult` を拡張して PCA 座標を持たせる）。

**方針**: ウィジェット内部でキャッシュした PCA 投影を保持し、データが変化したときのみ再計算する。

## WidgetStates 拡張 🔵

**信頼性**: 🔵 *既存 `widget_states.rs` 調査より*

`egui-app/src/ui/widget_states.rs` に4フィールドを追加:

```rust
pub struct WidgetStates {
    // 既存フィールド...
    pub parallel_coords: ParallelCoordsChart,
    pub scatter_matrix: ScatterMatrix,
    pub sensitivity_heatmap: SensitivityHeatmap,
    pub cluster_scatter: ClusterScatter,
}
```

`Default` 実装も合わせて更新。

## grid_canvas.rs の修正箇所 🔵

**信頼性**: 🔵 *`grid_canvas.rs` の `show_chart()` 調査より*

現在の "not yet implemented" ラベルを実際の `show()` 呼び出しに置換:

```rust
ChartId::ParallelCoordinates => {
    widgets.parallel_coords.show(ui, &trial_rows, &param_names, &obj_names);
}
ChartId::ScatterMatrix => {
    widgets.scatter_matrix.show(ui, &trial_rows, &param_names, &obj_names);
}
ChartId::SensitivityHeatmap => {
    widgets.sensitivity_heatmap.show(ui, sensitivity.as_ref());
}
ChartId::ClusterScatter => {
    widgets.cluster_scatter.show(ui, &trial_rows, app_state.cluster_result.as_ref(), &param_names);
}
```

## 各チャートの実装詳細

### ParallelCoordinates 🔵

**信頼性**: 🔵 *`parallel_coords.rs` 既存コード調査より*

- `egui::Painter` でライン描画（`painter.line_segment()`）
- 軸はパラメータ名 + 目的関数名を並べる
- 各試行を多段折れ線で描画
- ブラシ（範囲選択）はドラッグで `brush_ranges: HashMap<usize, (f64, f64)>` に格納
- ハイライト試行（選択中 trial）を太線で描画

**`show()` シグネチャ**:
```rust
pub fn show(
    &mut self,
    ui: &mut egui::Ui,
    trial_rows: &[TrialRow],
    param_names: &[String],
    obj_names: &[String],
)
```

### ScatterMatrix 🔵

**信頼性**: 🔵 *`scatter_matrix.rs` 既存コード調査より*

- 利用可能な描画関数がすべて実装済み
- `show()` は利用可能領域を N×N グリッドに分割し、各セルで適切な描画関数を呼ぶ
- 対角: ヒストグラム（`draw_histogram_cell`）
- 上三角: 相関係数（`draw_correlation_cell`）
- 下三角: 散布図（`draw_scatter_cell`）

**`show()` シグネチャ**:
```rust
pub fn show(
    &mut self,
    ui: &mut egui::Ui,
    trial_rows: &[TrialRow],
    param_names: &[String],
    obj_names: &[String],
)
```

### SensitivityHeatmap 🔵

**信頼性**: 🔵 *`sensitivity_heatmap.rs` + `app_state.rs` 調査より*

- `SensitivityResult.spearman` (params × objectives の f64 行列) を色付きグリッドで表示
- セル色は `diverging_colormap(v)` で決定（既実装）
- `egui::Painter::rect_filled` で各セルを塗る
- 行ヘッダ: パラメータ名、列ヘッダ: 目的関数名

**`show()` シグネチャ**:
```rust
pub fn show(
    &mut self,
    ui: &mut egui::Ui,
    sensitivity: Option<&SensitivityResult>,
)
```

### ClusterScatter 🟡

**信頼性**: 🟡 *ユーザヒアリング（外部クレート方針）+ 既存コード調査より*

- `app_state.cluster_result` から `labels` と `n_clusters` を取得
- `trial_rows` のパラメータ値を linfa + ndarray で PCA 2D投影
- 投影結果を `ClusterScatter` 内部フィールドとしてキャッシュ（`cached_pca: Option<Vec<[f32; 2]>>`）
- `egui_plot::Plot` + `egui_plot::Points` でクラスタ別に色分け散布図を描画
- データ変化検知: `trial_rows.len()` と `cluster_result.n_clusters` が変化したらキャッシュ無効化

**`show()` シグネチャ**:
```rust
pub fn show(
    &mut self,
    ui: &mut egui::Ui,
    trial_rows: &[TrialRow],
    cluster_result: Option<&ClusterResult>,
    param_names: &[String],
)
```

## ディレクトリ構造 🔵

**信頼性**: 🔵 *既存プロジェクト構造より*

```
egui-app/src/
├── state/
│   └── app_state.rs              # SensitivityResult, ClusterResult (変更なし)
├── ui/
│   ├── grid_canvas.rs            # show_chart() の4箇所を修正
│   ├── widget_states.rs          # 4フィールド追加
│   └── widgets/
│       ├── parallel_coords.rs    # show() 追加
│       ├── scatter_matrix.rs     # show() 追加
│       ├── sensitivity_heatmap.rs # show() 追加
│       └── cluster_scatter.rs    # show() + PCA キャッシュ追加
egui-app/
└── Cargo.toml                    # linfa 系クレート追加
```

## スコープ外 🔵

**信頼性**: 🔵 *ユーザヒアリングより*

- **ParetoScatter3D**: wgpu GPU レンダリングが必要なため今回スコープ外

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **ヒアリング記録**: [design-interview.md](design-interview.md)

## 信頼性レベルサマリー

- 🔵 青信号: 12件 (86%)
- 🟡 黄信号: 2件 (14%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
