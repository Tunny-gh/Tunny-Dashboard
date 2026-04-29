# AHP データフロー図

**作成日**: 2026-04-29
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連要件定義**: [../../spec/ahp/requirements.md](../../spec/ahp/requirements.md)

**【信頼性レベル凡例】**:

- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実なフロー
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測によるフロー
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測によるフロー

---

## システム全体のデータフロー 🔵

**信頼性**: 🔵 _既存 egui-app アーキテクチャ・ユーザヒアリングより_

```
ユーザー操作（AhpChart.show_rank_chart()）
  │
  ├─ 一対比較行列入力（DragValue、上三角 Saaty 1-9）
  │    └─ pairwise フィールドの更新のみ（即時プレビューなし）
  │
  └─ "Run" ボタン押下
       └─ 計算フロー（compute_ahp 実行）
```

---

## フロー 1: 計算実行フロー 🔵

**信頼性**: 🔵 _既存 TOPSIS/VIKOR spawn_task パターン・ユーザヒアリングより_

**関連要件**: REQ-AHP-001, REQ-AHP-004, REQ-AHP-007, REQ-AHP-010, REQ-AHP-013

```
[UI: AhpChart.show_rank_chart()]
  "Run" ボタン押下
    │
    ▼
  pending_compute = Some(AhpComputeRequest {
      objectives:      Vec<f64>,   // フラット行優先 [n_trials × n_objectives]
      n_trials:        usize,
      n_objectives:    usize,
      pairwise_matrix: Vec<f64>,   // 上三角 row-major, len = n*(n-1)/2
      is_minimize:     Vec<bool>,
  })
  ahp_chart.computing = true
    │
    ▼ (次フレームで chart_registry.rs がポーリング)
    │
[chart_registry.rs: if let Some(req) = widget_states.ahp_chart.pending_compute.take()]
    │
    ▼
  spawn_task(tx.clone(), move || {
      tunny_core::mcdm::ahp::compute_ahp(
          &objectives, n_trials, n_objectives, &pairwise_matrix, &is_minimize
      )
  })
    │
    ▼ (std::thread で非同期実行)
    │
[rust_core: compute_ahp()]
    │
    ├─ validate_inputs()          // n_trials, pairwise_matrix 長さ整合チェック
    ├─ filter_valid_indices()     // NaN を含む試行を除外
    │
    ├─ 一対比較行列フル展開 (上三角 → n×n、対角=1.0、下三角=逆数)
    │
    ├─ 優先度ベクトル導出 (固有ベクトル近似法):
    │    ├─ Step1: 各列を列合計で除算 → 正規化行列 B
    │    └─ Step2: 各行の平均 → priority_vector w [n_objectives]
    │
    ├─ 整合性チェック:
    │    ├─ λmax = Σ_j (A のj列合計 × w[j])
    │    ├─ CI = (λmax - n) / (n - 1)   ※ n=1,2 は CI=0
    │    ├─ RI = RI_TABLE[min(n, 5)]
    │    ├─ CR = if RI > 0 { CI / RI } else { 0.0 }
    │    └─ is_consistent = CR <= 0.10
    │
    ├─ Min-Max 正規化 + 加重和スコア計算:
    │    ├─ min_j / max_j: 有効試行のみで計算
    │    ├─ per objective j:
    │    │    is_minimize[j]=true:  norm = (max_j - v) / (max_j - min_j)
    │    │    is_minimize[j]=false: norm = (v - min_j) / (max_j - min_j)
    │    │    max_j == min_j:       norm = 0.0
    │    └─ score[i] = Σ_j w[j] × norm[i][j]  (NaN 試行は score=0.0)
    │
    ├─ ranked_indices: score 降順ソート、NaN 試行は末尾
    │
    └─ Ok(AhpResult {
           priority_vector, scores, ranked_indices,
           lambda_max, ci, ri, cr, is_consistent,
           duration_ms
       })
    │
    ▼ (Ok の場合)
  AppMessage::AhpDone(AhpResult { ... })

    ▼ (Err の場合)
  AppMessage::Error(format!("AHP computation failed: {e}"))
    │
    ▼
  tx.send(message)  // SyncSender<AppMessage> でメインスレッドに送信
```

---

## フロー 2: メッセージ受信・状態更新フロー 🔵

**信頼性**: 🔵 _既存 message_handler.rs パターン・ユーザヒアリングより_

**関連要件**: REQ-AHP-013-A, REQ-AHP-013-B

```
[app.rs: poll_messages()]
  rx.try_recv() で AppMessage を受信
    │
    ▼
[message_handler.rs: handle_message()]
    │
  AppMessage::AhpDone(result) の場合:
    │
    ├─ app_state.ahp_result = Some(result)
    └─ widget_states.ahp_chart.computing = false
    │
    ▼
  AppMessage::Error(msg) の場合:
    ├─ widget_states.ahp_chart.computing = false
    └─ widget_states.error_message = Some(msg)
```

---

## フロー 3: UI 描画フロー 🔵

**信頼性**: 🔵 _REQ-AHP-022〜025・ユーザヒアリングより_

**関連要件**: REQ-AHP-020, REQ-AHP-021, REQ-AHP-022, REQ-AHP-024, REQ-AHP-025

