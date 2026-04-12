# chart-implementation 設計ヒアリング記録

**作成日**: 2026-04-12
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

`grid_canvas.rs` の `show_chart()` で "not yet implemented" と表示されている5チャートの実装方針を明確化するためのヒアリングを実施した。

## 質問と回答

### Q1: 設計作業の規模について

**質問日時**: 2026-04-12
**カテゴリ**: 作業規模
**背景**: 5チャートの実装設計に必要なドキュメントの粒度を確認するため

**質問**: 「この設計の作業規模について教えてください」

**回答**: 軽量設計（推奨）— architecture.md と dataflow.md のみ作成

**信頼性への影響**:
- 設計文書の範囲が確定。interfaces.rs、database-schema、api-endpoints は不要。

---

### Q2: ParetoScatter3D の対応方針

**質問日時**: 2026-04-12
**カテゴリ**: 技術選択
**背景**: ParetoScatter3D は wgpu GPU レンダリングが必要で、他の4チャートと実装難度が大きく異なるため、スコープに含めるか確認が必要だった

**質問**: 「3D Paretoチャート（ParetoScatter3D）はwgpu GPU描画が必要で実装が複雑ですが、どう対応しますか？」

**回答**: 今回スコープ外

**信頼性への影響**:
- ParetoScatter3D を設計・実装スコープから除外確定。
- 対象チャートが5→4に絞り込まれた（信頼性: 🔵）。

---

### Q3: ClusterScatter の実装方針

**質問日時**: 2026-04-12
**カテゴリ**: 技術選択
**背景**: ClusterScatter は k-means クラスタリングと PCA 2D投影が必要。`app_state::ClusterResult` は `labels` と `n_clusters` のみを保持しており、PCA 座標は未計算。外部クレートを使用するか、ウィジェット内部で計算するかの方針確認が必要だった

**質問**: 「ClusterScatterは k-means クラスタリングと PCA 2D投影が必要です。どちらを優先しますか？」

**回答**: 外部クレート使用（推奨）— linfa 等の Rust ML クレートを使用

**信頼性への影響**:
- `Cargo.toml` に `linfa`、`linfa-reduction`、`ndarray` を追加確定（信頼性: 🔵）。
- k-means は `app_state` 側で既に実行済み（`labels` 保持）のため、PCA のみウィジェット内部でキャッシュ計算する方針が確定（信頼性: 🟡）。

---

## ヒアリング結果サマリー

### 確認できた事項
- 設計ドキュメントは軽量設計（architecture.md + dataflow.md + design-interview.md）
- ParetoScatter3D は今回スコープ外
- ClusterScatter は linfa 外部クレートで PCA 2D投影を実装
- 対象チャート: ParallelCoordinates, ScatterMatrix, SensitivityHeatmap, ClusterScatter の4つ

### 設計方針の決定事項
- 各チャートは既存ウィジェット構造体に `show()` メソッドを追加する方針
- `WidgetStates` に4フィールドを追加
- `grid_canvas.rs` の `show_chart()` で4箇所の "not yet implemented" を実際の呼び出しに置換
- ClusterScatter は PCA 結果をウィジェット内部でキャッシュ（`cached_pca` フィールド）

### 残課題
- ClusterScatter の PCA キャッシュ無効化条件の詳細（`trial_rows.len()` と `n_clusters` の変化検知）
- ParallelCoordinates のブラシ選択と他チャートへの連携（今回スコープ外の可能性）

### 信頼性レベル分布

**ヒアリング前**:
- 🔵 青信号: 8件
- 🟡 黄信号: 4件
- 🔴 赤信号: 2件

**ヒアリング後**:
- 🔵 青信号: 14件 (+6)
- 🟡 黄信号: 4件 (+0)
- 🔴 赤信号: 0件 (-2)

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
