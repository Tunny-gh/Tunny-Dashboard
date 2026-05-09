# Widget Help 要件定義書

## 概要

Tunny Dashboard の全ウィジェットにヘルプ機能を追加する。セルツールバーの「?」ボタンからモーダルウィンドウを開き、Theory フォルダに準拠した手法の概要・選び方ガイド・各手法の詳細をタブ切替で表示する。Theory フォルダを `theory/ja/` と `theory/en/` に再構成し、ヘルプ表示は英語とする。

## 関連文書

- **ヒアリング記録**: [interview-record.md](interview-record.md)
- **ユーザストーリー**: [user-stories.md](user-stories.md)
- **受け入れ基準**: [acceptance-criteria.md](acceptance-criteria.md)
- **コンテキストノート**: [note.md](note.md)
- **準備タスク**: [prep.md](prep.md)

## 機能要件（EARS記法）

**【信頼性レベル凡例】**:
- 🔵 **青信号**: PRD・EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な要件
- 🟡 **黄信号**: PRD・EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による要件
- 🔴 **赤信号**: PRD・EARS要件定義書・設計文書・ユーザヒアリングにない推測による要件

### ヘルプボタン

- REQ-001: システムは、各グリッドセルのツールバーにヘルプボタン（「?」アイコン）を表示しなければならない 🔵 *ユーザヒアリング: セルツールバー配置より*
- REQ-002: ヘルプボタンは、移動ボタンと閉じるボタンの間、タイトルの右隣に配置しなければならない 🔵 *ユーザヒアリング: セルツールバー配置より*
- REQ-003: ヘルプボタンをクリックした場合、システムはウィジェットに対応するヘルプモーダルを開かなければならない 🔵 *ユーザヒアリング: モーダルウィンドウより*

### ヘルプモーダル

- REQ-004: ヘルプモーダルは、ウィジェットのタイトルをヘッダーに表示しなければならない 🔵 *ユーザヒアリング: モーダルウィンドウより*
- REQ-005: ヘルプモーダルは、ユーザが閉じるボタンまたは Esc キーで閉じられるようにしなければならない 🔵 *ユーザヒアリング: モーダルウィンドウより*
- REQ-006: ヘルプモーダルは、複数のタブを持ち、タブ切替で異なる情報レベルを表示しなければならない 🔵 *ユーザヒアリング: モーダル内タブ切替より*
- REQ-007: ヘルプモーダルのサイズは、内容が読みやすい十分な幅と高さを確保しなければならない 🟡 *既存 artifact_modal パターンから妥当な推測*

### タブ構成 — Theory情報ありウィジェット

- REQ-010: Theory 情報を持つウィジェット（ImportanceChart, SensitivityHeatmap, PdpChart, PdpChart2D, McdmRankChart, McdmScatterChart, McdmTable, AhpRankChart, AhpTable, ClusterScatter）の場合、第1タブに「概要」を表示しなければならない 🔵 *ユーザヒアリング: 概要＋選び方ガイドより*
- REQ-011: 概要タブには、該当カテゴリの手法一覧表（手法名・特徴・値域・目安コスト）を表示しなければならない 🔵 *Theory README.md の構造より*
- REQ-012: 概要タブには、手法の選び方フローチャートまたは決定木をテキスト形式で表示しなければならない 🔵 *Theory README.md の選び方セクションより*
- REQ-013: 第2タブ以降に各手法の詳細情報を表示しなければならない 🔵 *ユーザヒアリング: モーダル内タブ切替より*
- REQ-014: 各手法の詳細タブには、手法の概要・特徴・注意点・使い方を表示しなければならない 🔵 *Theory 個別 md ファイルの構造より*
- REQ-015: 各手法の詳細タブには、数式をプレーンテキスト表現で表示しなければならない 🔵 *ユーザヒアリング: プレーンテキスト表現より*

### タブ構成 — Theory情報なしウィジェット

- REQ-020: Theory 情報を持たないウィジェット（ParetoScatter2D, ParetoScatter3D, ParallelCoordinates, ScatterMatrix, OptimizationHistory, HvHistory, SliceChart, TrialTable）の場合、単一タブで使い方ガイドを表示しなければならない 🔵 *ユーザヒアリング: 使い方ガイドより*
- REQ-021: 使い方ガイドには、チャートの概要・操作方法・読み方を表示しなければならない 🔵 *ユーザヒアリング: 使い方ガイドより*

### ウィジェットとTheoryの対応

