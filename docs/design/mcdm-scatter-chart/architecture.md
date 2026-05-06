# MCDM Scatter Chart - アーキテクチャ設計

**ドキュメント作成日**: 2026-05-06  
**設計範囲**: フル設計（Full Design）  
**対象ファイル**: `egui-app/src/ui/widgets/mcdm_scatter_chart.rs` (新規)  
**関連ファイル**: `egui-app/src/ui/widgets/mcdm_chart.rs` (拡張)

---

## 1. 全体アーキテクチャ

### 1.1 4層メッセージパッシング構造

MCDM Scatter Chart は既存の Tunny Dashboard アーキテクチャに準拠した4層構造で実装されます。

```
┌─────────────────────────────────────────────────────────────┐
│ Layer 1: UI Layer (egui-app/src/ui/widgets/)                │
│  - McdmRankChart (既存: バーチャート＋テーブル)             │
│  - McdmScatterChart (新規: 散布図, 本タスク)                │
│  └─ TabState による タブ間状態管理                           │
└─────────────────────────────────────────────────────────────┘
                          ↑ ↓
┌─────────────────────────────────────────────────────────────┐
│ Layer 2: Dispatch Layer (egui-app/src/ui/chart_registry.rs) │
│  - show_chart() で pending_compute フラグを検知             │
│  - spawn_task() でバックグラウンド実行を起動                │
│  - 結果を AppMessage::McdmDone として返却                   │
└─────────────────────────────────────────────────────────────┘
                          ↑ ↓
┌─────────────────────────────────────────────────────────────┐
│ Layer 3: State Layer (egui-app/src/state/)                  │
│  - WidgetStates: per-method キャッシュ (cached_vikor等)     │
│  - AppState: 統合結果 (mcdm_result)                         │
│  - message_handler.rs: AppMessage を状態に変換              │
└─────────────────────────────────────────────────────────────┘
                          ↑ ↓
┌─────────────────────────────────────────────────────────────┐
│ Layer 4: Compute Layer (rust_core/src/mcdm/)                │
│  - vikor.rs, topsis.rs, promethee.rs, ahp.rs                │
│  - 純Rust実装、スレッドセーフ                                │
│  - TrialRow 集合 → McdmResult 型で返却                      │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. McdmScatterChart ウィジェット設計

### 2.1 ウィジェット状態構造

```
McdmScatterChart
├── State Management (独立管理)
│   ├── x_axis: String              // 選択X軸 ("Objective1", "Q_Score"等)
│   ├── y_axis: String              // 選択Y軸
│   ├── color_threshold: TopN        // 色分け閾値 (Top5/Top10/Top20)
│   ├── use_downsample: bool        // ダウンサンプリング有効フラグ
│   ├── display_rows_cache: Option<Vec<(f64, f64, Color32)>>  // キャッシュ
│   └── cache_key: (trial_count, axis_hash)  // キャッシュ無効化検知
│
├── Computation
│   ├── pending_compute: Option<()>  // 計算リクエストペンディング
│   └── computing: bool             // 計算中フラグ
│
└── Display
    ├── hover_info: Option<(usize, String)>  // ホバー情報
    └── selected_point: Option<usize>        // 選択ポイント
```

**重要**: McdmScatterChart は独立した状態管理を持ちます（選択肢: 新規状況管理 ✅）。
- McdmRankChart との状態共有は最小化
- ウィジェット内の軸選択やキャッシュは独立

### 2.2 UI レイアウト

```
McdmRankChart
├── Tab1: "Ranking" (既存)
│   ├── Bar Chart (MCDM ranking by score)
│   └── Table (Trial ID, Rank, Scores)
│
└── Tab2: "Scatter Plot" (新規) ← McdmScatterChart
    ├── [Control Row]
    │   ├── X Axis: ComboBox [Objective1 ▼]
    │   ├── Y Axis: ComboBox [Objective2 ▼]
    │   ├── Color By: Dropdown [Top5 / Top10 / Top20]
    │   └── ⚙ Downsample: Checkbox ✓
    │
    ├── [Plot Area]
    │   └── egui_plot::Plot
    │       └── Points (colored by rank)
    │
    └── [Status Bar]
        └── "Rendering 150 points (downsampled from 500 trials)"
```

---

## 3. データフロー設計

### 3.1 Request → Compute → Render フロー

1. **User Interaction**: コンボボックス変更 (X軸, Y軸, Top N)
   ```
   x_axis: "Objective1" → "Q_Score"  (VIKOR)
   ```

2. **State Update**: キャッシュ無効化
   ```rust
   self.cache_key = (trial_count, hash(x_axis, y_axis))
   self.display_rows_cache = None  // 無効化
   ```

3. **Dispatch**: 計算リクエスト生成
   ```rust
   chart_registry::show_chart() {
       if self.display_rows_cache.is_none() {
           self.pending_compute = Some(())
       }
   }
   ```

4. **Background Compute**: spawn_task でスレッド実行
   ```rust
   spawn_task(tx, move || {
       // 両軸データ抽出、正規化、キャッシュ作成
       let points = compute_scatter_points(
           &mcdm_result,
           &x_axis,
           &y_axis,
           use_downsample
       );
       tx.send(AppMessage::McdmScatterComputed(points))
   })
   ```

5. **State Update**: メッセージハンドリング
   ```rust
   message_handler::handle_mcdm_scatter_computed(points) {
       widget_state.display_rows_cache = Some(points)
       widget_state.computing = false
   }
   ```

6. **Render**: egui_plot で表示
   ```rust
   egui_plot::Plot::show(ui, |ui| {
       for (x, y, color) in &self.display_rows_cache {
           ui.points(Points::new(vec![(x, y)]).color(color))
       }
   })
   ```

### 3.2 キャッシング戦略

**両軸キャッシュ戦略** ✅ (ユーザー選択)

```
Input: (X軸, Y軸, Top N, downsample_flag)
       ↓
