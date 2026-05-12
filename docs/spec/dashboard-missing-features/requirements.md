# ダッシュボード不足機能 要件定義書

## 概要

Tunny Dashboard（Rust/egui製デスクトップアプリ）において、仕様書・設計文書には定義されているが未実装または実装が不完全な機能を洗い出し、要件を明確化する。対象機能は8つに絞り、ユーザーヒアリングにより全て「高優先」と確認された。

- **入力**: Optuna Journal形式（.log）
- **対象規模**: 最大4目的関数 / 30変数 / 50,000サンプル
- **技術スタック**: Rust + egui (eframe)

## 関連文書

- **ヒアリング記録**: [💬 interview-record.md](interview-record.md)
- **ユーザストーリー**: [📖 user-stories.md](user-stories.md)
- **受け入れ基準**: [✅ acceptance-criteria.md](acceptance-criteria.md)
- **コンテキストノート**: [📝 note.md](note.md)
- **元要件定義書**: [tunny-dashboard-requirements.md](../tunny-dashboard-requirements.md)
- **初期設計仕様書**: [first_spec.md](../first_spec.md)

---

## 機能要件（EARS記法）

**【信頼性レベル凡例】**:
- 🔵 **青信号**: PRD・EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な要件
- 🟡 **黄信号**: PRD・EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による要件
- 🔴 **赤信号**: PRD・EARS要件定義書・設計文書・ユーザヒアリングにない推測による要件

---

### F-001: CSVエクスポート UI

#### 通常要件

- REQ-001-A: システムはツールバーに「Export CSV」ボタンを表示しなければならない 🔵 *tunny-dashboard-requirements.md REQ-150・ユーザヒアリング2026-05-12より*
- REQ-001-B: ユーザーが「Export CSV」ボタンをクリックした場合、システムはエクスポート対象選択ダイアログ（全データ / 選択データのみ / Pareto解のみ）を表示しなければならない 🔵 *REQ-150・io/export.rs ExportTarget enumより*
- REQ-001-C: 対象選択後、システムはOS標準のファイル保存ダイアログ（`rfd::FileDialog::save_file()`）を開かなければならない 🔵 *デスクトップアプリ設計・既存rfd使用パターンより*
- REQ-001-D: 保存CSVには trial_id、全パラメータ値、全目的関数値、pareto_rank、cluster_id を列として含めなければならない 🔵 *REQ-151より*

#### 制約要件

- REQ-001-E: `ToolbarAction` enumに `ExportCsv(ExportTarget)` バリアントを追加しなければならない 🔵 *io/export.rsの既存設計より*
- REQ-001-F: Studyが未選択の場合、「Export CSV」ボタンは無効状態（disabled）で表示しなければならない 🟡 *既存toolbar実装パターンから妥当な推測*

---

### F-002: Comparison Study 追加 UI

#### 通常要件

- REQ-002-A: システムはツールバーに「Add Comparison Study」ボタンを表示しなければならない 🔵 *comparison_panel.rs "Add comparison studies via toolbar"メッセージ・ユーザヒアリングより*
- REQ-002-B: ユーザーが「Add Comparison Study」ボタンをクリックした場合、システムはファイル選択ダイアログを開き、選択されたJournalを別スレッドでパースしなければならない 🔵 *既存OpenJournalアクションパターン・app_state.comparison_studiesより*
- REQ-002-C: パース完了後、システムは新しい `StudyContext` を `app_state.comparison_studies` に追加し、レイアウトモードを「Comparison」に自動切り替えしなければならない 🟡 *message_handler.rs L156の既存コードから妥当な推測*
- REQ-002-D: Comparison モードの LayoutMode が選択されている場合、システムは比較Study一覧をツールバーに表示しなければならない 🟡 *設計文書から妥当な推測*
- REQ-002-E: 比較Study一覧の各Studyには削除ボタン（×）を表示し、クリックで該当Studyを `comparison_studies` から除去しなければならない 🟡 *妥当な操作UXから推測*

#### 制約要件

- REQ-002-F: `ToolbarAction` enumに `AddComparisonStudy` バリアントを追加しなければならない 🔵 *既存ToolbarAction設計より*
- REQ-002-G: メインStudyを削除・変更した場合、システムは `comparison_studies` をリセットしなければならない 🟡 *データ整合性から妥当な推測*

---

### F-003: ピン留め（ブックマーク）UI

#### 通常要件

