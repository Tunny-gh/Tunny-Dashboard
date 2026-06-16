# Observed Contour ウィジェット アーキテクチャ設計

**作成日**: 2026-06-15
**関連文書**: [dataflow.md](dataflow.md) / [interfaces.rs](interfaces.rs)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 現行コードを根拠にした確実な設計
- 🟡 **黄信号**: 現行コードから妥当と判断した推測を含む設計
- 🔴 **赤信号**: 根拠が薄く追加検証が必要な設計

---

## 1. 背景と狙い 🔵

Tunny Dashboard はブラックボックス最適化（Optuna）結果の分析ツールである。
既存の PDP（`PdpChart` / `PdpChart2D`）はサロゲートモデルの**周辺化**で目的関数の
応答を描くが、最適化の文脈では次の理由でユーザーに誤解を与えやすい:

- サンプリングが代表分布でない（オプティマイザが有望領域に偏らせる）ため、周辺化の意味が曖昧。
- サロゲートの**外挿**（疎な領域の予測＝モデルの想像）を、あたかも目的関数そのものとして見せる。
- パラメータ相関下で非現実的な組合せを評価する。

過去に追加した「ResponseSurfacePlot」はサロゲート掃引であったため PDP と差別化できず削除された
（PR #84 → #85）。本ウィジェットはその反省を踏まえ、**サロゲートを一切使わず、観測トライアル点
だけを補間**し、**データの無い領域はマスク**する「観測事実」ベースの可視化として設計する。

### 差別化（なぜ誤解が減るか）🔵

| | PDP 2D | 旧 ResponseSurface | **Observed Contour** |
|---|---|---|---|
| データ源 | サロゲート（周辺化） | サロゲート（固定スライス） | 観測点の補間のみ |
| 外挿 | する（見える） | する（見える） | しない（マスク） |
| 軸に目的関数 | 不可（params→objective 固定） | 不可 | 可（データなので安全） |

**副産物**: データ接地ゆえに **X / Y / 値（色）すべてにパラメータでも目的関数でも置ける**。
これにより obj1×obj2→obj3 のような「目的関数空間のトレードオフ面」を、モデルを介さず honest に
描ける。これが PDP（モデル・周辺化）とも旧応答曲面（モデル掃引）とも本質的に別物である理由。

## 2. システム構成 🔵

既存の非同期計算パターン（`pending_compute` → `poll_chart` で `spawn_task` →
`AppMessage` → `message_handler` → `propagate`）にそのまま乗せる。

```
egui-app (UI)                         rust_core (計算)
─────────────                         ─────────────────
ObservedContourState ──pending──▶ poll_chart_work
   (軸/値/スライダー/トグル)            │  numeric_column で (x,y,value) 抽出
                                       │  spawn_task
                                       ▼
                              contour::observed_surface ──▶ ObservedSurface
                                       │  Delaunay + 重心補間 + 疎ガード + 凸包外マスク
                                       ▼
   result ◀── message_handler ◀── AppMessage::ObservedContourDone
   draw_heatmap_masked + 点重畳
```

### 2.1 コア計算（`rust_core/src/contour/`）🔵

新モジュール。サロゲート非依存・純粋関数でバックグラウンドスレッドから呼べる。

- 入力: 観測点 `[[x, y, value]; N]`、格子点数 `n_grid`、疎ガード閾値 `max_edge_ratio`。
- 手順:
  1. 非有限を除去、(x,y) のほぼ重複を統合。
  2. `delaunator` で (x,y) を **Delaunay 三角形分割**。
  3. **疎ガード**: 三角形の最長辺 > `max_edge_ratio × bbox対角` なら破棄（離れたクラスタを偽の面で繋がない）。
  4. 観測範囲で `math::grid::linspace` により格子生成。
  5. 各格子点について含む三角形を探索し、見つかれば 3 頂点 value を**重心補間**、無ければ `None`（マスク）。
- 出力: `ObservedSurface { x_values, y_values, z: Vec<Vec<Option<f64>>> }`。

