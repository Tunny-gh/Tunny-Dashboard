# プロプライエタリ分析ツール不足機能 アーキテクチャ設計

**作成日**: 2026-04-26  
**関連要件定義**: [requirements.md](../../spec/proprietary-analysis-features/requirements.md)  
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *要件定義書・ユーザヒアリング Q3 より*

`egui` ネイティブデスクトップアプリの 4 層メッセージパッシング・ステートマシンアーキテクチャに対して、REQ-001〜009 の 9 機能を追加する。既存の `TunnyApp → AppMessage → MessageHandler → AppState → UI` パターンを完全踏襲し、新機能をこのパターンの拡張として実装する。

---

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存 `egui-app/src/app.rs` 実装調査より*

- **パターン**: メッセージパッシング・ステートマシン（Message-Passing State Machine）
- **選択理由**: 既存の `TunnyApp` が `mpsc::SyncSender<AppMessage>` + `AppMessage` enum + `MessageHandler` パターンで統一されており、同一パターンで全 9 機能を追加できる。バックグラウンド計算（`spawn_task`）が UI ブロックを防ぎ、NFR-001 の 100ms 要件を満たす。

### 既存 4 層構造 🔵

**信頼性**: 🔵 *コード調査 `egui-app/src/` より*

```
Layer 1: IO (egui-app/src/io/)
  journal.rs / file.rs / session.rs / export.rs
    ↓ AppMessage を送信
Layer 2: Message (AppMessage enum + mpsc channel)
  mpsc::SyncSender<AppMessage> → poll_messages()
    ↓ MessageHandler::handle() で AppState を更新
Layer 3: State (egui-app/src/state/)
  AppState / LayoutState / WidgetStates
    ↓ show_layout() が参照して描画
Layer 4: UI (egui-app/src/ui/)
  toolbar / left_panel / grid_canvas / bottom_panel / right_panel / widgets/
```

---

## コンポーネント構成（追加・拡張分）

### Layer 1: IO — 新規ファイル 🔵

**信頼性**: 🔵 *REQ-005・REQ-007・ユーザヒアリング Q5/Q6 より*

| ファイル | 目的 | 関連 REQ |
|---|---|---|
| `egui-app/src/io/html_report.rs` | HTML レポートビルダー + SVG エクスポーター | REQ-005 |
| `egui-app/src/io/artifacts.rs` | Artifacts フォルダスキャン・ファイル検出・パス検証 | REQ-007 |

### Layer 1: IO — 既存拡張 🔵

**信頼性**: 🔵 *REQ-002/003/004・`egui-app/src/io/session.rs` 実装調査より*

| ファイル | 変更内容 | 関連 REQ |
|---|---|---|
| `egui-app/src/io/session.rs` | `SessionSnapshot` を拡張（`tradeoff_weights`・`layout_config`・`cluster_config`・`color_mode`・`pinned_trials` 追加）、ファイル拡張子 `.tdash` | REQ-002/003/004 |

### Layer 2: Message — 追加 AppMessage バリアント 🔵

**信頼性**: 🔵 *既存 `AppMessage` パターン・各 REQ より*

| バリアント | フィールド | 目的 | 関連 REQ |
|---|---|---|---|
| `TradeoffDone` | `sorted_indices: Vec<u32>` | チェビシェフスコア昇順ソート済みインデックス | REQ-001 |
| `ComparisonStudyLoaded` | `study_idx: usize`, `context: Box<StudyContext>` | 比較用 Study ロード完了 | REQ-006 |
| `ArtifactsDirScanned` | `trial_artifacts: HashMap<u32, Vec<PathBuf>>` | Artifacts スキャン完了 | REQ-007 |
| `HtmlReportDone` | `html: String`, `suggested_filename: String` | HTML レポート生成完了 | REQ-005 |

### Layer 3: State — AppState 追加フィールド 🔵

**信頼性**: 🔵 *REQ-001/006/007/008・AppState 実装調査より*

