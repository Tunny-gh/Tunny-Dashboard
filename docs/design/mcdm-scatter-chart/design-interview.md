# MCDM Scatter Chart - 設計ヒアリング記録

**作成日**: 2026-05-06  
**設計スコープ**: フル設計  
**対象者**: データアナリスト、エンジニア

---

## 1. ヒアリング概要

設計フェーズで以下のポイントについてユーザーと協議し、実装方針を確定した。

| 質問 | ユーザー選択 | 理由 |
|------|----------|------|
| **ウィジェット状態管理** | 新規状態管理（独立） | McdmScatterChart を独立したウィジェットとして管理し、状態変更（軸選択等）を分離 |
| **散布図描画方法** | 既存パターン踏襲 | ParetoScatter2D の実装パターンを参考に、egui_plot::Points で実装 |
| **正規化戦略** | 既存パターン踏襲 | ParetoScatter2D と同一の Min-Max正規化アルゴリズムを採用 |
| **キャッシュ戦略** | 両軸キャッシュ | X軸・Y軸変更時に両軸のキャッシュを無効化し、再計算 |
| **色分けルール** | ランキングベース | ranked_indices から順位を逆変換し、ランキング位置で色分け |

---

## 2. 詳細ヒアリング結果

### 2.1 ウィジェット状態管理: 新規状態管理（独立）

**質問**: McdmScatterChart をどのように状態管理しますか？
- A. 新規状態管理（段方的で独立）
- B. 統合管理（McdmRankChart に統合）

**ユーザー選択**: **新規状態管理（独立）** ✅

**理由と設計上の含意**:

1. **独立性の確保**: 
   - McdmScatterChart は McdmRankChart の子ウィジェットだが、状態は独立
   - 軸選択（ComboBox）、ダウンサンプリング設定、キャッシュは McdmScatterChart 内部で管理
   - McdmRankChart とのデータ共有は最小化（mcdm_result のみ参照）

2. **状態構造**:
   ```rust
   pub struct McdmScatterChart {
       pub x_axis: String,                    // 独立
       pub y_axis: String,                    // 独立
       pub color_threshold: TopN,             // 独立
       pub use_downsample: bool,              // 独立
       pub display_rows_cache: Option<...>,   // 独立
   }
   ```

3. **実装パターン**:
   - egui-app/src/ui/widgets/mcdm_scatter_chart.rs に McdmScatterChart::show(&mut self, ...) を実装
   - self への可変参照で状態を管理

4. **利点**:
   - タブ切り替え時に状態が保持される
   - ユーザーが同じ軸設定に戻すと前のビューが復元される
   - 他のウィジェット（McdmRankChart）への影響なし

---

### 2.2 散布図描画方法: 既存パターン踏襲

**質問**: 散布図の描画方法はどうしますか？
- A. egui_plot::Points のみ使用（粗）
- B. Scatter + Images を併用（標準）
- C. Custom rendering（複雑）

**ユーザー選択**: **既存パターン踏襲** (ParetoScatter2D パターン) ✅

**ユーザーコメント**: 「既存の散布図と同様としてください」

**設計上の含意**:

1. **参考パターン**: egui-app/src/ui/widgets/pareto_2d.rs

2. **実装方式**:
   ```rust
   egui_plot::Plot::new("mcdm_scatter")
       .legend(egui_plot::Legend::default())
       .allow_drag(true)
       .show(ui, |plot_ui| {
           // Partition points by color
           for (color, points) in &grouped_points {
               plot_ui.points(
                   egui_plot::Points::new(points)
                       .color(*color)
                       .radius(4.0)
               );
           }
       });
   ```

3. **主な特徴**:
   - egui_plot::Points で各点をプロット
   - 色別にグループ分け（Red, Orange, Yellow, Gray）
   - ドラッグでパン、スクロールでズーム可能
   - Hover で座標表示

4. **レンダリング最適化**:
   - 色別に別々の Points オブジェクトにすると、egui_plot が自動的に レイヤー順序を管理
   - Red (Top5) → Orange → Yellow → Gray の順で描画（上位ランキングが上に）

---

### 2.3 正規化戦略: 既存パターン踏襲

**質問**: 軸データの正規化をどうしますか？
- A. Min-Max正規化（全データ）
- B. Z スコア正規化（外れ値除去）
- C. 粗いカテゴライズ

**ユーザー選択**: **既存パターン踏襲** (Min-Max正規化) ✅

**ユーザーコメント**: 「既存の散布図と同じにしてください」

**設計上の含意**:

1. **アルゴリズム**: Min-Max正規化
   ```rust
   normalized = (value - min) / (max - min)
   ```

2. **適用対象**:
   - 目的関数値（ユーザー指定の任意軸）
   - MCDM スコア（Q値, S値, R値等）
   - すべての軸は [0.0, 1.0] に統一

3. **エッジケース処理**:
   - 全値同一 → 0.5 (中央)
   - NaN/Inf → フィルタリング
   - 空配列 → スキップ

4. **UI表示**:
   - 軸ラベル: "Objective1 (Normalized)"
   - ホバー時: 元値も表示 "X: 0.5 (150m)"

5. **根拠**:
   - ParetoScatter2D が同じアルゴリズムで実装済み
   - 既存ユーザーの期待値と一致

---

### 2.4 キャッシュ戦略: 両軸キャッシュ

**質問**: ダウンサンプル結果をどうキャッシュしますか？
- A. 両軸キャッシュ（X軸 or Y軸 変更時に再計算）
- B. 第一軸キャッシュのみ（論理的な分岐等）
- C. キャッシュなし（毎フレーム再計算）

