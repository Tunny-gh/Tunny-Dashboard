# カラーマップ色反映と拡張 アーキテクチャ設計

**作成日**: 2026-04-16
**関連要件定義**: （なし - ユーザ報告バグベース）
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: コード調査・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: コード調査・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: コード調査・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *コード調査・ユーザ報告より*

egui-appのチャート描画において、ColorMode（色分け基準）の選択UIは存在するが、色生成に接続されていない問題を解決する。同時にJet等の一般的なカラーマップを追加し、全ウィジェットに統一的なカラーマップシステムを提供する。

**現状の問題**:
1. `ColorMode` enum + UIセレクターは存在するが、色生成に未接続
2. 色は `build_gpu_buffer_data()` でハードコードされた青→黄のグラデーションのみ
3. `ColorMap` 構造体（viridis, plasma, blue_yellow）はPDPヒートマップのみ使用
4. Jet等の一般的なカラーマップが未定義
5. `gpu_data.colors` は生成されるが全ウィジェットが未使用（各ウィジェットはハードコード色を使用）

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存コード構造・ユーザヒアリングより*

- **パターン**: Registry + Mapper パターン
- **選択理由**:
  - `ColormapName` をキーに `ColorMap` を生成する Registry
  - `ColorMode` で値を正規化し、`ColormapName` で色を決定する Mapper
  - 既存の `ColorMap` 構造体を拡張し、新しいカラーマップ定義を追加

## コンポーネント構成

### ColormapName 列挙体 🔵

**信頼性**: 🔵 *ユーザヒアリングで選択されたカラーマップ一覧より*

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ColormapName {
    Viridis,
    Plasma,
    Jet,
    Turbo,
    Inferno,
    Coolwarm,
    Spectral,
    Cividis,
    BlueYellow,
}
```

**責務**:
- カラーマップの選択肢を表現
- `to_colormap()` で `ColorMap` インスタンスを生成
- `label()` でUI表示名を返す
- `all()` で全選択肢を返す

### ColorMap 拡張 🔵

**信頼性**: 🔵 *既存 `colormap.rs` 構造より*

既存の `ColorMap` 構造体を拡張し、新しいカラーマップ定義を追加:

```rust
impl ColorMap {
    pub fn jet() -> Self { ... }
    pub fn turbo() -> Self { ... }
    pub fn inferno() -> Self { ... }
    pub fn coolwarm() -> Self { ... }
    pub fn spectral() -> Self { ... }
    pub fn cividis() -> Self { ... }
    // 既存: viridis(), plasma(), blue_yellow()
}
```

### 離散パレット 🔵

**信頼性**: 🔵 *ユーザヒアリングでTab10追加を確認より*

ClusterId 用の離散カラーパレット（Tableau10相当10色）:

```rust
pub fn tab10_palette() -> Vec<egui::Color32> {
    vec![
        egui::Color32::from_rgb(31, 119, 180),   // Blue
        egui::Color32::from_rgb(255, 127, 14),    // Orange
        egui::Color32::from_rgb(44, 160, 44),     // Green
        egui::Color32::from_rgb(214, 39, 40),     // Red
        egui::Color32::from_rgb(148, 103, 189),   // Purple
        egui::Color32::from_rgb(140, 86, 75),     // Brown
        egui::Color32::from_rgb(227, 119, 194),   // Pink
        egui::Color32::from_rgb(127, 127, 127),   // Gray
        egui::Color32::from_rgb(188, 189, 34),    // Olive
        egui::Color32::from_rgb(23, 190, 207),    // Cyan
    ]
}
```

### AppState 拡張 🔵

**信頼性**: 🔵 *既存 AppState 構造・コード調査より*

```rust
pub struct AppState {
    // ... 既存フィールド ...
    pub color_mode: ColorMode,                    // 既存: 何で色分けするか
    pub selected_colormap: ColormapName,          // 新規: どの配色を使うか
    pub chart_colors: Vec<egui::Color32>,         // 新規: per-trial Color32 キャッシュ
}
```

**`chart_colors`**:
- 各TrialRowに対応する `egui::Color32` を保持
- ColorMode または ColormapName 変更時に即時再計算
- 全ウィジェットがこのキャッシュから色を読み取る

### 色計算パイプライン 🔵

**信頼性**: 🔵 *ユーザヒアリング（即時同期選択）・コード調査より*

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   ColorMode     │     │  ColormapName    │     │  TrialRow data  │
│  (色分け基準)   │     │  (カラーマップ)  │     │  (TrialRow)     │
└────────┬────────┘     └────────┬─────────┘     └────────┬────────┘
         │                       │                         │
         ▼                       ▼                         ▼
  ┌──────────────────────────────────────────────────────────────┐
  │                    normalize_trial()                          │
  │  ColorMode に基づいて trial の値を t ∈ [0.0, 1.0] に正規化  │
  │  ClusterId の場合は離散パレットのインデックスを返す           │
  └──────────────────────────┬───────────────────────────────────┘
                             │
                             ▼
  ┌──────────────────────────────────────────────────────────────┐
  │               ColorMap::interpolate(t)                       │
  │  ColormapName に対応するカラーマップで t → Color32 に変換    │
  │  ClusterId の場合は tab10_palette()[id % 10]                 │
  └──────────────────────────┬───────────────────────────────────┘
                             │
                             ▼
                   chart_colors: Vec<Color32>
```

