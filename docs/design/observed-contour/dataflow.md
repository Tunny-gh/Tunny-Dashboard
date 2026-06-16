# Observed Contour データフロー

**関連**: [architecture.md](architecture.md) / [interfaces.rs](interfaces.rs)

## 1. 計算トリガ

軸（X/Y/Value）・Coverage スライダー・Feasible only の変更、または初回表示時に、
`ObservedContourState` が自分の `pending_compute` をセットする（計算は軽量なので
Run ボタンではなく自動再計算。スライダーはドラッグ終了でトリガ）。

```
 user 操作（軸/値/スライダー変更）
  └─ render_chart: observed_contour::show() が選択を読み、
       署名（x,y,value,coverage,feasible_only）が前回結果と異なれば
       state.pending_compute = Some(ObservedContourRequest{...}); state.computing = true
```

## 2. ディスパッチ（poll_chart_work, バックグラウンド）

```
ChartId::ObservedContour:
  if let Some(req) = widgets.observed_contour.pending_compute.take():
    ctx = current_study
    cols:
      x_col   = ctx.view.numeric_column(req.x)
      y_col   = ctx.view.numeric_column(req.y)
      val_col = ctx.view.numeric_column(req.value)
    pts: Vec<[f64;3]> = (0..n)
        .filter(feasible_only ? feas.is_feasible(i) : true)
        .filter_map(|i| 全列が有限なら [x_i, y_i, val_i])
    spawn_task(move || {
        let surf = tunny_core::contour::observed_surface(&pts, req.n_grid, req.max_edge_ratio);
        AppMessage::ObservedContourDone(ObservedContourResult{
            x_name, y_name, value_name, surface: surf, points: pts(表示用),
        })
    })
```

注: `pts` は表示用の観測点重畳にも使うため、結果に同梱して UI へ返す
（UI 側で再抽出しても良いが、feasible フィルタの一貫性のため結果に持たせる）。

## 3. 完了ハンドリング（message_handler）

```
AppMessage::ObservedContourDone(result):
    widget_states.observed_contour.result = Some(result)
    widget_states.observed_contour.computing = false
AppMessage::ObservedContourFailed(err):   // 点不足・共線など
    widget_states.observed_contour.error_message = Some(err)
    widget_states.observed_contour.computing = false
```

## 4. キャンバス各アイテムへの伝播（app.rs）

既存様式に合わせ `ComputeSyncKind::ObservedContour` を追加し、
`propagate` で `w.observed_contour.adopt_compute_state(&global.observed_contour)`
（computing / result / error を取り込み、軸・値・スライダー等の UI 選択は各アイテム維持）。

## 5. 描画（render_chart → observed_contour::show）

```
1. 軸/値セレクタ（params∪objectives）、Coverage スライダー、Show points、Feasible only を描画
2. state.computing なら spinner、return
3. result が無ければプレースホルダ
4. result.surface.z（Vec<Vec<Option<f64>>>）を draw_heatmap_masked で描画
   - 値域は Some セルのみで value_range
   - 右脇に draw_colorbar_simple
5. Show points: result.points を value で色付けして重畳
   （Phase 2: hit_test_nearest で最近傍点クリック→ TrialDetailModal）
6. サブタイトル「blank = no data (not extrapolated)」
```

## 6. シーケンス図

```
User        render_chart        poll_chart        spawn_task         message_handler
 │  軸変更      │                    │                 │                    │
 │────────────▶│ pending_compute=   │                 │                    │
 │             │ Some, computing    │                 │                    │
 │             │───────────────────▶│ take()          │                    │
 │             │                    │ 列抽出           │                    │
 │             │                    │────────────────▶│ observed_surface   │
 │             │                    │                 │ (Delaunay+補間+mask)│
 │             │                    │                 │───────────────────▶│ result格納
 │             │                    │                 │                    │ computing=false
 │  再描画      │ draw_heatmap_masked│                 │                    │ propagate
 │◀────────────│ + 点重畳            │                 │                    │
```
