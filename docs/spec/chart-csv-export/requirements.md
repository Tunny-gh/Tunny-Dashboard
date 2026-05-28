# chart-csv-export 要件定義書

## 概要

Tunny Dashboard の各チャートセルに「Save as CSV」ボタンを追加し、チャートが表示しているデータをCSV形式でファイル保存できるようにする。後処理・分析の利便性向上が目的。ボタンは既存の⋯メニュー（Save as PNG / Help）と同じツールバーに追加する。

## 関連文書

- **ヒアリング記録**: [💬 interview-record.md](interview-record.md)
- **ユーザストーリー**: [📖 user-stories.md](user-stories.md)
- **受け入れ基準**: [✅ acceptance-criteria.md](acceptance-criteria.md)
- **コンテキストノート**: [📝 note.md](note.md)

---

## 機能要件（EARS記法）

**【信頼性レベル凡例】**:
- 🔵 **青信号**: PRD・設計文書・ユーザヒアリングを参考にした確実な要件
- 🟡 **黄信号**: 設計文書・ユーザヒアリングから妥当な推測による要件
- 🔴 **赤信号**: PRD・設計文書・ユーザヒアリングにない推測による要件

---

### REQ-001: ボタン配置

- REQ-001: 各チャートセルのツールバー ⋯ メニューに「Save as CSV」ボタンを追加しなければならない 🔵 *ユーザヒアリング Q1 より*
- REQ-002: 「Save as CSV」ボタンは既存の「Save as PNG」「Help」と同じ⋯ポップアップメニュー内に並べなければならない 🔵 *ユーザヒアリング Q1 より*
- REQ-003: `CellToolbarAction` enum に `SaveAsCsv(PanelItem)` バリアントを追加しなければならない 🔵 *既存コード（`grid_canvas.rs`）の設計パターンより*

---

### REQ-010: CSV内容（チャート固有データ）

- REQ-010: 「Save as CSV」クリック時、チャートが現在表示しているデータをCSV形式で出力しなければならない 🔵 *ユーザヒアリング Q2 より*
- REQ-011: ファイル保存ダイアログ（rfd）を開き、ユーザーが保存先を選択できなければならない 🔵 *既存 `save_csv_to_file()` の実装より*
- REQ-012: ダイアログのデフォルトファイル名は `{chart_id_snake_case}.csv` 形式でなければならない（例: `optimization_history.csv`） 🟡 *利便性からの妥当な推測*
- REQ-013: CSV の先頭行はヘッダー行（英語スネークケース）でなければならない 🟡 *既存 `build_csv_string()` の慣例より*

---

### REQ-020: チャート別 CSV 出力仕様

#### OptimizationHistory

- REQ-020: `OptimizationHistory` チャートは、現在選択されている目的関数の試行履歴データを出力しなければならない 🔵 *ユーザヒアリング Q2 + ウィジェット実装より*
  - 列: `trial_index`, `objective_value`, `best_value`（累積ベスト）
  - `show_moving_avg=true` のとき: `moving_avg_value` 列を追加してもよい 🟡

#### HvHistory

- REQ-021: `HvHistory` チャートは、ハイパーボリューム履歴を出力しなければならない 🔵 *HvHistoryChart の実装より*
  - 列: `trial_index`, `hypervolume`

#### ImportanceChart

- REQ-022: `ImportanceChart` チャートは、現在表示されている重要度スコアを出力しなければならない 🔵 *ImportanceChart の実装より*
  - 列: `variable`, `importance_score`, `method`（permutation/sobol/etc.）

#### PdpChart

- REQ-023: `PdpChart` チャートは、PDPの予測値データを出力しなければならない 🔵 *PdpChart の実装より*
  - 列: `variable`, `variable_value`, `predicted_objective`

#### PdpChart2D

- REQ-024: `PdpChart2D` チャートは、2次元PDPの予測値データを出力しなければならない 🟡 *既存PDPチャートから妥当な推測*
  - 列: `param1`, `param1_value`, `param2`, `param2_value`, `predicted_objective`

#### ParallelCoordinates

- REQ-025: `ParallelCoordinates` チャートは、表示されている全試行データを出力しなければならない 🔵 *ユーザヒアリング Q2 + ウィジェット実装より*
  - 列: `trial_id`, `trial_number`, `{param_names...}`, `{objective_names...}`

#### ScatterMatrix

- REQ-026: `ScatterMatrix` チャートは、表示されている全試行データを出力しなければならない 🔵 *ウィジェット実装より*
  - 列: `trial_id`, `trial_number`, `{param_names...}`, `{objective_names...}`

#### SensitivityHeatmap

