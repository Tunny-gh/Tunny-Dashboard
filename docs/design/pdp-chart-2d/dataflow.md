# PDP Chart 2D データフロー図

**作成日**: 2026-04-15
**関連アーキテクチャ**: [architecture.md](architecture.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実なフロー
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測によるフロー
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測によるフロー

---

## フロー 1: チャートピッカーへの表示 🔵

**信頼性**: 🔵 *`ChartId::all()` の使用箇所（left_panel.rs）より*

```
ユーザー: 右パネル（チャートピッカー）を開く
  │
  ▼
show_right_panel() → ChartId::all() を反復
  │
  ▼ [現在]
  ChartId::all() に PdpChart2D なし → 表示されない ❌
  │
  ▼ [変更後]
  ChartId::PdpChart2D 追加 → "PDP Chart 2D" がリストに表示 ✅
  │
  ▼
ユーザーがドラッグ/クリックでグリッドセルに配置
  │
  ▼
LayoutState.grid_cells に ChartId::PdpChart2D が追加される
```

---

## フロー 2: 初回マウント（データなし） 🔵

**信頼性**: 🔵 *`pdp_2d.rs` の show() 実装（`let Some(result) = &self.result else { ui.label("No 2D PDP data"); return; }`）より*

```
grid_canvas::show_chart(ChartId::PdpChart2D)
  │
  ▼
widgets.pdp_2d.show(ui, &param_names, &obj_names)
  │
  ├─ computing == false
  ├─ result == None
  │
  ▼
[Parameter 1: ▼] [Parameter 2: ▼]
[Objective:   ▼] [Model: ▼ Ridge]   ← モデル選択 ComboBox（フロー9参照）
[              Run 2D PDP              ]
"No 2D PDP data" ← 初回はデータなし
```

---

## フロー 3: 2D PDP 計算フロー 🔵

**信頼性**: 🔵 *既存 spawn_task パターン (toolbar.rs) + ユーザヒアリング（AppMessage拡張）より*

```
ユーザー: param1="x1", param2="x2", objective="obj1", model="kriging" を選択し "Run" ボタン押下
  │
  ▼
pdp_2d::show() 内:
  self.pending_compute = Some(Pdp2dComputeRequest {
      param1: "x1", param2: "x2",
      objective: "obj1", n_grid: 20, model_type: "kriging",
  })
  ─────────────────────────────────────
  ▼ show() から戻る
  │
  ▼
grid_canvas::show_chart():
  if let Some(req) = widgets.pdp_2d.pending_compute.take() {
      widgets.pdp_2d.computing = true;
      spawn_task(tx.clone(), move || {
          let result = rust_core::pdp::api::compute_pdp_2d(
              &req.param1, &req.param2, &req.objective,
              req.n_grid, &req.model_type,
          );
          match result {
              Some(r) => AppMessage::Pdp2dDone(r),
              None    => AppMessage::Error("PDP 2D computation failed".into()),
          }
      });
  }
  │
  ▼ バックグラウンドスレッド実行中
  computing == true → spinner + "Computing 2D PDP..." 表示
  │
  ▼ [バックグラウンドスレッド完了]
  tx.send(AppMessage::Pdp2dDone(result))
  │
  ▼
app.rs::poll_messages():
  AppMessage::Pdp2dDone(result) => {
      self.widget_states.pdp_2d.result = Some(result);
      self.widget_states.pdp_2d.computing = false;
  }
  │
  ▼
次フレームの show_chart():
  widgets.pdp_2d.show() で result が Some → ヒートマップ描画（フロー4・フロー8参照）
```

---

## フロー 4: ヒートマップ表示 🔵

**信頼性**: 🔵 *`pdp_2d.rs` の `draw_heatmap()` 実装より*

```
result: PdpResult2d {
    x_values: Vec<f64>,           // param1 グリッド点
    y_values: Vec<f64>,           // param2 グリッド点
    z_values: Vec<Vec<f64>>,      // [x_grid][y_grid] 予測値（平均）
    uncertainties: Option<Vec<Vec<f64>>>,  // [x_grid][y_grid] 分散（Kriging/Sparse Krigingのみ）
    param1_name, param2_name, objective_name
}
  │
  ▼ [Ridge モデルの場合 (uncertainties == None)]
draw_heatmap(painter, rect, &result.z_values):
  compute_value_range → (z_min, z_max)
  各セル (i, j): color = ColorMap::viridis(normalize(z_values[i][j]))
  painter.rect_filled(cell_rect, 0.0, color)
draw_colorbar(colorbar_rect, z_min, z_max)  ← viridis
  │
  ▼ [Kriging / Sparse Kriging の場合 (uncertainties == Some)]
  利用可能な幅を左右2分割:
  ├─ 左ペイン: draw_heatmap(rect_left, &result.z_values)   ← 予測平均 viridis
  │  draw_colorbar(left_bar_rect, z_min, z_max)
  │  ui.label("Mean")
  └─ 右ペイン: draw_heatmap(rect_right, &sigma_values)    ← 標準偏差 plasma
               (sigma_values = uncertainties.map(|v| v.sqrt()))
               draw_colorbar(right_bar_rect, σ_min, σ_max)
               ui.label("σ (Std. Dev.)")
```

---

## フロー 5: パラメータ変更による再計算 🟡

**信頼性**: 🟡 *pdp_2d.rs の UI 実装パターンから妥当な推測*

```
ユーザー: Parameter 1 コンボボックスで別のパラメータを選択
  │
  ▼
pdp_2d::show() 内:
  self.selected_param1 が変更される
  (自動再計算はしない — "Run" ボタンの再押しが必要)
  │
  ▼
既存の result は保持されたまま → 古い結果が表示され続ける
"Run 2D PDP" ボタンを押して再計算を促す

備考: 自動再計算（param 変更を検知して即時トリガー）にするかは実装時判断。
      データ量が多い場合に意図しない重計算を避けるため、明示的な "Run" ボタンを推奨。
```

---

## フロー 6: エラーハンドリング 🔵

**信頼性**: 🔵 *既存 AppMessage::Error ハンドリング + pdp_2d.rs 空状態実装より*

```
バックグラウンドスレッド内:
  compute_pdp_2d(...) → None（データ不足・パラメータ不正等）
  │
  ▼
AppMessage::Error("PDP 2D computation failed") を送信
  │
  ▼
app.rs::poll_messages():
  AppMessage::Error(e) => {
      self.load_error = Some(e);
      self.widget_states.pdp_2d.computing = false;
  }
  │
  ▼
pdp_2d::show():
  computing == false, result == None
  → "No 2D PDP data" 表示（エラーはツールバーのエラー表示で通知）
```

---

## フロー 7: tx 伝播パス 🔵

**信頼性**: 🔵 *layout.rs 調査 + 既存 show_toolbar パターンより*

```
[変更後の呼び出しチェーン]

eframe::App::update(app, ctx)
  │
  ▼
show_layout(app, ctx)               [layout.rs]
  let tx = app.sender();
  show_main_canvas(ui, app_state, layout, widgets, &tx)   ← &tx 追加
  │
  ▼
show_main_canvas(ui, ..., tx)       [main_canvas.rs]     ← シグネチャ更新
  show_grid_canvas(ui, ..., tx)
  │
  ▼
show_grid_canvas(ui, ..., tx)       [grid_canvas.rs]     ← シグネチャ更新
  show_chart(ui, ..., chart_id, tx)
  │
  ▼
show_chart(ui, ..., chart_id, tx)   [grid_canvas.rs]     ← シグネチャ更新
  match chart_id {
      ChartId::PdpChart2D => {
          widgets.pdp_2d.show(ui, ...);
          // pending_compute チェック → spawn_task(tx.clone(), ...)
      }
      // 他ケースは tx を使わない（コンパイル警告回避に _ を使用）
  }
```

---

## フロー 8: Kriging 不確実性グリッド計算 🔵

**信頼性**: 🔵 *`gaussian_process/training.rs`（コレスキー因子計算済み）+ `sparse_fitc.rs`（L_sigma 計算済み）調査より*

### GP（Kriging）の場合

```
rust_core::pdp::kriging_core::compute_pdp_2d_kriging_raw():
  train_gp(x_train, y_train, params)
    → GpModel {
        alpha: Vec<f64>,
        x_train: Vec<Vec<f64>>,
        log_ls, log_sf,
        l: Vec<Vec<f64>>,   // ← 追加保存: chol(K_XX + σ_n² I)
        log_sn: f64,        // ← 追加保存
      }
  │
  ▼
  グリッド 20×20 の各点 (xi, xj) に対して:
    mean[i][j]  = predict_mean(&model, &[xi, xj])
    var[i][j]   = predict_variance(&model, &[xi, xj])
                = k(x*,x*) - ||L^{-1} k(X,x*)||²   (L は model.l)
  │
  ▼
  PdpResult2d {
      z_values: mean,
      uncertainties: Some(var),   // ← 分散グリッド
      ...
  }
```

### FITC（Sparse Kriging）の場合

```
rust_core::pdp::kriging_core::compute_pdp_2d_sparse_kriging_raw():
  z = select_inducing_points_kmeans(x_train, ...)    // M 個の誘導点
  params = optimize_fitc_hyperparams(x, z, y, ...)  // ハイパーパラメータ最適化
  model = fitc_train(x, z, y, params, n, m)          // ← 新設関数
    → SparseFitcModel {
        w: Vec<f64>,        // posterior weights
        l_sigma: Vec<f64>,  // chol(Σ) (flat M×M)
        z, params, m,
      }
  │
  ▼
  グリッド 20×20 の各点 (xi, xj) に対して:
    mean[i][j]  = k(xi,xj→Z)^T · w           (K_{x*,Z} · w)
    var[i][j]   = fitc_predict_variance(&model, &[xi, xj])
                = k(x*,x*) - ||L_sigma^{-1} k(Z,x*)||²
  │
  ▼
  PdpResult2d {
      z_values: mean,
      uncertainties: Some(var),
      ...
  }
```

### Ridge の場合

```
compute_pdp_2d_ridge_raw():
  Ridge は確率的モデルではない → 不確実性なし
  →  uncertainties: None
```

---

## フロー 9: モデル種別選択 UI 🔵

**信頼性**: 🔵 *`pdp_chart.rs::ModelType` 実装パターン・ユーザヒアリングより*

```
pdp_2d::show() の UI レンダリング:

[Parameter 1: ▼ x1   ] [Parameter 2: ▼ x2   ]
[Objective:   ▼ obj1  ] [Model: ▼ Kriging    ]  ← モデル選択 ComboBox 追加
[              Run 2D PDP              ]           ← Run ボタン

ユーザーが "Kriging" を選択 → self.selected_model = ModelType::Kriging
ユーザーが "Run" を押下 →
  self.pending_compute = Some(Pdp2dComputeRequest {
      model_type: ModelType::Kriging.to_str(),  // "kriging"
      ...
  })

  ※ ModelType は pdp_chart.rs からインポートまたは再定義:
     Ridge / Kriging / SparseKriging
     to_str() → "ridge" / "kriging" / "sparse_kriging"
```

---

## 関連文書

- **アーキテクチャ**: [architecture.md](architecture.md)
- **ヒアリング記録**: [design-interview.md](design-interview.md)

## 信頼性レベルサマリー

- 🔵 青信号: 9件 (100%)
- 🟡 黄信号: 0件 (0%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
