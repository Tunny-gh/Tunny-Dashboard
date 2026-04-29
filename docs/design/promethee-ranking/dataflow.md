# PROMETHEE Ranking データフロー図

**作成日**: 2026-04-29
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連要件定義**: [../../spec/promethee-ranking/requirements.md](../../spec/promethee-ranking/requirements.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実なフロー
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測によるフロー
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測によるフロー

---

## システム全体のデータフロー 🔵

**信頼性**: 🔵 *既存 egui-app アーキテクチャ・ユーザヒアリングより*

```
ユーザー操作（McdmRankChart.show()）
  │
  ├─ ComboBox 切替（PrometheeI/II）
  │    └─ キャッシュ復元フロー（再計算なし）
  │
  └─ "Run" ボタン押下
       └─ 計算フロー（compute_promethee 実行）
```

---

## フロー 1: 計算実行フロー 🔵

**信頼性**: 🔵 *既存 TOPSIS/VIKOR spawn_task パターン・ユーザヒアリングより*

**関連要件**: REQ-PR-001, REQ-PR-010, REQ-PR-013, REQ-PR-014

```
[UI: McdmRankChart.show()]
  "Run" ボタン押下
    │
    ▼
  pending_compute = Some(McdmComputeRequest {
      method: McdmMethod::PrometheeI または PrometheeII,
      weights: Vec<f64>,       // スライダー値
      v: f64,                  // VIKOR 互換フィールド（PROMETHEE では未使用）
  })
    │
    ▼ (次フレームで chart_registry.rs がポーリング)
    │
[chart_registry.rs: if let Some(req) = widgets.mcdm_chart.pending_compute.take()]
    │
    ▼
  McdmMethod::PrometheeI | McdmMethod::PrometheeII の場合:
    │
    ├─ objectives: Vec<f64>     ← app_state から取得
    ├─ n_trials: usize
    ├─ n_objectives: usize
    ├─ weights: Vec<f64>
    ├─ is_minimize: Vec<bool>
    └─ method: McdmMethod       ← move でクロージャにキャプチャ
    │
    ▼
  spawn_task(tx.clone(), move || {
      tunny_core::mcdm::promethee::compute_promethee(
          &objectives, n_trials, n_objectives, &weights, &is_minimize
      )
  })
    │
    ▼ (std::thread で非同期実行)
    │
[rust_core: compute_promethee()]
    │
    ├─ validate_inputs()         // n_trials, weights, is_minimize 整合チェック
    ├─ filter_valid_indices()    // NaN を含む試行を除外
    │
    ├─ [NaN のみの場合]
    │    └─ Ok(PrometheeResult { all zeros })
    │
    └─ [有効試行あり]
         ├─ range_j = max_j - min_j  per objective
         ├─ p_j = 0.2 × range_j      (range=0 なら p_j=0)
         ├─ q_j = 0.0
         │
         ├─ π(a,b) 集約選好行列の計算 (O(n²) ループ)
         │    └─ 全ペア (a,b) に対して:
         │         Σ_j weight_j × P_j(d_j(a,b))  where d_j = value_j(a) - value_j(b) (minimize 調整済)
         │
         ├─ Φ+(i) = Σ_b π(i,b) / (n-1)   正フロー
         ├─ Φ-(i) = Σ_b π(b,i) / (n-1)   負フロー
         ├─ Φnet(i) = Φ+(i) - Φ-(i)       純フロー
         │
         ├─ ranked_indices_i: sort by Φ+ desc, tiebreak Φ- asc, NaN 末尾
         ├─ ranked_indices_ii: sort by Φnet desc, NaN 末尾
         │
         └─ Ok(PrometheeResult { phi_plus, phi_minus, phi_net,
                                  ranked_indices_i, ranked_indices_ii,
                                  duration_ms })
    │
    ▼ (Ok の場合)
  let result = PrometheeResult { ... };  // egui-app 側の PrometheeResult に変換
  let mcdm = if method == McdmMethod::PrometheeI {
      McdmResult::PrometheeI(result)
  } else {
      McdmResult::PrometheeII(result)
  };
  AppMessage::McdmDone(mcdm)

    ▼ (Err の場合)
  AppMessage::Error(format!("PROMETHEE computation failed: {e}"))
    │
    ▼
  tx.send(message)  // SyncSender<AppMessage> でメインスレッドに送信
```

---

## フロー 2: メッセージ受信・状態更新フロー 🔵

**信頼性**: 🔵 *既存 message_handler.rs パターン・ユーザヒアリングより*

**関連要件**: REQ-PR-011, REQ-PR-012

```
[app.rs: poll_messages()]
  rx.try_recv() で AppMessage を受信
    │
    ▼
[message_handler.rs: handle_message()]
    │
  AppMessage::McdmDone(result) の場合:
    │
    ├─ match &result {
    │    McdmResult::Topsis(r) => {
    │        widget_states.mcdm_chart.cached_topsis = Some(r.clone());
    │    }
    │    McdmResult::Vikor(r) => {
    │        widget_states.mcdm_chart.cached_vikor = Some(r.clone());
    │    }
    │    McdmResult::PrometheeI(r) | McdmResult::PrometheeII(r) => {
    │        widget_states.mcdm_chart.cached_promethee = Some(r.clone());
    │    }
    │  }
    │
    ├─ app_state.mcdm_result = Some(result)
    └─ widget_states.mcdm_chart.computing = false
    │
    ▼
  AppMessage::Error(msg) の場合:
    └─ widget_states.error_message = Some(msg)
```

---

## フロー 3: UI 描画フロー 🔵

**信頼性**: 🔵 *REQ-PR-022, REQ-PR-023・既存 mcdm_chart.rs パターン・ユーザヒアリングより*

**関連要件**: REQ-PR-020, REQ-PR-021, REQ-PR-022, REQ-PR-023

```
[chart_registry.rs: 次フレーム]
  mcdm_chart.show(ui, obj_names, &app_state.mcdm_result, trial_rows)
    │
    ▼
[mcdm_chart.rs: McdmRankChart::show()]
    │
  match &app_state.mcdm_result {
    │
    ├─ None → "Run を押してください" 表示
    │
    ├─ Some(McdmResult::PrometheeI(r)) →
    │    ranked_indices = &r.ranked_indices_i
    │    for idx in ranked_indices:
    │      行描画: "Trial #idx"
    │        Φ+ バー: 幅 = r.phi_plus[idx], 色 = #0c6ac0 (青)
    │        Φ- バー: 幅 = r.phi_minus[idx], 色 = #c02020 (赤)
    │
    └─ Some(McdmResult::PrometheeII(r)) →
         ranked_indices = &r.ranked_indices_ii
         for idx in ranked_indices:
           行描画: "Trial #idx"
             Φnet バー:
               幅 = r.phi_net[idx].abs()
               色 = if r.phi_net[idx] >= 0.0 { #0c6ac0 (青) }
                    else                      { #e07000 (オレンジ) }
  }
```

---

## フロー 4: キャッシュ切替フロー 🔵

**信頼性**: 🔵 *REQ-PR-020, REQ-PR-021・ユーザヒアリング（キャッシュ共有）より*

**関連要件**: REQ-PR-020, REQ-PR-021

```
[UI: ComboBox 切替]
  PrometheeII → PrometheeI に切替:
    │
    ├─ widget.selected_method = McdmMethod::PrometheeI
    │
    └─ if let Some(cached) = &widget.cached_promethee {
           // cached_promethee は PrometheeResult を共有
           // phi_plus / phi_minus / phi_net / ranked_indices_i/ii がすべて入っている
           // "Run" 不要で即座に PrometheeI ビュー描画
       }

  PrometheeI → PrometheeII に切替:
    同上（ranked_indices_ii / phi_net を使用）

  Promethee* → Topsis に切替:
    └─ if let Some(cached) = &widget.cached_topsis { 既存 TOPSIS 表示 }

  (キャッシュがない場合は "Run" が必要 → pending_compute なし)
```

---

## フロー 5: NaN 試行の処理フロー 🔵

**信頼性**: 🔵 *REQ-PR-004・既存 filter_valid_indices パターンより*

```
compute_promethee() 内部
    │
  filter_valid_indices(values, n_trials, n_objectives)
    │
    ├─ 各試行 i: values[i*n_obj .. (i+1)*n_obj] に NaN があれば除外
    ├─ valid_indices: Vec<usize> （有効試行のインデックス）
    │
    ├─ valid_indices.is_empty() の場合:
    │    └─ phi_plus  = vec![0.0; n_trials]
    │       phi_minus = vec![0.0; n_trials]
    │       phi_net   = vec![0.0; n_trials]
    │       ranked_indices_i  = (0..n_trials as u32).collect()
    │       ranked_indices_ii = (0..n_trials as u32).collect()
    │       Ok(PrometheeResult { ... })
    │
    └─ valid_indices 非空の場合:
         NaN 試行の phi_plus/phi_minus/phi_net = 0.0 のまま（初期値）
         ランキング: NaN 試行は末尾に配置
           ranked_indices_i  → valid 試行を Φ+ 降順、その後 NaN 試行
           ranked_indices_ii → valid 試行を Φnet 降順、その後 NaN 試行
```

---

## エラーフロー 🔵

**信頼性**: 🔵 *NFR-PR-020・既存エラー処理パターンより*

```
compute_promethee() → Err(msg) の場合:
    │
    ▼
  AppMessage::Error(format!("PROMETHEE computation failed: {msg}"))
    │
    ▼
  message_handler.rs:
    widget_states.error_message = Some(msg)
    widget_states.mcdm_chart.computing = false
    │
    ▼
  次フレームの UI 描画:
    egui::Label でエラーメッセージを表示（クラッシュなし）
```

---

## データ型フロー 🔵

**信頼性**: 🔵 *既存 results.rs パターン・ユーザヒアリングより*

```
rust_core 側の型:
  tunny_core::mcdm::promethee::PrometheeResult {
      phi_plus:          Vec<f64>,   // len = n_trials
      phi_minus:         Vec<f64>,   // len = n_trials
      phi_net:           Vec<f64>,   // len = n_trials
      ranked_indices_i:  Vec<u32>,   // len = n_trials, PROMETHEE I 順
      ranked_indices_ii: Vec<u32>,   // len = n_trials, PROMETHEE II 順
      duration_ms:       f64,
  }
    │ (chart_registry.rs でフィールドコピー)
    ▼
egui-app 側の型:
  crate::state::results::PrometheeResult {
      // 同一フィールド構成
  }
    │ (McdmResult でラップ)
    ▼
  McdmResult::PrometheeI(PrometheeResult)   // ranked_indices_i 使用
  McdmResult::PrometheeII(PrometheeResult)  // ranked_indices_ii 使用
    │ (app_state.mcdm_result に格納 / cached_promethee にクローン)
    ▼
  UI 描画: McdmRankChart::show() が参照
```

---

## 関連文書

- **アーキテクチャ**: [architecture.md](architecture.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **実装ガイド**: [implementation-guide.md](implementation-guide.md)
- **要件定義**: [../../spec/promethee-ranking/requirements.md](../../spec/promethee-ranking/requirements.md)

## 信頼性レベルサマリー

- 🔵 青信号: 7 件 (100%)
- 🟡 黄信号: 0 件 (0%)
- 🔴 赤信号: 0 件 (0%)

**品質評価**: 高品質
