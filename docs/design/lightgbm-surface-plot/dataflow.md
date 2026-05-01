# LightGBM Surface Plot データフロー図

**作成日**: 2026-05-01
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連要件定義**: [requirements.md](../../spec/lightgbm-surface-plot/requirements.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実なフロー
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測によるフロー
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測によるフロー

---

## 1D PDP LightGBM フロー 🔵

**信頼性**: 🔵 *chart_registry.rs L249-314・pdp/api.rs・lgbm.rs より*

**関連要件**: REQ-001, REQ-002, REQ-013, REQ-021

```
┌─────────────────────────────────────────────────────────────┐
│ egui-app: pdp_chart.rs show()                               │
│  1. ユーザーが ModelType::RandomForest を選択               │
│  2. "Run PDP" ボタンクリック                                │
│  3. n_grid = 30 (REQ-021)                                   │
│  4. PdpComputeRequest { model_type: "random_forest", n_grid }│
│     → pending_compute にセット                              │
└──────────────────────┬──────────────────────────────────────┘
                       │ pending_compute を chart_registry が取得
                       ▼
┌─────────────────────────────────────────────────────────────┐
│ egui-app: chart_registry.rs ChartId::PdpChart               │
│  メインスレッドで x_matrix・y を TrialRow から抽出          │
│  spawn_task(tx, move || { ... })                            │
└──────────────────────┬──────────────────────────────────────┘
                       │ バックグラウンドスレッドで実行
                       ▼
┌─────────────────────────────────────────────────────────────┐
│ rust_core: pdp/api.rs compute_pdp_from_data()               │
│  model_type == "random_forest" ← 新規ディスパッチ (REQ-002) │
│  → compute_pdp_1d_lgbm(x_matrix, y, target_idx, 30)         │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│ rust_core: core/lgbm.rs compute_pdp_1d_lgbm() (REQ-001)     │
│                                                             │
│  1. 入力ガード:                                              │
│     n < 2 or n_grid < 2 or param_idx >= p → None            │
│                                                             │
│  2. LightGBM 学習:                                          │
│     config = LgbmRfConfig { num_iterations: 100, ..default }│
│     booster = train_lgbm_rf(x_matrix, y, &config)?           │
│                                                             │
│  3. グリッド生成:                                            │
│     col = x_matrix[:, param_idx]                            │
│     grid = pdp_linspace(min, max, n_grid=30)                │
│                                                             │
│  4. PDP値計算（全行を使った周辺化）:                         │
│     for v in grid:                                          │
│       rows = x_matrix.map(|r| r[param_idx] = v)            │
│       preds = lgbm_predict(&booster, &rows)                 │
│       pdp[v] = mean(preds)                                  │
│                                                             │
│  5. R² 計算:                                                │
│     mse = lgbm_mse(&booster, x_matrix, y)                   │
│     r² = mse_to_r_squared(mse, y)                           │
│                                                             │
│  6. 戻り値: Some((grid, values, r_squared))                 │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│ rust_core: pdp/api.rs（タプル → PdpResult1d 変換）          │
│  Some((grid, values, r²)) →                                 │
│    PdpResult1d { grid, values, r_squared: r²,               │
│                  y_upper: None, y_lower: None }             │
│  None → compute_pdp_from_matrix() (Ridge フォールバック)    │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│ egui-app: chart_registry.rs（PdpResult1d → messages変換）   │
│  AppMessage::PdpDone {                                      │
│    result: PdpResult::OneDim(messages::PdpResult1d {        │
│      x_values: r.grid,                                      │
│      y_values: r.values,                                    │
│      r2: Some(r.r_squared),                                 │
│      y_upper: r.y_upper,  // None                           │
│      y_lower: r.y_lower,  // None                           │
│      ice_lines: vec![],   // 空                             │
│    })                                                       │
│  }                                                          │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│ egui-app: message_handler.rs                                │
│  AppMessage::PdpDone → pdp_chart.result, computing=false    │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│ egui-app: pdp_chart.rs show_1d()                            │
│  R² 表示: "R²: 0.91 (Good)" (REQ-032)                      │
│  PDP 曲線（青線）描画                                        │
│  ICE ライン: なし（ice_lines=[]）(REQ-003)                  │
│  信頼区間バンド: なし（y_upper=None）(REQ-003)              │
└─────────────────────────────────────────────────────────────┘
```

---

## 2D PDP LightGBM フロー 🔵

**信頼性**: 🔵 *chart_registry.rs L316-347・pdp/api.rs・lgbm.rs の既存実装より*

**関連要件**: REQ-014, REQ-022, REQ-031

```
┌─────────────────────────────────────────────────────────────┐
│ egui-app: pdp_2d.rs show()                                  │
│  1. ユーザーが ModelType::RandomForest を選択               │
│  2. "Run 2D PDP" ボタンクリック                             │
│  3. n_grid = 30 if RandomForest else 20 (REQ-022)           │
│  4. Pdp2dComputeRequest { model_type: "random_forest", n_grid: 30 }│
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│ egui-app: chart_registry.rs ChartId::PdpChart2D             │
│  spawn_task: compute_pdp_2d(&p1, &p2, &obj, 30, "random_forest")│
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│ rust_core: pdp/api.rs compute_pdp_2d()                      │
│  model_type == "random_forest" → 既実装ディスパッチ         │
│  → compute_pdp_2d_lgbm(&x_matrix, &y, p1_idx, p2_idx, 30)  │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│ rust_core: core/lgbm.rs compute_pdp_2d_lgbm() (既実装)      │
│  x2d = x_matrix[:, [p1_idx, p2_idx]]  ← 2 特徴量のみ       │
│  booster = train_lgbm_rf(&x2d, y, &cfg{iter:100})?          │
│  grid1 = linspace(min1, max1, 30)                           │
│  grid2 = linspace(min2, max2, 30)                           │
│  z_values[30][30] = lgbm_predict(grid_points)               │
│  r² = mse_to_r_squared(lgbm_mse(...))                       │
│  → Some((grid1, grid2, z_values, r²))                       │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│ egui-app: chart_registry.rs（変換）                         │
│  AppMessage::Pdp2dDone(PdpResult2d {                        │
│    x_values: grid1,                                         │
│    y_values: grid2,                                         │
│    z_values,                                                │
│    uncertainties: None,  ← LightGBM は返さない              │
│  })                                                         │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│ egui-app: pdp_2d.rs show() 描画                             │
│  uncertainties == None → 単一ヒートマップ表示 (REQ-031)     │
│  draw_heatmap_values(30×30 グリッド)                        │
│  draw_colorbar()                                            │
└─────────────────────────────────────────────────────────────┘
```

---

## フォールバックフロー 🔵

**信頼性**: 🔵 *EDGE-002・REQ-002 フォールバック要件より*

```
compute_pdp_1d_lgbm()
    │
    ├── n < 2 or n_grid < 2 or param_idx 越境 → None
    │
    └── train_lgbm_rf() 失敗 → None（DLL なし等）
            │
            ▼
    pdp/api.rs: None にマッチ
            │
            ▼
    compute_pdp_from_matrix()  ← Ridge フォールバック
            │
            ▼
    PdpResult1d（Ridge 結果）
            │
            ▼
    chart_registry → AppMessage::PdpDone
            │
            ▼
    pdp_chart.show_1d() → PDP 曲線表示（Ridge ベース）
```

---

## n_grid による計算量の比較 🟡

**信頼性**: 🟡 *NFR-001/002 要件・LightGBM 計算特性から妥当な推測*

| モデル | 1D n_grid | 計算量（1D） | 2D n_grid | 計算量（2D） |
|---|---|---|---|---|
| Ridge | 50 | O(1) | 20 | O(1) |
| Kriging | 30 | O(N²) | 20 | O(N²×grid²) |
| Sparse Kriging | 30 | O(N²) | 20 | O(N²×grid) |
| **RandomForest** | **30** | **O(N×grid×trees)** | **30** | **O(N×grid²×trees/N_train²)** |

1D LightGBM の計算コスト内訳:
- 学習: O(N × D × T) — N=行数, D=特徴量数, T=イテレーション数(100)
- 推論: O(n_grid × N × T) — n_grid 回の N 行一括予測

---

## 状態遷移（UI） 🔵

**信頼性**: 🔵 *既存 pdp_chart.rs・pdp_2d.rs の computing フラグパターンより*

```
初期状態
  │
  ├─ ユーザー: Model=RandomForest, Run PDP クリック
  │         ↓
  │   computing = true
  │   pending_compute = Some(req)
  │         ↓
  │   chart_registry が pending を消費して spawn_task
  │         ↓
  │   バックグラウンドスレッドで compute_pdp_from_data()
  │         ↓
  │   AppMessage::PdpDone または Error
  │         ↓
  │   message_handler: computing = false, result = Some(...)
  │         ↓
  └─ 描画: show_1d() or AppMessage::Error 表示
```

## 関連文書

- **アーキテクチャ**: [architecture.md](architecture.md)
- **設計ヒアリング記録**: [design-interview.md](design-interview.md)
- **要件定義**: [requirements.md](../../spec/lightgbm-surface-plot/requirements.md)

## 信頼性レベルサマリー

- 🔵 青信号: 8件 (89%)
- 🟡 黄信号: 1件 (11%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質
