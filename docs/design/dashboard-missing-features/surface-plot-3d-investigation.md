# Surface Plot 3D描画方式調査

**作成日**: 2026-05-12  
**対象タスク**: TASK-2240  
**前提**: TASK-2239 で 2D Heatmap/Contour が実装済み。本文書は 3D 拡張方針の決定に使用する。

---

## 1. 候補方式の比較

### 方式 A: `egui_plot` の Mesh2D / Custom Paint

**概要**: `egui_plot` の `plot_ui.add()` で独自 Item を描き、painter API で三角メッシュを塗る。

| 観点 | 評価 |
|------|------|
| 実装コスト | 低 — 既存 egui_plot import に追加のみ |
| 3D 感 | 弱 — 等色面（伪3D/投影）は可能。カメラ回転は手書き必要 |
| インタラクション | ドラッグ回転を手実装すると 200～400 行規模 |
| スクリーンショット互換性 | ○ — egui レイヤーなのでキャプチャ可 |
| 追加 crate | なし |
| **総評** | 2.5D 等高線投影には十分。ガチな 3D 回転には不向き |

### 方式 B: `egui_wgpu` PaintCallback

**概要**: `egui::PaintCallback` でカスタム wgpu render pass を追加し、z-buffer + 頂点バッファで 3D を描く。

| 観点 | 評価 |
|------|------|
| 実装コスト | 高 — vertex/fragment shader、depth buffer、MVP matrix が必要 |
| 3D 感 | 強 — フル 3D、回転・ズーム可 |
| インタラクション | egui の入力を手書きで 3D カメラに変換が必要（例: arcball） |
| スクリーンショット互換性 | △ — wgpu テクスチャからピクセル読み取りには `render_to_texture` + `copy_to_cpu` が必要 |
| 追加 crate | なし（eframe が既に wgpu 依存） |
| **総評** | 最もリッチな 3D が実現できるが、実装コスト・テストコスト共に最大 |

### 方式 C: オフスクリーン Texture + `egui::Image`

**概要**: CPU 側で z 値を色マップして `RgbaImage` を生成し、`egui::TextureHandle` で表示する。2.5D 疑似 3D。

| 観点 | 評価 |
|------|------|
| 実装コスト | 中 — image crate で RGBA ピクセル計算のみ |
| 3D 感 | 中 — 視点固定の投影は可。インタラクティブ回転は困難 |
| インタラクション | スライダーで仰角・方位角を変えることは可能 |
| スクリーンショット互換性 | ◎ — `egui::Image` なので通常の UI キャプチャで取得可 |
| 追加 crate | `image` crate（既に依存済み） |
| **総評** | 実装と性能のバランスが良い。Phase 1 の 3D として採用しやすい |

---

## 2. 推奨方式

### 短期 (Phase 1 — TASK-2240 相当)

**方式 C (オフスクリーン Texture)** を推奨。

理由:
- `image` crate は既に `Cargo.toml` に記載済み（`features = ["png", "jpeg"]`）
- 追加 crate なし・shader なし
- PNG キャプチャ（TASK-2245）と完全互換
- CPU 計算でデバッグ容易

最小実装例（疑似コード）:
```rust
fn render_surface_to_texture(
    ctx: &egui::Context,
    result: &SurfacePlotResult,
    size: (u32, u32),
) -> egui::TextureHandle {
    let mut img = image::RgbaImage::new(size.0, size.1);
    // ... z_values をピクセルに変換 ...
    ctx.load_texture("surface_plot", egui::ColorImage::from_rgba_unmultiplied(
        [size.0 as usize, size.1 as usize],
        img.as_raw(),
    ), egui::TextureOptions::default())
}
```

### 将来 (Phase 2 — オプション)

**方式 B (egui_wgpu PaintCallback)** へ移行。arcball カメラライブラリ（例: `egui-gizmo` や独自実装）を追加。  
条件: ユーザーからインタラクティブな 3D 回転の要求が発生した場合のみ実装する。

---

## 3. 採用しない方式の理由

- **方式 A (egui_plot custom paint)**: カメラ回転実装が 400 行超になる割に方式 C と品質差が小さい
- **方式 B (wgpu callback) の即時採用**: スクリーンショット互換性の問題と、shader のテストが困難なため Phase 1 には不向き

---

## 4. 既知リスクと前提条件

| リスク | 内容 | 対策 |
|--------|------|------|
| テクスチャ更新コスト | 毎フレーム更新すると CPU 負荷大 | 計算完了時のみ再生成（`result` が変わった時だけ） |
| メモリコスト | 512×512 RGBA = 1 MB/テクスチャ | n_grid を 30 以下に制限（TASK-2239 準拠） |
| PNG キャプチャ | 方式 C では egui レイヤーなので TASK-2245 と互換 | 確認済み |
| カメラ操作 | 方式 C では固定視点のみ | スライダーで仰角/方位角 UI として代替可 |
| wgpu feature | 方式 B の将来移行時は `wgpu::Features::TEXTURE_BINDING_ARRAY` 不要 | eframe の default wgpu feature で対応可 |

---

## 5. ビルド確認

本調査タスクでは新規コードを本番モジュールに追加しないため、既存 `cargo build` への影響はない。  
TASK-2239 の `surface_plot.rs` が追加済みでありビルドが通ることで「prototype_module_compiles_without_breaking_existing_build」を満たす。

---

## 6. 次ステップ

- Phase 1 として方式 C の実装を検討する場合は、`render_chart.rs` の SurfacePlot arm を拡張する  
- 方式 B への移行判断は別タスクとして起票する（要 wgpu shader 設計）
