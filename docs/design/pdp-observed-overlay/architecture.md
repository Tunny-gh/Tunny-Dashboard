# pdp-observed-overlay アーキテクチャ設計

**作成日**: 2026-04-15
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 既存実装・ユーザーヒアリングを参考にした確実な設計
- 🟡 **黄信号**: 既存実装・ユーザーヒアリングから妥当な推測による設計
- 🔴 **赤信号**: 推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *ユーザーヒアリングより*

既存の `PdpChart`（1D PDP チャート）に**実際のトライアル観測データの散布図オーバーレイ**機能を追加する。
ボタンで表示・非表示をトグルできる。

対象ファイル:
- `egui-app/src/ui/widgets/pdp_chart.rs` — `PdpChart` 状態と描画ロジック
- `egui-app/src/ui/grid_canvas.rs` — `show_chart` の呼び出し側（`trial_rows` の受け渡し）

---

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存コードベース（featura/egui ブランチ）の実装パターンより*

- **パターン**: 即時モード UI（egui）+ 状態をウィジェット側に保持
- 新規ファイル・新規メッセージ型は不要。既存 `PdpChart` 構造体を拡張するだけ

---

## 変更コンポーネント

### `PdpChart` 構造体への追加フィールド 🔵

**信頼性**: 🔵 *ユーザーヒアリングより*

```rust
pub struct PdpChart {
    // 既存フィールド（変更なし）
    pub mode: PdpMode,
    pub selected_param: String,
    pub selected_objective: usize,
    pub model_type: ModelType,
    pub result: Option<PdpResult>,
    pub computing: bool,
    pub cache: HashMap<String, PdpResult1d>,
    // 新規追加
    pub show_observed: bool,   // 観測データ表示トグル
}
```

### `show()` シグネチャ変更 🔵

**信頼性**: 🔵 *既存実装の `show_chart` 呼び出しパターンより*

```rust
// 変更前
pub fn show(&mut self, ui: &mut egui::Ui, param_names: &[String], obj_names: &[String])

// 変更後
pub fn show(
    &mut self,
    ui: &mut egui::Ui,
    param_names: &[String],
    obj_names: &[String],
    trial_rows: &[TrialRow],   // 追加
)
```

`trial_rows` は `grid_canvas::show_chart` 内で既に `ctx.trial_rows.clone()` として確保されているため、
追加のクローンコストは不要（`&[TrialRow]` のスライス参照で受け取る）。

### UI 追加要素 🔵

**信頼性**: 🔵 *ユーザーヒアリングより*

既存の水平コントロール行に「観測データ表示」トグルボタンを追加：

```rust
ui.horizontal(|ui| {
    // ...既存: Parameter / Objective / Model セレクタ...
    // 追加:
    ui.separator();
    ui.toggle_value(&mut self.show_observed, "Show data");
});
```

### 描画処理追加 (`show_1d` 内) 🔵

**信頼性**: 🔵 *ユーザーヒアリング・既存 egui_plot 実装パターンより*

`show_observed == true` のとき、`egui_plot::Points` で観測データを赤丸プロットとして描画する。
既存の信頼区間バンド → ICEライン → 平均曲線 の描画順の**最後**（最前面）に配置する。

```
描画順（奥→手前）:
1. 信頼区間バンド（半透明青）
2. ICEライン（薄灰色）
3. PDP 平均曲線（青線）
4. 観測データ点（赤丸）← 新規追加、最前面
```

### 観測データ抽出ロジック 🔵

**信頼性**: 🔵 *`TrialRow.params: HashMap<String, f64>` の既存データ構造より*

```rust
fn extract_observed(
    trial_rows: &[TrialRow],
    param_name: &str,
    obj_idx: usize,
) -> Vec<[f64; 2]> {
    trial_rows
        .iter()
        .filter_map(|row| {
            let x = row.params.get(param_name).copied()?;
            let y = row.objectives.get(obj_idx).copied()?;
            Some([x, y])
        })
        .collect()
}
```

---

## ディレクトリ構造（変更対象のみ） 🔵

**信頼性**: 🔵 *既存プロジェクト構造より*

```
egui-app/src/
├── ui/
│   ├── widgets/
│   │   └── pdp_chart.rs     ← 変更（show_observed フィールド追加、show() 引数追加）
│   └── grid_canvas.rs       ← 変更（show() 呼び出しに &trial_rows 追加）
```

---

## 非機能要件

### パフォーマンス 🟡

**信頼性**: 🟡 *既存実装パターンから妥当な推測*

- 観測データ抽出は `O(N)` でトライアル数に比例。10,000 件以下では問題なし
- 毎フレーム再計算されるが、`show_observed == false` のときはゼロコスト

### 互換性 🔵

**信頼性**: 🔵 *既存実装より*

- `PdpChart::default()` に `show_observed: false` を追加するだけ
- 既存の `PdpChart2D` / その他チャートへの影響なし
- `AppMessage` の変更なし
- `rust_core` の変更なし

---

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **ヒアリング記録**: [design-interview.md](design-interview.md)
- **既存 PdpChart 実装**: `egui-app/src/ui/widgets/pdp_chart.rs`
- **既存 grid_canvas**: `egui-app/src/ui/grid_canvas.rs`

## 信頼性レベルサマリー

- 🔵 青信号: 10件 (91%)
- 🟡 黄信号: 1件 (9%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
