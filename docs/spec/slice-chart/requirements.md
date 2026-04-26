# Slice Chart 要件定義書

## 概要

Optuna の `plot_slice` に相当する Slice 図を egui-app の新規ウィジェットとして実装する。
各トライアルの「1つのパラメータ値 (X 軸) vs 目的関数値 (Y 軸)」を散布図で表示し、パレート最適トライアルを強調表示する。
ユーザーが ComboBox でパラメータと目的関数を切り替えながら感度を視覚的に把握できる。

## 関連文書

- **ヒアリング記録**: [💬 interview-record.md](interview-record.md)
- **ユーザストーリー**: [📖 user-stories.md](user-stories.md)
- **受け入れ基準**: [✅ acceptance-criteria.md](acceptance-criteria.md)
- **コンテキストノート**: [📝 note.md](note.md)
- **配線要件参照**: [chart-wiring-continuation-requirements.md](../chart-wiring-continuation-requirements.md)

## 信頼性レベル凡例

- 🔵 **青信号**: ヒアリング・既存設計文書から確実な要件
- 🟡 **黄信号**: 既存実装パターンから妥当な推測
- 🔴 **赤信号**: 推測のみ

---

## 機能要件（EARS記法）

### 通常要件

- REQ-001: システムは `SliceChart` 構造体と `show()` メソッドを `egui-app/src/ui/widgets/slice_chart.rs` に実装しなければならない 🔵 *ヒアリング（コンポーネントスコープ確認）*
- REQ-002: `show()` は `TrialRow` の `params` から選択されたパラメータ値を X 軸、`objectives` から選択された目的関数値を Y 軸とした散布図を描画しなければならない 🔵 *ヒアリング（Slice図の目的: パラメータ vs 目的関数）*
- REQ-003: システムは X 軸パラメータを選択する `egui::ComboBox`（id: `"slice_param_combo"`）を提供しなければならない 🔵 *ヒアリング（表示アクシス: パラメータ・目的の選択）*
- REQ-004: システムは Y 軸目的関数を選択する `egui::ComboBox`（id: `"slice_obj_combo"`）を提供しなければならない 🔵 *ヒアリング（表示アクシス: パラメータ・目的の選択）*
- REQ-005: `pareto_rank == 0` のトライアルは `ACCENT_BLUE`（`#2563EB`）で強調表示し、それ以外は通常色（`Color32::from_rgb(100, 149, 237)` 相当の薄い青）で描画しなければならない 🔵 *ヒアリング（表示アクシス: パレート強調）*
- REQ-006: `widgets/mod.rs` に `pub mod slice_chart;` を追加しなければならない 🟡 *既存 mod.rs パターンから*

### 条件付き要件

- REQ-101: `trial_rows` が空の場合、システムは "No trial data." と表示し、クラッシュしてはならない 🔵 *既存ウィジェット全ての EmptyState パターン*
- REQ-102: `param_names` が空の場合、システムは "No parameters." と表示しなければならない 🟡 *既存 scatter_matrix.rs パターン*
- REQ-103: 選択中のパラメータが `TrialRow.params` に存在しないトライアルは散布図から除外しなければならない（クラッシュ禁止） 🟡 *HashMap::get の None 処理*
- REQ-104: `obj_names` が空の場合、システムは "No objectives." と表示しなければならない 🟡 *既存ウィジェットパターン*

### 状態要件

- REQ-201: `selected_param_idx` は初期値 0（最初のパラメータ）で、ComboBox 操作で変更されなければならない 🔵 *ヒアリング（パラメータ・目的の選択）*
- REQ-202: `selected_obj_idx` は初期値 0（最初の目的関数）で、ComboBox 操作で変更されなければならない 🔵 *ヒアリング（パラメータ・目的の選択）*
- REQ-203: パラメータ数が 0 から増加したとき、`selected_param_idx` は範囲外にならないようクランプしなければならない 🟡 *既存ウィジェット（parallel_coords.rs）のインデックス管理パターン*

### 制約要件

- REQ-401: ComboBox の ID は `from_id_salt` を使用し、同一パネル内の他 ComboBox と衝突してはならない 🔵 *repo memory: ComboBox duplicate ID は禁止*
- REQ-402: ウィジェットは `egui::Ui` への参照のみで描画でき、外部状態や非同期処理に依存してはならない（同期描画） 🔵 *既存ウィジェット全ての同期パターン*
- REQ-403: 描画ロジックは `show()` と純粋関数（テスト可能）に分離しなければならない 🔵 *既存 pdp_chart.rs・scatter_matrix.rs パターン*

---

## 非機能要件

### パフォーマンス

- NFR-001: 10,000 トライアルの散布図描画は 16ms 以内（60fps）に完了しなければならない 🟡 *egui_plot 点描画コスト、fast-rendering-downsampling 設計から推測*

### セキュリティ

- NFR-101: ユーザー入力（param_name, obj_name）は文字列として安全に扱い、コマンドインジェクションの余地がないこと 🔵 *Rust の型安全性により保証*

---

## Edge ケース

### エラー処理

- EDGE-001: `objectives` の長さが `selected_obj_idx` を下回るトライアルは安全にスキップする 🟡 *objectives が可変長の場合*
- EDGE-002: X 範囲・Y 範囲がすべて同一値（点が1つ）の場合、軸が潰れず最小幅で表示される 🟡 *egui_plot の min_bounds 設定で対処*
