# VIKOR データフロー図

**作成日**: 2026-04-24
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連要件定義**: [../../spec/vikor/requirements.md](../../spec/vikor/requirements.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 既存実装・ユーザヒアリングを参考にした確実なフロー
- 🟡 **黄信号**: 既存実装・アルゴリズム仕様から妥当な推測によるフロー
- 🔴 **赤信号**: ヒアリングにない推測によるフロー

---

## システム全体のデータフロー 🔵

**信頼性**: 🔵 *既存MCDMフロー・chart_registry.rs実装より*

```
ユーザー
  │  Run ボタン押下（method=Vikor, weights, v=0.5）
  ▼
McdmRankChart.show()
  │  pending_compute = Some(McdmComputeRequest { method: Vikor, weights, v })
  │  computing = true
  ▼
chart_registry::show_chart()
  │  pending_compute.take() → McdmComputeRequest
  │  objectives, n_trials, n_objectives, is_minimize を app_state から収集
  ▼
crate::app::spawn_task()  ← バックグラウンドスレッド
  │
  ├─ tunny_core::vikor::compute_vikor(
  │      &objectives, n_trials, n_objectives,
  │      &weights, &is_minimize, v
  │  )
  │  ↓ Ok(VikorResult)
  └─ AppMessage::McdmDone(McdmResult::Vikor(VikorResult { ... }))
       ↓ mpsc::SyncSender
message_handler
  │  app_state.mcdm_result = Some(McdmResult::Vikor(...))
  │  widget_states.mcdm_chart.computing = false
  ▼
次フレームの show()
  │  result = app_state.mcdm_result.as_ref()
  └─ バーチャート/テーブル描画
```

---

## VIKOR計算ステップ詳細 🔵

**信頼性**: 🔵 *VIKORアルゴリズム仕様・REQ-001・REQ-102〜104より*

```
compute_vikor(values, n_trials, n_objectives, weights, is_minimize, v)
  │
  ├─ 1. validate_inputs()
  │     n_trials>0, n_objectives>0, lengths一致確認
  │     → Err(String) if invalid
  │
  ├─ 2. NaNフィルタリング
  │     valid_indices = trials without NaN objectives
  │     if valid_indices.is_empty():
  │       → all q_values=1.0, ranked末尾
  │
  ├─ 3. best/worst値の決定（線形正規化の分母）
  │     for j in 0..n_objectives:
  │       minimize: best_j = min(f_ij), worst_j = max(f_ij)
  │       maximize: best_j = max(f_ij), worst_j = min(f_ij)
  │     ガード: best_j == worst_j → range_j = 0（寄与分=0）
  │
  ├─ 4. S_i / R_i の計算（valid試行のみ）
  │     for i in valid_indices:
  │       for j in 0..n_objectives:
  │         if range_j > 0:
  │           contrib_ij = weights[j] * (best_j - f_ij) / range_j
  │         else:
  │           contrib_ij = 0.0
  │       S_i = Σ_j contrib_ij
  │       R_i = max_j(contrib_ij)
  │
  ├─ 5. Q_i の計算
  │     S* = min(S_i), S- = max(S_i)
  │     R* = min(R_i), R- = max(R_i)
  │     for i in valid_indices:
  │       term1 = if (S- - S*) > ε: v * (S_i - S*) / (S- - S*) else 0.0
  │       term2 = if (R- - R*) > ε: (1-v) * (R_i - R*) / (R- - R*) else 0.0
  │       Q_i = term1 + term2
  │
  ├─ 6. NaN試行の後処理
  │     NaN試行: s=0.0, r=0.0, q=1.0（最悪値）
  │
  ├─ 7. ranked_indices の生成
  │     Q昇順でソート（Q低い = 良い）
  │     NaN試行は末尾に配置
  │
  └─ 8. VikorResult 返却
        { s_values, r_values, q_values,
          ranked_indices, best_values, worst_values, duration_ms }
```

---

## primary_scores() フロー 🔵

**信頼性**: 🔵 *REQ-003・既存McdmResult.primary_scores()インターフェースより*

```
McdmResult::Vikor(r).primary_scores()
  │
  ├─ オプションA: q_valuesフィールドを返す
  │   → バーチャート側で (1.0 - score) として表示
  │
  └─ オプションB: VikorResultにdisplay_scores = 1.0 - q を格納
      → primary_scores() = &r.display_scores
      → バーチャートは変更なし（既存TOPSISと完全互換）

推奨: オプションB（既存バーチャートコードへの変更を最小化）
```

---

## UIフロー: vスライダー 🔵

**信頼性**: 🔵 *ユーザヒアリング・NFR-201より*

```
McdmRankChart.show()
  │
  ├─ 手法コンボボックス表示（TOPSIS / VIKOR）
  │
  ├─ Top N コンボボックス
  │
  ├─ Run ボタン + spinner
  │
  └─ "Weights" collapsing セクション
       │
       ├─ for i in 0..obj_count:
       │    Slider(weights[i], 0〜1) + 正規化後の値表示
       │
       └─ if method == McdmMethod::Vikor:
            Slider(v_param, 0.0〜1.0)
            ラベル: "Strategy weight v (0=min-regret, 1=max-consensus)"
            ※ TOPSIS選択時は非表示
```

---

## エラーハンドリングフロー 🔵

**信頼性**: 🔵 *既存MCDMエラーハンドリングパターンより*

```
compute_vikor() → Err(msg)
  │
  └─ AppMessage::Error(format!("VIKOR computation failed: {}", msg))
       ↓
     message_handler
       │  computing = false
       └─ UIにエラー表示（既存Errorハンドリングパターン）
```

---

## データ変換サマリー 🔵

**信頼性**: 🔵 *アルゴリズム仕様・REQ-001〜REQ-003より*

| フェーズ | 入力 | 出力 |
|---------|------|------|
| 入力収集 | `trial_rows[].objectives` + `ctx.meta.directions` | `values: Vec<f64>`, `is_minimize: Vec<bool>` |
| VIKOR計算 | values, weights, is_minimize, v | `VikorResult { s, r, q, ranked, best, worst }` |
| 状態格納 | VikorResult | `app_state.mcdm_result = Some(McdmResult::Vikor(...))` |
| バーチャート表示 | `primary_scores()` = 1.0 - Q (or display_scores) | 上位N件の棒グラフ |
| テーブル表示 | `ranked_indices` + `primary_scores()` | Rank/Trial/Score/目的値の表 |

---

## 関連文書

- **アーキテクチャ**: [architecture.md](architecture.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **設計ヒアリング**: [design-interview.md](design-interview.md)

## 信頼性レベルサマリー

- 🔵 青信号: 8件 (100%)
- 🟡 黄信号: 0件 (0%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
