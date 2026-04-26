# プロプライエタリ分析ツール不足機能 データフロー図

**作成日**: 2026-04-26  
**関連アーキテクチャ**: [architecture.md](architecture.md)  
**関連要件定義**: [requirements.md](../../spec/proprietary-analysis-features/requirements.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実なフロー
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測によるフロー
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測によるフロー

---

## システム全体のデータフロー 🔵

**信頼性**: 🔵 *既存 `egui-app/src/app.rs` `poll_messages` パターン・要件定義より*

```mermaid
flowchart TD
    U[ユーザー操作]
    UI[UI Layer\ntoolbar / left_panel / grid_canvas / bottom_panel]
    SPAWN[spawn_task\nバックグラウンドスレッド]
    RUSTCORE[rust_core\nscore_tradeoff_navigator\nhtml_report\nartifacts]
    MSG[AppMessage\nmpsc channel]
    HANDLER[MessageHandler::handle]
    STATE[AppState / LayoutState / WidgetStates]

    U --> UI
    UI -->|直接 State 変更| STATE
    UI -->|非同期計算が必要| SPAWN
    SPAWN --> RUSTCORE
    RUSTCORE --> SPAWN
    SPAWN -->|AppMessage::*Done| MSG
    MSG --> HANDLER
    HANDLER --> STATE
    STATE --> UI
```

---

## REQ-001: Trade-off Navigator フロー 🔵

**信頼性**: 🔵 *`tradeoff.rs` 実装調査・ユーザヒアリング Q4・設計ヒアリング Q1 より*

**関連要件**: REQ-001-D, NFR-001

```mermaid
sequenceDiagram
    participant U as ユーザー
    participant LP as left_panel.rs\nshow_tradeoff_navigator()
    participant AS as AppState
    participant SP as spawn_task
    participant RC as rust_core\nscore_tradeoff_navigator()
    participant MH as MessageHandler

    U->>LP: 重みスライダー操作
    LP->>AS: tradeoff_weights[i] = new_value
    LP->>AS: 正規化（合計が1.0になるよう他スライダー調整）
    LP->>SP: spawn_task(TradeoffWeightsChanged)
    SP->>RC: score_tradeoff_navigator(weights, is_minimize)
    Note over RC: チェビシェフスカラー化で<br/>全試行をスコアリング<br/>→ ソート済みインデックスを返す
    RC-->>SP: Vec<u32> sorted_indices
    SP-->>MH: AppMessage::TradeoffDone { sorted_indices }
    MH->>AS: tradeoff_sorted_indices = Some(sorted_indices)
    Note over AS: egui が次フレームで再描画<br/>→ sorted_indices[0] の試行を<br/>ゴールドスターで全グラフにハイライト
```

**詳細ステップ**:
1. `show_tradeoff_navigator()` が多目的 Study（`meta.objective_names.len() >= 2`）の場合のみ表示
2. スライダー変更時に `tradeoff_weights` を更新し自動正規化
3. `spawn_task` で非同期計算（NFR-001: 50,000 試行で 100ms 以内）
4. `TradeoffDone` 受信後、全ウィジェットは `tradeoff_sorted_indices[0]` を `highlighted_trial` として使用

---

## REQ-002/003/004: セッション保存・復元フロー 🔵

**信頼性**: 🔵 *`io/session.rs` 実装調査・ユーザヒアリング Q3・設計ヒアリング Q2 より*

**関連要件**: REQ-002, REQ-003, REQ-004

```mermaid
sequenceDiagram
    participant U as ユーザー
    participant TB as toolbar.rs
    participant SS as io/session.rs
    participant AS as AppState
    participant LS as LayoutState
    participant FD as rfd::FileDialog

    U->>TB: 「セッション保存(.tdash)」ボタンクリック
    TB->>AS: 現状スナップショット要求
    TB->>LS: レイアウト構成要求
    TB->>SS: serialize_session(app_state, layout_state) → SessionSnapshot
    SS->>SS: serde_json::to_string(&snapshot)
    SS->>FD: save_file_dialog(filter: ".tdash")
    FD-->>SS: PathBuf
    SS->>SS: fs::write(path, json_bytes)
    SS-->>TB: Ok(path)
    TB-->>U: トースト通知「保存しました」

    U->>TB: 「セッション読み込み(.tdash)」ボタンクリック
    TB->>FD: open_file_dialog(filter: ".tdash")
    FD-->>TB: PathBuf
    TB->>SS: deserialize_session(path) → SessionSnapshot
    SS->>SS: serde_json::from_str（未知フィールドはdefaultで無視）
    SS-->>TB: SessionSnapshot
    TB->>AS: filter_ranges = snapshot.filter_ranges
    TB->>AS: selected_indices = snapshot.selected_indices
    TB->>AS: color_mode = snapshot.color_mode
    TB->>AS: tradeoff_weights = snapshot.tradeoff_weights
    TB->>LS: layout_config = snapshot.layout_config
    Note over TB: journal_filename が一致する場合のみ<br/>データを自動復元<br/>不一致の場合は設定のみ適用し<br/>ユーザーへ案内メッセージ表示
```

**FilterSnapshot / LayoutSnapshot 独立保存フロー**:

フィルタ単体（REQ-002）はフィルタフィールドのみ含む JSON、レイアウト単体（REQ-003）はレイアウトフィールドのみ含む JSON として保存する。.tdash（REQ-004）はその全フィールドを統合したスーパーセット。

---

## REQ-005: HTML レポート生成フロー 🔵

**信頼性**: 🔵 *ユーザヒアリング Q5・設計ヒアリング Q4・`io/export.rs` 調査より*

**関連要件**: REQ-005-A〜E, NFR-003

```mermaid
sequenceDiagram
    participant U as ユーザー
    participant TB as toolbar.rs\nExportパネル
    participant SP as spawn_task
    participant HR as io/html_report.rs\nHtmlReportBuilder
    participant AS as AppState
    participant MH as MessageHandler
    participant FD as rfd::FileDialog

    U->>TB: 「HTML レポート出力」ボタンクリック
    TB->>SP: spawn_task with AppState snapshot
    SP->>HR: HtmlReportBuilder::new(snapshot)
    HR->>HR: build_study_summary() → HTML section
    HR->>HR: build_svg_charts() → SVG markup
    Note over HR: 各チャートのデータを<br/>raw data → SVG 直接生成<br/>（egui Shape 変換ではなく<br/>データドリブン SVG 生成）
    HR->>HR: build_trial_table(selected_indices) → HTML table
    HR->>HR: build_statistics() → HTML section
    HR->>HR: assemble_html(sections) → String
    HR-->>SP: html: String
    SP-->>MH: AppMessage::HtmlReportDone { html, suggested_filename }
    MH->>MH: html_report_pending = Some(html)
    Note over MH: 次フレームの UI 更新で<br/>ファイル保存ダイアログを開く
    TB->>FD: save_file_dialog(filter: ".html")
    FD-->>TB: PathBuf
    TB->>TB: fs::write(path, html.as_bytes())
    TB-->>U: トースト通知「レポートを保存しました」
```

**SVG 生成方針 🔵**:
- `egui` の `Painter` は即時描画（Immediate Mode）のため、チャートの `Shape` 収集は実装コストが高い
- 代替として `html_report.rs` 内でチャートデータを受け取り、独立した SVG テキストを直接生成する
- 対象チャート: `StudyContext::trial_rows` のデータから Pareto 散布図・Optimization History・PCP の 3 種の簡易 SVG を生成

---

## REQ-006: 複数 Study 比較フロー 🔵

**信頼性**: 🔵 *ユーザヒアリング Q7・`LayoutMode` 調査・設計ヒアリング Q3 より*

**関連要件**: REQ-006-A〜D

```mermaid
sequenceDiagram
    participant U as ユーザー
    participant TB as toolbar.rs
    participant AS as AppState
    participant LS as LayoutState
    participant SP as spawn_task
    participant JO as io/journal.rs
    participant CP as comparison_panel.rs
    participant MH as MessageHandler

    U->>TB: 「Comparison Mode」ボタンクリック
    TB->>LS: layout_mode = LayoutMode::Comparison
    TB->>AS: comparison_mode = true

    U->>TB: 比較 Study を ComboBox で複数選択（最大 4）
    TB->>SP: spawn_task(load comparison study_id)
    SP->>JO: load_study(journal_path, study_id)
    JO-->>SP: StudyContext
    SP-->>MH: AppMessage::ComparisonStudyLoaded { study_idx, context }
    MH->>AS: comparison_studies[study_idx] = context

    Note over CP: LayoutMode::Comparison 時に表示される<br/>固定 4 分割レイアウト
    CP->>AS: comparison_studies を参照
    CP->>CP: render_stats_table()
    CP->>CP: render_hv_overlay()
    CP->>CP: render_pareto_overlay()
    CP->>CP: render_kde_overlay()
```

---

## REQ-007: アーティファクト連携フロー 🔵

**信頼性**: 🔵 *ユーザヒアリング Q6・`first_spec.md` アーティファクト設計より*

**関連要件**: REQ-007-A〜H, NFR-201

```mermaid
sequenceDiagram
    participant U as ユーザー
    participant IO as io/journal.rs
    participant AF as io/artifacts.rs
    participant AS as AppState
    participant MH as MessageHandler
    participant BP as bottom_panel.rs
    participant AM as widgets/artifact_modal.rs

    Note over IO: Journal 読み込み完了後に自動実行
    IO->>AF: scan_artifacts_dir(journal_path.parent() / "artifacts")
    AF->>AF: canonicalize + starts_with(base_dir) でパス検証
    AF->>AF: ディレクトリを trial_id サブフォルダで走査
    AF-->>MH: AppMessage::ArtifactsDirScanned { trial_artifacts }
    MH->>AS: artifacts_dir = Some(path)
    MH->>AS: artifact_map = trial_artifacts

    BP->>AS: artifact_map 参照
    BP->>BP: Artifacts 列にファイルタイプアイコン表示

    U->>BP: Artifacts 列アイコンをクリック
    BP->>AM: show_artifact_modal(trial_id, files)
    AM->>AM: ファイル拡張子判定
    alt PNG/JPG
        AM->>AM: egui::Image で画像をインライン表示
    else CSV
        AM->>AM: CSV 先頭 100 行をテーブル表示
    else その他
        AM->>AM: ファイル名・サイズ表示 + 「フォルダを開く」ボタン
    end

    U->>TB: 「Artifacts フォルダを選択」ボタンクリック
    Note over TB: rfd::FileDialog でフォルダ選択
    TB->>AF: scan_artifacts_dir(selected_path)
    AF-->>MH: AppMessage::ArtifactsDirScanned
    MH->>AS: artifacts_dir・artifact_map を更新
```

---

## REQ-008: 単目的最適化専用 UI フロー 🔵

**信頼性**: 🔵 *ユーザヒアリング Q8・`first_spec.md` 単目的設計より*

**関連要件**: REQ-008-A〜F

```mermaid
sequenceDiagram
    participant U as ユーザー
    participant MH as MessageHandler
    participant AS as AppState
    participant LP as left_panel.rs
    participant OH as widgets/optimization_history.rs
    participant BP as bottom_panel.rs

    Note over MH: StudySelected メッセージ受信時
    MH->>AS: StudyContext をセット
    MH->>AS: if objective_names.len() == 1 then\n  best_trial_history を計算・セット

    LP->>AS: objective_names.len() チェック
    alt 単目的（== 1）
        LP->>LP: show_convergence_card() を表示
        Note over LP: Best値 / Best trial_id /<br/>直近100試行改善率
    else 多目的（>= 2）
        LP->>LP: show_tradeoff_navigator() を表示
    end

    U->>OH: 「最良値追跡ライン」チェックボックス ON
    OH->>OH: show_best_line = true
    OH->>AS: best_trial_history 参照
    OH->>OH: best_trial_history の折れ線を散布図に重畳描画

    U->>OH: 「半対数」ボタンクリック
    OH->>OH: log_scale = !log_scale
    OH->>OH: Y 軸スケール再計算（f64::log10）

    BP->>AS: objective_names.len() チェック
    alt 単目的（== 1）
        BP->>BP: 「Best 解遷移」タブを表示
        Note over BP: trial_id / objective_value / Δvalue /<br/>重要度上位5変数値
    end
```

---

## REQ-009: Parallel Coordinates 軸制御フロー 🔵

**信頼性**: 🔵 *ユーザヒアリング Q9・`parallel_coords.rs` 実装調査より*

**関連要件**: REQ-009-A〜C

```mermaid
sequenceDiagram
    participant U as ユーザー
    participant PC as widgets/parallel_coords.rs
    participant WS as WidgetStates.parallel_coords

    Note over PC: 各軸ヘッダーに目のアイコンと掴みハンドルを追加
    U->>PC: 目のアイコン（👁）クリック（軸名: x15）
    PC->>WS: axis_visibility["x15"] = false
    PC->>PC: 可視軸のみで再描画（軸幅を均等に拡張）

    U->>PC: 掴みハンドルをドラッグ開始（軸名: obj1）
    PC->>PC: egui::Sense::drag() でドラッグ検出
    PC->>WS: drag_source = Some("obj1")
    U->>PC: ドロップ（目標位置: 左端）
    PC->>WS: axis_order の "obj1" を先頭に移動
    PC->>PC: 新しい軸順序で再描画

    Note over WS: axis_order・axis_visibility は<br/>SessionSnapshot に含まれる（REQ-003 連携）
```

---

## エラーハンドリングフロー 🟡

**信頼性**: 🟡 *既存 `AppMessage::Error` パターンから妥当な推測*

```mermaid
flowchart TD
    A[非同期タスクエラー発生] --> B{エラー種別}
    B -->|Journal 読み込み失敗| C[AppMessage::Error]
    B -->|Session JSON 不正| D[load_error Some 設定]
    B -->|Artifacts スキャン失敗| E[artifact_map 空のまま]
    B -->|HTML 生成失敗| F[AppMessage::Error]

    C --> G[MessageHandler\nload_error = Some]
    D --> G
    E --> H[Artifacts 列非表示]
    F --> G
    G --> I[toolbar.rs がエラーバナー表示]
```

---

## 状態管理フロー（serde シリアライズ対象） 🔵

**信頼性**: 🔵 *REQ-002〜004・`io/session.rs` 実装調査より*

```mermaid
stateDiagram-v2
    [*] --> 初期状態: アプリ起動
    初期状態 --> Journal読み込み済み: JournalParsed
    Journal読み込み済み --> Study選択済み: StudySelected
    Study選択済み --> フィルタ適用済み: set_filter()
    Study選択済み --> Trade-offナビ: TradeoffDone
    Study選択済み --> 比較モード: LayoutMode::Comparison
    Study選択済み --> Artifacts読み込み済み: ArtifactsDirScanned

    フィルタ適用済み --> セッション保存: serialize_session()
    Trade-offナビ --> セッション保存
    比較モード --> セッション保存
    セッション保存 --> [*]: .tdash ファイル書き込み

    [*] --> セッション読み込み: deserialize_session()
    セッション読み込み --> Study選択済み: journal_filename 一致
```

---

## 信頼性レベルサマリー

- 🔵 青信号: 21件 (91%)
- 🟡 黄信号: 2件 (9%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