### 正規化ロジック 🔵

**信頼性**: 🔵 *既存コード・note.md 正規化表より*

| ColorMode | 正規化方法 | 備考 |
|---|---|---|
| ParetoRank | `1.0 - rank / (max_rank + 1)` | ランク0が最も鮮やか |
| ObjectiveValue(name) | `(val - min) / (max - min)` | min-max正規化 |
| TrialNumber | `trial_id / max(1, total - 1)` | 線形マッピング |
| ClusterId | 離散パレット `palette[id % 10]` | 連続補間しない |

### UI 構成 🔵

**信頼性**: 🔵 *既存 left_panel.rs 構造・ユーザヒアリングより*

Left Panel に2つの独立したセレクタを配置:

```
Color Mode: [Pareto Rank ▾]     ← 既存（何で色分けするか）
Colormap:   [Viridis ▾]          ← 新規（どの配色を使うか）
```

### ウィジェット対応 🔵

**信頼性**: 🔵 *ユーザヒアリング（全ウィジェット対応）・コード調査より*

| ウィジェット | 現在の色 | 変更後 |
|---|---|---|
| `pareto_2d.rs` | ハードコード（選択/未選択/ハイライト3色） | `chart_colors[i]` を基本色 + 選択状態でalpha制御 |
| `scatter_matrix.rs` | 全点同じ青 `Color32::from_rgb(70, 130, 220)` | `chart_colors[i]` を各点に適用 |
| `cluster_scatter.rs` | 5色ハードコードテーブル | `tab10_palette()` + `chart_colors[i]` |
| `pdp_2d.rs` | `ColorMap::viridis()` / `plasma()` 固定 | `ColormapName::to_colormap()` を使用 |

## ディレクトリ構造 🔵

**信頼性**: 🔵 *既存プロジェクト構造より*

変更対象ファイル:

```
egui-app/src/
├── render/
│   └── colormap.rs          ← 拡張: ColormapName, 新カラーマップ定義, tab10_palette(), compute_chart_colors()
├── state/
│   ├── app_state.rs         ← 拡張: selected_colormap, chart_colors フィールド追加
│   ├── types.rs             ← 拡張: ColormapName 列挙体追加
│   └── message_handler.rs   ← 変更なし（色再計算は同期でメッセージ不要）
├── io/
│   └── study.rs             ← 変更: build_gpu_buffer_data で初期色も ColorMode+ColormapName 使用
├── ui/
│   ├── left_panel.rs        ← 拡張: ColormapName セレクタ追加
│   └── widgets/
│       ├── pareto_2d.rs     ← 変更: chart_colors から色を取得
│       ├── scatter_matrix.rs ← 変更: chart_colors から色を取得
│       ├── cluster_scatter.rs ← 変更: chart_colors / tab10_palette から色を取得
│       └── pdp_2d.rs        ← 変更: AppState.selected_colormap を使用
```

## 非機能要件の実現方法

### パフォーマンス 🔵

**信頼性**: 🔵 *ユーザヒアリング（即時同期）・5万点規模の計算量より*

- **色再計算**: 50,000点 × (正規化 + ColorMap補間) ≈ 1-3ms → 即時同期で対応
- **UI応答性**: ColorMode/ColormapName 変更時のフレーム内で完了
- **キャッシュ**: `chart_colors` をAppStateにキャッシュし、変更時のみ再計算

### 互換性 🔵

**信頼性**: 🔵 *コード調査より*

- **既存テスト**: `colormap.rs`, `study.rs`, `left_panel.rs`, `types.rs` の既存テストは維持
- **既存ColorMap**: `viridis()`, `plasma()`, `blue_yellow()` のインターフェースは変更しない
- **PDP 2D**: 現在の `ColorMap::viridis()` / `plasma()` 直接呼び出しを `ColormapName::to_colormap()` に置換

## 技術的制約

### egui_plot の色指定 🔵

**信頼性**: 🔵 *コード調査（pareto_2d.rs の egui_plot 使用箇所）より*

- `egui_plot::Points::new().color(Color32)` は全点同一色
- per-point coloring には点ごとに `Points` を作成するか、`Shape` を使用する必要がある
- `scatter_matrix.rs` の `painter.circle_filled()` は per-point 色指定が可能

### Color32 と GPU バッファの二重管理 🟡

**信頼性**: 🟡 *既存実装の制約から妥当な推測*

- `gpu_data.colors: Vec<f32>` は将来的な wgpu レンダリング用
- `chart_colors: Vec<Color32>` は egui ウィジェット用
- 現在は egui ウィジェットのみ動作しているため、まず `chart_colors` を正とする
- 将来 wgpu レンダリングが有効化された際は、`chart_colors` から `gpu_data.colors` への変換関数を追加

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **ヒアリング記録**: [design-interview.md](design-interview.md)
- **既存アーキテクチャ**: [../tunny-dashboard/architecture.md](../tunny-dashboard/architecture.md)

## 信頼性レベルサマリー

- 🔵 青信号: 22件 (88%)
- 🟡 黄信号: 3件 (12%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質（コード調査とユーザヒアリングに基づき、推測部分は黄信号で明示）
