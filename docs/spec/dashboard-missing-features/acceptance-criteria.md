# ダッシュボード不足機能 受け入れ基準

**作成日**: 2026-05-12
**関連要件定義**: [requirements.md](requirements.md)
**関連ユーザストーリー**: [user-stories.md](user-stories.md)
**ヒアリング記録**: [interview-record.md](interview-record.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: PRD・EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な基準
- 🟡 **黄信号**: PRD・EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による基準
- 🔴 **赤信号**: PRD・EARS要件定義書・設計文書・ユーザヒアリングにない推測による基準

---

## F-001: CSV エクスポート UI 🔵

**信頼性**: 🔵 *REQ-150・REQ-151・ユーザヒアリング2026-05-12より*

### Given（前提条件）
- Studyが読み込まれて `app_state.current_study` が `Some` である

### When（実行条件）
- ユーザーがツールバーの「Export CSV」ボタンをクリックする

### Then（期待結果）
- エクスポート対象選択ダイアログが表示される
- 対象選択後にファイル保存ダイアログが開く
- 保存後に成功ログが出力される（エラーなし）

### テストケース

#### 正常系

- [ ] **TC-001-01**: Study 読み込み済みで「全データ」選択して保存 🔵
  - **入力**: Study = 100 trial、ExportTarget::AllData
  - **期待結果**: 101行（ヘッダー + 100行）の CSV が保存される
  - **信頼性**: 🔵 *io/export.rs select_rows_for_export実装より*

- [ ] **TC-001-02**: Brushing で 10 trial 選択して「選択データのみ」 🔵
  - **入力**: selected_indices = 10件、ExportTarget::SelectedOnly
  - **期待結果**: 11行（ヘッダー + 10行）の CSV が保存される
  - **信頼性**: 🔵 *io/export.rs SelectedOnlyフィルターより*

- [ ] **TC-001-03**: 「Pareto解のみ」選択 🔵
  - **入力**: pareto_indices = 5件、ExportTarget::ParetoOnly
  - **期待結果**: 6行（ヘッダー + 5行）の CSV が保存される
  - **信頼性**: 🔵 *io/export.rs ParetoOnlyフィルターより*

- [ ] **TC-001-04**: CSV ヘッダーに全必須列が含まれる 🔵
  - **期待結果**: trial_id, trial_number, 全 param 列, 全 objective 列, pareto_rank, cluster_id の列が存在する
  - **信頼性**: 🔵 *REQ-151・TrialRow構造体より*

#### 異常系

- [ ] **TC-001-E01**: Study 未選択時に「Export CSV」ボタンが無効 🟡
  - **入力**: app_state.current_study = None
  - **期待結果**: ボタンが disabled 状態で表示され、クリックに反応しない
  - **信頼性**: 🟡 *既存toolbar無効化パターンから推測*

- [ ] **TC-001-E02**: ファイル保存キャンセル時にエラーなし 🟡
  - **入力**: ユーザーがファイル保存ダイアログをキャンセル
  - **期待結果**: エラー表示なし、ツールバーも変化なし
  - **信頼性**: 🟡 *rfd::FileDialogのNone返却時パターンから推測*

---

## F-002: Comparison Study 追加 UI 🔵

**信頼性**: 🔵 *comparison_panel.rs・REQ-120・ユーザヒアリング2026-05-12より*

### Given（前提条件）
- メインの Study が読み込まれている

### When（実行条件）
- ユーザーがツールバーの「Add Comparison Study」ボタンをクリックする

### Then（期待結果）
- Journal ファイル選択ダイアログが開く
- 選択した Journal がパースされて `comparison_studies` に追加される
- レイアウトモードが「Comparison」に切り替わる

### テストケース

#### 正常系

- [ ] **TC-002-01**: 有効な Journal ファイルを追加 🔵
  - **入力**: 有効な .log ファイル、1 Study 含む
  - **期待結果**: comparison_studies に 1 件追加、Comparison レイアウトに切り替わる
  - **信頼性**: 🔵 *message_handler.rs L156・LayoutMode::Comparisonより*

- [ ] **TC-002-02**: 複数の Journal を順次追加 🟡
  - **入力**: 2回「Add Comparison Study」→ 2 つの Journal を追加
  - **期待結果**: comparison_studies に 2 件登録される
  - **信頼性**: 🟡 *Vec<StudyContext>の push 動作から推測*

- [ ] **TC-002-03**: 比較 Study の削除（× ボタン） 🟡
  - **入力**: comparison_studies[0] の × ボタンをクリック
  - **期待結果**: comparison_studies から該当 Study が削除される
  - **信頼性**: 🟡 *REQ-002-E・Vec::remove動作から推測*

#### 異常系

- [ ] **TC-002-E01**: Study を含まない Journal の追加 🟡
  - **入力**: 空の Journal ファイル（Study なし）
  - **期待結果**: 「Studyが見つかりません」エラーメッセージ表示
  - **信頼性**: 🟡 *EDGE-002より推測*

- [ ] **TC-002-E02**: ファイル選択キャンセル時にエラーなし 🟡
  - **期待結果**: エラー表示なし、comparison_studies 変化なし
  - **信頼性**: 🟡 *既存OpenJournalキャンセルパターンから推測*

---

## F-003: ピン留め UI 🔵

**信頼性**: 🔵 *REQ-156・session.rs pinned_trials・ユーザヒアリング2026-05-12より*

### Given（前提条件）
- Studyが読み込まれて Trial Table が表示されている

### When（実行条件）
- ユーザーが Trial Table の行の📌ボタンをクリックする

### Then（期待結果）
- 該当 trial が `pinned_trials` に追加される
- 行が視覚的にハイライト（背景色・アイコン変化）される

### テストケース

#### 正常系

- [ ] **TC-003-01**: 試行を1件ピン留め 🔵
  - **入力**: trial_id = 42 の📌ボタンをクリック
  - **期待結果**: app_state.pinned_trials に 42 が追加、行がハイライト表示
  - **信頼性**: 🔵 *session.rs pinned_trialsフィールドより*

- [ ] **TC-003-02**: ピン留め試行はフィルター後も表示 🟡
  - **入力**: trial_id = 42 をピン留め後、フィルターで 42 が除外される条件を設定
  - **期待結果**: trial_id = 42 が Trial Table に残る（グレーアウト等で区別可）
  - **信頼性**: 🟡 *REQ-003-Eから推測*

- [ ] **TC-003-03**: セッション保存・復元でピン留め維持 🔵
  - **入力**: trial_id = 42 をピン留め → Save Session → Load Session
  - **期待結果**: 復元後も trial_id = 42 がピン留め状態
  - **信頼性**: 🔵 *session.rs pinned_trials serialization testより*

- [ ] **TC-003-04**: 📌再クリックでピン留め解除 🟡
  - **入力**: ピン留め済み trial_id = 42 の📌をクリック
  - **期待結果**: pinned_trials から 42 が削除、ハイライト解除
  - **信頼性**: 🟡 *トグルUIパターンから推測*

#### 境界値

- [ ] **TC-003-B01**: 20件ピン留め後に21件目を試みる 🔵
  - **入力**: 20件ピン留め済み → 21件目の📌クリック
  - **期待結果**: 「ピン留めは最大20件です」通知、追加されない
  - **信頼性**: 🔵 *REQ-156・EDGE-101より*

---

## F-004: PDP 観測データオーバーレイ 🔵

**信頼性**: 🔵 *docs/design/pdp-observed-overlay/・ユーザヒアリング2026-05-12より*

### Given（前提条件）
- PDP Chart が描画済み（サロゲートモデル計算完了）

### When（実行条件）
- チャートヘッダーの「Show Observed」トグルを ON にする

### Then（期待結果）
- PDP 曲線上に実際の試行点（x: パラメータ値, y: 目的関数値）が散布図で重ねて表示される

### テストケース

#### 正常系

- [ ] **TC-004-01**: 「Show Observed」ON で観測点が表示される 🔵
  - **入力**: PDP 計算済み、show_observed = true
  - **期待結果**: `trial_rows` の (param_value, objective_value) 点が描画される
  - **信頼性**: 🔵 *pdp-observed-overlay設計文書より*

- [ ] **TC-004-02**: Brushing 後に観測点が連動更新される 🔵
  - **入力**: selected_indices を変更
  - **期待結果**: 表示される観測点が selected_indices に含まれる試行のみになる
  - **信頼性**: 🔵 *REQ-004-C・Brushing設計より*

- [ ] **TC-004-03**: 「Show Observed」OFF で観測点が非表示 🟡
  - **入力**: show_observed = false
  - **期待結果**: PDP 曲線のみ表示、散布点なし
  - **信頼性**: 🟡 *トグルUIパターンから推測*

---

## F-005: Surface Plot ウィジェット 🔵

**信頼性**: 🔵 *docs/design/lightgbm-surface-plot/・ユーザヒアリング2026-05-12より*

### Given（前提条件）
- Studyが読み込まれている（2変数以上、10試行以上）

### When（実行条件）
- Surface Plot ウィジェットで x軸・y軸パラメータ・目的関数を選択して「Run」をクリック

### Then（期待結果）
- サロゲートモデル計算がバックグラウンドで実行される
- 完了後に 3D サーフェス（または等高線ヒートマップ）が描画される

### テストケース

#### 正常系

- [ ] **TC-005-01**: 2変数選択で Surface Plot が表示される 🔵
  - **入力**: param1, param2, objective 選択後 Run
  - **期待結果**: 3D サーフェス描画完了、スピナー非表示
  - **信頼性**: 🔵 *lightgbm-surface-plot設計文書より*

- [ ] **TC-005-02**: ChartId::SurfacePlot が右パネルに表示される 🔵
  - **期待結果**: 右パネル「Variable Analysis」グループに「Surface Plot」が含まれる
  - **信頼性**: 🔵 *REQ-005-F・right_panel.rsグループ設計より*

- [ ] **TC-005-03**: 計算中はスピナーが表示される 🔵
  - **期待結果**: Run クリック後〜完了まで spinner が表示される
  - **信頼性**: 🔵 *既存非同期チャートパターン（ClusterScatter等）より*

#### 異常系

- [ ] **TC-005-E01**: 試行数不足（< 10件）でエラー表示 🟡
  - **入力**: trial_rows.len() < 10
  - **期待結果**: 「試行数が不足しています（最低10件必要）」を表示
  - **信頼性**: 🟡 *EDGE-003から推測*

---

## F-006: Brushing & Linking 🔵

**信頼性**: 🔵 *REQ-041・ユーザヒアリング2026-05-12より*

### Given（前提条件）
- Studyが読み込まれてチャートが描画済み

### When（実行条件）
- Pareto Scatter 2D チャート上でドラッグして矩形選択

### Then（期待結果）
- 矩形内の試行が selected_indices に設定される
- 全チャートが連動更新される（100ms 以内）

### テストケース

#### 正常系

- [ ] **TC-006-01**: 矩形選択で試行が絞り込まれる 🔵
  - **入力**: Pareto Scatter 2D 上でドラッグ選択
  - **期待結果**: selected_indices が更新、チャート連動更新
  - **信頼性**: 🔵 *REQ-006-A・REQ-006-Bより*

- [ ] **TC-006-02**: チャート空白クリックで選択解除 🟡
  - **入力**: 矩形選択後にチャート空白部をクリック
  - **期待結果**: selected_indices が全試行インデックスにリセット
  - **信頼性**: 🟡 *REQ-006-Fから推測*

- [ ] **TC-006-03**: PCP 軸ドラッグで値範囲フィルター 🔵
  - **入力**: Parallel Coordinates の1軸をドラッグして範囲設定
  - **期待結果**: selected_indices が絞り込まれ、全チャート連動
  - **信頼性**: 🔵 *REQ-006-C・REQ-006-Dより*

- [ ] **TC-006-04**: PCP 複数軸の AND フィルター 🔵
  - **入力**: 2軸に範囲設定
  - **期待結果**: 両方の範囲を満たす試行のみが selected_indices に残る
  - **信頼性**: 🔵 *REQ-006-Dより*

#### パフォーマンス

- [ ] **TC-006-P01**: 50,000試行でBrush Selection後の更新が100ms以内 🟡
  - **測定項目**: Brush Selection後の selected_indices 更新 + チャート再描画時間
  - **目標値**: 100ms 以内
  - **信頼性**: 🟡 *REQ-006-G・REQ-042から推測*

---

## F-007: Comparison Diff タブ 🔵

**信頼性**: 🔵 *REQ-123・REQ-124・ユーザヒアリング2026-05-12より*

### テストケース

#### 正常系

- [ ] **TC-007-01**: Comparison パネルに「Diff」タブが表示される 🔵
  - **期待結果**: Diff タブが Stats/HV History/Pareto/KDE の隣に表示される
  - **信頼性**: 🔵 *REQ-007-Aより*

- [ ] **TC-007-02**: Diff タブに差分指標が表示される 🟡
  - **期待結果**: HV 差分・最良値差分・試行数差分が数値で表示される
  - **信頼性**: 🟡 *REQ-007-Bから推測*

- [ ] **TC-007-03**: Diff タブに Pareto 支配率が表示される 🔵
  - **期待結果**: メイン Study が比較 Study を支配する割合（%）が表示される
  - **信頼性**: 🔵 *REQ-007-C・REQ-124より*

#### 異常系

- [ ] **TC-007-E01**: 目的数不一致の場合に Diff タブが非活性 🔵
  - **入力**: comparison_study の目的数が main study と異なる
  - **期待結果**: 「目的が異なるStudyとは比較できません」と表示
  - **信頼性**: 🔵 *REQ-007-D・REQ-122より*

---

## F-008: チャート PNG 保存 🔵

**信頼性**: 🔵 *REQ-152・ユーザヒアリング2026-05-12より*

### テストケース

#### 正常系

- [ ] **TC-008-01**: 各チャートヘッダーに「⋯」ボタンが表示される 🔵
  - **期待結果**: 全 ChartId のチャートヘッダー右端に「⋯」ボタンが存在する
  - **信頼性**: 🔵 *REQ-008-Aより*

- [ ] **TC-008-02**: 「⋯」クリックで「Save as PNG」メニューが表示される 🔵
  - **期待結果**: ポップアップメニューに「Save as PNG」「Help」の項目が表示される
  - **信頼性**: 🔵 *REQ-008-B・ユーザヒアリングより*

- [ ] **TC-008-03**: 「Save as PNG」でファイル保存ダイアログが開く 🔵
  - **期待結果**: rfd::FileDialog::save_file() が呼ばれる
  - **信頼性**: 🔵 *REQ-008-Cより*

---

## テストケースサマリー

### カテゴリ別件数

| カテゴリ | 正常系 | 異常系 | 境界値 | パフォーマンス | 合計 |
|---------|--------|--------|--------|---------|------|
| F-001 CSV Export | 4 | 2 | 0 | 0 | 6 |
| F-002 Comparison UI | 3 | 2 | 0 | 0 | 5 |
| F-003 ピン留め | 4 | 0 | 1 | 0 | 5 |
| F-004 PDP Overlay | 3 | 0 | 0 | 0 | 3 |
| F-005 Surface Plot | 3 | 1 | 0 | 0 | 4 |
| F-006 Brushing | 4 | 0 | 0 | 1 | 5 |
| F-007 Comparison Diff | 3 | 1 | 0 | 0 | 4 |
| F-008 PNG 保存 | 3 | 0 | 0 | 0 | 3 |
| **合計** | **27** | **6** | **1** | **1** | **35** |

### 信頼性レベル分布

- 🔵 青信号: 27件 (77%)
- 🟡 黄信号: 8件 (23%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質（🔵 が 77%）

### 優先度別テストケース

- **Must Have**: 35件（全件）
- **Should Have**: 0件
- **Could Have**: 0件

---

## テスト実施計画

### Phase 1: UI 表示確認テスト（単体）

- TC-001-E01（CSV ボタン無効）, TC-002-01〜03, TC-003-01, TC-005-02, TC-008-01〜02
- 優先度: Must Have
- 前提: 各ウィジェット追加・ToolbarAction 追加完了後

### Phase 2: データ処理テスト

- TC-001-01〜04（CSV 内容確認）, TC-003-03（セッション復元）, TC-006-01〜04（Brushing）
- 優先度: Must Have
- 前提: rust_core レベルの処理が完了後

### Phase 3: パフォーマンステスト

- TC-006-P01（50,000試行での Brushing 応答）
- 前提: 大規模テストデータ（fixtures）が準備完了後