```
[chart_registry.rs: 次フレーム]
    │
    ├─ AhpChart.show_rank_chart(ui, obj_names, &app_state.ahp_result)  [ChartId::AhpRankChart]
    │     │
    │     ├─ 一対比較行列グリッド表示:
    │     │    for i in 0..n_objectives:
    │     │      for j in 0..n_objectives:
    │     │        if i == j: Label("1.0")
    │     │        if i < j:  DragValue (上三角, 1.0〜9.0, 入力値)
    │     │        if i > j:  Label(format!("{:.3}", 1.0 / pairwise[upper_idx]))
    │     │
    │     ├─ "Run" ボタン（computing=true の間は非活性スピナー表示）
    │     │
    │     └─ match &app_state.ahp_result {
    │            None → (何も表示しない)
    │            Some(r) → {
    │              CR 表示:
    │                if r.is_consistent:
    │                  Label("CR = {:.3}  ✓ Consistent", color=GREEN)
    │                else:
    │                  Label("CR = {:.3}  ⚠ Inconsistent (CR > 0.10)", color=RED)
    │
    │              優先度ベクトルバーチャート:
    │                for j in 0..n_objectives:
    │                  Label("{obj_names[j]}: {:.3}", r.priority_vector[j])
    │                  横バー(幅 = r.priority_vector[j] × max_width, 色=#0c6ac0)
    │            }
    │          }
    │
    └─ AhpChart.show_table(ui, obj_names, &app_state.ahp_result, trial_rows)  [ChartId::AhpTable]
          │
          ├─ Top5/10/20 コンボボックス
          │
          └─ match &app_state.ahp_result {
                 None → "Run を押してください" 表示
                 Some(r) → {
                   テーブルヘッダ: 順位 | Trial ID | AHP Score | obj_names[0] | ... | obj_names[n-1]
                   for (rank, &idx) in r.ranked_indices[..top_n].iter().enumerate():
                     行: rank+1 | idx | r.scores[idx] | trial_rows[idx][0] | ... | trial_rows[idx][n-1]
                 }
               }
```

---

## フロー 4: Study 切替リセットフロー 🔵

**信頼性**: 🔵 _REQ-AHP-027・既存 StudySelected ハンドラパターンより_

**関連要件**: REQ-AHP-027, REQ-AHP-013-B

```
[UI: Study セレクタ切替]
    │
    ▼
  AppMessage::StudySelected(study_id)
    │
    ▼
[message_handler.rs: handle_message()]
    │
  AppMessage::StudySelected の場合:
    │
    ├─ app_state.selected_study = Some(study_id)
    ├─ app_state.ahp_result = None          // AHP 結果クリア
    ├─ widget_states.ahp_chart = AhpChart::default()
    │    // pairwise = vec![1.0; n*(n-1)/2], computing=false, pending=None
    └─ (既存: mcdm_result クリア等)
```

---

## フロー 5: NaN 試行の処理フロー 🔵

**信頼性**: 🔵 _REQ-AHP-009・既存 filter_valid_indices パターンより_

```
compute_ahp() 内部
    │
  filter_valid_indices(values, n_trials, n_objectives)
    │
    ├─ 各試行 i: values[i*n_obj .. (i+1)*n_obj] に NaN があれば除外
    ├─ valid_indices: Vec<usize>
    │
    ├─ valid_indices.is_empty() の場合:
    │    └─ scores = vec![0.0; n_trials]
    │       ranked_indices = (0..n_trials as u32).collect()
    │       Ok(AhpResult { priority_vector=w, scores, ranked_indices, ... })
    │
    └─ valid_indices 非空の場合:
         min_j / max_j は valid_indices 内のみで計算
         NaN 試行の scores[i] = 0.0 のまま（初期値）
         ランキング: valid 試行をスコア降順、その後 NaN 試行を末尾に配置
```

---

## フロー 6: エラーフロー 🔵

**信頼性**: 🔵 _NFR-AHP-020・既存エラー処理パターンより_

```
compute_ahp() → Err(msg) の場合:
    │
    ▼
  AppMessage::Error(format!("AHP computation failed: {msg}"))
    │
    ▼
  message_handler.rs:
    widget_states.ahp_chart.computing = false
    widget_states.error_message = Some(msg)
    │
    ▼
  次フレームの UI 描画:
    egui::Label でエラーメッセージを表示（クラッシュなし）
```

---

## データ型フロー 🔵

**信頼性**: 🔵 _既存 results.rs パターン・interfaces.rs より_

```
rust_core 側の型:
  tunny_core::mcdm::ahp::AhpResult {
      priority_vector: Vec<f64>,   // len = n_objectives
      scores:          Vec<f64>,   // len = n_trials
      ranked_indices:  Vec<u32>,   // len = n_trials, score 降順
      lambda_max:      f64,
      ci:              f64,
      ri:              f64,
      cr:              f64,
      is_consistent:   bool,       // cr <= 0.10
      duration_ms:     f64,
  }
    │ (chart_registry.rs でフィールドコピー → AhpResult 構築)
    ▼
egui-app 側の型:
  crate::state::results::AhpResult {
      // 同一フィールド構成
  }
    │ (AppMessage::AhpDone でラップ)
    ▼
  AppMessage::AhpDone(AhpResult)
    │ (app_state.ahp_result に格納)
    ▼
  UI 描画: AhpChart::show_rank_chart() / show_table() が参照
```

---

## 関連文書

- **アーキテクチャ**: [architecture.md](architecture.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **実装ガイド**: [implementation-guide.md](implementation-guide.md)
- **要件定義**: [../../spec/ahp/requirements.md](../../spec/ahp/requirements.md)

## 信頼性レベルサマリー

- 🔵 青信号: 7 件 (100%)
- 🟡 黄信号: 0 件 (0%)
- 🔴 赤信号: 0 件 (0%)

**品質評価**: ✅ 高品質