- REQ-027: `SensitivityHeatmap` チャートは、感度指数行列を出力しなければならない 🔵 *SensitivityHeatmap の実装より*
  - 列: `variable`, `{objective_names...}` (各セルが感度指数値)

#### ClusterScatter

- REQ-028: `ClusterScatter` チャートは、クラスタID付き試行データを出力しなければならない 🔵 *ClusterScatter の実装より*
  - 列: `trial_id`, `trial_number`, `{param_names...}`, `{objective_names...}`, `cluster_id`

#### ParetoScatter2D

- REQ-029: `ParetoScatter2D` チャートは、パレートフロントの試行データを出力しなければならない 🔵 *pareto_indices の実装より*
  - 列: `trial_id`, `trial_number`, `{objective_names...}`, `pareto_rank`

#### ParetoScatter3D

- REQ-030: `ParetoScatter3D` チャートは、パレートフロントの試行データを出力しなければならない 🔵 *REQ-029 と同様*
  - 列: `trial_id`, `trial_number`, `{objective_names...}`, `pareto_rank`

#### McdmRankChart / McdmTable / McdmScatterChart

- REQ-031: `McdmRankChart` チャートは、MCDMランキング結果を出力しなければならない 🔵 *mcdm_result の構造より*
  - 列: `trial_id`, `rank`, `score`
- REQ-032: `McdmTable` チャートは、MCDM詳細テーブルデータを出力しなければならない 🔵 *McdmTable の実装より*
  - 列: `trial_id`, `{mcdm_columns...}`, `rank`
- REQ-033: `McdmScatterChart` チャートは、MCDMスキャッタプロットデータを出力しなければならない 🟡 *McdmScatterChart の実装から妥当な推測*
  - 列: `trial_id`, `x_score`, `y_score`, `rank`

#### AhpRankChart / AhpTable

- REQ-034: `AhpRankChart` チャートは、AHPランキング結果を出力しなければならない 🔵 *ahp_result の構造より*
  - 列: `trial_id`, `rank`, `ahp_score`
- REQ-035: `AhpTable` チャートは、AHP詳細テーブルデータを出力しなければならない 🔵 *AhpChart の実装より*
  - 列: `trial_id`, `{ahp_columns...}`, `rank`

#### SliceChart

- REQ-036: `SliceChart` チャートは、スライスチャートの予測値データを出力しなければならない 🔵 *SliceChart の実装より*
  - 列: `variable`, `variable_value`, `{objective_names...}`

#### SurfacePlot（スキップ）

- REQ-103: `SurfacePlot` チャートは、CSV出力の対象外とする 🔵 *ユーザヒアリング Q3 より（3Dグリッドデータのため）*
  - ⋯メニューに「Save as CSV」を表示しない、または常にグレーアウト

---

### REQ-200: データ未準備時の挙動

- REQ-201: チャートのデータが未計算・空の場合、「Save as CSV」ボタンをグレーアウトしなければならない 🔵 *ユーザヒアリング Q4 より*
- REQ-202: グレーアウトされたボタンにホバーした場合、「No data available」ツールチップを表示しなければならない 🔵 *ユーザヒアリング Q4 より*
- REQ-203: ファイルダイアログがキャンセルされた場合、エラーなく正常終了しなければならない 🔵 *既存 `save_csv_to_file()` の実装より*
- REQ-204: ファイル書き込みに失敗した場合、エラーメッセージを表示しなければならない 🟡 *既存エラーハンドリングパターンから妥当な推測*

---

## 非機能要件

### パフォーマンス

- NFR-001: CSV文字列の生成は、10,000件程度の試行データに対して1秒以内に完了しなければならない 🟡 *既存 `build_csv_string()` の性能から妥当な推測*
- NFR-002: ファイルダイアログが開くまでUIがブロックされてはならない 🟡 *デスクトップアプリUXの妥当な要件*

### ユーザビリティ

- NFR-101: 「Save as CSV」の文言は英語で統一し、既存の「Save as PNG」「Help」と一貫したスタイルにしなければならない 🔵 *既存UIより*
- NFR-102: CSVファイルはUTF-8エンコードでなければならない 🔵 *Rust の標準 String/&str はUTF-8のため*

---

## Edgeケース

- EDGE-001: データが0件のチャートでCSVダウンロードした場合、ヘッダー行のみのCSVを出力しなければならない 🟡 *既存 `build_csv_string()` の挙動から妥当な推測*
- EDGE-002: パラメータ名・目的関数名にカンマが含まれる場合、CSVエスケープを正しく処理しなければならない 🟡 *CSV仕様からの妥当な推測*
- EDGE-003: StudyがNone（未読み込み）状態のセルでは「Save as CSV」ボタンは表示しない、またはグレーアウトしなければならない 🔵 *既存コードの `is_none()` チェックパターンより*
