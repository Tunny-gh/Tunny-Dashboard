# Entropy Weight Method データフロー図

**作成日**: 2026-04-24
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連要件定義**: [requirements.md](../../spec/entropy-weight/requirements.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実なフロー
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測によるフロー
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測によるフロー

---

## システム全体のデータフロー 🔵

**信頼性**: 🔵 *要件定義・ユーザストーリー・既存MCDMフローより*

```
ユーザー
  │  Weight Mode を "Entropy" に切替
  ▼
McdmRankChart.show()
  │  weight_mode = WeightMode::Entropy
  │  pending_entropy = true
  ▼
chart_registry::show_chart()
  │  pending_entropy.take()
  │  objectives, n_trials, n_objectives を app_state から収集
  ▼
crate::app::spawn_task()  ← バックグラウンドスレッド
  │
  ├─ tunny_core::entropy::compute_entropy_weights(
  │      &objectives, n_trials, n_objectives
  │  )
  │  ↓ Ok(EntropyResult)
  └─ AppMessage::EntropyDone(EntropyResult { ... })
       ↓ mpsc::SyncSender
message_handler
  │  widget_states.mcdm_chart.entropy_result = Some(EntropyResult { ... })
  │  widget_states.mcdm_chart.weights = entropy_result.weights
  │  widget_states.mcdm_chart.pending_entropy = false
  ▼
次フレームの show()
  │  weight_mode == Entropy → スライダー読み取り専用 + エントロピーテーブル表示
  └─ ユーザーが Run ボタン押下 → McdmComputeRequest { weights: entropy_weights, ... }
```

---

## エントロピー計算ステップ詳細 🔵

**信頼性**: 🔵 *REQ-001・REQ-002・アルゴリズム仕様より*

```
compute_entropy_weights(values, n_trials, n_objectives)
  │
  ├─ 1. 入力検証
  │     values.len() == n_trials * n_objectives 確認
  │     n_trials >= 1, n_objectives >= 1 確認
  │     → Err(String) if invalid
  │
  ├─ 2. NaNフィルタリング
  │     valid_indices = filter_valid_indices(values, n_trials, n_objectives)
  │     if valid_indices.is_empty():
  │       → Err("No valid trials for entropy computation")
  │
  ├─ 3. 負の値チェックと前処理 🔵
  │     for j in 0..n_objectives:
  │       has_negative = any(values[j+col] < 0 for valid trials)
  │     if has_negative:
  │       min-max正規化で [0, 1] に変換
  │       負の値がない場合はスキップ
  │
  ├─ 4. 比例正規化 🔵
  │     for j in 0..n_objectives:
  │       sum_j = Σ_i x_ij (valid trials only)
  │       if sum_j == 0:
  │         → その目的は分散ゼロ扱い（p_ij = 0 for all i）
  │       p_ij = x_ij / sum_j
  │
  ├─ 5. 情報エントロピーの計算 🔵
  │     ln_m = ln(valid_count) as f64
  │     for j in 0..n_objectives:
  │       e_j = -(1/ln_m) * Σ_i p_ij * ln(p_ij)
  │       ※ p_ij == 0 の項は 0 として扱う（lim x→0+ x·ln(x) = 0）
  │       ※ ln_m == 0 (valid_count == 1) の場合: e_j = 0.0
  │
  ├─ 6. 分散度の計算 🔵
  │     for j in 0..n_objectives:
  │       d_j = 1.0 - e_j
  │
  ├─ 7. 重みの計算 🔵
  │     sum_d = Σ_j d_j
  │     if sum_d == 0.0:
  │       weights = vec![1.0/n_objectives; n_objectives]  // 均等重み
  │     else:
  │       w_j = d_j / sum_d
  │
  └─ 8. EntropyResult 返却
        { weights, entropies, diversities, normalized_matrix, duration_ms }
```

---

## UIフロー: WeightModeセレクタ 🔵

**信頼性**: 🔵 *ユーザヒアリング「手法セレクタの横」・REQ-006より*

```
McdmRankChart.show()
  │
  ├─ 横並びレイアウト:
  │   [手法: TOPSIS ▼]  [Weight: Manual ▼]
  │
  ├─ Weight Mode = Manual の場合:
  │   │
  │   ├─ 重みスライダー: 編集可能
  │   └─ エントロピーテーブル: 非表示
  │
  └─ Weight Mode = Entropy の場合:
      │
      ├─ エントロピー計算が未実行:
      │   └─ pending_entropy = true → バックグラウンド計算開始
      │
      ├─ エントロピー計算中:
      │   └─ spinner 表示
      │
      ├─ エントロピー計算完了:
      │   ├─ 重みスライダー: 読み取り専用（entropy_result.weights を表示）
      │   └─ エントロピーテーブル: 表示
      │       | 目的名 | Entropy (e_j) | Diversity (d_j) | Weight (w_j) |
      │       |-------|---------------|-----------------|-------------|
      │       | obj0  | 0.xxx         | 0.xxx           | 0.xxx       |
      │       | obj1  | 0.xxx         | 0.xxx           | 0.xxx       |
      │
      └─ Run ボタン押下時:
          └─ McdmComputeRequest { weights: entropy_result.weights.clone(), ... }
```

---

## WeightMode切替フロー 🔵

**信頼性**: 🔵 *ユーザヒアリング「Weight Mode切替時」・REQ-201/REQ-202より*

```
Manual → Entropy 切替:
  │
  ├─ pending_entropy = true
  ├─ バックグラウンドで compute_entropy_weights() 実行
  ├─ 結果受信: weights = entropy_result.weights
  └─ スライダーを読み取り専用に変更

Entropy → Manual 切替:
  │
  ├─ entropy_result はキャッシュとして保持
  ├─ 現在の weights（エントロピー重み）はそのまま維持
  ├─ スライダーを編集可能に変更
  └─ エントロピーテーブルを非表示
```

---

## MCDM計算フロー（Entropy重み使用時） 🔵

**信頼性**: 🔵 *既存MCDMフロー・REQ-005より*

```
ユーザー Run ボタン押下
  │  (WeightMode::Entropy の場合)
  ▼
McdmRankChart
  │  weights = entropy_result.weights.clone()
  │  (WeightMode::Manual の場合: weights = self.weights.clone())
  ▼
pending_compute = Some(McdmComputeRequest { method, weights, v })
  │
  ▼ （既存のMCDMフローに合流 - TOPSIS/VIKOR dispatch は変更なし）
```

---

## エラーハンドリングフロー 🔵

**信頼性**: 🔵 *既存MCDMエラーハンドリングパターン・REQ-103より*

```
compute_entropy_weights() → Err(msg)
  │
  └─ AppMessage::Error(format!("Entropy computation failed: {}", msg))
       ↓
     message_handler
       │  pending_entropy = false
       └─ UIにエラー表示

全試行NaN → Err("No valid trials...")
負の値含む → min-max前処理後に計算（エラーにしない）
全データ同一 → 均等重み [1/n] を返す（エラーにしない）
```

---

## データ変換サマリー 🔵

**信頼性**: 🔵 *REQ-001〜REQ-002・アルゴリズム仕様より*

| フェーズ | 入力 | 出力 |
|---------|------|------|
| 入力収集 | `trial_rows[].objectives` | `values: Vec<f64>`, `n_trials`, `n_objectives` |
| NaNフィルタ | values | valid_indices: Vec<usize> |
| 前処理 | valid values (負含む可能性) | 非負値（min-max正規化済み or そのまま） |
| 比例正規化 | 非負値 | p_ij matrix (各列合計=1.0) |
| エントロピー計算 | p_ij, ln(m) | e_j ∈ [0, 1] |
| 分散度計算 | e_j | d_j = 1 - e_j |
| 重み計算 | d_j | w_j ≥ 0, sum(w_j) = 1.0 |
| UI反映 | w_j | McdmRankChart.weights 更新 |

---

## 関連文書

- **アーキテクチャ**: [architecture.md](architecture.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **設計ヒアリング**: [design-interview.md](design-interview.md)

## 信頼性レベルサマリー

- 🔵 青信号: 10件 (100%)
- 🟡 黄信号: 0件 (0%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