**ユーザー選択**: **両軸キャッシュ** ✅

**設計上の含意**:

1. **キャッシュキー**:
   ```rust
   cache_key = (trial_count, hash(x_axis + y_axis))
   ```

2. **無効化条件**:
   - X軸 変更 → キャッシュ無効化
   - Y軸 変更 → キャッシュ無効化
   - Trial 追加 → キャッシュ無効化

3. **再利用条件**:
   - Top N 変更（色のみ） → キャッシュ再利用（色分けのみ再計算）
   - ダウンサンプル有効/無効切り替え → キャッシュ再構築

4. **メリット**:
   - ユーザーが同じ軸に戻すと、以前のキャッシュから復元（即座）
   - Top N 変更は高速（色のみ再計算）
   - メモリ効率: 300 点 × (f64 + f64 + Color32) = ~9.6 KB

---

### 2.5 色分けルール: ランキングベース

**質問**: 色分けルールをどうしますか？
- A. ランキング順位ベース（ranked_indices から逆変換）
- B. 固定閾値ベース（Top5/10/20 の値で色分け）
- C. スコア連続値ベース（スコアの大小で段階的着色）

**ユーザー選択**: **ランキングベース** ✅

**設計上の含意**:

1. **色分けマッピング**:
   ```
   ranked_indices = [5, 2, 8, 1, 3]
   
   逆変換:
   Trial 1 → rank 3 → Yellow
   Trial 2 → rank 1 → Orange
   Trial 3 → rank 4 → Yellow
   Trial 5 → rank 0 → Red
   Trial 8 → rank 2 → Orange
   ```

2. **色スキーム**:
   - Top5 (rank 0-4): 🔴 Red (255, 0, 0)
   - Top10 (rank 5-9): 🟠 Orange (255, 165, 0)
   - Top20 (rank 10-19): 🟡 Yellow (255, 255, 0)
   - Others (rank 20+): ⚪ Gray (200, 200, 200)

3. **UI制御**:
   - ComboBox で Top5/Top10/Top20 を選択
   - 選択により表示色の範囲が変わる
   - Top N 変更時は色分けのみ再計算（ポイント座標は不変）

4. **データ流**:
   ```rust
   for (trial_idx, &point_rank) in ranked_indices.iter().enumerate() {
       let rank_pos = point_rank;  // Rank position in ranking
       let color = map_rank_to_color(rank_pos, threshold);
       // Plot with color
   }
   ```

---

## 3. 合意事項のまとめ

### 3.1 実装方針

| 項目 | 決定 | 根拠 |
|------|------|------|
| **State Architecture** | 独立管理 | ウィジェット自立性, タブ切り替え対応 |
| **Rendering** | egui_plot::Points | 既存実装との統一, 性能 |
| **Normalization** | Min-Max | 既存パターン, シンプル |
| **Caching** | 両軸キャッシュ | 軸変更の再計算効率 |
| **Coloring** | ランキング順位 | MCDM ランキング結果の直感的可視化 |

### 3.2 残された疑問（None）

前回のヒアリング（要件定義フェーズ）で詳細なヒアリングが済んでいるため、設計フェーズでの追加疑問なし。

---

## 4. 次ステップ

### 4.1 実装前チェックリスト

- [ ] interfaces.rs で Rust 型定義を確認
- [ ] architecture.md でアーキテクチャを確認
- [ ] dataflow.md でシーケンスを確認
- [ ] ParetoScatter2D 実装を参照し、同じパターンを踏襲
- [ ] rust_core/src/mcdm/ で既存アルゴリズムを理解

### 4.2 実装タスク（TASK-1501 想定）

1. **Week 1**: 基本構造実装
   - [ ] mcdm_scatter_chart.rs ウィジェット作成
   - [ ] 軸選択 ComboBox 実装
   - [ ] 基本的な散布図レンダリング

2. **Week 2-3**: 機能追加
   - [ ] 正規化・ダウンサンプリング実装
   - [ ] 色分けロジック実装
   - [ ] ホバー/選択機能

3. **Week 4**: 最適化・テスト
   - [ ] パフォーマンス最適化
   - [ ] エッジケーステスト
   - [ ] UI/UX 改善

---

## 5. 設計確度評価

**設計信頼性レベル**: 🔵🔵🔵🔵🔵 **100% (5/5)**

### 根拠:

1. **既存パターン参照** ✅
   - ParetoScatter2D がほぼ同一の設計パターンで実装済み
   - 軸選択, egui_plot 使用, キャッシング戦略が直接応用可能

2. **要件フェーズの完全性** ✅
   - 26 個の EARS 要件が全て合意済み
   - 9 個のユーザーストーリーで具体的ユースケースが明確

3. **アーキテクチャの一貫性** ✅
   - 4層メッセージパッシング構造が確立
   - MCDM 結果型（VikorResult, TopsisResult等）が既に定義済み

4. **実装リスク低** ✅
   - 新規計算不要（既存 mcdm_result を再利用）
   - UI のみの追加実装（バックエンド無し）
   - 参考実装（ParetoScatter2D）が本番環境で稼働中

---

## 6. 参考資料

- **要件定義書**: [../spec/mcdm-scatter-chart/requirements.md](../spec/mcdm-scatter-chart/requirements.md)
- **要件ヒアリング記録**: [../spec/mcdm-scatter-chart/interview-record.md](../spec/mcdm-scatter-chart/interview-record.md)
- **参考実装**: `egui-app/src/ui/widgets/pareto_2d.rs`
- **関連型定義**: `egui-app/src/state/results.rs`
