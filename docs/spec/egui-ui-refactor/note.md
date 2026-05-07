# egui UI リファクタリング コンテキストノート

**作成日**: 2026-05-08
**要件名**: egui-ui-refactor

---

## 技術スタック

- **言語**: Rust (edition 2021)
- **UI フレームワーク**: egui 0.x + eframe
- **コアクレート**: rust_core（tunny_core として egui-app からリンク）
- **スレッド通信**: std::sync::mpsc（SyncChannel）
- **ビルド対象**: ネイティブデスクトップのみ（WASM 対応不要）

---

## 開発ルール（layer-contract.md より）

- `state/*` は `egui` / `theme` に依存しない
- `show_*` などのUI関数が `AppState` を直接書き換える責務を増やさない
- 描画用の色情報キャッシュは UI 層状態に保持する
- `Pure Logic/Data` 層（rust_core）は UI 型（`egui::Color32` 等）禁止

---

## 関連ファイル

### 変更対象ファイル

| ファイル | 行数 | 変更内容 |
|---|---|---|
| `egui-app/src/ui/chart_registry.rs` | ~750行 | render_chart.rs + poll_chart.rs に分割 |
| `egui-app/src/ui/left_panel.rs` | ~450行 | 計算ロジック削除、UI 分割 |
| `egui-app/src/app.rs` | ~160行 | HTML レポート構築ロジック移動 |
| `egui-app/src/io/html_report.rs` | 既存 | スナップショット構築関数追加 |
| `rust_core/src/` | 既存 | 収束・トレードオフ計算関数追加 |

### 新規作成ファイル

| ファイル | 内容 |
|---|---|
| `egui-app/src/ui/render_chart.rs` | 描画専用（poll 除く） |
| `egui-app/src/ui/poll_chart.rs` | 非同期ディスパッチ専用 |
| `egui-app/src/ui/widgets/tradeoff_navigator.rs` | Trade-off Navigator UI |
| `egui-app/src/ui/widgets/convergence_card.rs` | Convergence Card UI |
| `rust_core/src/convergence.rs` | 収束診断計算関数 |

### 既存設計文書

- `docs/design/responsibility-separation-refactoring/architecture.md` — Phase 1-3 設計（実装済み）
- `docs/design/responsibility-separation-refactoring/layer-contract.md` — 層境界契約

---

## 注意事項

- Phase 1-3 リファクタリング（state/ 分割・chart_registry 抽出・MessageHandler 抽出）は **実装済み**
- 本要件は Phase 1-3 に加えた **追加リファクタリング**
- `cargo test` がグリーンを維持することが必須条件
- WASM 対応は一切不要（ネイティブ API 自由使用可）
