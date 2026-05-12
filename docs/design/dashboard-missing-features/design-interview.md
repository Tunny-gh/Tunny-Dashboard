# ダッシュボード不足機能 設計ヒアリング記録

**作成日**: 2026-05-12
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

要件定義時点の「未実装」認識と、現行ワークスペースのコード状態に差分があるため、設計対象を再確定することを目的にヒアリングを行った。

特に以下を設計判断対象とした。

- どの機能が完全新規で、どの機能が既存実装の仕上げか
- Surface Plot をどの描画方式で実装するか
- Comparison セッションをいつリセットするか
- Pinning / Brushing / PDP overlay をどう統合するか
- PNG 保存をどこで責務分離するか

---

## 質問と回答

### Q1: 設計規模

**カテゴリ**: 設計方針  
**背景**: `dashboard-missing-features` は 8 機能を横断し、単一ウィジェット設計では足りない

**回答**: フル設計

**信頼性への影響**:

- `architecture.md` / `dataflow.md` / `design-interview.md` / `interfaces.rs` を作成する方針を確定
- 変更対象ファイル一覧と実装順序まで設計に含める

---

### Q2: PDP Observed Overlay は本当に未実装か

**カテゴリ**: 現状把握  
**背景**: 要件定義時のノートでは未実装扱いだったが、現行 `pdp_chart.rs` には `show_observed` と散布点描画が存在する

**回答**: 新規実装ではなく、**Selection / Pinning 連動の仕上げ対象**として扱う

**信頼性への影響**:

- F-004 の設計対象を「描画実装」から「入力データの絞り込み」に縮小
- `render_chart.rs` で `trial_rows` 全件を渡している箇所を、実効可視集合へ切り替える設計を採用

---

### Q3: Surface Plot の描画方式

**カテゴリ**: 技術選択  
**背景**: `egui_plot` は 3D を直接サポートしない。一方で要件は 3D 相当の理解支援が主目的である

**回答**: **Phase 1 は Heatmap / Contour の 2 モード**、必要時のみ Phase 2 で wgpu 3D を検討する

**信頼性への影響**:

- `SurfacePlotRenderMode::{Heatmap, Contour}` を設計に追加
- Surface Plot を既存の 2D グリッド結果と相性の良い widget として設計する方針を確定
- 3D 専用 renderer の導入は今回の必須スコープから外す

---

### Q4: Comparison Study のセッション境界

**カテゴリ**: UX / データ整合性  
**背景**: 現行 `AppState::clear()` は comparison_studies を維持するが、要件上はメイン Study 変更時のリセットが妥当

**回答**: **メイン Study が変わったら comparison_studies をリセットする**

**信頼性への影響**:

- `comparison_base_study: Option<u32>` を設計に追加
- `SelectStudy` / `OpenJournal` 後の比較セッション初期化規約を確定
- Diff タブの互換性判定が単純化される

---

### Q5: ピン留めと選択の優先関係

**カテゴリ**: 状態管理  
**背景**: Pinning 要件は「フィルターやブラッシング後でも残す」、Brushing 要件は「選択で全体を連動更新する」

**回答**: **実効可視集合 = selected_indices ∪ pinned_trials**

**信頼性への影響**:

- Trial Table、PDP overlay、Pareto Scatter 2D で同じヘルパーを使う設計を確定
- ピン留め行が「消えない」一方で、選択の主役は `selected_indices` のまま維持できる

---

### Q6: PNG 保存の責務境界

**カテゴリ**: UI / 出力  
**背景**: 各チャートごとに個別 PNG renderer を持つと実装が拡散する。セルヘッダーは既に共通化されている

**回答**: **チャート単位ではなくセル単位で保存する**。セル矩形を記録し、ビュー全体のスクリーンショットから crop する

**信頼性への影響**:

- `grid_canvas.rs` に capture request の入口を集中させる方針を確定
- Plot / Table / 将来の custom renderer を同一フローで保存できる
- screenshot API 依存部分は `chart_capture.rs` に隔離する

---

## ヒアリング結果サマリー

### 確認できた事項

- F-004 は現行コード上すでに半分以上実装済みで、残課題は selection 連動だけ
- F-006 は Parallel Coordinates に brush state が存在し、ゼロからの設計ではない
- F-002 は `ComparisonStudyLoaded` メッセージ経路がすでに存在する
- F-005 は独立 widget として統合するのが最も自然
- F-008 はセルヘッダー共通化を活かした方が実装コストを抑えられる

### 設計方針の決定事項

1. `ToolbarAction` は Export / Comparison 追加 / Comparison 削除まで責務を広げる
2. `AppState` に `pinned_trials` と `comparison_base_study` を持たせる
3. Surface Plot は Phase 1 を Heatmap / Contour に限定する
4. PNG 保存はセル単位キャプチャとする
5. 実効可視集合は `selected ∪ pinned` とする

### 残課題

- viewport screenshot API の呼び出し名と戻り値型は、実装時に利用中の eframe API を確認すること
- Surface Plot の core 側 API は `compute_pdp_2d` 再利用か専用 API 追加かを実装時に最終決定すること

---

## 信頼性レベル分布

**ヒアリング前**:

- 🔵 青信号: 7件
- 🟡 黄信号: 3件
- 🔴 赤信号: 1件

**ヒアリング後**:

- 🔵 青信号: 10件 (+3)
- 🟡 黄信号: 1件 (-2)
- 🔴 赤信号: 0件 (-1)

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
- **型設計**: [interfaces.rs](interfaces.rs)
- **要件定義**: [requirements.md](../../spec/dashboard-missing-features/requirements.md)