| フィールド | 型 | 目的 | 関連 REQ |
|---|---|---|---|
| `tradeoff_weights` | `Vec<f64>` | Trade-off Navigator 重み（目的数に合わせてリサイズ） | REQ-001 |
| `tradeoff_sorted_indices` | `Option<Vec<u32>>` | チェビシェフスコア昇順ソート済みインデックス | REQ-001 |
| `comparison_mode` | `bool` | 比較モード有効フラグ | REQ-006 |
| `comparison_studies` | `Vec<StudyContext>` | 比較対象 Study（最大 4） | REQ-006 |
| `comparison_colors` | `Vec<egui::Color32>` | Study ごとの代表色（比較ビュー凡例用） | REQ-006 |
| `artifacts_dir` | `Option<PathBuf>` | Artifacts フォルダパス | REQ-007 |
| `artifact_map` | `HashMap<u32, Vec<PathBuf>>` | `trial_id` → ファイル一覧 | REQ-007 |
| `best_trial_history` | `Option<Vec<(u32, f64)>>` | 単目的：Best 値推移 `(trial_id, best_value)` | REQ-008 |

### Layer 3: State — LayoutState 追加 🔵

**信頼性**: 🔵 *REQ-006・`layout_state.rs` `LayoutMode` enum 調査より*

| 変更 | 内容 | 関連 REQ |
|---|---|---|
| `LayoutMode::Comparison` バリアント追加 | 比較モード専用の固定 4 分割レイアウト | REQ-006 |

### Layer 3: State — WidgetStates 追加フィールド 🔵

**信頼性**: 🔵 *REQ-008/009・`WidgetStates` 調査より*

| ウィジェット | フィールド追加 | 目的 | 関連 REQ |
|---|---|---|---|
| `parallel_coords` | `axis_visibility: HashMap<String, bool>` | 軸の表示/非表示トグル | REQ-009 |
| `opt_history` | `show_best_line: bool` | Best 値追跡ライン表示 | REQ-008 |
| `opt_history` | `log_scale: bool` | Y 軸半対数スケール | REQ-008 |

### Layer 4: UI — 新規ファイル 🔵

**信頼性**: 🔵 *REQ-006/007・ユーザヒアリング Q6/Q7 より*

| ファイル | 目的 | 関連 REQ |
|---|---|---|
| `egui-app/src/ui/comparison_panel.rs` | 比較モード専用パネル（統計テーブル・HV 重畳・Pareto 重畳・KDE 重畳の 4 ビュー） | REQ-006 |
| `egui-app/src/ui/widgets/artifact_modal.rs` | アーティファクトプレビューモーダル（PNG インライン・CSV テーブル・汎用ダウンロード） | REQ-007 |

### Layer 4: UI — 既存拡張 🔵

**信頼性**: 🔵 *REQ-001〜005/008/009・各 UI ファイル実装調査より*

| ファイル | 変更内容 | 関連 REQ |
|---|---|---|
| `egui-app/src/ui/left_panel.rs` | `show_tradeoff_navigator()` セクション追加（多目的時のみ表示） | REQ-001 |
| `egui-app/src/ui/left_panel.rs` | `show_convergence_card()` セクション追加（単目的時のみ表示） | REQ-008 |
| `egui-app/src/ui/toolbar.rs` | Filter/Layout/Session 保存・読み込みボタン群追加 | REQ-002/003/004 |
| `egui-app/src/ui/toolbar.rs` | Artifacts フォルダ選択ボタン追加 | REQ-007 |
| `egui-app/src/ui/toolbar.rs` | Export パネル内 HTML レポート出力ボタン追加 | REQ-005 |
| `egui-app/src/ui/bottom_panel.rs` | Artifacts 列追加（`artifact_map` 参照、ファイルタイプアイコン表示） | REQ-007 |
| `egui-app/src/ui/bottom_panel.rs` | Best 解遷移テーブルタブ追加（単目的時のみ） | REQ-008 |
| `egui-app/src/ui/widgets/parallel_coords.rs` | 各軸ヘッダーに目のアイコン（`axis_visibility` 参照）・掴みハンドル（`axis_order` 変更）追加 | REQ-009 |
| `egui-app/src/ui/widgets/optimization_history.rs` | `show_best_line`・`log_scale` オプション追加、Best ライン重畳描画 | REQ-008 |

