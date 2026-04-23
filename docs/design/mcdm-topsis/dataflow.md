# MCDM UI データフロー図

**作成日**: 2026-04-23
**更新日**: 2026-04-23 (McdmChart統一化)
**関連アーキテクチャ**: [architecture.md](architecture.md)
**理論文書**: [theory/topsis.md](../../../theory/topsis.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実なフロー
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測によるフロー
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測によるフロー

---

## システム全体のデータフロー 🔵

**信頼性**: 🔵 *egui-migration設計・既存メッセージパターンより*

```mermaid
flowchart TD
    A[ユーザー] --> B[McdmChart ウィジェット]
    B --> B1{手法選択}
    B1 -->|Topsis| C{Topsis選択時}
    B1 -->|Vikor| CV{VIKOR選択時 ※将来}
    B1 -->|Promethee2| CP{PROMETHEE II選択時 ※将来}
    C --> D[重みスライダー値を取得]
    CV --> D
    CP --> D
    D --> E[重みを正規化: w_i / sum_w]
    E --> F[spawn_task で非同期計算]
    F --> G{McdmMethod分岐}
    G -->|Topsis| G1[rust_core::compute_topsis]
    G -->|Vikor| G2[rust_core::compute_vikor ※将来]
    G -->|Promethee2| G3[rust_core::compute_promethee2 ※将来]
    G1 --> H[McdmDone メッセージ送信]
    G2 --> H
    G3 --> H
    H --> I[MessageHandler が受信]
    I --> J[AppState.mcdm_result に保存]
    J --> K[ウィジェット再描画]
    J --> L[ColorMode::McdmScore で色付け]
    K --> M[バーチャート + テーブル表示]
    L --> N[Pareto散布図に反映]
```

## 主要機能のデータフロー

### 機能1: MCDM計算の実行（TOPSIS） 🔵

**信頼性**: 🔵 *ImportanceChartのspawn_taskパターン・theory/topsis.mdより*

**関連要件**: 計算トリガー（オンデマンド・Runボタン）

```mermaid
sequenceDiagram
    participant U as ユーザー
    participant W as McdmChart
    participant R as ChartRegistry
    participant T as spawn_task
    participant C as rust_core
    participant M as MessageHandler
    participant S as AppState

    U->>W: 手法選択 (Topsis)
    U->>W: 重みスライダー調整
    W->>W: スライダー値更新（表示のみ）
    U->>W: Runボタン押下
    W->>W: computing = true
    W->>W: weights正規化
    W->>R: pending_compute = Some((Topsis, weights))
    R->>R: pending_compute検出
    R->>T: spawn_task(tx, || { ... })
    T->>C: compute_topsis(values, n, m, weights, is_minimize)
    C-->>T: Ok(TopsisResult)
    T->>M: tx.send(McdmDone(Topsis(result)))
    M->>S: mcdm_result = Some(McdmResult::Topsis(result))
    M->>W: widget.computing = false
    W->>U: バーチャート + テーブル表示
```

**詳細ステップ**:
1. ユーザーが手法ComboBoxでMCDM手法を選択（Phase 1: Topsisのみ）
2. 各目的関数の重みスライダー（0.0-1.0）を調整
3. Runボタン押下で `pending_compute` に `(McdmMethod, weights)` をセット
4. `chart_registry.rs` で `pending_compute` を検出、`spawn_task` でバックグラウンド実行
5. `McdmMethod` に応じて `rust_core` の対応関数を呼び出し
6. `McdmDone` メッセージで結果をメインスレッドに返却
7. `MessageHandler` が `AppState.mcdm_result` に保存
8. ウィジェットが再描画、バーチャート＋テーブルを表示

### 機能2: ランキングバーチャートの表示 🔵

**信頼性**: 🔵 *ユーザヒアリング・egui_plot BarChart APIより*

**関連要件**: 上位N件表示（5/10/20トグル）、バークリックでハイライト

```mermaid
flowchart TD
    A[mcdm_result 取得] --> B{result存在?}
    B -->|なし| C[Runボタン + スライダーのみ表示]
    B -->|あり| D[primary_scores と ranked_indices 取得]
    D --> E[上位N件に絞り込み]
    E --> F[egui_plot::BarChart 生成]
    F --> G[横棒グラフ描画]
    G --> H{バー Hover?}
    H -->|はい| I[Tooltip表示: Trial ID, Score, 目的関数値]
    H -->|クリック| J[AppState.set_highlight]
    J --> K[散布図ハイライト更新]
```

**バーチャートデータ構築**:
1. `McdmResult` から `primary_scores()` と `ranked_indices` を取得
2. 先頭N件を取得
3. 各Trialのスコアを棒の長さにマッピング
4. `BarChart::new()` で横棒グラフを生成
5. `PlotResponse` から `hovered_bar_item` でクリック検出

### 機能3: ランキングテーブルの表示 🟡

**信頼性**: 🟡 *egui::TableBuilder APIから妥当な推測*

**関連要件**: Trial ID・スコア・各目的関数値のテーブル表示

```mermaid
flowchart TD
    A[mcdm_result 取得] --> B[ranked_indices 先頭N件]
    B --> C[egui::TableBuilder 開始]
    C --> D[ヘッダ行: Rank, ID, Score, Obj1...ObjN]
    D --> E[各行のTrialRowを取得]
    E --> F[セル描画: rank, trial_id, score, objectives...]
    F --> G{行クリック?}
    G -->|はい| H[AppState.set_highlight]
    H --> I[散布図ハイライト更新]
```

### 機能4: 散布図カラーモード 🔵

**信頼性**: 🔵 *ユーザヒアリング・既存ColorModeパターンより*

**関連要件**: ColorMode::McdmScore追加

```mermaid
flowchart TD
    A[ユーザー: カラーモード選択] --> B{McdmScore選択}
    B --> C{mcdm_result存在?}
    C -->|なし| D[グレーアウト・未計算メッセージ]
    C -->|あり| E[primary_scores を取得]
    E --> F[0-1正規化]
    F --> G[colormap.interpolateで色生成]
    G --> H[GpuBuffer.colors_data更新]
    H --> I[wgpu::Queue::write_buffer]
    I --> J[散布図の色が更新]
```

**update_chart_colors 拡張**:
```rust
// app_state.rs の match color_mode に追加
ColorMode::McdmScore => {
    if let Some(ref mcdm) = self.mcdm_result {
        let scores = mcdm.primary_scores();
        for (i, &score) in scores.iter().enumerate() {
            colors[i * 4 + 3] = 1.0; // alpha
            let color = colormap.interpolate(score as f32);
            // RGB設定
        }
    }
}
```

## 状態管理フロー

### McdmChart ウィジェット状態 🔵

**信頼性**: 🔵 *ImportanceChartパターン・ユーザヒアリングより*

```mermaid
stateDiagram-v2
    [*] --> Idle: ウィジェット初期化
    Idle --> WeightEditing: スライダー操作
    WeightEditing --> Idle: スライダー離す
    Idle --> Computing: Run押下
    WeightEditing --> Computing: Run押下
    Computing --> ResultReady: McdmDone受信
    ResultReady --> WeightEditing: スライダー操作（結果表示維持）
    ResultReady --> Computing: Run押下（重み/手法変更後）
    Computing --> Error: エラー受信
    Error --> Idle: 再試行
```

### 非同期タスクフロー 🔵

**信頼性**: 🔵 *egui-migration設計・spawn_taskパターンより*

```
[メインスレッド]                      [ワーカースレッド]
McdmChart.show()
  ├── 手法ComboBox (McdmMethod)
  ├── スライダー表示
  ├── Run ボタン
  │    └── pending_compute = Some((method, weights))
  │
chart_registry.show_chart()
  ├── pending_compute.take()
  └── spawn_task(tx, || {
  │      match method {
  │          Topsis => compute_topsis(...),
  │          // 将来: Vikor => compute_vikor(...),
  │          // 将来: Promethee2 => compute_promethee2(...),
  │      }
  │      tx.send(McdmDone(result))
  │  })
  │
poll_messages()
  └── McdmDone(result)
       ├── mcdm_result = Some(result)
       └── computing = false
```

## エラーハンドリングフロー 🔵

**信頼性**: 🔵 *既存MessageHandlerパターンより*

```mermaid
flowchart TD
    A[MCDM計算実行] --> B{結果}
    B -->|Ok| C[McdmDone 送信]
    B -->|Err| D[Error メッセージ送信]
    C --> E[MessageHandler: mcdm_result 保存]
    D --> F[MessageHandler: load_error 設定]
    E --> G[ウィジェット: 結果表示]
    F --> H[ウィジェット: エラー表示]
```

**エラーケース**（rust_core側で既にハンドリング済み）:
- トライアル数0: `Err("n_trials must be > 0")`
- 値長不一致: `Err("values length mismatch")`
- 重み長不一致: `Err("weights length mismatch")`
- 全NaNトライアル: 縮退ケースとして全スコア0.5（エラーではない）

## 関連文書

- **アーキテクチャ**: [architecture.md](architecture.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **ヒアリング記録**: [design-interview.md](design-interview.md)
- **理論文書**: [theory/topsis.md](../../../theory/topsis.md)
- **egui移行設計**: [../egui-migration/dataflow.md](../egui-migration/dataflow.md)

## 信頼性レベルサマリー

- 🔵 青信号: 10件 (83%)
- 🟡 黄信号: 2件 (17%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
