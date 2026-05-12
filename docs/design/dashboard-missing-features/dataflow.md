# ダッシュボード不足機能 データフロー図

**作成日**: 2026-05-12
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連要件定義**: [requirements.md](../../spec/dashboard-missing-features/requirements.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 現行コードと要件から確実に導けるフロー
- 🟡 **黄信号**: 現行コードにない API 仮定を含むフロー
- 🔴 **赤信号**: 実装検証が未了の仮説フロー

---

## F-001 CSV Export フロー 🔵

**信頼性**: 🔵 *toolbar.rs・app.rs・io/export.rs より*

```text
┌──────────────────────────────────────────────────────────────┐
│ egui-app: toolbar.rs                                         │
│  1. ユーザーが "Export CSV" を開く                           │
│  2. menu_button で All / Selected / Pareto を選択            │
│  3. ToolbarAction::ExportCsv(target) を push                 │
└──────────────────────────┬───────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────┐
│ egui-app: app.rs apply_toolbar_actions()                     │
│  1. current_study を取得                                      │
│  2. selected_indices / pareto_indices を収集                  │
│  3. io::export::save_csv_for_current_study(...) を呼ぶ        │
└──────────────────────────┬───────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────┐
│ egui-app: io/export.rs                                        │
│  1. select_rows_for_export() で TrialRow を抽出               │
│  2. CSV 文字列へエンコード                                    │
│  3. rfd::FileDialog::save_file()                              │
│  4. std::fs::write(path, csv)                                 │
└──────────────────────────┬───────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────┐
│ UI                                                            │
│  成功: 軽い success 表示                                      │
│  失敗: Error(String) または load_error 相当のメッセージ表示   │
└──────────────────────────────────────────────────────────────┘
```

---

## F-002 Comparison Study 追加 + F-007 Diff 更新フロー 🔵

**信頼性**: 🔵 *app.rs・messages.rs・message_handler.rs・comparison_panel.rs より*

```text
┌──────────────────────────────────────────────────────────────┐
│ toolbar.rs                                                    │
│  Add Comparison Study クリック                                │
│  → ToolbarAction::AddComparisonStudy                          │
└──────────────────────────┬───────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────┐
│ app.rs                                                        │
│  1. pick_file()                                               │
│  2. dispatch_load_comparison_study(path, base_meta, tx)       │
└──────────────────────────┬───────────────────────────────────┘
                           │ バックグラウンド
                           ▼
┌──────────────────────────────────────────────────────────────┐
│ io/study_worker.rs                                            │
│  1. Journal parse                                              │
│  2. 比較対象 Study を選定                                      │
│  3. StudyContext を構築                                        │
│  4. AppMessage::ComparisonStudyLoaded { context }             │
│     または ComparisonStudyLoadFailed(err)                     │
└──────────────────────────┬───────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────┐
│ state/message_handler.rs                                      │
│  1. comparison_studies.push(context)                           │
│  2. comparison_mode = true                                     │
│  3. comparison_colors を再生成                                 │
└──────────────────────────┬───────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────┐
│ ui/comparison_panel.rs                                        │
│  1. Stats / HV / Pareto / KDE / Diff を表示                   │
│  2. Diff タブで comparison_metrics.rs を呼ぶ                  │
│  3. 互換なら差分表、非互換なら警告メッセージ                  │
└──────────────────────────────────────────────────────────────┘
```

メイン Study 変更時のリセット:

```text
ToolbarAction::SelectStudy(meta)
    ↓
app_state.current_study 差し替え
    ↓
comparison_base_study != meta.study_id ?
    ↓ Yes
reset_comparison_session()
    ↓
Diff タブも空状態へ戻す
```

---

## F-003 ピン留め + セッション保存フロー 🔵

**信頼性**: 🔵 *trial_table.rs・session.rs より*

```text
┌──────────────────────────────────────────────────────────────┐
│ ui/widgets/trial_table.rs                                     │
│  1. 行頭の Pin ボタンをクリック                               │
│  2. app_state.toggle_pinned_trial(trial_id)                   │
└──────────────────────────┬───────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────┐
│ state/app_state.rs                                            │
│  1. pinned_trials に追加 / 削除                                │
│  2. 最大20件超過時は Err(MaxPinnedReached)                    │
│  3. effective_visible_ids() を再計算                          │
└──────────────────────────┬───────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────┐
│ render_chart / trial_table / pdp_chart                        │
│  実効可視集合 selected ∪ pinned を使って再描画                │
└──────────────────────────┬───────────────────────────────────┘
                           │
                           ├────────────── Save Session ───────┐
                           │                                   │
                           ▼                                   ▼
┌──────────────────────────────────────────────────────────────┐
│ app.rs                                                        │
│  SessionSnapshot::new(...) を作成                             │
│  snapshot.pinned_trials = app_state.pinned_trials.clone()     │
└──────────────────────────┬───────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────┐
│ io/session.rs                                                 │
│  JSON serialize / deserialize                                  │
└──────────────────────────┬───────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────┐
│ Load Session 復元                                              │
│  pinned_trials を AppState へ戻す                              │
└──────────────────────────────────────────────────────────────┘
```

---

## F-004 / F-006 Selection-Linked Overlay フロー 🔵

**信頼性**: 🔵 *pdp_chart.rs・pareto_2d.rs・parallel_coords.rs より*

```text
Pareto Scatter 2D でドラッグ開始
    ↓
widget が plot 座標系の矩形を保持
    ↓
ドラッグ終了時に display_rows 内の trial_id を抽出
    ↓
AppState.selected_indices を更新
    ↓
effective_visible_ids = selected ∪ pinned
    ↓
TrialTable / Pareto / PDP Overlay / 将来の各 chart が再描画
```

Parallel Coordinates の場合:

```text
軸上のドラッグ開始
    ↓
brush_ranges[name] = Some((min_norm, max_norm))
    ↓
全軸のブラシ条件を AND で結合
    ↓
条件を満たす trial_id 一覧を selected_indices に反映
    ↓
実効可視集合を使って dashboard 全体が再描画
```

PDP overlay 連動の修正点:

```text
現状: render_chart.rs → pdp_chart.show(..., trial_rows 全件)
変更後: render_chart.rs → display_rows = filter_rows_for_display(...)
                          pdp_chart.show(..., display_rows)
```

これにより `Show data` の点群は Brushing と Pinning に追従する。

---

## F-005 Surface Plot 計算フロー 🟡

**信頼性**: 🟡 *既存 PdpChart2D 非同期パターンと既存設計文書より*

```text
┌──────────────────────────────────────────────────────────────┐
│ ui/widgets/surface_plot.rs                                    │
│  1. X/Y/Objectve/Model/RenderMode を選択                      │
│  2. Run クリック                                               │
│  3. pending_compute = Some(SurfacePlotComputeRequest)         │
└──────────────────────────┬───────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────┐
│ ui/poll_chart.rs                                              │
│  1. pending_compute を take                                    │
│  2. widgets.surface_plot.computing = true                     │
│  3. spawn_task(tx, move || compute_surface_grid(...))         │
└──────────────────────────┬───────────────────────────────────┘
                           │ バックグラウンド
                           ▼
┌──────────────────────────────────────────────────────────────┐
│ rust_core                                                     │
│  1. surrogate model で z grid を計算                           │
│  2. SurfacePlotResult { x_values, y_values, z_values, r2 }    │
│  3. AppMessage::SurfacePlotDone(result)                        │
└──────────────────────────┬───────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────┐
│ state/message_handler.rs                                      │
│  1. widgets.surface_plot.result = Some(result)                │
│  2. widgets.surface_plot.computing = false                    │
└──────────────────────────┬───────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────┐
│ ui/widgets/surface_plot.rs                                    │
│  Heatmap / Contour モードで z grid を描画                      │
└──────────────────────────────────────────────────────────────┘
```

---

## F-008 PNG 保存フロー 🟡

**信頼性**: 🟡 *grid_canvas.rs と viewport screenshot API 利用想定より*

```text
┌──────────────────────────────────────────────────────────────┐
│ ui/grid_canvas.rs                                             │
│  1. 各セル描画時に last_cell_rects[(row,col)] = cell_rect     │
│  2. セルヘッダーの ... → Save as PNG                          │
│  3. FileDialog::save_file() で保存先を取得                    │
│  4. pending_capture = Some(ChartCaptureRequest)               │
└──────────────────────────┬───────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────┐
│ app.rs / chart_capture.rs                                     │
│  1. viewport screenshot を要求                                │
│  2. 取得した RGBA 画像から cell_rect 領域を crop              │
│  3. PNG として保存                                             │
└──────────────────────────┬───────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────┐
│ UI                                                            │
│  成功: 保存完了通知                                             │
│  失敗: API unavailable / crop failure / write failure を通知   │
└──────────────────────────────────────────────────────────────┘
```

---

## 状態遷移サマリー 🔵

**信頼性**: 🔵 *現行 widget state / app state 分離設計より*

```text
初期状態
  │
  ├─ CSV Export           → 同期実行 → 完了
  ├─ Add Comparison Study → 非同期 parse → comparison_studies 更新
  ├─ Pin Trial            → AppState.pinned_trials 更新
  ├─ Brush / Link         → AppState.selected_indices 更新
  ├─ Surface Plot Run     → pending_compute → computing=true → result 反映
  └─ Save as PNG          → pending_capture → screenshot/crop → 完了
```

---

## 信頼性レベルサマリー

- 🔵 青信号: 5件 (83%)
- 🟡 黄信号: 1件 (17%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