計算量は N≈1000・格子60² で数百万回の三角形内判定程度＝軽量。点位置探索は MVP では全探索、
必要なら格子ビニングで O(1) 近傍化（Phase 後半の最適化候補）。

### 2.2 描画（`egui-app`）🔵

2D 真上から（contour/heatmap）。3D の“本物らしさ”による過信を避けるため 2D を正とする
（3D は Phase 3 の任意拡張）。

- `common/heatmap.rs` に**マスク対応版** `draw_heatmap_masked(painter, rect, &[Vec<Option<f64>>], cmap)`
  を追加（`None` セルは塗らない＝パネル背景のまま）。
- カラーバー・値域は既存 `draw_colorbar_simple` / `value_range` を再利用（`Option` を平坦化して算出）。
- 観測点を Value のカラーマップで重畳（`Show points`、既定 ON）。点クリック→詳細は
  `trial_detail_modal` の `hit_test_nearest` / `TrialDetailModal` を再利用（Phase 2）。

## 3. 再利用する既存資産 🔵

- `egui-app/src/ui/widgets/common/heatmap.rs`: `draw_colorbar_simple` / `value_range`（＋新規 masked 版）
- `egui-app/src/ui/widgets/common/trial_detail_modal.rs`: `hit_test_nearest` / `TrialDetailModal`
- `egui-app/src/theme/colormap.rs`: `ColorMap`
- `rust_core/src/math/grid.rs`: `linspace`
- `StudyView::numeric_column` / `feasibility()`
- 非同期様式: `crate::app::spawn_task` / `AppMessage` / `ComputeSyncKind`（`app.rs`）

## 4. 依存追加 🟡

`rust_core/Cargo.toml` に `delaunator`（純Rust・MIT・推移依存なし・実質1ファイル）を追加。
新規依存を避けたい場合は Bowyer–Watson の自前実装（~150行）も可。**推奨は `delaunator`**。

## 5. 段階実装 🔵

- **Phase 1 (MVP)**: コア（Delaunay+重心+マスク+疎ガード）、`draw_heatmap_masked`、点重畳、
  軸/Value セレクタ（params∪objectives）、Coverage スライダー、Feasible only、結線一式。
- **Phase 2**: 等高線（marching squares）、点クリック→詳細モーダル、対数スケール（色）、CSV エクスポート。
- **Phase 3**: 任意で 3D 表示、点密度シェーディング。

## 6. 誤解を減らすための既定 🔵

- マスク領域は塗らない（誤解を生む色を使わない）。
- 観測点は既定 ON。
- サブタイトル/凡例: 「Interpolated from observed trials; blank = no data (not extrapolated)」。
- ヘルプで「モデルではなく観測点の補間」「空白＝データなし（外挿しない）」を明示。

## 7. エッジケース 🔵

- 数値パラメータ/目的が 2 軸に満たない、点 < 3、共線、(x,y) 重複 → 空格子＋UI メッセージ。
- カテゴリカル列は `numeric_column` で除外。
- 制約付き Study は `feasible_only` で実行可能解のみ補間可能。

## 8. テスト方針 🔵

ライブラリ（delaunator）内部は再検証しない。自前結線のみ:
- 既知三角形内の点 → 重心補間値が一致。
- 凸包外 → `None`。
- 疎ガードで過大三角形が破棄され該当領域が `None`。
- 退化入力（<3点・共線・重複）→ panic しない・空格子。

## 9. 想定変更ファイル 🔵

- 新規: `rust_core/src/contour/mod.rs`（+ `lib.rs`）、`egui-app/src/ui/widgets/.../observed_contour.rs`、
  `theory/{en,ja}/widgets/observed-contour.md`
- 変更: `widget_states.rs`, `layout_state.rs`, `render_chart.rs`, `poll_chart.rs`, `messages.rs`,
  `message_handler.rs`, `app.rs`, `right_panel.rs`, `csv_export.rs`, `help_content.rs`,
  `common/heatmap.rs`, `rust_core/Cargo.toml`
