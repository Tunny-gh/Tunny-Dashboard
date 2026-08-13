# Tunny Dashboard × modeFRONTIER ギャップ分析レポート

日付: 2026-07-04
対象: modeFRONTIER 2025R3（ESTECO）をベンチマークとした機能比較

## TL;DR

Tunny Dashboard は「最適化結果のポスト処理・意思決定支援」という modeFRONTIER の中核領域において、既に高い水準で張り合えている。感度分析（9手法）・MCDM（TOPSIS/VIKOR/PROMETHEE）・サロゲート最適化（EHVI 含む）は mF と同等かそれ以上。一方で明確に欠けているのは以下の5点。

1. 統計・分布系の基本チャート（ヒストグラム、箱ひげ図、相関行列）
2. ロバスト性/信頼性解析
3. SOM（自己組織化マップ）
4. セッション（レイアウト）保存
5. Optuna SQLite ストレージ対応

差別化の主軸は「軽量・ライブ監視・Optuna との人間参加型ループ（enqueue JSON）」に置くのが有効。

## 現状の機能サマリ

UI は無限パン/ズームのフリーフォームキャンバス方式で、19種のチャートウィジェット + Trial Table を任意配置できる。

- **可視化**: Pareto Scatter 2D/3D、Parallel Coordinates、Scatter Matrix、Slice Chart、Observed Contour（Delaunay 補間・外挿なし）、PDP 1D/2D、Cluster Scatter 2D/3D、MCDM Ranking/Scatter、Optimization History、Convergence Indicators、Artifact Gallery
- **分析** (`rust_core`): k-means クラスタリング（エルボー法・PCA 付き）、9手法の重要度計算（Spearman / Ridge / RF-ANOVA / MDI / Sobol / SHAP / Permutation / ARD）、TOPSIS / VIKOR / PROMETHEE / Entropy による MCDM、Hypervolume / IGD+ / ε-indicator / R2 の収束指標
- **サロゲート最適化** (`surrogate_opt`): Ridge / GP-FITC / GP-VFE / GP-MoE / LightGBM のモデル自動選択（CV R²）、L-BFGS / NSGA-II / CMA-ES、EI / LCB / EHVI 獲得関数、制約対応、CV 検証レポート、Optuna `enqueue_trial` 形式の JSON コピー
- **入出力**: Optuna Journal（ライブポーリング更新対応）、DesignExplorer 形式 CSV、Artifact フォルダ連携、CSV / PNG エクスポート、複数 Study 比較
- コードベースに TODO / 未実装スタブは見つからず、現機能の完成度は高い

## mF と同等以上の領域（差別化ポイント）

| 領域 | Tunny Dashboard | modeFRONTIER 比 |
|---|---|---|
| 感度分析 | 9手法（SHAP / Sobol / ARD 含む） | **優位**。mF は screening + SS-ANOVA 中心 |
| MCDM | TOPSIS / VIKOR / PROMETHEE / Entropy 重み | 同等以上（mF は wizard 型 MCDM） |
| サロゲート | 5モデル + CV 検証レポート | 同等（mF の RSM ラインナップに相当） |
| サロゲート最適化 | 3手法 + 3獲得関数 + 制約対応 | 同等以上。**enqueue JSON による Optuna への還流は mF にない独自機能** |
| 多目的収束指標 | 4指標 + Study 間比較 | **優位**（mF に同粒度の収束指標比較 UI はない） |
| ライブ監視 | Journal ポーリングによる実行中スタディの差分更新 | **優位**（mF のポスト処理は基本静的） |
| UI | フリーフォームキャンバス + EN/JA ヘルプ同梱 | VOLTA のダッシュボード体験をデスクトップ単体で実現 |

## ギャップ分析（mF にあって Tunny にないもの）

### A. 基本統計・分布可視化 — 最も目立つ欠落

mF は「Multi-dimensional design charts + statistical assessment」を看板に掲げるが、Tunny には**ヒストグラム、箱ひげ図、分布フィッティング、相関行列ヒートマップ**といった基礎統計ウィジェットがない。Pearson 相関の theory ドキュメント（`theory/en/statistics/pearson-correlation.md`）は既にあるのに対応ウィジェットがない状態。19種のチャートを持ちながら単変量分布が見られないのは、ベンチマーク比較で最初に指摘される穴になる。

### B. ロバスト性・信頼性解析

mF の主要セールスポイントの一つ（Monte Carlo、多項式カオス、Six Sigma 的な故障確率評価）。Tunny は GP サロゲートが不確かさ（分散）を既に持っているため、**候補点まわりに入力ノイズを与えたサロゲート上の MC サンプリング → 出力分布・制約充足確率の表示**は比較的低コストで実装でき、Optuna エコシステムには存在しない機能なので差別化にも直結する。既存の `surrogate_opt` の実行可能性確率計算が土台に使える。

### C. SOM（自己組織化マップ）/ PCA バイプロット

mF が 2025 リリースで再設計して推している看板機能。多変数の設計空間を俯瞰する用途で parallel coordinates と補完関係にある。中規模の実装コストだが「mF ユーザーが探す機能」の筆頭。同様に、PCA は現在クラスタリング内部でしか使われていないため、**独立した PCA バイプロットウィジェット**に昇格させる価値がある。