Transformation:
  - 軸データ抽出 (extract_axis_values)
  - Min-Max正規化 (normalize_values)
  - ダウンサンプリング（有効時）
  - 色分けマッピング (color_by_rank)
       ↓
Output: Vec<(f64, f64, Color32)>
       ↓
Storage: display_rows_cache
       ↓
Invalidation:
  - X軸 or Y軸 変更 → キャッシュ無効
  - Top N 変更 → キャッシュ無効（色のみ再計算で最適化可能）
  - Trial 追加 → キャッシュ無効
```

---

## 4. 色分けアルゴリズム

### 4.1 ランキングベース色マッピング

入力: `McdmResult.ranked_indices` (昇順ランキング)

```rust
fn map_rank_to_color(rank_position: usize, color_threshold: TopN) -> Color32 {
    match rank_position {
        0..=4 if color_threshold >= TopN::Top5 => Color32::from_rgb(255, 0, 0),      // 🔴 Red
        5..=9 if color_threshold >= TopN::Top10 => Color32::from_rgb(255, 165, 0),   // 🟠 Orange
        10..=19 if color_threshold >= TopN::Top20 => Color32::from_rgb(255, 255, 0), // 🟡 Yellow
        _ => Color32::from_rgb(200, 200, 200),                                        // ⚪ Gray
    }
}
```

### 4.2 ランキング逆変換

```
ranked_indices = [5, 2, 8, 1, 3]  // 例
  ↓ (逆変換)
rank_position = [3, 0, 1, 4, 2]
  ↓ (Trial ID 3 の color = Red, Trial ID 2 の color = Gray等)
```

---

## 5. 正規化アルゴリズム

### 5.1 Min-Max正規化

目的関数値と MCDM スコア（スケール異なる）を統一フォーマットに正規化。

```rust
fn normalize_values(values: &[f64]) -> Vec<f64> {
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;

    values.iter().map(|&v| {
        if range == 0.0 {
            0.5  // All values equal → mid-point
        } else {
            (v - min) / range
        }
    }).collect()
}
```

### 5.2 エッジケース処理

| ケース | 処理 |
|--------|------|
| 全値同一 | スコア = 0.5 (中央値) |
| NaN/Inf | 除外またはフィルタリング |
| 空配列 | スキップ |

---

## 6. 軸選択パターン（ComboBox）

### 6.1 軸オプション生成ロジック

```rust
fn get_axis_options(mcdm_result: &McdmResult) -> Vec<(String, String)> {
    let mut options = Vec::new();

    // Objective functions
    for (i, name) in trial_names.iter().enumerate() {
        options.push((
            format!("Objective{}", i),
            format!("Objective {} ({})", i, name)
        ));
    }

    // MCDM scores (method-specific)
    match mcdm_result {
        McdmResult::Vikor(vikor) => {
            options.push(("VIKOR_Q".to_string(), "VIKOR Q Score"));
            options.push(("VIKOR_S".to_string(), "VIKOR S Value"));
            options.push(("VIKOR_R".to_string(), "VIKOR R Value"));
        },
        McdmResult::Topsis(topsis) => {
            options.push(("TOPSIS_Score".to_string(), "TOPSIS Score"));
        },
        // ...
    }

    options
}
```

---

## 7. ダウンサンプリング戦略

### 7.1 アルゴリズム

Trial 数が多い場合、視認性維持のため自動ダウンサンプリング。

```rust
fn downsample_points(points: &[(f64, f64)], max_points: usize) -> Vec<(f64, f64)> {
    if points.len() <= max_points {
        return points.to_vec();
    }

    let step = points.len() / max_points;
    points.iter().step_by(step).copied().collect()
}
```

**デフォルト**: ダウンサンプリング有効、max_points = 300

---

## 8. メッセージ拡張

### 8.1 新規 AppMessage 型

```rust
// egui-app/src/state/messages.rs に追加
pub enum AppMessage {
    // ... 既存
    McdmScatterComputed {
        x_axis: String,
        y_axis: String,
        points: Vec<(f64, f64, Color32)>,
        metadata: ScatterMetadata,
    },
}

pub struct ScatterMetadata {
    pub total_trials: usize,
    pub downsampled_count: usize,
    pub compute_time_ms: u128,
}
```

---

## 9. エラーハンドリング

### 9.1 予期されるエラーケース

| シナリオ | 処理 |
|---------|------|
| 軸が存在しない | UI に警告メッセージ表示 |
| MCDM 計算失敗 | バーチャートと同じ処理 |
| Downsample 失敗 | 非ダウンサンプル版を使用 |
| メモリ不足 | 計算キャンセル、UI 無効化 |

---

## 10. パフォーマンス考慮

### 10.1 キャッシング効果

- **軸選択 (10 trial ツール)**: キャッシュ無効化 → 再計算 ~0.5ms
- **Top N 変更 (色のみ)**: キャッシュ再利用 ~1ms
- **ダウンサンプリング**: 500 trials → 300 points ~2ms

### 10.2 Rendering Performance

egui_plot::Points レンダリング:
- 300 点: ~1-2ms (GPU acceleration)
- 1000+ 点: ダウンサンプリング推奨

---

## 11. 参考資料

- **既存実装パターン**: `egui-app/src/ui/widgets/pareto_2d.rs` (ParetoScatter2D)
- **メッセージハンドリング**: `egui-app/src/state/message_handler.rs`
- **MCDM 結果型**: `egui-app/src/state/results.rs`
- **UI ウィジェット親**: `egui-app/src/ui/widgets/mcdm_chart.rs` (McdmRankChart)