- REQ-030: ImportanceChart のヘルプには、感度分析手法群（Spearman, Ridge, MDI, RF-ANOVA, Permutation, SHAP, Sobol）の情報を表示しなければならない 🔵 *Theory/sensitivity-analysis/README.md より*
- REQ-031: SensitivityHeatmap のヘルプには、感度分析手法群のペアワイズ感度情報を表示しなければならない 🔵 *Theory/sensitivity-analysis/README.md より*
- REQ-032: PdpChart のヘルプには、PDP（1D）とサロゲートモデル（Ridge, RF, Kriging, Sparse Kriging）の情報を表示しなければならない 🔵 *Theory/sensitivity-analysis/pdp.md, Theory/surrogate-models/ より*
- REQ-033: PdpChart2D のヘルプには、PDP（2D）とサロゲートモデルの情報を表示しなければならない 🔵 *Theory/sensitivity-analysis/pdp.md, Theory/surrogate-models/ より*
- REQ-034: McdmRankChart, McdmScatterChart, McdmTable のヘルプには、MCDM 手法群（TOPSIS, VIKOR, PROMETHEE）とエントロピー重み法の情報を表示しなければならない 🔵 *Theory/mcdm/README.md より*
- REQ-035: AhpRankChart, AhpTable のヘルプには、AHP 手法の情報を表示しなければならない 🔵 *Theory/mcdm/ahp.md より*
- REQ-036: ClusterScatter のヘルプには、クラスタリング手法（k-means, エルボー法）と初期化戦略の情報を表示しなければならない 🔵 *Theory/clustering/README.md より*

### Theory フォルダ再構成

- REQ-040: 現在の theory/ 配下のファイルを theory/ja/ に移動しなければならない 🔵 *ユーザヒアリング: ja移動＋en新規作成より*
- REQ-041: theory/en/ に英語版 Theory コンテンツを作成しなければならない 🔵 *ユーザヒアリング: ja移動＋en新規作成より*
- REQ-042: theory/README.md を theory/ja/README.md に移動し、新たに theory/README.md に言語別のリンクを記載しなければならない 🟡 *フォルダ構成から妥当な推測*
- REQ-043: 英語版コンテンツは日本語版と同じ構造（概要表・選び方・詳細）を維持しなければならない 🟡 *コンテンツ一貫性から妥当な推測*

### ヘルプコンテンツの埋め込み

- REQ-050: ヘルプコンテンツはコンパイル時にバイナリに埋め込まれ、実行時に外部ファイルを読み込まないようにしなければならない 🟡 *egui ネイティブアプリの制約から妥当な推測*
- REQ-051: ヘルプコンテンツは `include_str!` マクロまたは同等の手法で theory/en/ のファイルから取り込むべきである 🟡 *Rust 慣行から妥当な推測*

## 非機能要件

### パフォーマンス

- NFR-001: ヘルプモーダルの表示は、ボタンクリックから 100ms 以内に完了しなければならない 🟡 *UX要件から妥当な推測*
- NFR-002: ヘルプコンテンツはコンパイル時に埋め込まれるため、実行時のファイル I/O を発生させてはならない 🔵 *Rust include_str! の特性より*

### ユーザビリティ

- NFR-010: ヘルプボタンは視覚的に区別可能で、誤操作を防ぐサイズでなければならない 🔵 *UI設計原則より*
- NFR-011: ヘルプモーダルはスクロール可能で、長いコンテンツでも全内容が閲覧可能でなければならない 🔵 *egui ScrollArea の特性より*
- NFR-012: ヘルプモーダルはメインウィンドウの中央に表示され、適切な初期サイズを持たなければならない 🟡 *既存 artifact_modal パターンから妥当な推測*
- NFR-013: ヘルプモーダルを開いている間も、背景の UI は表示されたままでなければならない 🟡 *egui Window のモーダル挙動から妥当な推測*

### 保守性

- NFR-020: ヘルプコンテンツの追加・修正が Theory フォルダのファイル編集のみで完結するようにしなければならない 🟡 *保守性要件から妥当な推測*
- NFR-021: 新しいウィジェット追加時にヘルプを簡単に追加できる仕組みを提供しなければならない 🟡 *拡張性要件から妥当な推測*

## Edgeケース

### エラー処理

- EDGE-001: ヘルプコンテンツが未定義のウィジェットの場合、「Help content not available」のプレースホルダを表示しなければならない 🟡 *既存実装パターンから妥当な推測*
- EDGE-002: ヘルプモーダルを複数同時に開くことはできず、既に開いている場合はフォーカスを移すべきである 🟡 *egui Window の挙動から妥当な推測*

### 境界値

- EDGE-010: ウィンドウサイズが非常に小さい場合でも、ヘルプモーダルは画面内に収まるようにしなければならない 🟡 *egui レスポンシブ挙動から妥当な推測*
- EDGE-011: 非常に長いヘルプコンテンツ（数式・表多数）でもスクロールで全内容が閲覧可能でなければならない 🔵 *egui ScrollArea の仕様より*
