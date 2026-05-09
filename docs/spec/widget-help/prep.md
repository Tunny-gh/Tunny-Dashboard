# Widget Help 準備タスク（ユーザー作業）

> **仕様**: [requirements.md](requirements.md)
> **生成日**: 2026-05-08

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 要件定義書・設計文書・ユーザヒアリングで明確に必要と判明したタスク
- 🟡 **黄信号**: 要件定義書・設計文書から妥当に推測されるタスク
- 🔴 **赤信号**: 推測による予防的タスク（実装時に不要と判明する可能性あり）

## 必須（実装開始前に完了が必要）

以下のタスクが完了していないと、実装フェーズでブロッカーになります。

- [ ] **Theory フォルダの ja 移動** 🔵 *ユーザヒアリング: ja移動＋en新規作成より*
  - 現在の `theory/` 配下の全ファイルを `theory/ja/` に移動する
  - ディレクトリ構造を維持（sensitivity-analysis/, mcdm/, clustering/, surrogate-models/, optimization/）
  - 移動後、既存のコード・ドキュメント内の theory/ パス参照を更新する
  - 関連要件: REQ-040

- [ ] **Theory 英語版コンテンツ作成** 🔵 *ユーザヒアリング: ja移動＋en新規作成より*
  - `theory/en/` に英語版を作成する。対象ファイル:
    - `README.md` — 全手法マップ・選び方
    - `sensitivity-analysis/README.md` — 感度分析概要・手法比較表
    - `sensitivity-analysis/spearman.md`, `ridge.md`, `sobol.md`, `mdi.md`, `rfanova.md`, `permutation.md`, `shap.md`, `pdp.md` — 個別手法詳細
    - `mcdm/README.md` — MCDM 概要・手法比較
    - `mcdm/topsis.md`, `vikor.md`, `promethee.md`, `entropy-weight.md`, `ahp.md` — 個別手法詳細
    - `clustering/README.md` — クラスタリング概要
    - `clustering/kmeans.md`, `elbow.md` — 個別手法詳細
    - `surrogate-models/ridge.md`, `random-forest.md`, `kriging.md`, `sparse-kriging.md` — サロゲートモデル詳細
    - `optimization/lbfgs.md` — L-BFGS 最適化
  - 構造は ja 版と同一（概要表・選び方・詳細）を維持する
  - 数式はプレーンテキスト表現に変換する
  - 関連要件: REQ-041, REQ-042, REQ-043

## 推奨（実装中に用意できればOK）

実装を開始できますが、該当機能の実装前までに準備してください。

- [ ] **Theory 情報なしウィジェットの使い方ガイド執筆** 🔵 *ユーザヒアリング: 使い方ガイドより*
  - 以下のウィジェットの使い方ガイドコンテンツ（英語）を作成する:
    - ParetoScatter2D — パレートフロント概要、ズーム/パン/ブラシ操作、読み方
    - ParetoScatter3D — 3D 操作（arcball カメラ）、パレートフロント読み方
    - ParallelCoordinates — 軸の意味、ハイライト/フィルタ操作、読み方
    - ScatterMatrix — ペアワイズ散布図の読み方、対角線の意味
    - OptimizationHistory — トライアル推移、ベスト値追跡、移動平均
    - HvHistory — Hypervolume 指標の意味、収束判定
    - SliceChart — スライスチャートの概要、操作方法
    - TrialTable — ソート、行選択、エクスポート操作
  - 標準フォーマット: Overview / Operations / How to Read
  - 必要になるフェーズ: Phase 3
  - 関連要件: REQ-020, REQ-021

## 確認事項（判断が必要）

実装方針に影響するため、早めの判断・確認が推奨されます。

- [ ] **ヘルプコンテンツの埋め込み方式の最終確認** 🟡 *要件定義書 REQ-050, REQ-051 より*
  - 選択肢: `include_str!` で theory/en/ をコンパイル時埋め込み vs 別構造体でハードコード
  - 影響範囲: コンテンツ更新時のビルド要件・バイナリサイズ
  - 関連要件: REQ-050, REQ-051

---

## サマリー

| 優先度 | 件数 | 🔵 | 🟡 | 🔴 |
|--------|------|-----|-----|-----|
| 必須 | 2 | 2 | 0 | 0 |
| 推奨 | 1 | 1 | 0 | 0 |
| 確認事項 | 1 | 0 | 1 | 0 |

## 関連文書

- **要件定義書**: [requirements.md](requirements.md)
- **ヒアリング記録**: [interview-record.md](interview-record.md)