- REQ-003-A: Trial Table の各行にピン留めトグルボタン（📌）を表示しなければならない 🔵 *REQ-156・ユーザヒアリング2026-05-12・session.rs pinned_trialsより*
- REQ-003-B: ピン留め済みの試行は Trial Table で視覚的に区別（背景色・アイコン）されなければならない 🔵 *REQ-156より*
- REQ-003-C: システムはピン留め試行をセッション保存に含めなければならない（`session.rs` の `pinned_trials` フィールドを利用） 🔵 *session.rs SessionSnapshot.pinned_trialsより*

#### 状態要件

- REQ-003-D: ピン留め試行数が20件を超えた場合、システムは追加を拒否し「ピン留めは最大20件です」と通知しなければならない 🔵 *REQ-156より*
- REQ-003-E: ピン留め済みの試行は全チャートで常に描画されなければならない（フィルターで除外されても表示を維持） 🟡 *ピン留め機能の目的から妥当な推測*

#### 制約要件

- REQ-003-F: ピン留め状態は `AppState` に `pinned_trials: Vec<u32>` として保持しなければならない 🔵 *session.rs既存設計より*

---

### F-004: PDP 観測データオーバーレイ

#### 通常要件

- REQ-004-A: PDP Chart (1D) において、実際の試行点（x: パラメータ値, y: 目的関数値）を散布図としてPDP曲線に重ねて表示するオプションを提供しなければならない 🔵 *docs/design/pdp-observed-overlay/・ユーザヒアリング2026-05-12より*
- REQ-004-B: オーバーレイ表示はチャートヘッダーのトグルボタン（「Show Observed」）で ON/OFF 切り替えできなければならない 🟡 *既存チャートヘッダーパターンから妥当な推測*
- REQ-004-C: オーバーレイ点は選択中の試行のみを表示し、`selected_indices` のフィルター状態と連動しなければならない 🔵 *pdp-observed-overlay設計文書より*

#### 条件付き要件

- REQ-004-D: フィルター変更時、システムはオーバーレイ点を即座に更新しなければならない（再計算不要） 🔵 *Brushing & Linking設計より*

#### 制約要件

- REQ-004-E: `PdpResult1d` 構造体に `observed_x: Vec<f64>` / `observed_y: Vec<f64>` フィールドを追加しなければならない（または描画時に `trial_rows` から直接取得） 🟡 *messages.rs PdpResult1d構造体から妥当な推測*

---

### F-005: Surface Plot ウィジェット

#### 通常要件

- REQ-005-A: システムに新しいウィジェット「Surface Plot」（`ChartId::SurfacePlot`）を追加しなければならない 🔵 *docs/design/lightgbm-surface-plot/・ユーザヒアリング2026-05-12より*
- REQ-005-B: Surface Plot は2変数と1目的関数を選択し、Ridge/Random Forest サロゲートモデルの予測値を3次元（x・y・z=目的値）で可視化しなければならない 🔵 *lightgbm-surface-plot設計文書より*
- REQ-005-C: Surface Plot チャートヘッダーにx軸パラメータ・y軸パラメータ・目的関数の選択UIを表示しなければならない 🟡 *既存PdpChart2Dの軸選択UIパターンから推測*
- REQ-005-D: 描画にはwgpuを用いたカスタムレンダラーまたはegui_plotの対応機能を使用しなければならない 🟡 *egui_plotの機能制約から推測*

#### 制約要件

- REQ-005-E: Surface Plot 計算は別スレッドで非同期実行し、計算中はスピナーを表示しなければならない 🔵 *既存非同期チャートパターン（ClusterScatter等）より*
- REQ-005-F: `ChartId::SurfacePlot` を `layout_state.rs` の `ChartId::all()` に追加し、右パネルの「Variable Analysis」グループに含めなければならない 🔵 *既存ChartId追加パターンより*

---

### F-006: Brushing & Linking（矩形選択・PCP軸ブラッシング）

#### 通常要件

- REQ-006-A: Pareto Scatter 2D チャートにおいて、マウスドラッグによる矩形範囲選択（Brush Selection）を実装しなければならない 🔵 *tunny-dashboard-requirements.md REQ-041・ユーザヒアリング2026-05-12より*
- REQ-006-B: 矩形選択後、システムは選択範囲内の試行を `app_state.selected_indices` に設定し、全チャートを連動更新しなければならない 🔵 *REQ-040・REQ-042より*
- REQ-006-C: Parallel Coordinates チャートにおいて、各軸上でのドラッグによる値範囲フィルター（軸ブラッシング）を実装しなければならない 🔵 *REQ-041・ユーザヒアリング2026-05-12より*
- REQ-006-D: PCP 軸ブラッシングは複数軸の AND 条件フィルターとして動作しなければならない 🔵 *REQ-041より*

#### 状態要件

