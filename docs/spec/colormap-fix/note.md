# カラーマップ色反映と拡張 コンテキストノート

**作成日**: 2026-04-16
**プロジェクト**: Tunny Dashboard (featura/egui ブランチ)

---

## 技術スタック

| 層 | 技術 |
| --- | --- |
| 言語 | Rust（egui-app クレート、rust_core クレート） |
| UI フレームワーク | egui（ネイティブ Rust GUI） |
| レンダリング | wgpu（GPU 散布図）、egui Painters |
| テスト | cargo test（Rust 単体テスト） |

**注意**: featura/egui ブランチでは TypeScript/React は使用しない。後方互換性は不要。

---

## 開発ルール

- **Rust のみ**: featura/egui ブランチに TypeScript コードは存在しない
- **GPU バッファ色形式**: `Vec<f32>`、1点につき 4 float（RGBA、0.0-1.0）
- **ColorMap 内部形式**: `egui::Color32` で停止点を定義、`interpolate()` も `Color32` を返す
- **GPU バッファへの変換**: `Color32` → `f32` RGBA への変換が必要（`Color32::r()` / `g()` / `b()` は `u8` を返すため `u8 / 255.0` で f32 に変換）
- **全チャートウィジェット**: `AppState.current_study.gpu_data.colors` から色を読み取る
- **既存テストを破壊しない**: `colormap.rs` / `study.rs` / `left_panel.rs` / `types.rs` の既存テストが引き続き通ること

---

## 関連実装

### ColorMode 列挙型

`egui-app/src/state/types.rs` (84-101行):

```rust
pub enum ColorMode {
    ParetoRank,
    ObjectiveValue(String),
    TrialNumber,
    ClusterId,
}
```

現在、`ColorMode` は「何で色分けするか」を決めるが、「どのカラーマップを使うか」は独立していない。

### ColorMap 構造体

`egui-app/src/render/colormap.rs`:

- `ColorMap { stops: Vec<(f32, egui::Color32)> }`
- 既存メソッド: `viridis()`, `plasma()`, `blue_yellow()`
- `interpolate(&self, t: f32) -> egui::Color32` — t を [0.0, 1.0] にクランプして停止点間を線形補間
- `compute_point_alpha(trial_id, selected_indices)` — 選択状態に基づくアルファ値計算

### GPU バッファ色生成

`egui-app/src/io/study.rs` (15-45行) — `build_gpu_buffer_data()`:

- **現在の問題**: ハードコードされた blue-to-yellow スケールのみ使用
- `ColorMode` を参照せず、常に Pareto ランクベースの固定色を生成
- `ColorMap::blue_yellow().interpolate(t)` を使っていない（手動で `r = t; g = 0.5 + t * 0.5; b = 1.0 - t` を計算）

```rust
// 現在のハードコード実装
let t = 1.0 - (rank as f32 / (max_rank + 1) as f32);
let r = t;
let g = 0.5 + t * 0.5;
let b = 1.0 - t;
let a = 0.8_f32;
```

### UI カラーモードセレクタ

`egui-app/src/ui/left_panel.rs` (88-123行) — `show_color_mode()`:

- `ComboBox` で `ColorMode` を選択（ParetoRank, TrialNumber, Objective, ClusterId）
- **カラーマップ選択UIは存在しない**（ColorMode とは独立した ColormapName セレクタが必要）

### GPU データ構造

`egui-app/src/state/types.rs`:

```rust
pub struct GpuBufferData {
    pub positions: Vec<f32>,
    pub positions3d: Vec<f32>,
    pub colors: Vec<f32>,     // RGBA per point, 4 floats each
    pub sizes: Vec<f32>,
    pub trial_count: u32,
}
```

### AppState

`egui-app/src/state/app_state.rs`:

```rust
pub struct AppState {
    // ...
    pub color_mode: ColorMode,  // 現在のカラーモード
    // selected_colormap フィールドは未実装
}
```

### PDP 2D ウィジェット

- `ColorMap::viridis()` と `ColorMap::plasma()` をヒートマップ表示に使用
- 散布図/パレート/クラスターウィジェットはハードコード色を使用

---

## 設計文書

| 文書 | パス | 内容 |
| --- | --- | --- |
| アーキテクチャ | `docs/design/tunny-dashboard/architecture.md` | 4層アーキテクチャ全体設計 |
| データフロー | `docs/design/tunny-dashboard/dataflow.md` | データフロー |
| チャート実装 | `docs/design/chart-implementation/` | チャート実装詳細 |
| 責務分離リファクタリング | `docs/design/responsibility-separation-refactoring/` | 最近のリファクタリング |

---

## 主な変更対象ファイル

| ファイル | 変更内容 |
| --- | --- |
| `egui-app/src/render/colormap.rs` | カラーマップ追加（Jet 等）、ColormapName 列挙体、ColorMode→ColorMap 適用ロジック |
| `egui-app/src/io/study.rs` | `build_gpu_buffer_data()` を `ColorMode` + `ColormapName` を参照するよう書き直し |
| `egui-app/src/ui/left_panel.rs` | カラーマップセレクタ UI 追加（ColorMode セレクタとは独立） |
| `egui-app/src/state/types.rs` | `ColormapName` 列挙体追加（または `colormap.rs` に定義） |
| `egui-app/src/state/app_state.rs` | `selected_colormap` フィールド追加 |
| ウィジェット各ファイル | `gpu_data.colors` を参照する箇所の確認・必要に応じる修正 |

---

## 注意事項

### Color32 → f32 RGBA 変換パターン

`ColorMap::interpolate()` は `egui::Color32` を返すが、GPU バッファは `Vec<f32>`（RGBA 0.0-1.0）。
変換ヘルパーを `colormap.rs` に追加する想定:

```rust
pub fn color32_to_rgba_f32(c: egui::Color32, alpha: f32) -> [f32; 4] {
    [
        c.r() as f32 / 255.0,
        c.g() as f32 / 255.0,
        c.b() as f32 / 255.0,
        alpha,
    ]
}
```

### ColorMode ごとの正規化ロジック

各 ColorMode で値を [0.0, 1.0] に正規化する方法が異なる:

| ColorMode | 正規化方法 |
| --- | --- |
| ParetoRank | `1.0 - rank / (max_rank + 1)`（ランク0が最も高い） |
| ObjectiveValue | `(val - min) / (max - min)`（min-max正規化） |
| TrialNumber | `trial_id / (total - 1)`（線形） |
| ClusterId | クラスタID → 離散色マップ（等間隔 or ハッシュベース） |

### ClusterId の離散色の扱い

ClusterId は連続値ではなく離散値のため、連続カラーマップの補間では適切に表現できない。
離散パレット（tab10 相当）を用意するか、`ColormapName::Discrete` のような分類が必要。

### PDP ヒートマップとの統合

PDP 2D ウィジェットはすでに `ColorMap::viridis()` / `plasma()` を直接使用している。
`ColormapName` をAppState に追加した場合、PDP ウィジェットもそれを参照するか、
PDP 用のカラーマップ選択を独立させるかの設計判断が必要。