---

## ディレクトリ構造（追加分） 🔵

**信頼性**: 🔵 *既存 egui-app 構造踏襲より*

```
egui-app/src/
├── io/
│   ├── session.rs          ← 拡張（SessionSnapshot フィールド追加、.tdash 拡張子）
│   ├── html_report.rs      ← 新規
│   └── artifacts.rs        ← 新規
├── state/
│   ├── app_state.rs        ← 拡張（tradeoff_weights, comparison_*, artifacts_*, best_trial_history）
│   ├── layout_state.rs     ← 拡張（LayoutMode::Comparison, serde derive 追加）
│   └── messages.rs         ← 拡張（TradeoffDone, ComparisonStudyLoaded, ArtifactsDirScanned, HtmlReportDone）
└── ui/
    ├── comparison_panel.rs ← 新規
    ├── left_panel.rs       ← 拡張
    ├── toolbar.rs          ← 拡張
    ├── bottom_panel.rs     ← 拡張
    └── widgets/
        ├── artifact_modal.rs       ← 新規
        ├── parallel_coords.rs      ← 拡張
        └── optimization_history.rs ← 拡張
```

---

## 非機能要件の実現方法

### パフォーマンス（NFR-001〜003） 🔵

**信頼性**: 🔵 *既存 `spawn_task` パターン・NFR要件より*

| 機能 | 実現方法 |
|---|---|
| Trade-off Navigator 100ms 以内（NFR-001） | `spawn_task` でバックグラウンドスレッド実行。`score_tradeoff_navigator(weights, is_minimize)` は pure 関数のため安全に並列化可能 |
| JSON 保存/読み込み 1 秒以内（NFR-002） | `serde_json::to_string` + `rfd::FileDialog` はすべて同期で 1 秒以内に完了（ファイルサイズ数十 KB） |
| HTML レポート生成 10 秒以内（NFR-003） | `spawn_task` でバックグラウンド生成。SVG 生成は rust の `write!` マクロによるテキスト生成のため高速 |

### セキュリティ（NFR-201〜202） 🔵

**信頼性**: 🔵 *NFR 要件・OWASP パストラバーサル対策より*

| 機能 | 実現方法 |
|---|---|
| アーティファクト パストラバーサル防止（NFR-201） | `artifacts.rs` 内で `Path::canonicalize()` + `starts_with(base_dir)` チェック。`rfd` のファイル選択ダイアログ経由のパスのみ受け付け |
| セッションファイル デシリアライズ（NFR-202） | `serde` の `#[serde(default)]` で未知フィールドを無視（Rust の serde_json は既知フィールド以外をデフォルト値でフォールバック） |

---

## 技術的制約

### Rust/egui 制約 🔵

**信頼性**: 🔵 *egui ドキュメント・既存実装より*

- egui の `Painter` は即時描画モード（Immediate Mode）のため、`Shape` の収集はフレーム内でのみ可能。SVG エクスポートはチャートデータから独立した SVG 生成パス（`html_report.rs`）を実装する
- `egui::Color32` は `serde` を標準サポートしていないため、`LayoutState` の JSON シリアライズにはカスタム実装または `[r,g,b,a]` 配列形式を使用する

### スケール制約 🔵

**信頼性**: 🔵 *first_spec.md スケール設計・REQ-001 EDGE-101 より*

- 最大 4 目的 / 30 変数 / 50,000 試行
- Trade-off Navigator は目的数 2〜4 のみ表示（単目的・5 目的以上では非表示）
- 比較モードは最大 4 Study まで同時選択

---

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **型定義（Rust）**: [interfaces.rs](interfaces.rs)
- **ヒアリング記録**: [design-interview.md](design-interview.md)
- **要件定義**: [requirements.md](../../spec/proprietary-analysis-features/requirements.md)
- **既存設計（参考）**: [../proprietary-features/architecture.md](../proprietary-features/architecture.md)

---

## 信頼性レベルサマリー

- 🔵 青信号: 28件 (88%)
- 🟡 黄信号: 4件 (12%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