- REQ-006-E: Brush Selection 開始時（ドラッグ開始）、システムは既存の `selected_indices` をクリアしてはならない（Shift+ドラッグで追加選択） 🟡 *標準Brushing UIパターンから妥当な推測*
- REQ-006-F: 選択解除（チャート空白部クリック）時、システムは `selected_indices` を全試行インデックスにリセットしなければならない 🟡 *既存選択リセット動作から妥当な推測*

#### パフォーマンス要件

- REQ-006-G: Brush Selection 後の全チャート更新は 100ms 以内に完了しなければならない（50,000点） 🟡 *REQ-043・既存app_stateキャッシュ設計から推測*

---

### F-007: Comparison Diff モード

#### 通常要件

- REQ-007-A: Comparison パネルに「Diff」タブを追加しなければならない 🔵 *REQ-123・ユーザヒアリング2026-05-12より*
- REQ-007-B: Diff タブは、メインStudyと各比較Studyとの Hypervolume 差分・最良値差分・試行数差分を数値で表示しなければならない 🟡 *REQ-123・REQ-124から妥当な推測*
- REQ-007-C: Diff タブは Study 間 Pareto 支配関係（支配率の集計）を表形式で表示しなければならない 🔵 *REQ-124より*

#### 制約要件

- REQ-007-D: Diff タブは目的数・方向が一致するStudy間のみ利用可能とし、不一致の場合は「目的が異なるStudyとは比較できません」を表示しなければならない 🔵 *REQ-122より*

---

### F-008: チャート PNG 保存（コンテキストメニュー）

#### 通常要件

- REQ-008-A: 各チャートウィジェットのヘッダーに「⋯」（メニュー）ボタンを表示しなければならない 🔵 *ユーザヒアリング2026-05-12・REQ-152より*
- REQ-008-B: メニューには「Save as PNG」項目を含めなければならない 🔵 *REQ-152・ユーザヒアリングより*
- REQ-008-C: 「Save as PNG」選択時、システムは当該チャートを PNG としてOS標準のファイル保存ダイアログで保存しなければならない 🔵 *REQ-152より*
- REQ-008-D: egui フレームワークの制約上、チャートウィジェットの描画内容をテクスチャ経由でキャプチャする実装を選択してよい 🟡 *egui の描画モデルから妥当な推測*

#### 制約要件

- REQ-008-E: メニューボタンはチャートヘッダーの右端に配置しなければならない 🟡 *既存チャートヘッダーパターンから推測*

---

## 非機能要件

### パフォーマンス

- NFR-001: CSV エクスポート（50,000試行）は1秒以内に完了しなければならない 🟡 *REQ-150・Rustネイティブ処理速度から推測*
- NFR-002: Brushing 矩形選択後の全チャート更新は 100ms 以内に完了しなければならない（50,000点） 🟡 *REQ-042・REQ-043から推測*
- NFR-003: Surface Plot 計算（Ridge, 10,000点グリッド）は 3秒以内に完了しなければならない 🟡 *lightgbm-surface-plot設計から推測*

### ユーザビリティ

- NFR-101: 各新機能は既存のegui UIスタイル（TOOLBAR_BTN_FG, TOOLBAR_TEXT, ERROR_COLOR 等のテーマ定数）と一貫したビジュアルで実装しなければならない 🔵 *theme.rs・既存実装パターンより*
- NFR-102: 重い処理（Surface Plot, Comparison Study Load）は別スレッドで実行し、UI がフリーズしてはならない 🔵 *既存非同期パターン（ClusterScatter, ImportanceChart等）より*

### セキュリティ

- NFR-201: ファイル保存時は `rfd::FileDialog` を必ず経由し、任意パスへの書き込みを行ってはならない 🔵 *OWASP・既存io/file.rsパターンより*

---

## Edge ケース

### エラー処理

- EDGE-001: CSV エクスポート先のファイルへの書き込みに失敗した場合、システムはエラーをツールバーに表示しなければならない 🟡 *既存エラー表示パターンから推測*
- EDGE-002: Comparison Study として読み込まれたJournalにStudyが存在しない場合、システムは「Studyが見つかりません」と表示しなければならない 🟡 *既存JournalParsedエラー処理から推測*
- EDGE-003: Surface Plot 計算に必要な試行数が不足している場合（< 10件）、システムは「試行数が不足しています」と表示しなければならない 🟡 *既存PDP/Clustering最低試行数チェックから推測*

### 境界値

- EDGE-101: ピン留め件数が正確に20件の時、21件目のピン留め操作は拒否されなければならない 🔵 *REQ-003-D・REQ-156より*
- EDGE-102: Brushing 矩形が試行を一件も含まない場合、`selected_indices` は空配列になり全チャートは「No Selection」状態を表示しなければならない 🟡 *標準Brushing動作から推測*