### D. セッション/ダッシュボード保存

VOLTA 2025R3 の目玉は「ダッシュボード構成の再利用（データセットを差し替えても可視化構成を維持）」。Tunny のキャンバスレイアウト・各ウィジェット設定（クラスタ数、MCDM 重み、軸選択など）をプロジェクトファイルとして保存/復元できれば、この体験をデスクトップ単体で先取りできる。HTML レポート出力は削除済みのため、報告用途は「レイアウト保存 + PNG 一括エクスポート」で置き換える設計が自然。

### E. デザイン比較ビュー

mF にはピックした設計案同士を比較する機能（レーダー/スターチャート、並列テーブル）がある。Tunny には trial 詳細モーダルと 📌 ピン留めが既にあるため、**ピン留めした複数 trial のレーダーチャート比較ウィジェット**は小さい追加で意思決定フェーズの体験を大きく改善する。

### F. Optuna SQLite ストレージ対応（mF とは無関係だが最重要級）

現状 Journal と CSV のみ対応で、**Optuna で最も一般的な `sqlite:///` ストレージが読めない**。ベンチマーク上の論点ではないが、ユーザー獲得の観点ではどのギャップよりも影響が大きい可能性がある。

### G. スコープ外と判断してよいもの

ワークフロー自動化・分散実行・ソルバー統合は mF の中核だが、Tunny では Grasshopper + Optuna が担う設計のため追う必要はない。「重い統合プラットフォーム vs 軽量特化ツール」という対比自体が差別化メッセージになる。

## 推奨ロードマップ（優先度順）

| 優先度 | 項目 | コスト | 効果 |
|---|---|---|---|
| 1 | 統計ウィジェット群（ヒストグラム・箱ひげ・相関行列） | 低 | 穴埋め効果大 |
| 2 | SQLite ストレージ対応 | 中 | 採用率への影響が最大 |
| 3 | セッション保存/復元 | 中 | VOLTA 2025 の目玉機能への対抗 |
| 4 | ロバスト性解析ウィジェット | 中 | 既存 GP 基盤を活かした高差別化機能 |
| 5 | ピン留め trial のレーダー比較 | 低 | 意思決定体験の向上 |
| 6 | SOM / PCA バイプロット | 中 | mF ユーザー移行の受け皿 |
| 7 | 階層クラスタリング、サロゲートベース 3D 応答曲面ビューア | 中 | 余力があれば |

## 実装状況（2026-07-04 時点・全項目完了）

| 項目 | 実装 | Theory |
|---|---|---|
| 1. 統計ウィジェット群 | ✅ Histogram / Box Plot / Correlation Matrix（PR #118） | statistics/{histogram, box-plot, correlation-matrix}.md |
| A. 分布フィッティング | ✅ Histogram の Fit セレクタ（Normal/LogNormal/Weibull MLE + AIC） | statistics/distribution-fitting.md |
| 2. SQLite ストレージ | ✅ `.db/.sqlite/.sqlite3` 読み込み（PR #119） | io/optuna-storages.md |
| 3. セッション保存/復元 | ✅ セッション JSON ファイル（PR #120、拡張子は .json） | io/session-files.md |
| 4. ロバスト性解析 | ✅ Robustness ウィジェット（MC ノイズ伝播 + 制約充足率、PR #121） | optimization/robustness-analysis.md, widgets/robustness.md, statistics/box-muller.md |
| 5. レーダー比較 | ✅ Radar Comparison（ピン留め trial、共有レンダラー） | widgets/radar-comparison.md |
| E. 並列比較テーブル | ✅ Comparison Table（方向考慮のベストセル強調） | widgets/comparison-table.md |
| 6. SOM | ✅ SOM Map（バッチ学習・U-matrix・成分プレーン・決定論的） | clustering/som.md |
| 6. PCA バイプロット | ✅ PCA Biplot（標準化 PCA・寄与率・ローディング矢印） | clustering/pca-biplot.md |
| 7. 階層クラスタリング | ✅ Dendrogram（Ward / NN-chain・カット・クラスタ着色） | clustering/hierarchical.md |
| 7. 3D 応答曲面ビューア | ✅ Response Surface 3D（サロゲート断面・アンカー固定） | widgets/response-surface-3d.md |
| D. PNG 一括エクスポート | ⏸ 対象外と判断（2026-07-04 ユーザー判断） | — |
| G. ワークフロー自動化ほか | ⏸ スコープ外（レポートの判断どおり） | — |

すべての実装は theory/{en,ja} の手法/ウィジェットドキュメント・README 両言語・手法の全体マップに登録済み。

## 参考資料

- [VOLTA and modeFRONTIER 2025R3 out now](https://www.esteco.com/news/volta-and-modefrontier-2025r3/)
- [modeFRONTIER capabilities](https://academy.esteco.com/modefrontier/modefrontier-capabilities/)
- [Response surface models | ESTECO](https://engineering.esteco.com/technology/response-surface-models-rsm/)
- [Simulation data analytics | ESTECO](https://engineering.esteco.com/technology/simulation-data-analytics/)
- [modeFRONTIER 2025 new features](https://engineering.esteco.com/events/unlocking-design-innovation-modefrontier-2025-new-features-explained/)
